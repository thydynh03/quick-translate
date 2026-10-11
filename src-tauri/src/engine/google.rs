//! Google Translate through the keyless web endpoint. Unofficial: it can be
//! rate-limited or change shape, so every field is read defensively.

use std::time::Duration;

use serde_json::Value;

use super::{lang, DictEntry, Translation};

const ENDPOINT: &str = "https://translate.googleapis.com/translate_a/single";

pub async fn translate(text: &str, from: &str, to: &str) -> anyhow::Result<Translation> {
    // QT_GOOGLE_ENDPOINT lets a test point the engine at an unreachable host.
    let endpoint = std::env::var("QT_GOOGLE_ENDPOINT").unwrap_or_else(|_| ENDPOINT.to_string());
    let mut url = format!("{endpoint}?client=gtx&sl={from}&tl={to}&dt=t&dt=rm");
    if lang::is_single_word(text) {
        url.push_str("&dt=bd");
    }
    let body = format!("q={}", urlencoding::encode(text));

    let client = reqwest::Client::builder().timeout(Duration::from_secs(12)).build()?;
    let response = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded;charset=utf-8")
        .body(body)
        .send()
        .await?;
    if !response.status().is_success() {
        anyhow::bail!("Google Translate trả về lỗi {}", response.status());
    }
    let json: Value = response.json().await?;
    parse(&json, from, to)
}

pub(crate) fn parse(json: &Value, from: &str, to: &str) -> anyhow::Result<Translation> {
    let segments = json.get(0).and_then(Value::as_array);
    let mut text = String::new();
    let mut phonetic = None;
    for segment in segments.into_iter().flatten() {
        if let Some(part) = segment.get(0).and_then(Value::as_str) {
            text.push_str(part);
        } else if let Some(source_reading) = segment.get(3).and_then(Value::as_str) {
            // The romanisation row: [null, null, target reading, source reading].
            phonetic = Some(source_reading.to_string());
        }
    }
    if text.trim().is_empty() {
        anyhow::bail!("Google Translate không trả về bản dịch");
    }

    let dict = json
        .get(1)
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let pos = entry.get(0)?.as_str()?.to_string();
                    let terms: Vec<String> = entry
                        .get(1)?
                        .as_array()?
                        .iter()
                        .filter_map(|t| t.as_str().map(str::to_string))
                        .take(8)
                        .collect();
                    (!terms.is_empty()).then_some(DictEntry { pos, terms })
                })
                .collect()
        })
        .unwrap_or_default();

    let detected = json.get(2).and_then(Value::as_str).unwrap_or(from).to_string();
    Ok(Translation {
        text,
        from: detected,
        to: to.to_string(),
        engine: "google".into(),
        phonetic,
        dict,
        definitions: Vec::new(),
        note: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_sentence() {
        let raw = json!([[["Xin chào ", "Hello ", null, null, 10], ["thế giới", "world", null, null, 10]], null, "en"]);
        let t = parse(&raw, "auto", "vi").unwrap();
        assert_eq!(t.text, "Xin chào thế giới");
        assert_eq!(t.from, "en");
        assert!(t.dict.is_empty());
    }

    #[test]
    fn parses_word_with_dictionary_and_reading() {
        let raw = json!([
            [["sách", "book", null, null, 10], [null, null, null, "bo͝ok"]],
            [["noun", ["sách", "quyển sách"], [], "book", 1], ["verb", ["đặt chỗ"], [], "book", 2]],
            "en"
        ]);
        let t = parse(&raw, "auto", "vi").unwrap();
        assert_eq!(t.text, "sách");
        assert_eq!(t.phonetic.as_deref(), Some("bo͝ok"));
        assert_eq!(t.dict.len(), 2);
        assert_eq!(t.dict[0].terms, vec!["sách", "quyển sách"]);
    }

    #[test]
    fn empty_is_an_error() {
        assert!(parse(&json!([null, null, "en"]), "auto", "vi").is_err());
    }
}
