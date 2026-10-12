use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// Overrides used while a given application is in front.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// Executable name, lower-case, for example "chrome.exe".
    pub app: String,
    /// Engine to use there; empty keeps the general one.
    pub engine: String,
    /// Language to translate into there; empty keeps the general one.
    pub primary_lang: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Global shortcut that translates the current selection.
    pub hotkey: String,
    /// Global shortcut that translates a screen region with OCR.
    pub ocr_hotkey: String,
    /// Show the small icon after a drag-select or double-click.
    pub auto_trigger: bool,
    /// Translate the word under the pointer while Ctrl is held.
    pub hover_translate: bool,
    /// "google" | "deepl" | "claude" | "offline"
    pub engine: String,
    /// Use the offline engine when the online one fails.
    pub fallback_offline: bool,
    /// Language everything is translated into.
    pub primary_lang: String,
    /// Target when the text is already in the primary language.
    pub secondary_lang: String,
    pub deepl_key: String,
    pub anthropic_key: String,
    /// Executable names (lower-case) where the auto trigger stays off.
    pub excluded_apps: Vec<String>,
    pub save_history: bool,
    /// Add English definitions and IPA when a single English word is looked up.
    pub english_definitions: bool,
    /// Oxford Dictionaries API credentials (optional).
    pub oxford_app_id: String,
    pub oxford_app_key: String,
    /// Global shortcut that translates every text block of the active window in place.
    pub window_hotkey: String,
    pub live_hotkey: String,
    /// Translate as soon as text is selected, without showing the icon first.
    pub instant_translate: bool,
    /// Popup text size in CSS pixels.
    pub popup_font_size: u32,
    /// Popup background opacity in percent.
    pub popup_opacity: u32,
    /// Any OpenAI-compatible chat API: OpenAI, Gemini's compatible endpoint, Ollama, LM Studio.
    pub openai_base: String,
    pub openai_key: String,
    pub openai_model: String,
    pub profiles: Vec<Profile>,
}

impl Settings {
    /// These settings with the profile of `app` applied, if it has one.
    pub fn for_app(&self, app: Option<&str>) -> Settings {
        let mut settings = self.clone();
        let profile = app.and_then(|name| self.profiles.iter().find(|p| p.app.trim().eq_ignore_ascii_case(name)));
        if let Some(profile) = profile {
            if !profile.engine.is_empty() {
                settings.engine = profile.engine.clone();
            }
            if !profile.primary_lang.is_empty() {
                // Keep the two languages different so text in the target still has somewhere to go.
                if profile.primary_lang == settings.secondary_lang {
                    settings.secondary_lang = settings.primary_lang.clone();
                }
                settings.primary_lang = profile.primary_lang.clone();
            }
        }
        settings
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            hotkey: "Alt+Q".into(),
            ocr_hotkey: "Alt+W".into(),
            auto_trigger: true,
            hover_translate: false,
            engine: "google".into(),
            fallback_offline: true,
            primary_lang: "vi".into(),
            secondary_lang: "en".into(),
            deepl_key: String::new(),
            anthropic_key: String::new(),
            excluded_apps: Vec::new(),
            save_history: true,
            english_definitions: true,
            oxford_app_id: String::new(),
            oxford_app_key: String::new(),
            window_hotkey: "Alt+E".into(),
            live_hotkey: "Alt+R".into(),
            instant_translate: false,
            popup_font_size: 14,
            popup_opacity: 100,
            openai_base: String::new(),
            openai_key: String::new(),
            openai_model: String::new(),
            profiles: Vec::new(),
        }
    }
}

pub fn data_dir(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("quick-translate"));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn path(app: &AppHandle) -> PathBuf {
    data_dir(app).join("settings.json")
}

pub fn load(app: &AppHandle) -> Settings {
    fs::read_to_string(path(app))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) -> anyhow::Result<()> {
    fs::write(path(app), serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_profile() -> Settings {
        let profile = Profile { app: "game.exe".into(), engine: "offline".into(), primary_lang: "en".into() };
        Settings { profiles: vec![profile], ..Default::default() }
    }

    #[test]
    fn profile_overrides_engine_and_language() {
        let settings = with_profile().for_app(Some("Game.exe"));
        assert_eq!(settings.engine, "offline");
        assert_eq!(settings.primary_lang, "en");
        assert_eq!(settings.secondary_lang, "vi");
    }

    #[test]
    fn other_apps_keep_the_general_settings() {
        let settings = with_profile();
        assert_eq!(settings.for_app(Some("notepad.exe")).engine, "google");
        assert_eq!(settings.for_app(None).primary_lang, "vi");
    }
}
