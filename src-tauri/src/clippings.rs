use chrono::{TimeZone, Utc};

use crate::model::Highlight;

pub fn to_markdown(highlights: &[Highlight]) -> String {
    let mut out = String::from("# Caderno de grifos\n\n");
    let mut current = String::new();
    for item in highlights {
        if item.book_title != current {
            current = item.book_title.clone();
            out.push_str(&format!("## {}\n\n*{}. *\n\n", item.book_title, item.author));
        }
        out.push_str(&format!("> {}\n\n", item.text.trim()));
        if !item.note.trim().is_empty() {
            out.push_str(&format!("{}\n\n", item.note.trim()));
        }
    }
    out
}

pub fn to_my_clippings(highlights: &[Highlight]) -> String {
    let mut out = String::new();
    for item in highlights {
        out.push_str(&format!("{} ({})\n", item.book_title, item.author));
        out.push_str(&format!("- Seu destaque | Adicionado em {}\n\n", stamp(item.created_at)));
        out.push_str(item.text.trim());
        out.push('\n');
        if !item.note.trim().is_empty() {
            out.push_str(item.note.trim());
            out.push('\n');
        }
        out.push_str("==========\n");
    }
    out
}

fn stamp(millis: i64) -> String {
    let secs = millis / 1000;
    Utc.timestamp_opt(secs, 0)
        .single()
        .map(|date| date.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "data desconhecida".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Highlight {
        Highlight {
            id: "1".into(),
            book_id: "cloud:B0CLOUD001".into(),
            book_title: "Maré de Pedra".into(),
            author: "Helena Vilar".into(),
            source: "cloud".into(),
            cfi: String::new(),
            text: "O faroleiro não salva o mar.".into(),
            note: "frase do diário".into(),
            color: "amber".into(),
            start_pos: 120,
            end_pos: 148,
            created_at: 1_730_000_001_000,
        }
    }

    #[test]
    fn exports_markdown_and_kindle_clippings() {
        let notes = vec![sample()];
        let markdown = to_markdown(&notes);
        assert!(markdown.contains("# Caderno de grifos"));
        assert!(markdown.contains("## Maré de Pedra"));
        assert!(markdown.contains("> O faroleiro não salva o mar."));
        assert!(markdown.contains("frase do diário"));
        let clippings = to_my_clippings(&notes);
        assert!(clippings.contains("Maré de Pedra (Helena Vilar)"));
        assert!(clippings.contains("=========="));
        assert!(clippings.contains("2024-10-27"));
    }
}
