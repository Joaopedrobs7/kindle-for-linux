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
    #[error("a Amazon pediu para entrar de novo na loja para ver os documentos pessoais")]
    NeedsReauth,
    #[error("{0}")]
    PersonalDocs(String),
}

#[derive(Debug)]
pub struct LibraryLoad {
    pub books: Vec<OwnedBook>,
    pub personal_docs_error: Option<String>,
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

    pub fn store_origin(self) -> &'static str {
        match self {
            Self::Br => "https://www.amazon.com.br",
            Self::Us => "https://www.amazon.com",
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
    cookie_header_for(cookies, false)
}

/// `prefer_store` picks www.amazon cookies over host-only ler.amazon / read.amazon cookies.
pub fn cookie_header_for(cookies: &[StoredCookie], prefer_store: bool) -> String {
    let mut chosen: Vec<&StoredCookie> = Vec::new();
    for cookie in cookies {
        if cookie.name.is_empty() || cookie.value.is_empty() {
            continue;
        }
        if let Some(slot) = chosen.iter_mut().find(|item| item.name.eq_ignore_ascii_case(&cookie.name)) {
            if domain_rank(&cookie.domain, prefer_store) > domain_rank(&slot.domain, prefer_store) {
                *slot = cookie;
            }
        } else {
            chosen.push(cookie);
        }
    }
    chosen
        .iter()
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ")
}

fn domain_rank(domain: &str, prefer_store: bool) -> u8 {
    let reader = is_reader_host(domain);
    let store = is_store_host(domain);
    if prefer_store {
        if store && !reader {
            3
        } else if !reader {
            2
        } else if store {
            1
        } else {
            0
        }
    } else if reader {
        3
    } else if store {
        2
    } else {
        1
    }
}

fn is_reader_host(domain: &str) -> bool {
    let host = domain.trim().trim_start_matches('.').to_ascii_lowercase();
    host.starts_with("ler.") || host.starts_with("read.")
}

fn is_store_host(domain: &str) -> bool {
    let host = domain.trim().trim_start_matches('.').to_ascii_lowercase();
    if host.is_empty() || is_reader_host(domain) {
        return false;
    }
    host == "amazon.com.br"
        || host == "www.amazon.com.br"
        || host == "amazon.com"
        || host == "www.amazon.com"
        || host.ends_with(".amazon.com.br")
        || host.ends_with(".amazon.com")
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
            if !books.iter().any(|existing: &OwnedBook| existing.title.eq_ignore_ascii_case(&book.title)) {
                books.push(book);
            }
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

pub async fn fetch_library(session: &SessionMaterial) -> Result<LibraryLoad, CloudError> {
    let client = http_client()?;
    let mut token: Option<String> = None;
    let mut pages = Vec::new();
    for _ in 0..20 {
        let url = library_search_url(session.region, token.as_deref());
        match send_get(&client, session, &url).await.and_then(|body| parse_library_document(&body, session.region)) {
            Ok((books, next)) => {
                pages.push(books);
                match next {
                    Some(value) => token = Some(value),
                    None => break,
                }
            }
            Err(err) => {
                if pages.is_empty() {
                    return Err(err);
                }
                break;
            }
        }
    }
    let personal_docs_error = match fetch_pdocs(&client, session).await {
        Ok(pdocs) => {
            if !pdocs.is_empty() {
                pages.push(pdocs);
            }
            None
        }
        Err(error) => Some(error.to_string()),
    };
    let mut merged = merge_pages(&pages);
    hydrate_library_progress(&client, session, &mut merged).await;
    Ok(LibraryLoad {
        books: merged,
        personal_docs_error,
    })
}

pub async fn fetch_position(session: &SessionMaterial, asin: &str) -> Result<ReadingPosition, CloudError> {
    let client = http_client()?;
    if let Some(pos) = fetch_reader_progress(&client, session, asin).await {
        return Ok(pos);
    }
    let url = start_reading_url(session.region, asin);
    let body = send_get(&client, session, &url).await?;
    let mut position = parse_reading_position(&body)?;
    if position.asin.is_empty() {
        position.asin = asin.to_string();
    }
    Ok(position)
}

async fn hydrate_library_progress(client: &reqwest::Client, session: &SessionMaterial, books: &mut [OwnedBook]) {
    let mut tasks = tokio::task::JoinSet::new();
    for (idx, book) in books.iter().enumerate() {
        if book.asin.len() > 12 || book.percentage > 0.0 {
            continue;
        }
        let client = client.clone();
        let session = session.clone();
        let asin = book.asin.clone();
        tasks.spawn(async move {
            let pos = fetch_reader_progress(&client, &session, &asin).await;
            (idx, pos)
        });
    }
    while let Some(Ok((idx, Some(pos)))) = tasks.join_next().await {
        if let Some(book) = books.get_mut(idx) {
            if pos.percentage > 0.0 {
                book.percentage = pos.percentage;
            }
        }
    }
}

pub async fn fetch_reader_progress(
    client: &reqwest::Client,
    session: &SessionMaterial,
    asin: &str,
) -> Option<ReadingPosition> {
    if asin.is_empty() || asin.len() > 12 {
        return None;
    }
    let url = reader_url(session.region, asin);
    let response = client
        .get(&url)
        .header("User-Agent", CHROME_UA)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7")
        .header("Cookie", cookie_header(&session.cookies))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let html = response.text().await.ok()?;
    parse_reader_html_progress(&html, asin)
}

pub fn parse_reader_html_progress(html: &str, asin: &str) -> Option<ReadingPosition> {
    let normalized = html.replace("\\x22", "\"").replace("\\\"", "\"");
    let pos_re = regex::Regex::new(r#""mostRecentPositionRead"\s*:\s*(\d+)"#).ok()?;
    let end_re = regex::Regex::new(r#""endReadingPosition"\s*:\s*(\d+)"#).ok()?;
    let srl_re = regex::Regex::new(r#""srl"\s*:\s*(\d+)"#).ok()?;
    let html = normalized.as_str();
    let pos: i64 = pos_re.captures(html)?.get(1)?.as_str().parse().ok()?;
    let end: i64 = end_re.captures(html)?.get(1)?.as_str().parse().ok()?;
    let srl: i64 = srl_re
        .captures(html)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0);
    let percentage = if pos > 1 && end > 0 && (srl <= 0 || pos > srl) {
        (pos as f64 / end as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Some(ReadingPosition {
        asin: asin.to_string(),
        position: pos,
        percentage,
        sync_time: 0,
    })
}

pub async fn fetch_pdocs(client: &reqwest::Client, session: &SessionMaterial) -> Result<Vec<OwnedBook>, CloudError> {
    let store = session.region.store_origin();
    let pages = [
        "/hz/mycd/digital-console/contentlist/pdocs/dateDsc",
        "/hz/mycd/digital-console/contentlist/pdocs",
        "/hz/mycd/digital-console/contentlist/pdocs/titleAsc",
    ];
    let batches = [
        r#"{"contentType":"KindlePDoc","contentCategoryReference":"pdocs","itemStatusList":["Active"],"isExtendedMYK":false,"fetchCriteria":{"sortOrder":"DESCENDING","sortIndex":"DATE","startIndex":0,"batchSize":50,"totalContentCount":-1},"surfaceType":"Desktop"}"#,
        r#"{"contentType":"KindlePDoc","contentCategoryReference":"pdocs","itemStatusList":["Active"],"isExtendedMYK":true,"fetchCriteria":{"sortOrder":"DESCENDING","sortIndex":"DATE","startIndex":0,"batchSize":100,"totalContentCount":-1},"surfaceType":"Desktop"}"#,
        r#"{"contentType":"KindlePDoc","contentCategoryReference":"pdocs","itemStatusList":["Active"],"isExtendedMYK":false,"fetchCriteria":{"sortOrder":"ASCENDING","sortIndex":"TITLE","startIndex":0,"batchSize":50,"totalContentCount":-1},"surfaceType":"Desktop"}"#,
    ];
    let mut cookies = session.cookies.clone();
    let mut last_error = CloudError::PersonalDocs("não achei o token da loja Amazon".into());
    for (index, page) in pages.iter().enumerate() {
        let page_url = format!("{store}{page}");
        let attempts = if index == 0 { 2 } else { 1 };
        let batch = batches[index.min(batches.len() - 1)];
        for attempt in 0..attempts {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            }
            match fetch_pdocs_once(client, session.region, store, &page_url, batch, &mut cookies).await {
                Ok(books) => return Ok(books),
                Err(CloudError::NeedsReauth) => return Err(CloudError::NeedsReauth),
                Err(error) => last_error = error,
            }
        }
    }
    Err(last_error)
}

async fn fetch_pdocs_once(
    client: &reqwest::Client,
    region: Region,
    store: &str,
    page_url: &str,
    activity_input: &str,
    cookies: &mut Vec<StoredCookie>,
) -> Result<Vec<OwnedBook>, CloudError> {
    let response = client
        .get(page_url)
        .header("User-Agent", CHROME_UA)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7")
        .header("Upgrade-Insecure-Requests", "1")
        .header("Sec-Fetch-Dest", "document")
        .header("Sec-Fetch-Mode", "navigate")
        .header("Sec-Fetch-Site", "none")
        .header("Cookie", cookie_header_for(cookies, true))
        .send()
        .await
        .map_err(|error| CloudError::PersonalDocs(format!("não abri a lista de documentos ({})", error_without_secrets(&error.to_string()))))?;
    let final_url = response.url().to_string();
    let status = response.status();
    absorb_set_cookie(cookies, response.headers());
    let html = response
        .text()
        .await
        .map_err(|error| CloudError::PersonalDocs(format!("resposta vazia da loja ({})", error_without_secrets(&error.to_string()))))?;
    if looks_like_signin(&final_url, &html) {
        return Err(CloudError::NeedsReauth);
    }
    if !status.is_success() {
        return Err(CloudError::PersonalDocs(format!(
            "a loja respondeu {} em {}",
            status.as_u16(),
            url_path_only(&final_url)
        )));
    }
    if let Some(books) = embedded_pdocs(&html, region) {
        return Ok(books);
    }
    let Some(csrf) = extract_csrf(&html) else {
        return Err(CloudError::PersonalDocs(format!(
            "não achei o token da loja em {}",
            url_path_only(&final_url)
        )));
    };
    let ajax_url = format!("{store}/hz/mycd/digital-console/ajax");
    let form_body = format!(
        "activity=GetContentOwnershipData&activityInput={}&clientId=MYCD_WebService&csrfToken={}",
        urlencoding(activity_input),
        urlencoding(&csrf)
    );
    let ajax = client
        .post(&ajax_url)
        .header("User-Agent", CHROME_UA)
        .header("Accept", "application/json, text/javascript, */*; q=0.01")
        .header("Accept-Language", "pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7")
        .header("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
        .header("Origin", store)
        .header("Referer", page_url)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Sec-Fetch-Dest", "empty")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "same-origin")
        .header("Cookie", cookie_header_for(cookies, true))
        .body(form_body)
        .send()
        .await
        .map_err(|error| CloudError::PersonalDocs(format!("a consulta dos documentos falhou ({})", error_without_secrets(&error.to_string()))))?;
    let ajax_url_final = ajax.url().to_string();
    let ajax_status = ajax.status();
    absorb_set_cookie(cookies, ajax.headers());
    let ajax_text = ajax
        .text()
        .await
        .map_err(|error| CloudError::PersonalDocs(format!("resposta vazia dos documentos ({})", error_without_secrets(&error.to_string()))))?;
    if looks_like_signin(&ajax_url_final, &ajax_text) {
        return Err(CloudError::NeedsReauth);
    }
    if !ajax_status.is_success() {
        return Err(CloudError::PersonalDocs(format!(
            "a consulta dos documentos respondeu {}",
            ajax_status.as_u16()
        )));
    }
    parse_pdocs_document(&ajax_text, region).map_err(|_| {
        CloudError::PersonalDocs("a loja não devolveu a lista de documentos".into())
    })
}

pub fn extract_csrf(html: &str) -> Option<String> {
    let normalized = html.replace("\\u0022", "\"").replace("\\\"", "\"").replace("\\x22", "\"");
    let patterns = [
        r#"csrfToken"\s*:\s*"([^"]+)""#,
        r#"csrfToken'\s*:\s*'([^']+)'"#,
        r#"csrfToken\s*[:=]\s*["']([^"']+)""#,
        r#"name=["']csrfToken["'][^>]*value=["']([^"']+)""#,
        r#"value=["']([^"']+)["'][^>]*name=["']csrfToken["']"#,
        r#"anti-csrftoken-a2z["']?\s*[:=]\s*["']([^"']+)""#,
        r#"data-csrf-token=["']([^"']+)""#,
        r#"<meta[^>]+name=["']csrf-token["'][^>]+content=["']([^"']+)""#,
        r#"<meta[^>]+content=["']([^"']+)["'][^>]+name=["']csrf-token["']"#,
    ];
    for pattern in patterns {
        let Ok(regex) = regex::Regex::new(pattern) else {
            continue;
        };
        if let Some(token) = regex.captures(&normalized).and_then(|cap| cap.get(1)).map(|item| item.as_str().trim().to_string()) {
            if csrf_token_ok(&token) {
                return Some(token);
            }
        }
    }
    None
}

fn csrf_token_ok(token: &str) -> bool {
    token.len() >= 8 && token.chars().all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_' || char == '/' || char == '+' || char == '=')
}

pub fn looks_like_signin(url: &str, body: &str) -> bool {
    let url = url.to_ascii_lowercase();
    if url.contains("/ap/signin") || url.contains("/ap/mfa") || url.contains("openid.pape") {
        return true;
    }
    let head: String = body.chars().take(20_000).collect::<String>().to_ascii_lowercase();
    head.contains("ap_signin_form") || head.contains("auth-signin-button") || head.contains("id=\"ap_email\"")
}

fn embedded_pdocs(html: &str, region: Region) -> Option<Vec<OwnedBook>> {
    let marker = "GetContentOwnershipData";
    let idx = html.find(marker)?;
    let start = html[..idx].rfind('{')?;
    let slice = json_object_at(html, start)?;
    parse_pdocs_document(slice, region).ok().filter(|books| !books.is_empty())
}

fn json_object_at(text: &str, start: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    if bytes.get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (offset, byte) in bytes[start..].iter().enumerate() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if *byte == b'\\' {
                escape = true;
                continue;
            }
            if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match *byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return text.get(start..start + offset + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn absorb_set_cookie(cookies: &mut Vec<StoredCookie>, headers: &reqwest::header::HeaderMap) {
    for header in headers.get_all(reqwest::header::SET_COOKIE) {
        let Ok(raw) = header.to_str() else {
            continue;
        };
        let Some(pair) = raw.split(';').next() else {
            continue;
        };
        let Some((name, value)) = pair.trim().split_once('=') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim().trim_matches('"');
        if name.is_empty() || value.is_empty() || value == "-" {
            continue;
        }
        let domain = raw
            .split(';')
            .filter_map(|part| part.trim().strip_prefix("Domain=").or_else(|| part.trim().strip_prefix("domain=")))
            .next()
            .unwrap_or("")
            .to_string();
        if let Some(existing) = cookies.iter_mut().find(|cookie| cookie.name.eq_ignore_ascii_case(name) && cookie.domain.eq_ignore_ascii_case(&domain)) {
            existing.value = value.to_string();
        } else {
            cookies.push(StoredCookie {
                name: name.to_string(),
                value: value.to_string(),
                domain,
            });
        }
    }
}

fn url_path_only(url: &str) -> String {
    url.split('?').next().unwrap_or(url).chars().take(180).collect()
}

fn error_without_secrets(message: &str) -> String {
    let mut text = message.split("cookie").next().unwrap_or(message).trim().to_string();
    if text.len() > 160 {
        text.truncate(160);
    }
    text
}

pub fn parse_pdocs_document(body: &str, region: Region) -> Result<Vec<OwnedBook>, CloudError> {
    let value: Value = serde_json::from_str(body).map_err(|error| CloudError::Json(error.to_string()))?;
    let items = value
        .get("GetContentOwnershipData")
        .and_then(|node| node.get("items"))
        .and_then(|node| node.as_array())
        .or_else(|| items_of(&value))
        .ok_or(CloudError::EmptyLibrary)?;
    let mut books = Vec::new();
    for item in items {
        if let Some(book) = book_from_item(item, region) {
            if !books.iter().any(|existing: &OwnedBook| existing.title.eq_ignore_ascii_case(&book.title)) {
                books.push(book);
            }
        }
    }
    Ok(books)
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
    let raw_title = first_string(item, &["title", "productTitle"]).unwrap_or_else(|| asin.clone());
    let title = clean_title(&raw_title);
    let author = authors_of(item);
    let cover_url = cover_of(item);
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
    let names = match item.get("authors").or_else(|| item.get("author")).or_else(|| item.get("sortableAuthors")) {
        Some(Value::String(value)) => vec![clean_author(value)],
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(|entry| entry.as_str().or_else(|| entry.get("name").and_then(|name| name.as_str())))
            .map(clean_author)
            .filter(|name| !name.is_empty())
            .collect(),
        _ => Vec::new(),
    };
    if names.is_empty() {
        "Autor desconhecido".into()
    } else {
        names.join(", ")
    }
}

pub fn decode_html_entities(input: &str) -> String {
    let mut s = input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&atilde;", "ã")
        .replace("&Atilde;", "Ã")
        .replace("&otilde;", "õ")
        .replace("&Otilde;", "Õ")
        .replace("&aacute;", "á")
        .replace("&Aacute;", "Á")
        .replace("&eacute;", "é")
        .replace("&Eacute;", "É")
        .replace("&iacute;", "í")
        .replace("&Iacute;", "Í")
        .replace("&oacute;", "ó")
        .replace("&Oacute;", "Ó")
        .replace("&uacute;", "ú")
        .replace("&Uacute;", "Ú")
        .replace("&acirc;", "â")
        .replace("&Acirc;", "Â")
        .replace("&ecirc;", "ê")
        .replace("&Ecirc;", "Ê")
        .replace("&ocirc;", "ô")
        .replace("&Ocirc;", "Ô")
        .replace("&agrave;", "à")
        .replace("&Agrave;", "À")
        .replace("&ccedil;", "ç")
        .replace("&Ccedil;", "Ç")
        .replace("&uuml;", "ü")
        .replace("&Uuml;", "Ü")
        .replace("&ntilde;", "ñ")
        .replace("&Ntilde;", "Ñ")
        .replace("&rsquo;", "’")
        .replace("&lsquo;", "‘")
        .replace("&ldquo;", "“")
        .replace("&rdquo;", "”")
        .replace("&ndash;", "–")
        .replace("&mdash;", "—")
        .replace("&hellip;", "…");
    if s.contains("&#") {
        if let Ok(dec_re) = regex::Regex::new(r"&#(\d+);") {
            s = dec_re
                .replace_all(&s, |caps: &regex::Captures| {
                    caps.get(1)
                        .and_then(|m| m.as_str().parse::<u32>().ok())
                        .and_then(char::from_u32)
                        .map(|c| c.to_string())
                        .unwrap_or_default()
                })
                .into_owned();
        }
        if let Ok(hex_re) = regex::Regex::new(r"(?i)&#x([0-9a-f]+);") {
            s = hex_re
                .replace_all(&s, |caps: &regex::Captures| {
                    caps.get(1)
                        .and_then(|m| u32::from_str_radix(m.as_str(), 16).ok())
                        .and_then(char::from_u32)
                        .map(|c| c.to_string())
                        .unwrap_or_default()
                })
                .into_owned();
        }
    }
    s.replace("c\u{0327}", "ç")
        .replace("C\u{0327}", "Ç")
        .replace("a\u{0303}", "ã")
        .replace("A\u{0303}", "Ã")
        .replace("o\u{0303}", "õ")
        .replace("O\u{0303}", "Õ")
}

pub fn clean_title(raw: &str) -> String {
    let mut s = decode_html_entities(raw).trim().to_string();
    if let Some((head, _)) = s.split_once(" -- ") {
        if head.trim().len() >= 3 {
            s = head.trim().to_string();
        }
    }
    let suffixes = [
        "(Portuguese Edition)",
        "(English Edition)",
        "(Spanish Edition)",
        "(Edição em Português)",
        "(Edição Portuguesa)",
        "[Print Replica]",
    ];
    loop {
        let trimmed = s.trim_end();
        let mut changed = false;
        for suffix in suffixes {
            let cut = trimmed.len().saturating_sub(suffix.len());
            if trimmed.len() > suffix.len()
                && trimmed.is_char_boundary(cut)
                && trimmed[cut..].eq_ignore_ascii_case(suffix)
            {
                s = trimmed[..trimmed.len() - suffix.len()].trim_end().to_string();
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    if !s.contains(' ') && (s.contains('-') || s.contains('_')) {
        s = s.replace(['-', '_'], " ");
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_author(raw: &str) -> String {
    let decoded = decode_html_entities(raw);
    let trimmed = decoded.trim().trim_end_matches(':').trim();
    if trimmed.eq_ignore_ascii_case("unknown") || trimmed.to_ascii_lowercase().contains("preparado para") {
        return "Documento pessoal".into();
    }
    let primary = if trimmed.contains(':') {
        trimmed
            .split(':')
            .map(str::trim)
            .find(|part| !part.is_empty())
            .unwrap_or(trimmed)
    } else {
        trimmed
    };
    let primary = primary.trim().trim_end_matches(':').trim();
    flip_last_first(primary)
}

fn flip_last_first(name: &str) -> String {
    let Some((last, first)) = name.split_once(',') else {
        return name.trim().to_string();
    };
    if first.contains(',') {
        return name.trim().to_string();
    }
    let last = last.trim();
    let first = first.trim().trim_end_matches(':').trim();
    if first.is_empty() || last.is_empty() {
        return name.trim().to_string();
    }
    format!("{first} {last}")
}

fn cover_of(item: &Value) -> String {
    for key in ["productUrl", "coverUrl", "imageUrl", "productImage", "largeCoverUrl", "thumbUrl"] {
        let Some(value) = item.get(key).and_then(|value| value.as_str()) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() || value.contains("MYX-Docs-Icons") {
            continue;
        }
        if key == "productUrl" && !looks_like_cover(value) {
            continue;
        }
        return upgrade_cover(value);
    }
    String::new()
}

fn looks_like_cover(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("media-amazon.com")
        || lower.contains("ssl-images-amazon.com")
        || lower.contains("images-amazon.com")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
}

fn upgrade_cover(url: &str) -> String {
    let Some(marker) = url.find("._") else {
        return url.to_string();
    };
    let rest = &url[marker + 2..];
    let Some(end) = rest.find('_') else {
        return url.to_string();
    };
    let token = &rest[..end];
    let digits: String = token.chars().filter(|char| char.is_ascii_digit()).collect();
    let Ok(size) = digits.parse::<u32>() else {
        return url.to_string();
    };
    let kind = token.trim_end_matches(|char: char| char.is_ascii_digit());
    if size < 600 && matches!(kind, "SY" | "SX" | "SL" | "SS" | "US" | "UL") {
        let suffix = &rest[end + 1..];
        return format!("{}._SL800_{suffix}", &url[..marker]);
    }
    url.to_string()
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
#kindleReaderHeader, #kindle_menu, #navbar, #navBar,
.a-navbar, #nav-belt, #navbar-main { visibility: hidden !important; }
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

    #[test]
    fn store_cookie_wins_over_reader_host_cookie() {
        let cookies = vec![
            StoredCookie { name: "session-id".into(), value: "from-reader".into(), domain: "ler.amazon.com.br".into() },
            StoredCookie { name: "session-id".into(), value: "from-store".into(), domain: ".amazon.com.br".into() },
            StoredCookie { name: "at-acbbr".into(), value: "only-reader".into(), domain: "ler.amazon.com.br".into() },
        ];
        let store = cookie_header_for(&cookies, true);
        assert!(store.contains("session-id=from-store"));
        assert!(!store.contains("from-reader"));
        assert!(store.contains("at-acbbr=only-reader"));
        let reader = cookie_header_for(&cookies, false);
        assert!(reader.contains("session-id=from-reader"));
        assert!(!reader.contains("from-store"));
    }

    #[test]
    fn csrf_and_signin_page_are_detected() {
        let html = r#"<script>var config = {"csrfToken":"abcDEF-12345678"};</script>"#;
        assert_eq!(extract_csrf(html).as_deref(), Some("abcDEF-12345678"));
        let meta = r#"<meta name="csrf-token" content="metaTOKEN1234">"#;
        assert_eq!(extract_csrf(meta).as_deref(), Some("metaTOKEN1234"));
        assert!(looks_like_signin("https://www.amazon.com.br/ap/signin?openid=1", ""));
        assert!(looks_like_signin("https://www.amazon.com.br/hz/mycd", "<form id=\"ap_signin_form\">"));
        assert!(!looks_like_signin(
            "https://www.amazon.com.br/hz/mycd/digital-console/contentlist/pdocs/dateDsc",
            r#"{"GetContentOwnershipData":{"items":[]}}"#,
        ));
    }

    #[test]
    fn personal_doc_payload_parses() {
        let body = r#"{"GetContentOwnershipData":{"items":[{"asin":"0123456789abcdef0123456789abcdef","title":"Codigo limpo (Portuguese Edition)","authors":["Robert C. Martin"]}]}}"#;
        let books = parse_pdocs_document(body, Region::Br).expect("pdocs");
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "Codigo limpo");
        assert!(books[0].asin.len() > 12);
    }

    #[test]
    fn real_brazil_library_has_covers_and_clean_authors() {
        let body = include_str!("../fixtures/amazon-br-library.json");
        assert_library_authors_and_covers(body);
        if let Ok(live) = std::fs::read_to_string("/cursor/stores/bot/real-amazon-br-library.json") {
            assert_library_authors_and_covers(&live);
        }
    }

    fn assert_library_authors_and_covers(body: &str) {
        let (books, _) = parse_library_document(body, Region::Br).expect("biblioteca");
        assert_eq!(books.len(), 10);
        let expected = [
            ("B084RMWS4D", "Arthur Schopenhauer"),
            ("B0GX3FDWTX", "Homero de Esmirna"),
            ("B088GVJXTN", "Santo Agostinho"),
            ("B0BY15WKL4", "Fiódor Dostoiévski"),
            ("B07522WHGC", "Robert Greene"),
            ("B0HFPXM52X", "Tech Cert Academy"),
            ("B091BD2MJ9", "Caio Costa"),
            ("B08B81415B", "Fiódor Dostoiévski"),
            ("B08NYZRF16", "Robert W. Chambers"),
            ("B09Z8K9MYQ", "Yogesh Mali"),
        ];
        for (asin, author) in expected {
            let book = books.iter().find(|book| book.asin == asin).unwrap_or_else(|| panic!("{asin}"));
            assert_eq!(book.author, author, "{}", book.title);
            assert!(!book.author.contains(':'), "{}", book.author);
            assert!(
                book.cover_url.starts_with("https://m.media-amazon.com/"),
                "{}",
                book.cover_url
            );
            assert!(!book.cover_url.is_empty());
        }
    }
}
