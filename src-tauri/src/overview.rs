use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameHit {
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub words: u32,
    pub names: Vec<NameHit>,
    pub percentage: f64,
    pub minutes_left: u32,
}

const STOP: &[&str] = &[
    "Quando", "Onde", "Ela", "Ele", "Uma", "Para", "Como", "Mais", "Depois", "Antes", "Ainda", "Cada",
    "Pela", "Pelo", "Esta", "Esse", "Essa", "Isso", "Aqui", "Hoje", "Ontem", "Amanhã", "Capítulo",
    "Sumário", "The", "And", "Na", "No", "Nas", "Nos", "Ao", "Aos", "Das", "Dos", "Se", "Mas", "Por",
    "Com", "Sem", "Que", "Não", "Sim", "Era", "Eram", "Foi", "Ser", "Há",
];

pub fn overview_from_text(text: &str, percentage: f64, words_per_minute: u32) -> Overview {
    let words = text.split_whitespace().filter(|word| word.chars().any(|ch| ch.is_alphanumeric())).count() as u32;
    let mut counts: Vec<(String, u32)> = Vec::new();
    for raw in text.split_whitespace() {
        let name = raw.trim_matches(|ch: char| !ch.is_alphabetic());
        if name.len() < 3 || !name.chars().next().is_some_and(|ch| ch.is_uppercase()) {
            continue;
        }
        if STOP.iter().any(|stop| stop.eq_ignore_ascii_case(name)) {
            continue;
        }
        if let Some(entry) = counts.iter_mut().find(|(known, _)| known.eq_ignore_ascii_case(name)) {
            entry.1 += 1;
        } else {
            counts.push((name.to_string(), 1));
        }
    }
    counts.retain(|(_, count)| *count >= 2);
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts.truncate(8);
    let remaining = ((1.0 - percentage.clamp(0.0, 1.0)) * words as f64).max(0.0);
    let pace = words_per_minute.max(80) as f64;
    let minutes_left = (remaining / pace).ceil() as u32;
    Overview {
        words,
        names: counts
            .into_iter()
            .map(|(name, count)| NameHit { name, count })
            .collect(),
        percentage,
        minutes_left,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epub_meta::epub_plain_text;
    use std::path::Path;

    #[test]
    fn sample_overview_names_the_island() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/amostra.epub");
        let text = epub_plain_text(&path).expect("texto");
        let overview = overview_from_text(&text, 0.4, 220);
        assert!(overview.words > 400);
        assert!(overview.names.iter().any(|name| name.name.contains("Ilha") || name.name == "Rasa"));
        assert!(overview.minutes_left > 0);
    }
}
