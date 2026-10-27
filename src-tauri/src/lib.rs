mod clippings;
mod db;
mod epub_meta;
mod kindle_cloud;
mod lookup;
mod model;
mod overview;

use std::fs;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::db::Db;
use crate::kindle_cloud::{
    absorb_tokens, annotations_url, fetch_annotations, fetch_library, fetch_position, interesting_names, login_url,
    owned_content_url, parse_cookie_header, reader_url, start_reading_url, Region, SessionMaterial, READER_QUIET_CSS,
};
use crate::model::{BookmarkInput, HighlightInput, ProgressInput, Settings, StoredCookie, StoredSession};

struct AppState {
    db: Mutex<Db>,
}

#[tauri::command]
fn library_snapshot(state: State<AppState>) -> Result<model::Snapshot, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.scan()?;
    db.snapshot()
}

#[tauri::command]
fn import_epub(state: State<AppState>, path: String) -> Result<model::Snapshot, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.import_path(std::path::Path::new(&path))?;
    db.snapshot()
}

#[tauri::command]
fn read_book(state: State<AppState>, book_id: String) -> Result<Vec<u8>, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    let path = db.book_path(&book_id)?;
    fs::read(path).map_err(|error| error.to_string())
}

#[tauri::command]
fn book_cfi(state: State<AppState>, book_id: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.cfi_for(&book_id)
}

#[tauri::command]
fn save_progress(state: State<AppState>, input: ProgressInput) -> Result<(), String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.save_progress(&input)
}

#[tauri::command]
fn add_highlight(state: State<AppState>, input: HighlightInput) -> Result<model::Highlight, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.add_highlight(&input)
}

#[tauri::command]
fn update_highlight(state: State<AppState>, id: String, note: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.update_highlight_note(&id, &note)
}

#[tauri::command]
fn remove_highlight(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.remove_highlight(&id)
}

#[tauri::command]
fn add_bookmark(state: State<AppState>, input: BookmarkInput) -> Result<(), String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.add_bookmark(&input)
}

#[tauri::command]
fn remove_bookmark(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.remove_bookmark(&id)
}

#[tauri::command]
fn remove_book(state: State<AppState>, id: String) -> Result<model::Snapshot, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.remove_book(&id)?;
    db.snapshot()
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> Result<model::Snapshot, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.save_settings(&settings)?;
    db.snapshot()
}

#[tauri::command]
fn export_clippings(state: State<AppState>, format: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    let notes = db.snapshot()?.highlights;
    Ok(if format == "clippings" {
        clippings::to_my_clippings(&notes)
    } else {
        clippings::to_markdown(&notes)
    })
}

#[tauri::command]
async fn define_word(word: String) -> lookup::Definition {
    lookup::lookup_online(&word).await
}

#[tauri::command]
fn book_overview(state: State<AppState>, book_id: String, words_per_minute: Option<u32>) -> Result<overview::Overview, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    let snap = db.snapshot()?;
    let book = snap
        .books
        .into_iter()
        .find(|item| item.id == book_id)
        .ok_or_else(|| "Livro não encontrado".to_string())?;
    let text = if book.file_path.ends_with(".epub") {
        epub_meta::epub_plain_text(std::path::Path::new(&book.file_path)).unwrap_or_default()
    } else {
        String::new()
    };
    Ok(overview::overview_from_text(&text, book.percentage, words_per_minute.unwrap_or(220)))
}

#[tauri::command]
fn cloud_open_login(app: AppHandle, region: String) -> Result<(), String> {
    let parsed = Region::parse(&region);
    {
        let state = app.state::<AppState>();
        let db = state.db.lock().map_err(|error| error.to_string())?;
        let mut session = db.session()?;
        session.region = parsed.as_str().into();
        db.save_session(&session)?;
    }
    open_amazon_window(&app, "amazon-login", "Entrar na Amazon", &login_url(parsed), "")
}

#[tauri::command]
fn cloud_capture(app: AppHandle) -> Result<model::SessionStatus, String> {
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|error| error.to_string())?;
    let mut session = db.session()?;
    let region = Region::parse(&session.region);
    let origin = region.origin();
    let window = app
        .get_webview_window("amazon-login")
        .or_else(|| app.get_webview_window("cloud-reader"))
        .or_else(|| app.get_webview_window("main"))
        .ok_or_else(|| "Abra o login da Amazon antes de capturar a sessão.".to_string())?;
    let url = url::Url::parse(origin).map_err(|error| error.to_string())?;
    let cookies = window.cookies_for_url(url).map_err(|error| error.to_string())?;
    session.cookies = cookies
        .into_iter()
        .map(|cookie| StoredCookie {
            name: cookie.name().to_string(),
            value: cookie.value().to_string(),
            domain: cookie.domain().unwrap_or("").to_string(),
        })
        .filter(|cookie| !cookie.value.is_empty())
        .collect();
    absorb_tokens(&session.cookies, &mut session.uhura, &mut session.device_session_token);
    if session.cookies.is_empty() {
        return Err("Nenhum cookie da Amazon nessa janela. Termine o login e tente de novo.".into());
    }
    db.save_session(&session)?;
    db.session_status()
}

