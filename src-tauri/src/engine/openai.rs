//! Translation through any OpenAI-compatible chat completions API: OpenAI,
//! Gemini's compatible endpoint, or a local server such as Ollama or LM Studio.

use std::time::Duration;

use serde_json::{json, Value};

use super::Translation;
use crate::settings::Settings;

fn is_local(base: &str) -> bool {
    base.contains("://localhost") || base.contains("://127.0.0.1")
}

/// A base URL and model are required; a key only for remote services.
pub fn configured(settings: &Settings) -> bool {
    let base = settings.openai_base.trim();
    !base.is_empty() && !settings.openai_model.trim().is_empty() && (is_local(base) || !settings.openai_key.trim().is_empty())
}

pub(crate) fn language(code: &str) -> &str {
    match code {
        "vi" => "Vietnamese",
        "en" => "English",
        "ja" => "Japanese",
        "ko" => "Korean",
        "zh" => "Chinese",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        "ru" => "Russian",
        other => other,
    }
}

pub fn parse(json: &Value) -> anyhow::Result<String> {
    let text = json
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    anyhow::ensure!(!text.is_empty(), "API không trả về bản dịch");
    Ok(text.to_string())
}

pub async fn translate(base: &str, key: &str, model: &str, text: &str, from: &str, to: &str) -> anyhow::Result<Translation> {
    let (base, key, model) = (base.trim().trim_end_matches('/'), key.trim(), model.trim());
    anyhow::ensure!(!base.is_empty() && !model.is_empty(), "Chưa nhập địa chỉ API và tên model trong Cài đặt");
    let source = if from == "auto" { "the language it is written in" } else { language(from) };
    let system = format!(
        "You are a translation engine. Translate the user's text from {source} into {}. \
         Reply with the translation only: no quotes, no notes, no explanations.",
        language(to)
    );
    let body = json!({
        "model": model,
        "messages": [{ "role": "system", "content": system }, { "role": "user", "content": text }],
    });

    // Local models can take a while to load.
    let timeout = Duration::from_secs(if is_local(base) { 90 } else { 30 });
    let client = reqwest::Client::builder().timeout(timeout).build()?;
    let mut request = client.post(format!("{base}/chat/completions")).json(&body);
    if !key.is_empty() {
        request = request.bearer_auth(key);
    }
    let response = request.send().await?;
    if !response.status().is_success() {
        anyhow::bail!("API trả về lỗi {}", response.status());
    }
    Ok(Translation {
        text: parse(&response.json().await?)?,
        from: from.to_string(),
        to: to.to_string(),
        engine: "openai".into(),
        ..Default::default()
    })
}

/// Meaning of `word` inside `sentence` and the sentence's translation, both in `to`.
pub async fn explain(settings: &Settings, word: &str, sentence: &str, to: &str) -> anyhow::Result<super::context::Explained> {
    let base = settings.openai_base.trim().trim_end_matches('/');
    let system = super::context::prompt(language(to));
    let body = json!({
        "model": settings.openai_model.trim(),
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": super::context::question(word, sentence) }
        ],
    });
    let timeout = Duration::from_secs(if is_local(base) { 90 } else { 30 });
    let client = reqwest::Client::builder().timeout(timeout).build()?;
    let mut request = client.post(format!("{base}/chat/completions")).json(&body);
    let key = settings.openai_key.trim();
    if !key.is_empty() {
        request = request.bearer_auth(key);
    }
    let response = request.send().await?;
    anyhow::ensure!(response.status().is_success(), "API trả về lỗi {}", response.status());
    super::context::parse_answer(&parse(&response.json().await?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_first_choice() {
        let json = json!({ "choices": [{ "message": { "role": "assistant", "content": " Xin chào \n" } }] });
        assert_eq!(parse(&json).unwrap(), "Xin chào");
    }

    #[test]
    fn empty_reply_is_an_error() {
        assert!(parse(&json!({ "choices": [] })).is_err());
    }

    #[test]
    fn local_server_needs_no_key() {
        let mut settings =
            Settings { openai_base: "http://localhost:11434/v1".into(), openai_model: "gemma3".into(), ..Default::default() };
        assert!(configured(&settings));
        settings.openai_base = "https://api.openai.com/v1".into();
        assert!(!configured(&settings));
    }
}
