mod clippings;
mod db;
mod epub_meta;
mod kindle_cloud;
mod lookup;
mod model;
mod overview;

use std::collections::HashSet;
use std::fs;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{
    webview::WebviewBuilder, AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalSize, State, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

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
    open_amazon_window(&app, "amazon-login", "Entrar na Amazon", &login_url(parsed), "")?;
    spawn_login_watcher(app, parsed);
    Ok(())
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
async fn cloud_sync(app: AppHandle, region: Option<String>) -> Result<model::Snapshot, String> {
    let material = {
        let state = app.state::<AppState>();
        let db = state.db.lock().map_err(|error| error.to_string())?;
        let mut session = db.session()?;
        if let Some(region) = region {
            session.region = Region::parse(&region).as_str().into();
            db.save_session(&session)?;
        }
        material_from(&session)?
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

fn cloud_reader_exit_url(url: &str) -> bool {
    if url.contains("kl-close") {
        return true;
    }
    // Match the library path only. A sign-in URL often echoes `/kindle-library`
    // inside a return_to query, and that must not dismiss the reader.
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.contains("/kindle-library")
}

#[tauri::command]
fn cloud_open_reader(app: AppHandle, asin: String, title: Option<String>) -> Result<(), String> {
    let _ = title;
    let (region, session) = {
        let state = app.state::<AppState>();
        let db = state.db.lock().map_err(|error| error.to_string())?;
        let session = db.session()?;
        (Region::parse(&session.region), session)
    };
    if let Some(existing) = app.get_webview("cloud-reader") {
        let _ = existing.close();
    }
    // Cookies go into the shared WebKit jar before the reader is created.
    // Hiding "main" lets "cloud-reader" take the whole GtkBox (Linux packs every
    // webview with expand+fill and ignores LogicalPosition).
    if let Some(main_wv) = app.get_webview("main") {
        push_session_cookies(&main_wv, &session);
        let _ = main_wv.hide();
    }
    let window = app.get_window("main").ok_or_else(|| "Janela principal ausente".to_string())?;
    // Linux packs the child into a GtkBox. add_child's size becomes a GTK
    // size_request, which would stop the window from shrinking or tiling.
    // A small request lets expand+fill follow the real allocation.
    #[cfg(target_os = "linux")]
    let child_size = LogicalSize::new(200.0, 200.0);
    #[cfg(not(target_os = "linux"))]
    let child_size = {
        let scale = window.scale_factor().unwrap_or(1.0).max(0.5);
        let physical = window.inner_size().map_err(|error| error.to_string())?;
        LogicalSize::new(physical.width as f64 / scale, physical.height as f64 / scale)
    };
    let target = reader_url(region, &asin);
    let reader: url::Url = target.parse().map_err(|error: url::ParseError| error.to_string())?;
    let blank: url::Url = "about:blank".parse().map_err(|error: url::ParseError| error.to_string())?;
    let navigate_app = app.clone();
    let builder = WebviewBuilder::new("cloud-reader", WebviewUrl::External(blank))
        .user_agent(kindle_cloud::CHROME_UA)
        .initialization_script(reader_init_script())
        .on_navigation(move |url| {
            if cloud_reader_exit_url(url.as_str()) {
                let app = navigate_app.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(30)).await;
                    close_cloud_reader_internal(&app);
                });
                return false;
            }
            true
        });
    let webview = window
        .add_child(builder, LogicalPosition::new(0.0, 0.0), child_size)
        .map_err(|error| error.to_string())?;
    push_session_cookies(&webview, &session);
    let _ = webview.navigate(reader);
    let _ = webview.set_focus();
    spawn_reader_close_watcher(app);
    Ok(())
}

fn close_cloud_reader_internal(app: &AppHandle) {
    if let Some(main_wv) = app.get_webview("main") {
        let _ = main_wv.show();
        let _ = main_wv.set_focus();
    }
    if let Some(webview) = app.get_webview("cloud-reader") {
        let _ = webview.close();
    }
    let _ = app.emit("cloud-reader-closed", ());
}

