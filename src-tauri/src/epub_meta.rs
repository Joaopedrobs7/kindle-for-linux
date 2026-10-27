use std::fs::File;
use std::io::Read;
use std::path::Path;

use base64::Engine;
use regex::Regex;
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct EpubMeta {
    pub title: String,
    pub author: String,
    pub cover_data_url: String,
}

pub fn read_epub_meta(path: &Path) -> Result<EpubMeta, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|error| error.to_string())?;
    let container = read_zip_string(&mut archive, "META-INF/container.xml")?;
    let opf_path = attr(&container, "full-path").ok_or("EPUB sem rootfile")?;
    let opf = read_zip_string(&mut archive, &opf_path)?;
    let title = tag_text(&opf, "title").unwrap_or_else(|| fallback_title(path));
    let author = tag_text(&opf, "creator").unwrap_or_else(|| "Autor desconhecido".into());
    let cover = cover_data_url(&mut archive, &opf, &opf_path);
    Ok(EpubMeta {
        title,
        author,
        cover_data_url: cover,
    })
}

pub fn epub_plain_text(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut chunks = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name().to_string();
        if !(name.ends_with(".xhtml") || name.ends_with(".html") || name.ends_with(".htm")) {
            continue;
        }
        let mut raw = String::new();
        entry.read_to_string(&mut raw).map_err(|error| error.to_string())?;
        chunks.push(strip_tags(&raw));
    }
    Ok(chunks.join("\n"))
}

fn cover_data_url(archive: &mut ZipArchive<File>, opf: &str, opf_path: &str) -> String {
    let href = cover_href(opf);
    let Some(href) = href else {
        return String::new();
    };
    let base = opf_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let name = if base.is_empty() {
        href
    } else {
        format!("{base}/{href}")
    };
    let Ok(mut entry) = archive.by_name(&name) else {
        return String::new();
    };
    if entry.size() > 450_000 {
        return String::new();
    }
    let mut bytes = Vec::new();
    if entry.read_to_end(&mut bytes).is_err() {
        return String::new();
    }
    let mime = if name.ends_with(".jpg") || name.ends_with(".jpeg") {
        "image/jpeg"
    } else if name.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:{mime};base64,{encoded}")
}

fn cover_href(opf: &str) -> Option<String> {
    let cover_id = Regex::new(r#"<meta[^>]*name\s*=\s*"cover"[^>]*content\s*=\s*"([^"]+)""#)
        .ok()?
        .captures(opf)
        .or_else(|| {
            Regex::new(r#"<meta[^>]*content\s*=\s*"([^"]+)"[^>]*name\s*=\s*"cover""#)
                .ok()?
                .captures(opf)
        })
        .and_then(|cap| cap.get(1).map(|item| item.as_str().to_string()));
    if let Some(id) = cover_id {
        let pattern = format!(r#"<item[^>]*id\s*=\s*"{id}"[^>]*href\s*=\s*"([^"]+)""#);
        if let Some(cap) = Regex::new(&pattern).ok()?.captures(opf) {
            return cap.get(1).map(|item| item.as_str().to_string());
        }
        let pattern = format!(r#"<item[^>]*href\s*=\s*"([^"]+)"[^>]*id\s*=\s*"{id}""#);
        if let Some(cap) = Regex::new(&pattern).ok()?.captures(opf) {
            return cap.get(1).map(|item| item.as_str().to_string());
        }
    }
    Regex::new(r#"<item[^>]*properties\s*=\s*"cover-image"[^>]*href\s*=\s*"([^"]+)""#)
        .ok()?
        .captures(opf)
        .and_then(|cap| cap.get(1).map(|item| item.as_str().to_string()))
}

fn read_zip_string(archive: &mut ZipArchive<File>, name: &str) -> Result<String, String> {
    let mut entry = archive.by_name(name).map_err(|error| error.to_string())?;
    let mut raw = String::new();
    entry.read_to_string(&mut raw).map_err(|error| error.to_string())?;
    Ok(raw)
}

fn attr(xml: &str, name: &str) -> Option<String> {
    let pattern = format!(r#"{name}\s*=\s*"([^"]+)""#);
    Regex::new(&pattern)
        .ok()?
        .captures(xml)
        .and_then(|cap| cap.get(1).map(|item| item.as_str().to_string()))
}

fn tag_text(xml: &str, local: &str) -> Option<String> {
    let pattern = format!(r"(?s)<(?:[\w-]+:)?{local}[^>]*>(.*?)</(?:[\w-]+:)?{local}>");
    let text = Regex::new(&pattern).ok()?.captures(xml)?.get(1)?.as_str();
    let clean = strip_tags(text);
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.is_empty() {
        None
    } else {
        Some(clean)
    }
}

fn strip_tags(value: &str) -> String {
    let without = Regex::new(r"(?s)<[^>]+>")
        .map(|pattern| pattern.replace_all(value, " ").into_owned())
        .unwrap_or_else(|_| value.to_string());
    without
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn fallback_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Sem título")
        .replace(['-', '_'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sample_title_and_text() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/amostra.epub");
        let meta = read_epub_meta(&path).expect("amostra");
        assert_eq!(meta.title, "O Farol da Ilha Rasa");
        assert_eq!(meta.author, "Edição de amostra");
        assert!(meta.cover_data_url.starts_with("data:image/png"));
        let text = epub_plain_text(&path).expect("texto");
        assert!(text.contains("Ilha Rasa"));
        assert!(text.contains("cadernos de três gerações"));
    }
}
