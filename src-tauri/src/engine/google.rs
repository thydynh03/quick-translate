//! Google Translate through the keyless web endpoint. Unofficial: it can be
//! rate-limited or change shape, so every field is read defensively.

use std::time::Duration;

use serde_json::Value;

use super::{lang, DictEntry, Translation};

const ENDPOINT: &str = "https://translate.googleapis.com/translate_a/single";
/// A second keyless endpoint with its own quota, for when the first one
/// refuses. It returns the translation only: no dictionary, no reading.
const SPARE: &str = "https://clients5.google.com/translate_a/t";
/// Requests that do not look like a browser are limited much sooner.
/// Where `warm` opens the connection to.
pub(crate) const HOST: &str = "https://translate.googleapis.com/";
const AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

pub async fn translate(text: &str, from: &str, to: &str) -> anyhow::Result<Translation> {
    // QT_GOOGLE_ENDPOINT lets a test point the engine at an unreachable host.
    let endpoint = std::env::var("QT_GOOGLE_ENDPOINT").unwrap_or_else(|_| ENDPOINT.to_string());
    let mut url = format!("{endpoint}?client=gtx&sl={from}&tl={to}&dt=t&dt=rm");
    if lang::is_single_word(text) {
        url.push_str("&dt=bd");
    }
    let body = format!("q={}", urlencoding::encode(text));

    let client = super::http();
    let response = client
        .post(url)
        .timeout(Duration::from_secs(12))
        .header("Content-Type", "application/x-www-form-urlencoded;charset=utf-8")
        .header("User-Agent", AGENT)
        .body(body.clone())
        .send()
        .await?;
    super::google_used();
    if !response.status().is_success() {
        // Usually 429: this network address has used up its share. The spare
        // endpoint counts separately, so try it before giving up.
        let status = response.status();
        let spare = client
            .post(format!("{SPARE}?client=dict-chrome-ex&sl={from}&tl={to}"))
            .timeout(Duration::from_secs(12))
            .header("Content-Type", "application/x-www-form-urlencoded;charset=utf-8")
            .header("User-Agent", AGENT)
            .body(body)
            .send()
            .await;
        if let Ok(spare) = spare {
            if spare.status().is_success() {
                if let Ok(translation) = parse_spare(&spare.json().await?, from, to) {
                    return Ok(translation);
                }
            }
        }
        anyhow::bail!("Google Translate trả về lỗi {status}");
    }
    let json: Value = response.json().await?;
    parse(&json, from, to)
}

/// The spare endpoint answers `["text"]` when the source language was given
/// and `[["text", "detected language"]]` when it was not.
pub(crate) fn parse_spare(json: &Value, from: &str, to: &str) -> anyhow::Result<Translation> {
    let first = json.get(0);
    let text = first.and_then(Value::as_str).or_else(|| first.and_then(|pair| pair.get(0)).and_then(Value::as_str));
    let detected = first.and_then(|pair| pair.get(1)).and_then(Value::as_str).unwrap_or(from);
    match text.map(str::trim).filter(|text| !text.is_empty()) {
        Some(text) => Ok(Translation {
            text: text.to_string(),
            from: detected.to_string(),
            to: to.to_string(),
            engine: "google".into(),
            ..Default::default()
        }),
        None => anyhow::bail!("Google Translate không trả về bản dịch"),
    }
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
        context: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_shapes_of_the_spare_endpoint() {
        let known = parse_spare(&serde_json::json!(["tổ chức"]), "en", "vi").unwrap();
        assert_eq!((known.text.as_str(), known.from.as_str()), ("tổ chức", "en"));
        let detected = parse_spare(&serde_json::json!([["Xin chào.", "en"]]), "auto", "vi").unwrap();
        assert_eq!((detected.text.as_str(), detected.from.as_str()), ("Xin chào.", "en"));
        assert!(parse_spare(&serde_json::json!([]), "en", "vi").is_err());
    }

    /// Prints how long a cold and a warm request take; the warm one reuses the connection.
    #[test]
    #[ignore]
    fn times_a_cold_and_a_warm_request() {
        tauri::async_runtime::block_on(async {
            for text in ["The meeting was moved to Thursday.", "Please send me the report.", "institution"] {
                let start = std::time::Instant::now();
                let result = translate(text, "en", "vi").await;
                println!("{:?} {:?}", start.elapsed(), result.map(|found| found.text));
            }
        });
    }

    /// The spare endpoint answers a real request, as it must when the main one refuses.
    #[test]
    #[ignore]
    fn spare_endpoint_answers() {
        let client = reqwest::Client::new();
        let result = tauri::async_runtime::block_on(async {
            let response = client
                .post(format!("{SPARE}?client=dict-chrome-ex&sl=auto&tl=vi"))
                .header("Content-Type", "application/x-www-form-urlencoded;charset=utf-8")
                .header("User-Agent", AGENT)
                .body("q=Good%20morning.")
                .send()
                .await
                .unwrap();
            parse_spare(&response.json().await.unwrap(), "auto", "vi")
        });
        println!("{result:?}");
        assert_eq!(result.unwrap().from, "en");
    }

    /// The main endpoint answers 429 (simulated), so the spare one must answer.
    /// Run alone: cargo test survives_a_429 -- --ignored --nocapture
    #[test]
    #[ignore]
    fn survives_a_429() {
        unsafe { std::env::set_var("QT_GOOGLE_ENDPOINT", "https://httpbin.org/status/429") };
        let result = tauri::async_runtime::block_on(translate("The meeting has been moved to Thursday.", "auto", "vi"));
        println!("{result:?}");
        let translation = result.unwrap();
        assert_eq!(translation.engine, "google");
        assert!(translation.dict.is_empty());
    }

    /// Calls the real service: run with `cargo test -- --ignored google_answers`.
    #[test]
    #[ignore]
    fn google_answers() {
        let result = tauri::async_runtime::block_on(translate("meeting", "en", "vi"));
        println!("{result:?}");
        assert!(result.is_ok());
    }
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