fn spawn_reader_close_watcher(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let Some(webview) = app.get_webview("cloud-reader") else {
                break;
            };
            let page = webview.url().map(|url| url.to_string()).unwrap_or_default();
            if cloud_reader_exit_url(&page) {
                close_cloud_reader_internal(&app);
                break;
            }
        }
    });
}

#[tauri::command]
fn cloud_close_reader(app: AppHandle) -> Result<(), String> {
    close_cloud_reader_internal(&app);
    Ok(())
}

#[tauri::command]
fn set_fullscreen(app: AppHandle, enabled: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.set_fullscreen(enabled).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn spawn_login_watcher(app: AppHandle, region: Region) {
    tauri::async_runtime::spawn(async move {
        let mut library_misses = 0u8;
        loop {
            tokio::time::sleep(Duration::from_millis(900)).await;
            let Some(window) = app.get_webview_window("amazon-login") else {
                break;
            };
            let cookies = collect_login_cookies(&window, region);
            let page = window.url().map(|url| url.to_string()).unwrap_or_default();
            if !login_is_ready(&cookies, &page) {
                continue;
            }
            let mut uhura = String::new();
            let mut device = String::new();
            absorb_tokens(&cookies, &mut uhura, &mut device);
            let material = SessionMaterial {
                region,
                cookies,
                uhura,
                device_session_token: device,
            };
            match fetch_library(&material).await {
                Ok(books) => {
                    if finish_login(&app, &material, Some(&books)).is_ok() {
                        let _ = window.close();
                        let _ = app.emit("library-changed", ());
                    }
                    break;
                }
                Err(_) if page.contains("/kindle-library") && !page.contains("/ap/") => {
                    library_misses += 1;
                    if library_misses >= 3 && finish_login(&app, &material, None).is_ok() {
                        let _ = window.close();
                        let _ = app.emit("library-changed", ());
                        break;
                    }
                }
                Err(_) => {}
            }
        }
    });
}

fn finish_login(app: &AppHandle, material: &SessionMaterial, books: Option<&[kindle_cloud::OwnedBook]>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let db = state.db.lock().map_err(|error| error.to_string())?;
    let mut session = db.session()?;
    session.region = material.region.as_str().into();
    session.cookies = material.cookies.clone();
    session.uhura = material.uhura.clone();
    session.device_session_token = material.device_session_token.clone();
    db.save_session(&session)?;
    if let Some(books) = books {
        db.upsert_cloud(books)?;
    }
    Ok(())
}

fn collect_login_cookies(window: &WebviewWindow, region: Region) -> Vec<StoredCookie> {
    let mut urls = vec![
        region.origin().to_string(),
        "https://www.amazon.com.br".into(),
        "https://ler.amazon.com.br".into(),
        "https://www.amazon.com".into(),
        "https://read.amazon.com".into(),
        "https://amazon.com.br".into(),
        "https://amazon.com".into(),
    ];
    if let Ok(current) = window.url() {
        urls.push(origin_of(&current));
        urls.push(current.to_string());
    }
    let mut cookies = Vec::new();
    let mut seen = HashSet::new();
    for raw in urls {
        let Ok(url) = url::Url::parse(&raw) else {
            continue;
        };
        let Ok(found) = window.cookies_for_url(url) else {
            continue;
        };
        for cookie in found {
            let name = cookie.name().to_string();
            let value = cookie.value().to_string();
            if value.is_empty() || !seen.insert(name.to_ascii_lowercase()) {
                continue;
            }
            cookies.push(StoredCookie {
                name,
                value,
                domain: cookie.domain().unwrap_or("").to_string(),
            });
        }
    }
    cookies
}

fn origin_of(url: &url::Url) -> String {
    format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""))
}

fn login_is_ready(cookies: &[StoredCookie], page: &str) -> bool {
    if page.contains("/ap/") {
        return false;
    }
    let has = |name: &str| cookies.iter().any(|cookie| cookie.name.eq_ignore_ascii_case(name) && !cookie.value.is_empty());
    let strong = has("at-acbbr") || has("at-main") || has("sess-at-acbbr") || has("sess-at-main");
    if strong && has("session-id") {
        return true;
    }
    let on_library = page.contains("/kindle-library");
    on_library && (has("ubid-acbbr") || has("ubid-main"))
}

