//! Bundled English–Vietnamese word list for single-word lookups without a
//! network. The translation models are trained on sentences and do badly on
//! bare words, so the offline engine asks here first.
//!
//! There are two lists: a short one with plain translations in both
//! directions, and a large English one with definitions and pronunciation.
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

// ------------------------------------------------------- the large word list

/// English words with Vietnamese definitions from the Vietnamese Wiktionary:
/// `word <tab> IPA <tab> pos=gloss|gloss <tab> …`, sorted by the bytes of the
/// word so a lookup is a binary search with nothing to load.
/// Built by `scripts/build-dictionary.py`; see `data/NOTICE.md`.
const BIG: &str = include_str!("../../data/en-vi-viwiktionary.tsv");

/// The line of `data` whose first field is `key`.
fn line_of(data: &'static str, key: &str) -> Option<&'static str> {
    let bytes = data.as_bytes();
    let (mut low, mut high) = (0usize, bytes.len());
    while low < high {
        let middle = (low + high) / 2;
        let start = bytes[..middle].iter().rposition(|&b| b == b'\n').map_or(0, |at| at + 1);
        let end = bytes[start..].iter().position(|&b| b == b'\n').map_or(bytes.len(), |at| start + at);
        let line = &data[start..end];
        let head = line.split('\t').next().unwrap_or("");
        match head.as_bytes().cmp(key.as_bytes()) {
            std::cmp::Ordering::Equal => return Some(line),
            std::cmp::Ordering::Less => low = end + 1,
            std::cmp::Ordering::Greater => high = start,
        }
    }
    None
}

/// IPA and senses of an English word in the large list.
fn big(key: &str) -> Option<(Option<String>, Vec<DictEntry>)> {
    let mut fields = line_of(BIG, key)?.split('\t').skip(1);
    let ipa = fields.next().filter(|ipa| !ipa.is_empty()).map(str::to_string);
    let senses: Vec<DictEntry> = fields
        .filter_map(|field| field.split_once('='))
        .map(|(pos, glosses)| DictEntry { pos: pos.to_string(), terms: glosses.split('|').map(str::to_string).collect() })
        .collect();
    (!senses.is_empty()).then_some((ipa, senses))
}

fn is_vowel(c: char) -> bool {
    "aeiou".contains(c)
}

/// Dictionary forms an inflected English word may come from, likeliest first.
pub(crate) fn stems(word: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    // "hoping" drops the e of "hope"; "hopping" doubles the p of "hop". So a
    // stem ending consonant-vowel-consonant most likely lost an e.
    let lost_e = |base: &str| {
        let tail: Vec<char> = base.chars().rev().take(3).collect();
        tail.len() >= 2
            && !is_vowel(tail[0])
            && !"wxy".contains(tail[0])
            && is_vowel(tail[1])
            && tail.get(2).is_none_or(|&c| !is_vowel(c))
    };
    let mut verb = |base: &str| {
        let chars: Vec<char> = base.chars().collect();
        let doubled = chars.len() >= 3 && chars[chars.len() - 1] == chars[chars.len() - 2] && !is_vowel(chars[chars.len() - 1]);
        if doubled {
            out.push(chars[..chars.len() - 1].iter().collect());
        }
        if lost_e(base) {
            out.push(format!("{base}e"));
            out.push(base.to_string());
        } else {
            out.push(base.to_string());
            out.push(format!("{base}e"));
        }
    };
    if let Some(base) = word.strip_suffix("ing") {
        verb(base);
    }
    if let Some(base) = word.strip_suffix("ied") {
        out.push(format!("{base}y"));
    } else if let Some(base) = word.strip_suffix("ed") {
        verb(base);
    }
    if let Some(base) = word.strip_suffix("ies") {
        out.push(format!("{base}y"));
    }
    if let Some(base) = word.strip_suffix('s') {
        out.push(base.to_string());
    }
    if let Some(base) = word.strip_suffix("es") {
        out.push(base.to_string());
    }
    out.retain(|stem| stem.chars().count() >= 2);
    out
}

/// What the word lists know about a word.
pub struct Found {
    pub senses: Vec<DictEntry>,
    pub phonetic: Option<String>,
    /// The dictionary form the senses belong to, when the word itself is an
    /// inflected form ("leveraging" → "leverage").
    pub stem: Option<String>,
}

fn english(key: &str) -> Option<Found> {
    // The short list has plain translations, the large one definitions: show both.
    let mut senses = lookup(key, "en", "vi").unwrap_or_default();
    let mut phonetic = None;
    if let Some((ipa, more)) = big(key) {
        phonetic = ipa;
        senses.extend(more);
    }
    (!senses.is_empty()).then_some(Found { senses, phonetic, stem: None })
}

/// Looks a word or short phrase up for an en→vi or vi→en translation.
pub fn find(text: &str, from: &str, to: &str) -> Option<Found> {
    let key = text.trim().trim_matches(|c: char| c.is_ascii_punctuation()).to_lowercase();
    if (from, to) != ("en", "vi") {
        return lookup(&key, from, to).map(|senses| Found { senses, phonetic: None, stem: None });
    }
    english(&key).or_else(|| {
        let mut found = stems(&key).into_iter().find_map(|stem| english(&stem).map(|found| Found { stem: Some(stem), ..found }))?;
        // "leveraging" and "operated" are verb forms: lead with the verb.
        if key.ends_with("ing") || key.ends_with("ed") {
            found.senses.sort_by_key(|sense| sense.pos != "verb");
        }
        Some(found)
    })
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

    #[test]
    fn the_large_list_is_sorted_for_binary_search() {
        let mut previous: &[u8] = b"";
        for line in BIG.lines() {
            let head = line.split('\t').next().unwrap().as_bytes();
            assert!(previous < head, "{line}");
            previous = head;
        }
    }

    #[test]
    fn finds_words_missing_from_the_short_list() {
        let found = find("institution", "en", "vi").unwrap();
        assert!(found.stem.is_none() && found.phonetic.is_some());
        assert!(found.senses.iter().any(|sense| sense.terms.iter().any(|term| term.contains("Cơ quan"))));
        assert!(find("zzzzqq", "en", "vi").is_none());
        // The first and the last line are reachable too.
        let first = BIG.lines().next().unwrap().split('\t').next().unwrap();
        let last = BIG.lines().last().unwrap().split('\t').next().unwrap();
        assert!(big(first).is_some() && big(last).is_some());
    }

    #[test]
    fn inflected_words_fall_back_to_their_dictionary_form() {
        for (word, stem) in [("leveraging", "leverage"), ("operates", "operate"), ("stopped", "stop"), ("companies", "company"), ("hoping", "hope"), ("being", "be"), ("boxes", "box")] {
            // Some inflected forms have an entry of their own; otherwise the stem answers.
            let found = find(word, "en", "vi").expect(word);
            assert!(found.stem.is_none() || found.stem.as_deref() == Some(stem), "{word}: {:?}", found.stem);
            assert!(stems(word).iter().any(|candidate| candidate == stem), "{word}");
        }
        assert_eq!(find("leveraging", "en", "vi").unwrap().stem.as_deref(), Some("leverage"));
        assert_eq!(stems("hoping")[0], "hope");
        assert_eq!(stems("being")[0], "be");
    }
}
