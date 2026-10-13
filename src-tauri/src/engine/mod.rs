//! Translation engines. `translate` picks the direction, calls the engine the
//! user chose and falls back to the offline model when that fails.

use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::settings::Settings;

pub mod claude;
pub mod context;
pub mod deepl;
pub mod dictionary;
pub mod google;
pub mod lang;
pub mod ocr_vi;
pub mod offline;
pub mod openai;
pub mod wordbook;

/// One client for the short requests, so that a translation reuses the open
/// connection instead of paying for a new TLS handshake every time.
pub(crate) fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder().pool_idle_timeout(Duration::from_secs(240)).build().unwrap_or_default()
    })
}

/// Seconds since the app started.
pub(crate) fn uptime() -> u64 {
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_secs()
}

/// When the connection to Google was last used, in `uptime` seconds, plus one.
static GOOGLE_USED: AtomicU64 = AtomicU64::new(0);

pub(crate) fn google_used() {
    GOOGLE_USED.store(uptime() + 1, Ordering::Relaxed);
}

/// Opens the connection to Google ahead of the translation when it has been
/// idle long enough to have been closed. Call when a translation is likely,
/// e.g. while the selection is still being read.
pub fn warm(settings: &Settings) {
    const IDLE: u64 = 120;
    if settings.engine != "google" {
        return;
    }
    let (now, last) = (uptime() + 1, GOOGLE_USED.load(Ordering::Relaxed));
    if last != 0 && now.saturating_sub(last) < IDLE {
        return;
    }
    google_used();
    tauri::async_runtime::spawn(async {
        // A failed attempt opened nothing: let the next selection try again.
        if http().head(google::HOST).timeout(Duration::from_secs(5)).send().await.is_err() {
            GOOGLE_USED.store(0, Ordering::Relaxed);
        }
    });
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DictEntry {
    /// Part of speech ("noun", "verb"…), as reported by the engine.
    pub pos: String,
    pub terms: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Translation {
    pub text: String,
    pub from: String,
    pub to: String,
    pub engine: String,
    /// Pronunciation of the source text, for single words.
    pub phonetic: Option<String>,
    /// Dictionary senses, for single words.
    pub dict: Vec<DictEntry>,
    /// English definitions of a single English word (Free Dictionary API).
    pub definitions: Vec<DictEntry>,
    /// Extra information for the user, e.g. that a fallback engine answered.
    pub note: Option<String>,
    /// The word explained through the sentence it was selected in.
    #[serde(default)]
    pub context: Option<context::Context>,
}

/// Source and target language for `text` under the user's language pair.
/// Source is "auto" unless the text is recognisably some language.
pub fn direction(settings: &Settings, text: &str) -> (String, String) {
    let guess = lang::guess(text);
    if guess.as_deref() == Some(settings.primary_lang.as_str()) {
        (settings.primary_lang.clone(), settings.secondary_lang.clone())
    } else {
        (guess.unwrap_or_else(|| "auto".into()), settings.primary_lang.clone())
    }
}

async fn run_offline(models: &Path, text: &str, from: &str, to: &str) -> anyhow::Result<Translation> {
    let (models, text, from, to) = (models.to_path_buf(), text.to_string(), from.to_string(), to.to_string());
    tauri::async_runtime::spawn_blocking(move || offline::translate(&models, &text, &from, &to))
        .await
        .map_err(|e| anyhow::anyhow!("offline: {e}"))?
}

async fn run_engine(
    name: &str,
    settings: &Settings,
    models: &Path,
    text: &str,
    from: &str,
    to: &str,
) -> anyhow::Result<Translation> {
    match name {
        "google" => google::translate(text, from, to).await,
        "deepl" => deepl::translate(&settings.deepl_key, text, from, to).await,
        "claude" => claude::translate(&settings.anthropic_key, text, from, to).await,
        "offline" => run_offline(models, text, from, to).await,
        "openai" => {
            openai::translate(&settings.openai_base, &settings.openai_key, &settings.openai_model, text, from, to).await
        }
        other => anyhow::bail!("Engine \"{other}\" chưa được hỗ trợ"),
    }
}

/// Translates, then adds English definitions for a single English word.
pub async fn translate(settings: &Settings, models: &Path, text: &str) -> anyhow::Result<Translation> {
    let mut translation = translate_text(settings, models, text).await?;
    define(settings, text, &mut translation).await;
    Ok(translation)
}

/// Whether `define` has anything to look up for this translation.
pub fn wants_definitions(settings: &Settings, text: &str, translation: &Translation) -> bool {
    settings.english_definitions && translation.from == "en" && lang::is_single_word(text)
}

/// Adds English definitions for a single English word. Slower than the
/// translation itself, so the popup shows the translation first.
pub async fn define(settings: &Settings, text: &str, translation: &mut Translation) {
    if wants_definitions(settings, text, translation) && translation.from == "en" && lang::is_single_word(text) {
        // Oxford when the user has credentials, the free dictionary otherwise.
        let has_oxford = !settings.oxford_app_id.trim().is_empty() && !settings.oxford_app_key.trim().is_empty();
        let mut found = Err(anyhow::anyhow!("no oxford credentials"));
        if has_oxford {
            found = dictionary::lookup_oxford(&settings.oxford_app_id, &settings.oxford_app_key, text).await;
        }
        if found.is_err() {
            found = dictionary::lookup(text).await;
        }
        if let Ok(found) = found {
            translation.definitions = found.definitions;
            if translation.phonetic.is_none() {
                translation.phonetic = found.phonetic;
            }
        }
    }
}

/// `models` is the folder that holds the downloaded offline models.
pub async fn translate_text(settings: &Settings, models: &Path, text: &str) -> anyhow::Result<Translation> {
    let (from, to) = direction(settings, text);
    let mut result = run_engine(&settings.engine, settings, models, text, &from, &to).await;

    // Text that is already in the primary language but was not recognised
    // locally (e.g. Vietnamese typed without diacritics): flip the direction.
    if let Ok(first) = &result {
        if first.from == to && to != settings.secondary_lang {
            result = run_engine(&settings.engine, settings, models, text, &to, &settings.secondary_lang).await;
        }
    }

    let first_error = match result {
        Ok(translation) => return Ok(translation),
        Err(error) if !settings.fallback_offline => return Err(error),
        Err(error) => error,
    };
    crate::log(format!("engine {} failed: {first_error:#}", settings.engine));
    for name in fallbacks(settings) {
        if let Ok(mut translation) = run_engine(name, settings, models, text, &from, &to).await {
            translation.note = Some("Nguồn dịch chính không dịch được; đã dùng nguồn dự phòng.".to_string());
            return Ok(translation);
        }
    }
    // Why the chosen engine failed is the most useful thing to show.
    Err(first_error)
}

/// Engines to try, in order, after the chosen one fails: the on-device model
/// first (no network, no key), then the paid APIs the user has a key for.
pub fn fallbacks(settings: &Settings) -> Vec<&'static str> {
    let mut order = vec!["offline"];
    if !settings.anthropic_key.trim().is_empty() {
        order.push("claude");
    }
    if !settings.deepl_key.trim().is_empty() {
        order.push("deepl");
    }
    if openai::configured(settings) {
        order.push("openai");
    }
    order.push("google");
    order.retain(|name| *name != settings.engine);
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_order_is_offline_then_keyed_apis() {
        let mut settings = Settings::default();
        assert_eq!(fallbacks(&settings), ["offline"]);
        settings.anthropic_key = "k".into();
        assert_eq!(fallbacks(&settings), ["offline", "claude"]);
        settings.engine = "offline".into();
        assert_eq!(fallbacks(&settings), ["claude", "google"]);
    }

    /// Google unreachable (as with no network) must end in an offline
    /// translation. Needs the models from `offline_end_to_end` in the temp dir.
    /// Run with: cargo test falls_back_to_offline -- --ignored --nocapture
    #[test]
    #[ignore]
    fn falls_back_to_offline_when_google_is_unreachable() {
        unsafe { std::env::set_var("QT_GOOGLE_ENDPOINT", "https://unreachable.invalid/translate") };
        let models = std::env::temp_dir().join("qt-offline-test");
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let settings = Settings::default();
        let result = runtime.block_on(translate(&settings, &models, "The weather is beautiful today.")).unwrap();
        println!("{} | {:?}", result.text, result.note);
        assert_eq!(result.engine, "offline");
        assert!(result.note.is_some());

        let no_fallback = Settings { fallback_offline: false, ..Settings::default() };
        assert!(runtime.block_on(translate(&no_fallback, &models, "Hello there.")).is_err());
    }

    /// With the offline engine chosen, the bundled lists answer the words they
    /// know, and a word outside them is
    /// answered by Google instead of the sentence model. Needs a network and
    /// QT_MODELS pointing at a folder with the en-vi model.
    /// Run with: cargo test bare_word_goes_online -- --ignored --nocapture
    #[test]
    #[ignore]
    fn bare_word_goes_online() {
        let models = std::path::PathBuf::from(std::env::var("QT_MODELS").unwrap());
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let settings = Settings { engine: "offline".into(), ..Settings::default() };
        for known in ["institution", "leveraging", "operates"] {
            let found = runtime.block_on(translate_text(&settings, &models, known)).unwrap();
            println!("{known}: {} | {} | {:?} | {:?} | {:?}", found.text, found.engine, found.phonetic, found.note, found.dict);
            assert_eq!(found.engine, "offline");
        }
        let word = runtime.block_on(translate_text(&settings, &models, "serverless")).unwrap();
        println!("{} | {} | {:?}", word.text, word.engine, word.note);
        assert_eq!(word.engine, "google");
        let sentence = runtime.block_on(translate_text(&settings, &models, "A large financial institution operates a data center.")).unwrap();
        println!("{} | {}", sentence.text, sentence.engine);
        assert_eq!(sentence.engine, "offline");
    }

    #[test]
    fn direction_follows_the_language_pair() {
        let settings = Settings::default();
        assert_eq!(direction(&settings, "Hello world"), ("auto".to_string(), "vi".to_string()));
        assert_eq!(direction(&settings, "Xin chào thế giới, tôi đang học"), ("vi".to_string(), "en".to_string()));
        assert_eq!(direction(&settings, "こんにちは"), ("ja".to_string(), "vi".to_string()));
    }
}
