use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub word: String,
    pub part_of_speech: String,
    pub senses: Vec<String>,
    pub wikipedia_title: String,
    pub wikipedia_extract: String,
    pub wikipedia_url: String,
    pub source: String,
}

pub fn define(word: &str) -> Definition {
    let clean = word.split_whitespace().next().unwrap_or(word).trim_matches(|ch: char| !ch.is_alphanumeric());
    if let Some((part, text, context)) = local_entry(clean) {
        return Definition {
            word: clean.to_string(),
            part_of_speech: part.into(),
            senses: vec![text.into()],
            wikipedia_title: clean.to_string(),
            wikipedia_extract: context.into(),
            wikipedia_url: String::new(),
            source: "local".into(),
        };
    }
    Definition {
        word: clean.to_string(),
        part_of_speech: String::new(),
        senses: Vec::new(),
        wikipedia_title: String::new(),
        wikipedia_extract: String::new(),
        wikipedia_url: String::new(),
        source: "none".into(),
    }
}

pub fn parse_wiktionary(body: &str) -> (String, Vec<String>) {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return (String::new(), Vec::new());
    };
    let mut part = String::new();
    let mut senses = Vec::new();
    let groups = value
        .get("pt")
        .or_else(|| value.get("en"))
        .and_then(|item| item.as_array())
        .cloned()
        .unwrap_or_default();
    for group in groups {
        if part.is_empty() {
            if let Some(label) = group.get("partOfSpeech").and_then(|item| item.as_str()) {
                part = label.to_string();
            }
        }
        let Some(definitions) = group.get("definitions").and_then(|item| item.as_array()) else {
            continue;
        };
        for definition in definitions {
            if let Some(text) = definition.get("definition").and_then(|item| item.as_str()) {
                let clean = strip_tags(text);
                if !clean.is_empty() {
                    senses.push(clean);
                }
            }
        }
    }
    senses.truncate(4);
    (part, senses)
}

pub fn parse_wikipedia(body: &str) -> (String, String, String) {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return (String::new(), String::new(), String::new());
    };
    let title = value.get("title").and_then(|item| item.as_str()).unwrap_or("").to_string();
    let extract = value.get("extract").and_then(|item| item.as_str()).unwrap_or("").to_string();
    let url = value
        .pointer("/content_urls/desktop/page")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .to_string();
    (title, extract, url)
}

pub async fn lookup_online(word: &str) -> Definition {
    let mut result = define(word);
    let client = match reqwest::Client::builder().timeout(std::time::Duration::from_secs(8)).build() {
        Ok(client) => client,
        Err(_) => return result,
    };
    let encoded = urlencoding(result.word.as_str());
    if result.senses.is_empty() {
        let url = format!("https://pt.wiktionary.org/api/rest_v1/page/definition/{encoded}");
        if let Ok(body) = client.get(url).send().await.and_then(|response| response.error_for_status()) {
            if let Ok(text) = body.text().await {
                let (part, senses) = parse_wiktionary(&text);
                if !senses.is_empty() {
                    result.senses = senses;
                    if result.part_of_speech.is_empty() {
                        result.part_of_speech = part.to_lowercase();
                    }
                    result.source = "wiktionary".into();
                }
            }
        }
    }
    let wiki = format!("https://pt.wikipedia.org/api/rest_v1/page/summary/{encoded}");
    if let Ok(body) = client.get(wiki).send().await.and_then(|response| response.error_for_status()) {
        if let Ok(text) = body.text().await {
            let (title, extract, url) = parse_wikipedia(&text);
            if !extract.is_empty() {
                result.wikipedia_title = title;
                result.wikipedia_extract = extract;
                result.wikipedia_url = url;
            }
        }
    }
    result
}

fn local_entry(word: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let key = fold(word);
    match key.as_str() {
        "farol" => Some((
            "substantivo masculino",
            "Torre com um facho de luz que orienta quem vem pelo mar.",
            "Estrutura costeira que projeta luz para orientar a navegação durante a noite.",
        )),
        "mare" => Some((
            "substantivo feminino",
            "Movimento periódico do mar, subida e descida da água.",
            "Subida e descida do nível do mar ao longo do dia.",
        )),
        "lamparina" => Some((
            "substantivo feminino",
            "Lâmpada pequena de pavio, usada quando a luz grande falha.",
            "Luz pequena de pavio, comum quando a iluminação principal falha.",
        )),
        "gaivota" => Some((
            "substantivo feminino",
            "Ave costeira, clara, que acompanha barcos e faróis.",
            "Ave marinha de plumagem clara, frequente no litoral.",
        )),
        "diario" => Some((
            "substantivo masculino",
            "Caderno em que se anota o dia: hora, maré, o que não se quer esquecer.",
            "Registo quotidiano de factos, horas e observações.",
        )),
        "chaleira" => Some((
            "substantivo feminino",
            "Vasilha de ferver água, quase sempre a primeira voz da cozinha.",
            "Recipiente para ferver água.",
        )),
        "serra" => Some((
            "substantivo feminino",
            "Cadeia de montanhas, ou o lugar alto de onde as cartas descem.",
            "Forma de relevo alto, ou o instrumento de cortar.",
        )),
        _ => None,
    }
}

fn fold(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch.to_lowercase().next().unwrap_or(ch) {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

fn strip_tags(value: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for ch in value.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_dictionary_and_remote_parsers() {
        let local = define("Farol");
        assert_eq!(local.source, "local");
        assert_eq!(local.part_of_speech, "substantivo masculino");
        assert!(local.senses[0].contains("facho"));
        assert!(local.wikipedia_extract.contains("navegação"));
        assert!(define("maré").senses[0].contains("mar"));
        let (part, senses) = parse_wiktionary(include_str!("../fixtures/wiktionary.json"));
        assert!(part.eq_ignore_ascii_case("substantivo"));
        assert!(senses[0].contains("embarcações"));
        let (title, extract, url) = parse_wikipedia(include_str!("../fixtures/wikipedia.json"));
        assert_eq!(title, "Farol");
        assert!(extract.contains("navegante"));
        assert!(url.contains("wikipedia.org"));
    }
}
