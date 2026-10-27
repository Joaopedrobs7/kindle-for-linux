use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use crate::model::StoredCookie;

pub const CHROME_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.6778.204 Safari/537.36";

const INTERESTING: &[&str] = &[
    "ubid-acbbr",
    "x-acbbr",
    "at-acbbr",
    "session-id",
    "session-token",
    "x-main",
    "at-main",
    "uhura",
    "deviceSessionToken",
    "session-id-time",
];

#[derive(Debug, Error)]
pub enum CloudError {
    #[error("resposta da Kindle Cloud sem livros")]
    EmptyLibrary,
    #[error("JSON inválido da Kindle Cloud: {0}")]
    Json(String),
    #[error("posição de leitura ausente")]
    MissingPosition,
    #[error("rede: {0}")]
    Network(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Br,
    Us,
}

impl Region {
    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("us") {
            Self::Us
        } else {
            Self::Br
        }
    }

    pub fn origin(self) -> &'static str {
        match self {
            Self::Br => "https://ler.amazon.com.br",
            Self::Us => "https://read.amazon.com",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Br => "br",
            Self::Us => "us",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnedBook {
    pub asin: String,
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub percentage: f64,
    pub reader_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadingPosition {
    pub asin: String,
    pub position: i64,
    pub percentage: f64,
    pub sync_time: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CloudAnnotation {
    pub asin: String,
    pub kind: String,
    pub text: String,
    pub note: String,
    pub color: String,
    pub start_pos: i64,
    pub end_pos: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct SessionMaterial {
    pub region: Region,
    pub cookies: Vec<StoredCookie>,
    pub uhura: String,
    pub device_session_token: String,
}

pub fn library_search_url(region: Region, token: Option<&str>) -> String {
    let mut url = format!(
        "{}/kindle-library/search?query=&libraryType=BOOKS&sortType=recency&querySize=50",
        region.origin()
    );
    if let Some(token) = token {
        if !token.is_empty() {
            url.push_str("&paginationToken=");
            url.push_str(&urlencoding(token));
        }
    }
    url
}

pub fn start_reading_url(region: Region, asin: &str) -> String {
    format!(
        "{}/service/mobile/reader/startReading?asin={}&clientVersion=20000100",
        region.origin(),
        urlencoding(asin)
    )
}

pub fn annotations_url(region: Region) -> String {
    format!("{}/service/mobile/reader/getAnnotations", region.origin())
}

pub fn owned_content_url(region: Region, asin: &str) -> String {
    format!(
        "{}/service/mobile/reader/getOwnedContent?asin={}",
        region.origin(),
        urlencoding(asin)
    )
}

pub fn reader_url(region: Region, asin: &str) -> String {
    format!("{}/?asin={}", region.origin(), urlencoding(asin))
}

pub fn login_url(region: Region) -> String {
    format!("{}/kindle-library", region.origin())
}

pub fn request_headers(session: &SessionMaterial) -> Vec<(String, String)> {
    let mut headers = vec![
        ("User-Agent".into(), CHROME_UA.into()),
        ("Accept".into(), "application/json, text/javascript, */*; q=0.01".into()),
        ("Accept-Language".into(), "pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7".into()),
        ("Cookie".into(), cookie_header(&session.cookies)),
    ];
    if !session.device_session_token.is_empty() {
        headers.push(("x-amzn-session-token".into(), session.device_session_token.clone()));
    }
    if !session.uhura.is_empty() {
        headers.push(("x-amzn-uhura".into(), session.uhura.clone()));
    }
    headers
}

pub fn cookie_header(cookies: &[StoredCookie]) -> String {
    cookies
        .iter()
        .filter(|cookie| !cookie.name.is_empty())
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn parse_cookie_header(header: &str, domain: &str) -> Vec<StoredCookie> {
    header
        .split(';')
        .filter_map(|part| {
            let (name, value) = part.trim().split_once('=')?;
            let name = name.trim();
            if name.is_empty() || name.contains(' ') {
                return None;
            }
            let value = value.trim().trim_matches('"').to_string();
            Some(StoredCookie {
                name: name.to_string(),
                value,
                domain: domain.to_string(),
            })
        })
        .collect()
}

pub fn interesting_names(cookies: &[StoredCookie]) -> Vec<String> {
    let mut names: Vec<String> = cookies
        .iter()
        .filter(|cookie| INTERESTING.iter().any(|name| cookie.name.eq_ignore_ascii_case(name)))
        .map(|cookie| cookie.name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

pub fn absorb_tokens(cookies: &[StoredCookie], uhura: &mut String, device: &mut String) {
    for cookie in cookies {
        if cookie.name.eq_ignore_ascii_case("uhura") && uhura.is_empty() {
            *uhura = cookie.value.clone();
        }
        if cookie.name.eq_ignore_ascii_case("deviceSessionToken") && device.is_empty() {
            *device = cookie.value.clone();
        }
    }
}

pub fn parse_library_document(body: &str, region: Region) -> Result<(Vec<OwnedBook>, Option<String>), CloudError> {
    let value: Value = serde_json::from_str(body).map_err(|error| CloudError::Json(error.to_string()))?;
    let items = items_of(&value).ok_or(CloudError::EmptyLibrary)?;
    let mut books = Vec::new();
    for item in items {
        if let Some(book) = book_from_item(item, region) {
            books.push(book);
        }
    }
    if books.is_empty() {
        return Err(CloudError::EmptyLibrary);
    }
    let token = value
        .get("paginationToken")
        .and_then(|token| token.as_str())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string());
    Ok((books, token))
}

pub fn merge_pages(pages: &[Vec<OwnedBook>]) -> Vec<OwnedBook> {
    let mut merged: Vec<OwnedBook> = Vec::new();
    for page in pages {
        for book in page {
            if let Some(existing) = merged.iter_mut().find(|item| item.asin == book.asin) {
                if existing.title.is_empty() {
                    existing.title = book.title.clone();
                }
                if book.percentage > 0.0 {
                    existing.percentage = book.percentage;
                }
                if existing.cover_url.is_empty() {
                    existing.cover_url = book.cover_url.clone();
                }
            } else {
                merged.push(book.clone());
            }
        }
    }
    merged
}

pub fn load_fixture_library() -> Result<Vec<OwnedBook>, CloudError> {
    let (first, _) = parse_library_document(include_str!("../fixtures/library_page1.json"), Region::Br)?;
    let (second, _) = parse_library_document(include_str!("../fixtures/library_page2.json"), Region::Br)?;
    Ok(merge_pages(&[first, second]))
}

pub fn fixture_position() -> Result<ReadingPosition, CloudError> {
    parse_reading_position(include_str!("../fixtures/start_reading.json"))
}

pub fn fixture_annotations() -> Result<Vec<CloudAnnotation>, CloudError> {
    parse_annotations(include_str!("../fixtures/annotations.json"))
}

pub fn parse_reading_position(body: &str) -> Result<ReadingPosition, CloudError> {
    let value: Value = serde_json::from_str(body).map_err(|error| CloudError::Json(error.to_string()))?;
    let asin = value.get("asin").and_then(|item| item.as_str()).unwrap_or("").to_string();
    let nested = value.get("lastPageRead").cloned().unwrap_or(Value::Null);
    let position = first_i64(&[&nested, &value], &["position", "lastPageReadPosition"]).ok_or(CloudError::MissingPosition)?;
    let start = first_i64(&[&value], &["startPosition", "srl"]).unwrap_or(0);
    let end = first_i64(&[&value], &["endPosition", "yel"]).unwrap_or(0);
    let explicit = first_f64(&[&value, &nested], &["percentage", "percentageRead", "percentRead"]);
    let percentage = match explicit {
        Some(value) if value > 1.0 => (value / 100.0).clamp(0.0, 1.0),
        Some(value) => value.clamp(0.0, 1.0),
        None if end > start => ((position - start) as f64 / (end - start) as f64).clamp(0.0, 1.0),
        None => 0.0,
    };
    let sync_time = first_i64(&[&nested, &value], &["syncTime", "syncDate"]).unwrap_or(0);
    Ok(ReadingPosition {
        asin,
        position,
        percentage,
        sync_time,
    })
}

pub fn parse_annotations(body: &str) -> Result<Vec<CloudAnnotation>, CloudError> {
    let value: Value = serde_json::from_str(body).map_err(|error| CloudError::Json(error.to_string()))?;
    let asin = value.get("asin").and_then(|item| item.as_str()).unwrap_or("").to_string();
    let items = value
        .get("annotations")
        .or_else(|| value.get("highlights"))
        .or_else(|| value.get("items"))
        .and_then(|item| item.as_array())
        .ok_or(CloudError::EmptyLibrary)?;
    let mut notes = Vec::new();
    for item in items {
        let text = first_string(item, &["highlight", "text", "content", "quote"]).unwrap_or_default();
        if text.trim().is_empty() {
            continue;
        }
        notes.push(CloudAnnotation {
            asin: item
                .get("asin")
                .and_then(|value| value.as_str())
                .unwrap_or(&asin)
                .to_string(),
            kind: first_string(item, &["type", "kind"]).unwrap_or_else(|| "highlight".into()),
            text,
            note: first_string(item, &["note", "noteText"]).unwrap_or_default(),
            color: normalize_color(&first_string(item, &["color"]).unwrap_or_else(|| "yellow".into())),
            start_pos: first_i64(&[item], &["start", "startPosition"]).unwrap_or(0),
            end_pos: first_i64(&[item], &["end", "endPosition"]).unwrap_or(0),
            created_at: first_i64(&[item], &["creationTime", "createdAt", "lastModificationTime"]).unwrap_or(0),
        });
    }
    Ok(notes)
}

pub async fn fetch_library(session: &SessionMaterial) -> Result<Vec<OwnedBook>, CloudError> {
    let client = http_client()?;
    let mut token: Option<String> = None;
    let mut pages = Vec::new();
    for _ in 0..20 {
        let url = library_search_url(session.region, token.as_deref());
        let body = send_get(&client, session, &url).await?;
        let (books, next) = parse_library_document(&body, session.region)?;
        pages.push(books);
        match next {
            Some(value) => token = Some(value),
            None => break,
        }
    }
    Ok(merge_pages(&pages))
}

pub async fn fetch_position(session: &SessionMaterial, asin: &str) -> Result<ReadingPosition, CloudError> {
    let client = http_client()?;
    let url = start_reading_url(session.region, asin);
    let body = send_get(&client, session, &url).await?;
    let mut position = parse_reading_position(&body)?;
    if position.asin.is_empty() {
        position.asin = asin.to_string();
    }
    Ok(position)
}

pub async fn fetch_annotations(session: &SessionMaterial, asin: &str) -> Result<Vec<CloudAnnotation>, CloudError> {
    let client = http_client()?;
    let url = annotations_url(session.region);
    let payload = serde_json::json!({ "asin": asin, "type": "all" });
    let body = send_post(&client, session, &url, &payload).await?;
    let mut notes = parse_annotations(&body)?;
    for note in &mut notes {
        if note.asin.is_empty() {
            note.asin = asin.to_string();
        }
    }
    Ok(notes)
}

fn http_client() -> Result<reqwest::Client, CloudError> {
    reqwest::Client::builder()
        .user_agent(CHROME_UA)
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .map_err(|error| CloudError::Network(error.to_string()))
}

async fn send_get(client: &reqwest::Client, session: &SessionMaterial, url: &str) -> Result<String, CloudError> {
    let mut request = client.get(url);
    for (name, value) in request_headers(session) {
        request = request.header(name, value);
    }
    let response = request.send().await.map_err(|error| CloudError::Network(error.to_string()))?;
    response.text().await.map_err(|error| CloudError::Network(error.to_string()))
}

async fn send_post(
    client: &reqwest::Client,
    session: &SessionMaterial,
    url: &str,
    payload: &Value,
) -> Result<String, CloudError> {
    let mut request = client.post(url).json(payload);
    for (name, value) in request_headers(session) {
        request = request.header(name, value);
    }
    let response = request.send().await.map_err(|error| CloudError::Network(error.to_string()))?;
    response.text().await.map_err(|error| CloudError::Network(error.to_string()))
}

fn items_of(value: &Value) -> Option<&Vec<Value>> {
    for key in ["itemsList", "items", "books", "libraryItems"] {
        if let Some(list) = value.get(key).and_then(|item| item.as_array()) {
            return Some(list);
        }
    }
    None
}

fn book_from_item(item: &Value, region: Region) -> Option<OwnedBook> {
    let asin = first_string(item, &["asin", "ASIN"])?;
    if asin.len() < 6 {
        return None;
    }
    let title = first_string(item, &["title", "productTitle"]).unwrap_or_else(|| asin.clone());
    let author = authors_of(item);
    let cover_url = first_string(item, &["coverUrl", "imageUrl", "productImage"]).unwrap_or_default();
    let percentage = percentage_field(item);
    let reader = first_string(item, &["webReaderUrl", "readerUrl"])
        .filter(|url| url.starts_with("http"))
        .unwrap_or_else(|| reader_url(region, &asin));
    Some(OwnedBook {
        asin,
        title,
        author,
        cover_url,
        percentage,
        reader_url: reader,
    })
}

fn authors_of(item: &Value) -> String {
    match item.get("authors").or_else(|| item.get("author")) {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(|entry| entry.as_str().or_else(|| entry.get("name").and_then(|name| name.as_str())))
            .collect::<Vec<_>>()
            .join(", "),
        _ => "Autor desconhecido".into(),
    }
}

fn percentage_field(item: &Value) -> f64 {
    let value = first_f64(&[item], &["percentageRead", "percentRead", "percentage"]).unwrap_or(0.0);
    if value > 1.0 {
        (value / 100.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn first_string(item: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = item.get(*key).and_then(|value| value.as_str()) {
            if !value.trim().is_empty() {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn first_i64(nodes: &[&Value], keys: &[&str]) -> Option<i64> {
    for node in nodes {
        if node.is_null() {
            continue;
        }
        for key in keys {
            if let Some(value) = node.get(*key) {
                if let Some(number) = value.as_i64() {
                    return Some(number);
                }
                if let Some(number) = value.as_u64() {
                    return Some(number as i64);
                }
                if let Some(text) = value.as_str() {
                    if let Ok(number) = text.parse::<i64>() {
                        return Some(number);
                    }
                }
            }
        }
    }
    None
}

fn first_f64(nodes: &[&Value], keys: &[&str]) -> Option<f64> {
    for node in nodes {
        if node.is_null() {
            continue;
        }
        for key in keys {
            if let Some(value) = node.get(*key) {
                if let Some(number) = value.as_f64() {
                    return Some(number);
                }
                if let Some(number) = value.as_i64() {
                    return Some(number as f64);
                }
            }
        }
    }
    None
}

fn normalize_color(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "green" | "moss" | "verde" => "moss".into(),
        "blue" | "slate" | "azul" => "slate".into(),
        "pink" | "rose" | "rosa" => "rose".into(),
        _ => "amber".into(),
    }
}

fn urlencoding(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub const READER_QUIET_CSS: &str = r#"
#kindleReaderHeader, #reader-header, #kindle_menu, #navbar, #navBar,
.topNav, .top-nav, .kindle-header, #chrome, #top-bar, #readerHeader,
.a-navbar, #nav-belt, #navbar-main { display: none !important; }
body { background: #F7F5F0 !important; }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_pages_merge_and_keep_percentage() {
        let books = load_fixture_library().expect("fixture");
        assert_eq!(books.len(), 3);
        let mare = books.iter().find(|book| book.asin == "B0CLOUD001").unwrap();
        assert_eq!(mare.title, "Maré de Pedra");
        assert_eq!(mare.author, "Helena Vilar");
        assert!((mare.percentage - 0.42).abs() < 0.001);
        assert!(mare.reader_url.contains("asin=B0CLOUD001") || mare.reader_url.contains("B0CLOUD001"));
        assert!(books.iter().any(|book| book.title == "Casa de Chá"));
    }

    #[test]
    fn whispersync_position_is_42_percent() {
        let position = fixture_position().expect("posição");
        assert_eq!(position.asin, "B0CLOUD001");
        assert_eq!(position.position, 4201);
        assert!((position.percentage - 0.42).abs() < 0.001);
        assert_eq!(position.sync_time, 1_730_000_000_000);
    }

    #[test]
    fn annotations_keep_note_and_color() {
        let notes = fixture_annotations().expect("notas");
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].text, "O faroleiro não salva o mar.");
        assert_eq!(notes[0].note, "frase do diário");
        assert_eq!(notes[0].color, "amber");
        assert_eq!(notes[1].color, "slate");
        assert_eq!(notes[1].text, "reserva amarela");
    }

    #[test]
    fn cookie_header_keeps_brazil_and_us_names() {
        let header = "ubid-acbbr=abc; x-acbbr=1; at-acbbr=\"tok\"; session-id=999; session-token=st; x-main=m; at-main=am; uhura=u1; deviceSessionToken=dev";
        let cookies = parse_cookie_header(header, ".amazon.com.br");
        assert_eq!(cookies.len(), 9);
        let names = interesting_names(&cookies);
        assert!(names.iter().any(|name| name == "ubid-acbbr"));
        assert!(names.iter().any(|name| name == "at-main"));
        assert!(names.iter().any(|name| name == "deviceSessionToken"));
        let mut uhura = String::new();
        let mut device = String::new();
        absorb_tokens(&cookies, &mut uhura, &mut device);
        assert_eq!(uhura, "u1");
        assert_eq!(device, "dev");
        let session = SessionMaterial {
            region: Region::Br,
            cookies,
            uhura,
            device_session_token: device,
        };
        let headers = request_headers(&session);
        assert!(headers.iter().any(|(name, value)| name == "User-Agent" && value.contains("Chrome/131")));
        assert!(headers.iter().any(|(name, value)| name == "Cookie" && value.contains("session-id=999")));
        assert!(headers.iter().any(|(name, value)| name == "x-amzn-session-token" && value == "dev"));
        assert!(library_search_url(Region::Br, Some("page-2")).contains("paginationToken=page-2"));
        assert!(start_reading_url(Region::Us, "B0CLOUD001").contains("read.amazon.com"));
        assert!(annotations_url(Region::Br).contains("getAnnotations"));
        assert!(owned_content_url(Region::Br, "B0CLOUD001").contains("getOwnedContent"));
    }
}
