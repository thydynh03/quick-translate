//! Translation history, newest first, kept in a small JSON file.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::engine::Translation;
use crate::settings::data_dir;

const MAX_ENTRIES: usize = 500;
static LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    /// Unix time in seconds.
    pub at: u64,
    pub source: String,
    pub text: String,
    pub from: String,
    pub to: String,
    pub engine: String,
}

fn path(app: &AppHandle) -> PathBuf {
    data_dir(app).join("history.json")
}

fn read(app: &AppHandle) -> Vec<Entry> {
    fs::read_to_string(path(app))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Puts `entry` first, drops an older copy of the same source and caps the list.
pub fn push(entries: &mut Vec<Entry>, entry: Entry) {
    entries.retain(|e| !(e.source == entry.source && e.to == entry.to));
    entries.insert(0, entry);
    entries.truncate(MAX_ENTRIES);
}

pub fn add(app: &AppHandle, source: &str, translation: &Translation) {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut entries = read(app);
    let at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    push(
        &mut entries,
        Entry {
            at,
            source: source.to_string(),
            text: translation.text.clone(),
            from: translation.from.clone(),
            to: translation.to.clone(),
            engine: translation.engine.clone(),
        },
    );
    if let Ok(raw) = serde_json::to_string(&entries) {
        let _ = fs::write(path(app), raw);
    }
}

#[tauri::command]
pub fn history_list(app: AppHandle) -> Vec<Entry> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    read(&app)
}

#[tauri::command]
pub fn history_clear(app: AppHandle) {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = fs::remove_file(path(&app));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(source: &str, at: u64) -> Entry {
        Entry { at, source: source.into(), text: "x".into(), from: "en".into(), to: "vi".into(), engine: "google".into() }
    }

    #[test]
    fn newest_first_without_duplicates() {
        let mut list = Vec::new();
        push(&mut list, entry("a", 1));
        push(&mut list, entry("b", 2));
        push(&mut list, entry("a", 3));
        assert_eq!(list.iter().map(|e| e.source.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(list[0].at, 3);
    }

    #[test]
    fn capped() {
        let mut list = Vec::new();
        for i in 0..(MAX_ENTRIES + 20) {
            push(&mut list, entry(&i.to_string(), i as u64));
        }
        assert_eq!(list.len(), MAX_ENTRIES);
    }
}
