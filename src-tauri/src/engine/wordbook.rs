//! Bundled English–Vietnamese word list for single-word lookups without a
//! network. The translation models are trained on sentences and do badly on
//! bare words, so the offline engine asks here first.
//!
//! Data: `data/en-vi-enwiktionary.txt`, extracted from en.wiktionary.org by
//! Matthias Buchmeier (CC BY-SA 3.0 / GFDL). See `data/NOTICE.md`.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::DictEntry;

const DATA: &str = include_str!("../../data/en-vi-enwiktionary.txt");

#[derive(Default)]
struct Wordbook {
    /// English headword (lower-case) → (part of speech, Vietnamese terms).
    en: HashMap<String, Vec<(String, Vec<String>)>>,
    /// Vietnamese term (lower-case) → (part of speech, English headwords).
    vi: HashMap<String, Vec<(String, Vec<String>)>>,
}

fn pos_name(tag: &str) -> &str {
    match tag {
        "n" => "noun",
        "v" => "verb",
        "adj" => "adjective",
        "adv" => "adverb",
        "prep" => "preposition",
        "pron" => "pronoun",
        "conj" => "conjunction",
        "interj" => "interjection",
        "num" => "numeral",
        "prop" => "proper noun",
        "phrase" => "phrase",
        other => other,
    }
}

/// One line: `book {n} (collection of sheets) :: sách, [notebook] sổ`.
pub(crate) fn parse_line(line: &str) -> Option<(String, String, Vec<String>)> {
    if line.starts_with('#') || line.contains("SEE:") {
        return None;
    }
    let (left, right) = line.split_once(" :: ")?;
    let (head, rest) = left.split_once(" {")?;
    let pos = pos_name(rest.split('}').next()?).to_string();
    let terms: Vec<String> = right
        .split(", ")
        .map(|term| {
            // Drop a leading "[usage note]" and trailing "{gender}" / "(reading)".
            let term = term.trim();
            let term = if term.starts_with('[') { term.split_once("] ").map_or("", |(_, t)| t) } else { term };
            let term = term.split(" {").next().unwrap_or(term);
            term.split(" (").next().unwrap_or(term).trim().to_string()
        })
        .filter(|term| !term.is_empty())
        .collect();
    (!terms.is_empty()).then(|| (head.trim().to_string(), pos, terms))
}

fn push(map: &mut HashMap<String, Vec<(String, Vec<String>)>>, key: &str, pos: &str, values: &[String]) {
    let senses = map.entry(key.to_lowercase()).or_default();
    let index = match senses.iter().position(|(p, _)| p == pos) {
        Some(index) => index,
        None => {
            senses.push((pos.to_string(), Vec::new()));
            senses.len() - 1
        }
    };
    for value in values {
        if !senses[index].1.contains(value) {
            senses[index].1.push(value.clone());
        }
    }
}

fn wordbook() -> &'static Wordbook {
    static BOOK: OnceLock<Wordbook> = OnceLock::new();
    BOOK.get_or_init(|| {
        let mut book = Wordbook::default();
        for (head, pos, terms) in DATA.lines().filter_map(parse_line) {
            push(&mut book.en, &head, &pos, &terms);
            for term in &terms {
                push(&mut book.vi, term, &pos, std::slice::from_ref(&head));
            }
        }
        book
    })
}

/// Senses of a word or short phrase for an en→vi or vi→en lookup.
pub fn lookup(text: &str, from: &str, to: &str) -> Option<Vec<DictEntry>> {
    let key = text.trim().trim_matches(|c: char| c.is_ascii_punctuation()).to_lowercase();
    let book = wordbook();
    let senses = match (from, to) {
        ("en", "vi") => book.en.get(&key),
        ("vi", "en") => book.vi.get(&key),
        _ => None,
    }?;
    Some(
        senses
            .iter()
            .map(|(pos, terms)| DictEntry { pos: pos.clone(), terms: terms.iter().take(6).cloned().collect() })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_line() {
        let (head, pos, terms) =
            parse_line("book {n} (collection of sheets) :: sách, cuốn sách, [notebook] sổ").unwrap();
        assert_eq!(head, "book");
        assert_eq!(pos, "noun");
        assert_eq!(terms, ["sách", "cuốn sách", "sổ"]);
        assert!(parse_line("# English :: Vietnamese dictionary").is_none());
        assert!(parse_line("abc {n} :: SEE: alphabet ::").is_none());
    }

    #[test]
    fn looks_up_both_directions() {
        let book = lookup("Book", "en", "vi").unwrap();
        assert!(book.iter().any(|e| e.pos == "noun" && e.terms.contains(&"sách".to_string())));
        let beautiful = lookup("beautiful.", "en", "vi").unwrap();
        assert!(beautiful[0].terms.contains(&"xinh đẹp".to_string()));
        let reverse = lookup("máy tính", "vi", "en").unwrap();
        assert!(reverse.iter().any(|e| e.terms.contains(&"computer".to_string())));
        assert!(lookup("qwertyzzz", "en", "vi").is_none());
        assert!(lookup("book", "en", "fr").is_none());
    }
}