fn push_session_cookies(webview: &tauri::Webview, session: &StoredSession) {
    let domains: &[&str] = match Region::parse(&session.region) {
        Region::Br => &[".amazon.com.br", "amazon.com.br", "ler.amazon.com.br", "www.amazon.com.br"],
        Region::Us => &[".amazon.com", "amazon.com", "read.amazon.com", "www.amazon.com"],
    };
    for stored in &session.cookies {
        if stored.name.is_empty() || stored.value.is_empty() {
            continue;
        }
        for domain in domains {
            let cookie = tauri::webview::cookie::Cookie::build((stored.name.as_str(), stored.value.as_str()))
                .domain(*domain)
                .path("/")
                .secure(true)
                .build();
            let _ = webview.set_cookie(cookie);
        }
    }
}

fn reader_init_script() -> String {
    let css = format!(
        "{READER_QUIET_CSS}\n#kl-back{{position:fixed;top:12px;left:16px;z-index:2147483647;background:transparent;color:inherit;border:1px solid rgba(128,128,128,0.35);border-radius:999px;padding:7px 14px;font:500 13px/1 -apple-system,BlinkMacSystemFont,\"Helvetica Neue\",Roboto,sans-serif;cursor:pointer;backdrop-filter:blur(6px);}}"
    );
    let css = serde_json::to_string(&css).unwrap_or_else(|_| "\"\"".into());
    r#"(function(){
if (window.__klBound) return;
window.__klBound = true;
const css = CSS_PLACEHOLDER;
let leaving = false;
const goHome = () => {
  if (leaving) return;
  leaving = true;
  try {
    const invoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    if (invoke) invoke('cloud_close_reader');
  } catch (e) {}
  window.location.replace('/kindle-library#kl-close');
};
const rewriteText = (root) => {
  if (!root) return;
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let node;
  while ((node = walker.nextNode())) {
    if (node.nodeValue && node.nodeValue.indexOf('Kindle Library') !== -1) {
      node.nodeValue = node.nodeValue.replace('Kindle Library', 'Biblioteca');
    }
  }
  const elements = root.querySelectorAll ? root.querySelectorAll('*') : [];
  elements.forEach((el) => {
    if (el.shadowRoot) rewriteText(el.shadowRoot);
  });
};
const bindNative = (host) => {
  host.setAttribute('title', 'Voltar à Biblioteca');
  host.setAttribute('aria-label', 'Voltar à Biblioteca');
  host.querySelectorAll('ion-button, button, a').forEach((el) => {
    el.setAttribute('title', 'Voltar à Biblioteca');
    el.setAttribute('aria-label', 'Voltar à Biblioteca');
  });
  rewriteText(host);
  if (host.dataset.klBound === '1') return;
  host.dataset.klBound = '1';
  host.addEventListener('click', (event) => {
    event.preventDefault();
    event.stopPropagation();
    goHome();
  }, true);
};
const mount = () => {
  const root = document.documentElement;
  if (!root) return;
  let s = document.getElementById('kl-quiet');
  if (!s) {
    s = document.createElement('style');
    s.id = 'kl-quiet';
    root.appendChild(s);
  }
  if (s.textContent !== css) s.textContent = css;
  const native = document.querySelector('.top-chrome__back-button');
  let fallback = document.getElementById('kl-back');
  if (native) {
    bindNative(native);
    if (fallback) fallback.style.setProperty('display', 'none', 'important');
    return;
  }
  if (!fallback) {
    fallback = document.createElement('button');
    fallback.id = 'kl-back';
    fallback.type = 'button';
    fallback.textContent = '← Biblioteca';
    fallback.addEventListener('click', (event) => {
      event.preventDefault();
      event.stopPropagation();
      goHome();
    });
    root.appendChild(fallback);
  }
  fallback.style.removeProperty('display');
};
const onKey = (event) => {
  if (event.key === 'Escape') {
    event.preventDefault();
    goHome();
  }
};
window.addEventListener('click', (event) => {
  const button = document.getElementById('kl-back');
  if (!button || button.style.display === 'none') return;
  if (document.querySelector('.top-chrome__back-button')) return;
  const rect = button.getBoundingClientRect();
  const inside = event.clientX >= rect.left && event.clientX <= rect.right && event.clientY >= rect.top && event.clientY <= rect.bottom;
  if (event.target === button || button.contains(event.target) || inside) {
    event.preventDefault();
    event.stopPropagation();
    goHome();
  }
}, true);
window.addEventListener('keydown', onKey, true);
document.addEventListener('keydown', onKey, true);
const boot = () => {
  mount();
  if (document.documentElement) new MutationObserver(mount).observe(document.documentElement, {childList:true, subtree:true});
};
if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
else boot();
})();"#
        .replace("CSS_PLACEHOLDER", &css)
}

