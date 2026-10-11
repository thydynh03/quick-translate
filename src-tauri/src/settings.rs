use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

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
