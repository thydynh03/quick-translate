//! English definitions and IPA for single words: the Oxford Dictionaries API
//! when the user has credentials, otherwise the Free Dictionary API
//! (dictionaryapi.dev), which is free and needs no key.

use std::time::Duration;

use serde_json::Value;

use super::DictEntry;

pub struct Lookup {
    pub phonetic: Option<String>,
    pub definitions: Vec<DictEntry>,
}

pub async fn lookup(word: &str) -> anyhow::Result<Lookup> {
    let url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", urlencoding::encode(word.trim()));
    let response = super::http().get(url).timeout(Duration::from_secs(4)).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("không có mục từ");
    }
    parse(&response.json().await?).ok_or_else(|| anyhow::anyhow!("không có mục từ"))
}

/// Oxford Dictionaries API v2. Needs the user's own `app_id` and `app_key`.
pub async fn lookup_oxford(app_id: &str, app_key: &str, word: &str) -> anyhow::Result<Lookup> {
    let url = format!(
        "https://od-api.oxforddictionaries.com/api/v2/entries/en-us/{}?fields=definitions,pronunciations",
        urlencoding::encode(&word.trim().to_lowercase())
    );
    let client = reqwest::Client::builder().timeout(Duration::from_secs(5)).build()?;
    let response = client.get(url).header("app_id", app_id.trim()).header("app_key", app_key.trim()).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("Oxford trả về lỗi {}", response.status());
    }
    parse_oxford(&response.json().await?).ok_or_else(|| anyhow::anyhow!("không có mục từ"))
}

pub(crate) fn parse_oxford(json: &Value) -> Option<Lookup> {
    let mut phonetic = None;
    let mut definitions = Vec::new();
    for lexical in json.get("results")?.get(0)?.get("lexicalEntries")?.as_array()? {
        let pos = lexical.pointer("/lexicalCategory/text").and_then(Value::as_str).unwrap_or_default().to_lowercase();
        let mut terms = Vec::new();
        for entry in lexical.get("entries").and_then(Value::as_array).into_iter().flatten() {
            if phonetic.is_none() {
                phonetic = entry
                    .get("pronunciations")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .find_map(|p| p.get("phoneticSpelling").and_then(Value::as_str))
                    .map(str::to_string);
            }
            for sense in entry.get("senses").and_then(Value::as_array).into_iter().flatten() {
                if let Some(definition) = sense.pointer("/definitions/0").and_then(Value::as_str) {
                    terms.push(definition.to_string());
                }
            }
        }
        terms.truncate(2);
        if !terms.is_empty() {
            definitions.push(DictEntry { pos, terms });
        }
    }
    definitions.truncate(4);
    (!definitions.is_empty()).then_some(Lookup { phonetic, definitions })
}

pub(crate) fn parse(json: &Value) -> Option<Lookup> {
    let entry = json.get(0)?;
    let phonetic = entry
        .get("phonetic")
        .and_then(Value::as_str)
        .or_else(|| {
            entry.get("phonetics")?.as_array()?.iter().find_map(|p| p.get("text").and_then(Value::as_str))
        })
        .map(|p| p.trim_matches('/').to_string())
        .filter(|p| !p.is_empty());

    let definitions: Vec<DictEntry> = entry
        .get("meanings")?
        .as_array()?
        .iter()
        .filter_map(|meaning| {
            let pos = meaning.get("partOfSpeech")?.as_str()?.to_string();
            let terms: Vec<String> = meaning
                .get("definitions")?
                .as_array()?
                .iter()
                .filter_map(|d| d.get("definition").and_then(Value::as_str).map(str::to_string))
                .take(2)
                .collect();
            (!terms.is_empty()).then_some(DictEntry { pos, terms })
        })
        .take(4)
        .collect();
    (!definitions.is_empty()).then_some(Lookup { phonetic, definitions })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_entry() {
        let raw = json!([{
            "word": "book",
            "phonetics": [{ "audio": "" }, { "text": "/bʊk/" }],
            "meanings": [
                { "partOfSpeech": "noun", "definitions": [{ "definition": "A collection of sheets." }, { "definition": "A long work." }, { "definition": "Third." }] },
                { "partOfSpeech": "verb", "definitions": [{ "definition": "To reserve." }] }
            ]
        }]);
        let lookup = parse(&raw).unwrap();
        assert_eq!(lookup.phonetic.as_deref(), Some("bʊk"));
        assert_eq!(lookup.definitions.len(), 2);
        assert_eq!(lookup.definitions[0].terms.len(), 2);
    }

    #[test]
    fn parses_oxford_entry() {
        let raw = json!({ "results": [{ "lexicalEntries": [{
            "lexicalCategory": { "id": "noun", "text": "Noun" },
            "entries": [{
                "pronunciations": [{ "phoneticSpelling": "bʊk" }],
                "senses": [{ "definitions": ["a written or printed work"] }, { "definitions": ["a bound set of blank sheets"] }, { "definitions": ["third"] }]
            }]
        }] }] });
        let lookup = parse_oxford(&raw).unwrap();
        assert_eq!(lookup.phonetic.as_deref(), Some("bʊk"));
        assert_eq!(lookup.definitions[0].pos, "noun");
        assert_eq!(lookup.definitions[0].terms.len(), 2);
        assert!(parse_oxford(&json!({ "error": "No entry found" })).is_none());
    }

    #[test]
    fn missing_word_is_none() {
        assert!(parse(&json!({ "title": "No Definitions Found" })).is_none());
    }
}
