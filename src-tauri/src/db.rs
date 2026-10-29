use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::epub_meta::read_epub_meta;
use crate::kindle_cloud::{clean_title, decode_html_entities, CloudAnnotation, OwnedBook, ReadingPosition};
use crate::model::{
    Book, Bookmark, BookmarkInput, Highlight, HighlightInput, ProgressInput, SessionStatus, Settings, Snapshot,
    StoredSession,
};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS books (
  id TEXT PRIMARY KEY,
  source TEXT NOT NULL,
  asin TEXT NOT NULL DEFAULT '',
  title TEXT NOT NULL,
  author TEXT NOT NULL,
  file_path TEXT NOT NULL DEFAULT '',
  cover TEXT NOT NULL DEFAULT '',
  percent_read REAL NOT NULL DEFAULT 0,
  chapter_title TEXT NOT NULL DEFAULT '',
  missing INTEGER NOT NULL DEFAULT 0,
  added_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS highlights (
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL,
  source TEXT NOT NULL,
  cfi TEXT NOT NULL DEFAULT '',
  text TEXT NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  color TEXT NOT NULL DEFAULT 'amber',
  start_pos INTEGER NOT NULL DEFAULT 0,
  end_pos INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS bookmarks (
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL,
  cfi TEXT NOT NULL DEFAULT '',
  title TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS kv (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
"#;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let conn = Connection::open(path).map_err(|error| error.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|error| error.to_string())?;
        let db = Self { conn };
        if db.kv("settings")?.is_none() {
            db.save_settings(&Settings::default())?;
        }
        if db.kv("session")?.is_none() {
            db.save_session(&StoredSession {
                region: "br".into(),
                ..StoredSession::default()
            })?;
        }
        if db.kv("folders")?.is_none() {
            let folders = default_folders()
                .into_iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            db.save_folders(&folders)?;
        }
        let _ = db.ensure_default_folders();
        let _ = db.clean_existing_titles();
        Ok(db)
    }

    fn clean_existing_titles(&self) -> Result<(), String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, author FROM books")
            .map_err(|error| error.to_string())?;
        let rows: Vec<(String, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .collect();
        for (id, title, author) in rows {
            let cleaned_title = clean_title(&title);
            let cleaned_author = decode_html_entities(&author);
            if cleaned_title != title || cleaned_author != author {
                let _ = self.conn.execute(
                    "UPDATE books SET title = ?1, author = ?2 WHERE id = ?3",
                    params![cleaned_title, cleaned_author, id],
                );
            }
        }
        let _ = self.dedup_pdocs_against_local();
        Ok(())
    }

    fn dedup_pdocs_against_local(&self) -> Result<(), String> {
        let locals = self.books()?;
        let local_books: Vec<&Book> = locals.iter().filter(|b| b.source == "local" && !b.missing).collect();
        let cloud_pdocs: Vec<&Book> = locals.iter().filter(|b| b.source == "cloud" && b.asin.len() > 12).collect();
        for pdoc in cloud_pdocs {
            if local_books.iter().any(|loc| titles_match_local_and_pdoc(&loc.title, &loc.file_path, &pdoc.title)) {
                let _ = self.conn.execute("DELETE FROM books WHERE id = ?1", [&pdoc.id]);
            }
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Result<Snapshot, String> {
        Ok(Snapshot {
            books: self.books()?,
            highlights: self.highlights()?,
            bookmarks: self.bookmarks()?,
            settings: self.settings()?,
            session: self.session_status()?,
            folders: self.folders()?,
            sync_notice: self.sync_notice(),
        })
    }

    pub fn sync_notice(&self) -> String {
        self.kv("sync_notice").ok().flatten().unwrap_or_default()
    }

    pub fn set_sync_notice(&self, notice: &str) -> Result<(), String> {
        if notice.is_empty() {
            self.conn
                .execute("DELETE FROM kv WHERE key = 'sync_notice'", [])
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        self.put_kv("sync_notice", notice)
    }

    pub fn ensure_default_folders(&self) -> Result<(), String> {
        let mut existing = self.folders().unwrap_or_default();
        let mut changed = false;
        for def in default_folders() {
            let path = def.display().to_string();
            if path.is_empty() || existing.iter().any(|item| item == &path) {
                continue;
            }
            existing.push(path);
            changed = true;
        }
        if changed {
            self.save_folders(&existing)?;
        }
        Ok(())
    }

    pub fn scan(&self) -> Result<bool, String> {
        Ok(self.scan_local()?.changed)
    }

    pub fn scan_local(&self) -> Result<LocalScan, String> {
        let _ = self.ensure_default_folders();
        let folders = self.folders()?;
        let mut found = Vec::new();
        for folder in &folders {
            let path = PathBuf::from(folder);
            if path.is_dir() {
                collect_local_books(&path, &mut found, 0);
            }
        }
        let mut changed = false;
        for path in &found {
            if self.upsert_local(path)? {
                changed = true;
            }
        }
        let existing = self.local_paths()?;
        for (id, file_path) in existing {
            let path = PathBuf::from(&file_path);
            let covered = folders.iter().any(|folder| file_path.starts_with(folder));
            if covered && !found.iter().any(|item| item == &path) && path.is_absolute() {
                self.conn
                    .execute("UPDATE books SET missing = 1 WHERE id = ?1", [&id])
                    .map_err(|error| error.to_string())?;
                changed = true;
            }
        }
        Ok(LocalScan {
            changed,
            folders,
            files: found.iter().map(|path| path.display().to_string()).collect(),
        })
    }

    pub fn import_path(&self, source: &Path) -> Result<(), String> {
        if source.extension().and_then(|ext| ext.to_str()) != Some("epub") {
            return Err("Escolha um arquivo .epub".into());
        }
        let folders = self.folders()?;
        let dest_dir = folders
            .iter()
            .map(PathBuf::from)
            .find(|path| path.is_dir())
            .unwrap_or_else(|| PathBuf::from("/workspace/kindle"));
        fs::create_dir_all(&dest_dir).map_err(|error| error.to_string())?;
        let name = source
            .file_name()
            .ok_or_else(|| "nome de arquivo inválido".to_string())?;
        let dest = dest_dir.join(name);
        if source != dest {
            fs::copy(source, &dest).map_err(|error| error.to_string())?;
        }
        self.upsert_local(&dest)?;
        Ok(())
    }

    pub fn upsert_cloud(&self, books: &[OwnedBook]) -> Result<(), String> {
        let now = now_ms();
        let existing_locals: Vec<Book> = self
            .books()
            .unwrap_or_default()
            .into_iter()
            .filter(|b| b.source == "local" && !b.missing)
            .collect();
        for book in books {
            let title = clean_title(&book.title);
            if book.asin.len() > 12
                && existing_locals
                    .iter()
                    .any(|loc| titles_match_local_and_pdoc(&loc.title, &loc.file_path, &title))
            {
                let id = format!("cloud:{}", book.asin);
                let _ = self.conn.execute("DELETE FROM books WHERE id = ?1", [&id]);
                continue;
            }
            let id = format!("cloud:{}", book.asin);
            self.conn
                .execute(
                    "INSERT INTO books (id, source, asin, title, author, file_path, cover, percent_read, chapter_title, missing, added_at)
                     VALUES (?1, 'cloud', ?2, ?3, ?4, '', ?5, ?6, '', 0, ?7)
                     ON CONFLICT(id) DO UPDATE SET
                       title = excluded.title,
                       author = excluded.author,
                       cover = CASE WHEN excluded.cover = '' THEN books.cover ELSE excluded.cover END,
                       percent_read = CASE WHEN excluded.percent_read > 0.0 THEN excluded.percent_read ELSE books.percent_read END,
                       missing = 0",
                    params![id, book.asin, title, book.author, book.cover_url, book.percentage, now],
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn set_cloud_progress(&self, position: &ReadingPosition) -> Result<(), String> {
        if position.asin.is_empty() {
            return Ok(());
        }
        let id = format!("cloud:{}", position.asin);
        self.conn
            .execute(
                "UPDATE books SET percent_read = ?1 WHERE id = ?2",
                params![position.percentage, id],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn replace_cloud_annotations(&self, asin: &str, notes: &[CloudAnnotation]) -> Result<(), String> {
        let book_id = format!("cloud:{asin}");
        self.conn
            .execute(
                "DELETE FROM highlights WHERE book_id = ?1 AND source = 'cloud'",
                [&book_id],
            )
            .map_err(|error| error.to_string())?;
        for note in notes {
            let id = format!("cloud:{asin}:{}:{}", note.start_pos, note.end_pos);
            self.conn
                .execute(
                    "INSERT OR REPLACE INTO highlights (id, book_id, source, cfi, text, note, color, start_pos, end_pos, created_at)
                     VALUES (?1, ?2, 'cloud', '', ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![id, book_id, note.text, note.note, note.color, note.start_pos, note.end_pos, note.created_at],
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn save_progress(&self, input: &ProgressInput) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE books SET percent_read = ?1, chapter_title = ?2 WHERE id = ?3",
                params![
                    input.percentage.clamp(0.0, 1.0),
                    input.chapter_title.clone().unwrap_or_default(),
                    input.book_id
                ],
            )
            .map_err(|error| error.to_string())?;
        let id = format!("cfi:{}", input.book_id);
        self.conn
            .execute(
                "INSERT INTO kv (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![id, input.cfi],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn cfi_for(&self, book_id: &str) -> Result<String, String> {
        self.kv(&format!("cfi:{book_id}")).map(|value| value.unwrap_or_default())
    }

    pub fn add_highlight(&self, input: &HighlightInput) -> Result<Highlight, String> {
        let id = format!("local:{}:{}", input.book_id, now_ms());
        let color = input.color.clone().unwrap_or_else(|| "amber".into());
        let note = input.note.clone().unwrap_or_default();
        let created = now_ms();
        self.conn
            .execute(
                "INSERT INTO highlights (id, book_id, source, cfi, text, note, color, start_pos, end_pos, created_at)
                 VALUES (?1, ?2, 'local', ?3, ?4, ?5, ?6, 0, 0, ?7)",
                params![id, input.book_id, input.cfi, input.text, note, color, created],
            )
            .map_err(|error| error.to_string())?;
        self.highlight_by_id(&id)
    }

    pub fn update_highlight_note(&self, id: &str, note: &str) -> Result<(), String> {
        self.conn
            .execute("UPDATE highlights SET note = ?1 WHERE id = ?2", params![note, id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn remove_highlight(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM highlights WHERE id = ?1", [id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn add_bookmark(&self, input: &BookmarkInput) -> Result<(), String> {
        let id = format!("bm:{}:{}", input.book_id, now_ms());
        self.conn
            .execute(
                "INSERT INTO bookmarks (id, book_id, cfi, title, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, input.book_id, input.cfi, input.title, now_ms()],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn remove_bookmark(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM bookmarks WHERE id = ?1", [id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn remove_book(&self, id: &str) -> Result<(), String> {
        self.conn.execute("DELETE FROM books WHERE id = ?1", [id]).map_err(|error| error.to_string())?;
        self.conn
            .execute("DELETE FROM highlights WHERE book_id = ?1", [id])
            .map_err(|error| error.to_string())?;
        self.conn
            .execute("DELETE FROM bookmarks WHERE book_id = ?1", [id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn book_path(&self, id: &str) -> Result<PathBuf, String> {
        let path: String = self
            .conn
            .query_row("SELECT file_path FROM books WHERE id = ?1", [id], |row| row.get(0))
            .map_err(|_| "Livro não encontrado".to_string())?;
        if !path.ends_with(".epub") {
            return Err("Este livro da nuvem abre no leitor da Amazon.".into());
        }
        Ok(PathBuf::from(path))
    }

    pub fn settings(&self) -> Result<Settings, String> {
        let raw = self.kv("settings")?.unwrap_or_else(|| "{}".into());
        serde_json::from_str(&raw).or_else(|_| Ok(Settings::default()))
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        let raw = serde_json::to_string(settings).map_err(|error| error.to_string())?;
        self.put_kv("settings", &raw)
    }

    pub fn session(&self) -> Result<StoredSession, String> {
        let raw = self.kv("session")?.unwrap_or_else(|| "{}".into());
        serde_json::from_str(&raw).or_else(|_| Ok(StoredSession::default()))
    }

    pub fn save_session(&self, session: &StoredSession) -> Result<(), String> {
        let raw = serde_json::to_string(session).map_err(|error| error.to_string())?;
        self.put_kv("session", &raw)
    }

    pub fn session_status(&self) -> Result<SessionStatus, String> {
        let session = self.session()?;
        let mut names: Vec<String> = session.cookies.iter().map(|cookie| cookie.name.clone()).collect();
        names.sort();
        names.dedup();
        Ok(SessionStatus {
            signed_in: session.cookies.iter().any(|cookie| {
                matches!(
                    cookie.name.as_str(),
                    "session-id" | "at-acbbr" | "at-main" | "session-token"
                )
            }),
            region: if session.region.is_empty() { "br".into() } else { session.region },
            cookie_names: names,
            has_uhura: !session.uhura.is_empty(),
            has_device_token: !session.device_session_token.is_empty(),
        })
    }

    pub fn folders(&self) -> Result<Vec<String>, String> {
        let raw = self.kv("folders")?.unwrap_or_else(|| "[]".into());
        serde_json::from_str(&raw).map_err(|error| error.to_string())
    }

    pub fn save_folders(&self, folders: &[String]) -> Result<(), String> {
        let raw = serde_json::to_string(folders).map_err(|error| error.to_string())?;
        self.put_kv("folders", &raw)
    }

    fn upsert_local(&self, path: &Path) -> Result<bool, String> {
        let (clean_meta_title, author, cover) = if path.extension().and_then(|ext| ext.to_str()).unwrap_or("").eq_ignore_ascii_case("epub") {
            match read_epub_meta(path) {
                Ok(meta) => (clean_title(&meta.title), meta.author, meta.cover_data_url),
                Err(_) => (file_title(path), "Arquivo local".into(), String::new()),
            }
        } else {
            (file_title(path), "Arquivo local".into(), String::new())
        };
        let id = format!("local:{}", path.display());
        let existed: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM books WHERE id = ?1", [&id], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        self.conn
            .execute(
                "INSERT INTO books (id, source, asin, title, author, file_path, cover, percent_read, chapter_title, missing, added_at)
                 VALUES (?1, 'local', '', ?2, ?3, ?4, ?5, 0, '', 0, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                   title = excluded.title,
                   author = excluded.author,
                   cover = excluded.cover,
                   missing = 0",
                params![id, clean_meta_title, author, path.display().to_string(), cover, now_ms()],
            )
            .map_err(|error| error.to_string())?;
        let _ = self.dedup_pdocs_against_local();
        Ok(existed == 0)
    }

    fn books(&self) -> Result<Vec<Book>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, source, asin, title, author, file_path, cover, percent_read, chapter_title, missing, added_at
                 FROM books ORDER BY title COLLATE NOCASE",
            )
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Book {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    asin: row.get(2)?,
                    title: row.get(3)?,
                    author: row.get(4)?,
                    file_path: row.get(5)?,
                    cover: row.get(6)?,
                    percentage: row.get(7)?,
                    chapter_title: row.get(8)?,
                    missing: row.get::<_, i64>(9)? != 0,
                    added_at: row.get(10)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
    }

    fn highlights(&self) -> Result<Vec<Highlight>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT h.id, h.book_id, COALESCE(b.title, ''), COALESCE(b.author, ''), h.source, h.cfi, h.text, h.note, h.color, h.start_pos, h.end_pos, h.created_at
                 FROM highlights h LEFT JOIN books b ON b.id = h.book_id
                 ORDER BY h.created_at DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Highlight {
                    id: row.get(0)?,
                    book_id: row.get(1)?,
                    book_title: row.get(2)?,
                    author: row.get(3)?,
                    source: row.get(4)?,
                    cfi: row.get(5)?,
                    text: row.get(6)?,
                    note: row.get(7)?,
                    color: row.get(8)?,
                    start_pos: row.get(9)?,
                    end_pos: row.get(10)?,
                    created_at: row.get(11)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
    }

    fn highlight_by_id(&self, id: &str) -> Result<Highlight, String> {
        self.highlights()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| "Destaque não encontrado".to_string())
    }

    fn bookmarks(&self) -> Result<Vec<Bookmark>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, book_id, cfi, title, created_at FROM bookmarks ORDER BY created_at DESC")
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Bookmark {
                    id: row.get(0)?,
                    book_id: row.get(1)?,
                    cfi: row.get(2)?,
                    title: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
    }

    fn local_paths(&self) -> Result<Vec<(String, String)>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, file_path FROM books WHERE source = 'local'")
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
    }

    fn kv(&self, key: &str) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM kv WHERE key = ?1")
            .map_err(|error| error.to_string())?;
        let mut rows = stmt.query([key]).map_err(|error| error.to_string())?;
        if let Some(row) = rows.next().map_err(|error| error.to_string())? {
            Ok(Some(row.get(0).map_err(|error| error.to_string())?))
        } else {
            Ok(None)
        }
    }

    fn put_kv(&self, key: &str, value: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO kv (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

pub struct LocalScan {
    pub changed: bool,
    pub folders: Vec<String>,
    pub files: Vec<String>,
}

pub fn default_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let base = PathBuf::from(&home);
        let documents = user_dir("XDG_DOCUMENTS_DIR", &home);
        let downloads = user_dir("XDG_DOWNLOAD_DIR", &home);
        folders = home_library_folders(&base, documents.as_deref(), downloads.as_deref());
    }
    let workspace = PathBuf::from("/workspace/kindle");
    if workspace.is_dir() && !folders.iter().any(|path| path == &workspace) {
        folders.push(workspace);
    }
    folders
}

pub fn home_library_folders(home: &Path, documents: Option<&Path>, downloads: Option<&Path>) -> Vec<PathBuf> {
    let mut folders = Vec::new();
    let mut push = |path: PathBuf| {
        if path.as_os_str().is_empty() || folders.iter().any(|item| item == &path) {
            return;
        }
        folders.push(path);
    };
    let documents = documents.map(Path::to_path_buf).unwrap_or_else(|| home.join("Documents"));
    let downloads = downloads.map(Path::to_path_buf).unwrap_or_else(|| home.join("Downloads"));
    push(downloads);
    push(documents.clone());
    push(documents.join("Kindle"));
    push(documents.join("Livros"));
    push(home.join("Downloads"));
    push(home.join("Documents"));
    push(home.join("Documentos"));
    push(home.join("Livros"));
    push(home.join("Kindle"));
    push(home.join("Documents/Kindle"));
    push(home.join("Documents/Livros"));
    push(home.join("Documentos/Kindle"));
    push(home.join("Documentos/Livros"));
    folders
}

fn user_dir(key: &str, home: &str) -> Option<PathBuf> {
    let path = PathBuf::from(home).join(".config/user-dirs.dirs");
    let text = fs::read_to_string(path).ok()?;
    parse_user_dir(&text, key, home).filter(|dir| dir.is_dir())
}

fn parse_user_dir(text: &str, key: &str, home: &str) -> Option<PathBuf> {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        let Some(value) = rest.trim().strip_prefix('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        let expanded = value.replace("$HOME", home).replace("${HOME}", home);
        if !expanded.is_empty() {
            return Some(PathBuf::from(expanded));
        }
    }
    None
}

fn collect_local_books(dir: &Path, found: &mut Vec<PathBuf>, depth: u8) {
    if depth > 4 || found.len() > 400 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_local_books(&path, found, depth + 1);
        } else if is_local_book(name) {
            found.push(path);
        }
    }
}

fn is_local_book(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".epub") || lower.ends_with(".pdf") || lower.ends_with(".mobi") || lower.ends_with(".azw3")
}

fn file_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Livro")
        .replace(['_', '-'], " ")
}


fn normalize_slug(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn titles_match_local_and_pdoc(local_title: &str, file_path: &str, pdoc_title: &str) -> bool {
    let p_slug = normalize_slug(pdoc_title);
    if p_slug.len() < 5 {
        return false;
    }
    let l_slug = normalize_slug(local_title);
    if l_slug == p_slug || l_slug.starts_with(&p_slug) || p_slug.starts_with(&l_slug) {
        return true;
    }
    if let Some(stem) = Path::new(file_path).file_stem().and_then(|s| s.to_str()) {
        let s_slug = normalize_slug(stem.trim_end_matches("-kindle"));
        if !s_slug.is_empty() && (s_slug == p_slug || s_slug.starts_with(&p_slug) || p_slug.starts_with(&s_slug)) {
            return true;
        }
    }
    false
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clippings::{to_markdown, to_my_clippings};
    use crate::kindle_cloud::{fixture_annotations, fixture_position, load_fixture_library};

    #[test]
    fn fixture_sync_progress_and_clippings() {
        let dir = std::env::temp_dir().join(format!("kindle-db-{}", now_ms()));
        fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("library.db")).unwrap();
        db.save_folders(&["/workspace/kindle".into()]).unwrap();
        db.scan().unwrap();
        let local = db.snapshot().unwrap();
        assert!(local.books.iter().any(|book| book.title == "O Farol da Ilha Rasa"));
        assert!(local.books.iter().any(|book| book.title == "A Mesa da Cozinha"));

        let cloud = load_fixture_library().unwrap();
        db.upsert_cloud(&cloud).unwrap();
        let position = fixture_position().unwrap();
        db.set_cloud_progress(&position).unwrap();
        let notes = fixture_annotations().unwrap();
        db.replace_cloud_annotations("B0CLOUD001", &notes).unwrap();
        let snap = db.snapshot().unwrap();
        let mare = snap.books.iter().find(|book| book.asin == "B0CLOUD001").unwrap();
        assert!((mare.percentage - 0.42).abs() < 0.001);
        assert!(snap.highlights.iter().any(|item| item.text.contains("não salva o mar")));
        let markdown = to_markdown(&snap.highlights);
        let clippings = to_my_clippings(&snap.highlights);
        assert!(markdown.contains("Maré de Pedra"));
        assert!(clippings.contains("=========="));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn user_dirs_expand_home() {
        let text = "XDG_DOCUMENTS_DIR=\"$HOME/Documentos\"\nXDG_DOWNLOAD_DIR=\"${HOME}/Downloads\"\n";
        assert_eq!(
            parse_user_dir(text, "XDG_DOCUMENTS_DIR", "/home/joao").unwrap(),
            PathBuf::from("/home/joao/Documentos")
        );
        assert_eq!(
            parse_user_dir(text, "XDG_DOWNLOAD_DIR", "/home/joao").unwrap(),
            PathBuf::from("/home/joao/Downloads")
        );
        let folders = home_library_folders(
            Path::new("/home/joao"),
            Some(Path::new("/home/joao/Documentos")),
            Some(Path::new("/home/joao/Downloads")),
        );
        assert!(folders.iter().any(|path| path.ends_with("Documentos/Livros")));
        assert!(folders.iter().any(|path| path.ends_with("Documentos/Kindle")));
        assert!(folders.iter().any(|path| path == Path::new("/home/joao/Kindle")));
        assert!(folders.iter().any(|path| path == Path::new("/home/joao/Downloads")));
        assert!(is_local_book("livro.EPUB"));
        assert!(is_local_book("notas.pdf"));
        assert!(is_local_book("kindle.mobi"));
        assert!(is_local_book("sample.azw3"));
        assert!(!is_local_book("capa.jpg"));
    }
}
