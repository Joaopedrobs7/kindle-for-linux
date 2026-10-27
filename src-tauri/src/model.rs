use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderPrefs {
    pub theme: String,
    pub font_family: String,
    pub font_weight: u16,
    pub font_size: u16,
    pub line_height: f64,
    pub measure: u16,
    pub align: String,
    pub flow: String,
}

impl Default for ReaderPrefs {
    fn default() -> Self {
        Self {
            theme: "paper".into(),
            font_family: "serif".into(),
            font_weight: 440,
            font_size: 19,
            line_height: 1.65,
            measure: 68,
            align: "justify".into(),
            flow: "paginated".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub appearance: String,
    pub region: String,
    pub reader: ReaderPrefs,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: "paper".into(),
            region: "br".into(),
            reader: ReaderPrefs::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: String,
    pub source: String,
    pub asin: String,
    pub title: String,
    pub author: String,
    pub file_path: String,
    pub cover: String,
    pub percentage: f64,
    pub chapter_title: String,
    pub missing: bool,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Highlight {
    pub id: String,
    pub book_id: String,
    pub book_title: String,
    pub author: String,
    pub source: String,
    pub cfi: String,
    pub text: String,
    pub note: String,
    pub color: String,
    pub start_pos: i64,
    pub end_pos: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub id: String,
    pub book_id: String,
    pub cfi: String,
    pub title: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoredSession {
    pub region: String,
    pub cookies: Vec<StoredCookie>,
    pub uhura: String,
    pub device_session_token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub signed_in: bool,
    pub region: String,
    pub cookie_names: Vec<String>,
    pub has_uhura: bool,
    pub has_device_token: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub books: Vec<Book>,
    pub highlights: Vec<Highlight>,
    pub bookmarks: Vec<Bookmark>,
    pub settings: Settings,
    pub session: SessionStatus,
    pub folders: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressInput {
    pub book_id: String,
    pub cfi: String,
    pub percentage: f64,
    pub chapter_title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HighlightInput {
    pub book_id: String,
    pub cfi: String,
    pub text: String,
    pub note: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkInput {
    pub book_id: String,
    pub cfi: String,
    pub title: String,
}
