//! Translation history, newest first, kept in a small JSON file.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::engine::Translation;
use crate::settings::data_dir;

pub const DEFAULT_MAX_ENTRIES: usize = 500;
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

fn write(app: &AppHandle, entries: &[Entry]) {
    if let Ok(raw) = serde_json::to_string(entries) {
        let _ = fs::write(path(app), raw);
    }
}

/// Puts `entry` first, drops an older copy of the same source, prunes expired entries, and caps the list.
pub fn push_with_limits(entries: &mut Vec<Entry>, entry: Entry, max_entries: usize, retention_days: u32, now_secs: u64) {
    if retention_days > 0 && now_secs > 0 {
        let cutoff = now_secs.saturating_sub(retention_days as u64 * 86_400);
        entries.retain(|e| e.at >= cutoff);
    }
    entries.retain(|e| !(e.source == entry.source && e.to == entry.to));
    entries.insert(0, entry);
    if max_entries > 0 {
        entries.truncate(max_entries);
    }
}

#[allow(dead_code)]
pub fn push(entries: &mut Vec<Entry>, entry: Entry) {
    push_with_limits(entries, entry, DEFAULT_MAX_ENTRIES, 0, 0);
}

pub fn add_with_limits(app: &AppHandle, source: &str, translation: &Translation, max_entries: usize, retention_days: u32) {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut entries = read(app);
    let at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    push_with_limits(
        &mut entries,
        Entry {
            at,
            source: source.to_string(),
            text: translation.text.clone(),
            from: translation.from.clone(),
            to: translation.to.clone(),
            engine: translation.engine.clone(),
        },
        max_entries,
        retention_days,
        at,
    );
    write(app, &entries);
}

pub fn add(app: &AppHandle, source: &str, translation: &Translation) {
    let (max, retention) = match app.try_state::<crate::AppState>() {
        Some(state) => {
            let s = state.settings.read();
            (s.history_max_entries, s.history_retention_days)
        }
        None => (DEFAULT_MAX_ENTRIES, 30),
    };
    add_with_limits(app, source, translation, max, retention);
}

/// Prunes entries that are older than `retention_days` or beyond `max_entries`.
pub fn prune(app: &AppHandle, max_entries: usize, retention_days: u32) {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut entries = read(app);
    let original_len = entries.len();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if retention_days > 0 {
        let cutoff = now.saturating_sub(retention_days as u64 * 86_400);
        entries.retain(|e| e.at >= cutoff);
    }
    if max_entries > 0 {
        entries.truncate(max_entries);
    }
    if entries.len() != original_len {
        write(app, &entries);
    }
}

/// Runs auto-prune on startup in a background thread according to saved settings.
pub fn prune_on_start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let (max, retention) = match app.try_state::<crate::AppState>() {
            Some(state) => {
                let s = state.settings.read();
                (s.history_max_entries, s.history_retention_days)
            }
            None => (DEFAULT_MAX_ENTRIES, 30),
        };
        prune(&app, max, retention);
    });
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

#[tauri::command]
pub fn history_clear_older_than(app: AppHandle, days: u32) -> Result<usize, String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut entries = read(&app);
    let original_len = entries.len();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let cutoff = now.saturating_sub(days as u64 * 86_400);
    entries.retain(|e| e.at >= cutoff);
    let removed = original_len - entries.len();
    if removed > 0 {
        write(&app, &entries);
    }
    Ok(removed)
}

#[tauri::command]
pub fn history_clear_matches(app: AppHandle, query: String) -> Result<usize, String> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(0);
    }
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut entries = read(&app);
    let original_len = entries.len();
    entries.retain(|e| !e.source.to_lowercase().contains(&needle) && !e.text.to_lowercase().contains(&needle));
    let removed = original_len - entries.len();
    if removed > 0 {
        write(&app, &entries);
    }
    Ok(removed)
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
        for i in 0..(DEFAULT_MAX_ENTRIES + 20) {
            push(&mut list, entry(&i.to_string(), i as u64));
        }
        assert_eq!(list.len(), DEFAULT_MAX_ENTRIES);
    }

    #[test]
    fn prunes_by_days() {
        let mut list = Vec::new();
        let now = 100 * 86_400;
        let old = now - 35 * 86_400; // 35 days old
        let fresh = now - 5 * 86_400; // 5 days old
        push_with_limits(&mut list, entry("old", old), 500, 30, now);
        push_with_limits(&mut list, entry("fresh", fresh), 500, 30, now);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].source, "fresh");
    }
}
