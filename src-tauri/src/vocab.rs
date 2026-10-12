//! Saved words and sentences for flashcards, newest first, kept in a small JSON file.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::settings::data_dir;

const MAX_CARDS: usize = 5000;
static LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Card {
    /// Unix time in seconds.
    pub at: u64,
    /// The text the user selected.
    pub front: String,
    /// Its translation.
    pub back: String,
    /// Phonetic and dictionary senses, one line each; empty for sentences.
    pub detail: String,
    pub from: String,
    pub to: String,
}

fn path(app: &AppHandle) -> PathBuf {
    data_dir(app).join("vocab.json")
}

fn read(app: &AppHandle) -> Vec<Card> {
    fs::read_to_string(path(app))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn write(app: &AppHandle, cards: &[Card]) -> Result<(), String> {
    let raw = serde_json::to_string(cards).map_err(|e| e.to_string())?;
    fs::write(path(app), raw).map_err(|e| e.to_string())
}

fn same(a: &Card, front: &str, to: &str) -> bool {
    a.to == to && a.front.trim().eq_ignore_ascii_case(front.trim())
}

/// Puts `card` first, replacing an older card for the same text, and caps the list.
pub fn push(cards: &mut Vec<Card>, card: Card) {
    cards.retain(|c| !same(c, &card.front, &card.to));
    cards.insert(0, card);
    cards.truncate(MAX_CARDS);
}

#[tauri::command]
pub fn vocab_add(app: AppHandle, mut card: Card) -> Result<(), String> {
    card.front = card.front.trim().to_string();
    if card.front.is_empty() {
        return Err("Không có nội dung để lưu".into());
    }
    card.at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut cards = read(&app);
    push(&mut cards, card);
    write(&app, &cards)
}

#[tauri::command]
pub fn vocab_list(app: AppHandle) -> Vec<Card> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    read(&app)
}

#[tauri::command]
pub fn vocab_remove(app: AppHandle, front: String, to: String) -> Result<(), String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut cards = read(&app);
    cards.retain(|c| !same(c, &front, &to));
    write(&app, &cards)
}

#[tauri::command]
pub fn vocab_has(app: AppHandle, front: String, to: String) -> bool {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    read(&app).iter().any(|c| same(c, &front, &to))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(front: &str, at: u64) -> Card {
        Card { at, front: front.into(), back: "x".into(), to: "vi".into(), ..Default::default() }
    }

    #[test]
    fn newest_first_and_case_insensitive() {
        let mut list = Vec::new();
        push(&mut list, card("Attend", 1));
        push(&mut list, card("meeting", 2));
        push(&mut list, card("attend ", 3));
        assert_eq!(list.iter().map(|c| c.at).collect::<Vec<_>>(), [3, 2]);
    }

    #[test]
    fn old_files_without_detail_still_load() {
        let cards: Vec<Card> = serde_json::from_str(r#"[{"at":1,"front":"a","back":"b"}]"#).unwrap();
        assert_eq!(cards[0].detail, "");
    }
}
