use std::time::Duration;

use serde_json::{json, Value};

use super::Translation;

pub async fn translate(key: &str, text: &str, from: &str, to: &str) -> anyhow::Result<Translation> {
    let key = key.trim();
    if key.is_empty() {
        anyhow::bail!("Chưa nhập DeepL API key trong Cài đặt");
    }
    // Free-plan keys end in ":fx" and live on a separate host.
    let host = if key.ends_with(":fx") { "api-free.deepl.com" } else { "api.deepl.com" };

    let mut body = json!({ "text": [text], "target_lang": to.to_uppercase() });
    if from != "auto" {
        body["source_lang"] = json!(from.to_uppercase());
    }

    let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build()?;
    let response = client
        .post(format!("https://{host}/v2/translate"))
        .header("Authorization", format!("DeepL-Auth-Key {key}"))
        .json(&body)
        .send()
        .await?;
    if !response.status().is_success() {
        anyhow::bail!("DeepL trả về lỗi {}", response.status());
    }
    let json: Value = response.json().await?;
    let first = json
        .get("translations")
        .and_then(|t| t.get(0))
        .ok_or_else(|| anyhow::anyhow!("DeepL không trả về bản dịch"))?;
    let translated = first.get("text").and_then(Value::as_str).unwrap_or_default().to_string();
    let detected = first
        .get("detected_source_language")
        .and_then(Value::as_str)
        .map(str::to_lowercase)
        .unwrap_or_else(|| from.to_string());

    Ok(Translation {
        text: translated,
        from: detected,
        to: to.to_string(),
        engine: "deepl".into(),
        ..Default::default()
    })
}