#[tauri::command]
fn cloud_import_cookies(state: State<AppState>, header: String, region: String) -> Result<model::SessionStatus, String> {
    let db = state.db.lock().map_err(|error| error.to_string())?;
    let parsed = Region::parse(&region);
    let domain = if parsed == Region::Br { ".amazon.com.br" } else { ".amazon.com" };
    let cookies = parse_cookie_header(&header, domain);
    if cookies.is_empty() {
        return Err("Cole o cabeçalho Cookie com session-id, at-acbbr ou at-main.".into());
    }
    let mut session = db.session()?;
    session.region = parsed.as_str().into();
    session.cookies = cookies;
    absorb_tokens(&session.cookies, &mut session.uhura, &mut session.device_session_token);
    db.save_session(&session)?;
    let _ = interesting_names(&session.cookies);
    db.session_status()
}

#[tauri::command]
async fn cloud_sync(app: AppHandle) -> Result<model::Snapshot, String> {
    let material = {
        let state = app.state::<AppState>();
        let db = state.db.lock().map_err(|error| error.to_string())?;
        material_from(&db.session()?)?
    };
    let books = fetch_library(&material).await.map_err(|error| error.to_string())?;
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.upsert_cloud(&books)?;
    db.snapshot()
}

#[tauri::command]
async fn cloud_sync_book(app: AppHandle, asin: String) -> Result<model::Snapshot, String> {
    let material = {
        let state = app.state::<AppState>();
        let db = state.db.lock().map_err(|error| error.to_string())?;
        material_from(&db.session()?)?
    };
    let position = fetch_position(&material, &asin).await.map_err(|error| error.to_string())?;
    let notes = fetch_annotations(&material, &asin).await.map_err(|error| error.to_string())?;
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.set_cloud_progress(&position)?;
    db.replace_cloud_annotations(&asin, &notes)?;
    let _ = (start_reading_url(material.region, &asin), annotations_url(material.region), owned_content_url(material.region, &asin));
    db.snapshot()
}

#[tauri::command]
fn cloud_load_example(state: State<AppState>) -> Result<model::Snapshot, String> {
    let books = kindle_cloud::load_fixture_library().map_err(|error| error.to_string())?;
    let position = kindle_cloud::fixture_position().map_err(|error| error.to_string())?;
    let notes = kindle_cloud::fixture_annotations().map_err(|error| error.to_string())?;
    let db = state.db.lock().map_err(|error| error.to_string())?;
    db.upsert_cloud(&books)?;
    db.set_cloud_progress(&position)?;
    db.replace_cloud_annotations("B0CLOUD001", &notes)?;
    db.snapshot()
}

#[tauri::command]
fn cloud_open_reader(app: AppHandle, asin: String) -> Result<(), String> {
    let region = {
        let state = app.state::<AppState>();
        let db = state.db.lock().map_err(|error| error.to_string())?;
        Region::parse(&db.session()?.region)
    };
    let script = format!(
        "(function(){{const css={};const apply=()=>{{let s=document.getElementById('kl-quiet');if(!s){{s=document.createElement('style');s.id='kl-quiet';document.documentElement.appendChild(s);}}s.textContent=css;}};apply();new MutationObserver(apply).observe(document.documentElement,{{childList:true,subtree:true}});}})();",
        serde_json::to_string(READER_QUIET_CSS).unwrap_or_else(|_| "\"\"".into())
    );
    open_amazon_window(&app, "cloud-reader", "Leitura", &reader_url(region, &asin), &script)
}

#[tauri::command]
fn set_fullscreen(app: AppHandle, enabled: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.set_fullscreen(enabled).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn material_from(session: &StoredSession) -> Result<SessionMaterial, String> {
    if session.cookies.is_empty() {
        return Err("Entre na Amazon ou cole os cookies da sessão antes de sincronizar.".into());
    }
    Ok(SessionMaterial {
        region: Region::parse(&session.region),
        cookies: session.cookies.clone(),
        uhura: session.uhura.clone(),
        device_session_token: session.device_session_token.clone(),
    })
}

fn open_amazon_window(app: &AppHandle, label: &str, title: &str, target: &str, script: &str) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(label) {
        let _ = existing.close();
    }
    let url = target.parse().map_err(|error: url::ParseError| error.to_string())?;
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::External(url))
        .title(title)
        .inner_size(1100.0, 760.0)
        .user_agent(kindle_cloud::CHROME_UA)
        .initialization_script("if (!window.chrome) { window.chrome = { runtime: {} }; }");
    if !script.is_empty() {
        builder = builder.initialization_script(script);
    }
    builder.build().map_err(|error| error.to_string())?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|error| error.to_string())?;
            fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
            let db = Db::open(&dir.join("library.db")).map_err(|error| error.to_string())?;
            let _ = db.scan();
            app.manage(AppState { db: Mutex::new(db) });
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(5));
                let Some(state) = handle.try_state::<AppState>() else {
                    continue;
                };
                let changed = state.db.lock().ok().and_then(|db| db.scan().ok()).unwrap_or(false);
                if changed {
                    let _ = handle.emit("library-changed", ());
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library_snapshot,
            import_epub,
            read_book,
            book_cfi,
            save_progress,
            add_highlight,
            update_highlight,
            remove_highlight,
            add_bookmark,
            remove_bookmark,
            remove_book,
            save_settings,
            export_clippings,
            define_word,
            book_overview,
            cloud_open_login,
            cloud_capture,
            cloud_import_cookies,
            cloud_sync,
            cloud_sync_book,
            cloud_load_example,
            cloud_open_reader,
            set_fullscreen
        ])
        .run(tauri::generate_context!())
        .expect("erro ao abrir Kindle para Linux");
}