fn resize_cloud_reader(app: &AppHandle, size: PhysicalSize<u32>) {
    #[cfg(not(target_os = "linux"))]
    {
        let Some(webview) = app.get_webview("cloud-reader") else {
            return;
        };
        let Some(window) = app.get_window("main") else {
            return;
        };
        let scale = window.scale_factor().unwrap_or(1.0).max(0.5);
        let width = (size.width as f64 / scale).max(320.0);
        let height = (size.height as f64 / scale).max(240.0);
        let _ = webview.set_position(LogicalPosition::new(0.0, 0.0));
        let _ = webview.set_size(LogicalSize::new(width, height));
    }
    #[cfg(target_os = "linux")]
    {
        let _ = (app, size);
    }
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
        .inner_size(920.0, 580.0)
        .user_agent(kindle_cloud::CHROME_UA)
        .initialization_script("if (!window.chrome) { window.chrome = { runtime: {} }; }");
    if !script.is_empty() {
        builder = builder.initialization_script(script);
    }
    builder.build().map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{cloud_reader_exit_url, reader_init_script};

    #[test]
    fn reader_exit_url_ignores_signin_query() {
        assert!(cloud_reader_exit_url("https://ler.amazon.com.br/kindle-library#kl-close"));
        assert!(cloud_reader_exit_url("https://ler.amazon.com.br/kindle-library"));
        assert!(cloud_reader_exit_url("/kindle-library#kl-close"));
        assert!(!cloud_reader_exit_url("https://ler.amazon.com.br/?asin=B084RMWS4D"));
        assert!(!cloud_reader_exit_url(
            "https://www.amazon.com.br/ap/signin?openid.return_to=https://ler.amazon.com.br/kindle-library"
        ));
    }

    #[test]
    fn reader_script_returns_home_and_balances_braces() {
        let script = reader_init_script();
        assert!(script.contains("cloud_close_reader"));
        assert!(script.contains("← Biblioteca"));
        assert!(script.contains("Kindle Library"));
        assert!(script.contains("Voltar à Biblioteca"));
        assert!(script.contains("/kindle-library#kl-close"));
        assert!(script.contains("Escape"));
        assert!(script.contains(".top-chrome__back-button"));
        assert!(script.contains("display', 'none', 'important'"));
        assert!(script.contains("border-radius:999px"));
        assert!(script.contains("background:transparent"));
        assert!(!script.contains("background:#F6F3EC"));
        assert!(!script.contains(".top-chrome__back-button,"));
        assert!(script.contains("#kindleReaderHeader"));
        assert!(!script.contains("#reader-header"));
        let mut depth = 0i32;
        for ch in script.chars() {
            if ch == '{' {
                depth += 1;
            }
            if ch == '}' {
                depth -= 1;
            }
            assert!(depth >= 0, "{script}");
        }
        assert_eq!(depth, 0, "{script}");
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|error| error.to_string())?;
            fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
            let db = Db::open(&dir.join("library.db")).map_err(|error| error.to_string())?;
            let _ = db.scan();
            app.manage(AppState { db: Mutex::new(db) });
            if let Some(window) = app.get_window("main") {
                let resized = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Resized(size) = event {
                        let app = resized.clone();
                        let size = *size;
                        tauri::async_runtime::spawn(async move {
                            resize_cloud_reader(&app, size);
                        });
                    }
                });
            }
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
            cloud_close_reader,
            set_fullscreen
        ])
        .run(tauri::generate_context!())
        .expect("erro ao abrir Kindle para Linux");
}
