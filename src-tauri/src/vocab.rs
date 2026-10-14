//! Saved words and sentences for flashcards, newest first, kept in a small JSON file.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::settings::data_dir;

const MAX_CARDS: usize = 5000;
static LOCK: Mutex<()> = Mutex::new(());
type VocabCache = parking_lot::Mutex<Option<HashSet<(String, String)>>>;
static CACHE: OnceLock<VocabCache> = OnceLock::new();

fn cache() -> &'static VocabCache {
    CACHE.get_or_init(|| parking_lot::Mutex::new(None))
}

fn warm_cache(cards: &[Card]) {
    let mut set = HashSet::with_capacity(cards.len());
    for c in cards {
        set.insert((c.front.trim().to_lowercase(), c.to.clone()));
    }
    *cache().lock() = Some(set);
}

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

/// A missing file is an empty list; a damaged one is an error, so it is never overwritten.
fn read(app: &AppHandle) -> Result<Vec<Card>, String> {
    match fs::read_to_string(path(app)) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| format!("vocab.json bị hỏng: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}

fn write(app: &AppHandle, cards: &[Card]) -> Result<(), String> {
    let raw = serde_json::to_string(cards).map_err(|e| e.to_string())?;
    fs::write(path(app), raw).map_err(|e| e.to_string())
}

fn same(a: &Card, front: &str, to: &str) -> bool {
    a.to == to && a.front.trim().to_lowercase() == front.trim().to_lowercase()
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
    let mut cards = read(&app)?;
    push(&mut cards, card.clone());
    write(&app, &cards)?;
    // Update cache
    let key = (card.front.to_lowercase(), card.to);
    if let Some(set) = cache().lock().as_mut() {
        set.insert(key);
    }
    Ok(())
}

#[tauri::command]
pub fn vocab_list(app: AppHandle) -> Result<Vec<Card>, String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cards = read(&app)?;
    if cache().lock().is_none() {
        warm_cache(&cards);
    }
    Ok(cards)
}

#[tauri::command]
pub fn vocab_remove(app: AppHandle, front: String, to: String) -> Result<(), String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut cards = read(&app)?;
    cards.retain(|c| !same(c, &front, &to));
    write(&app, &cards)?;
    // Update cache
    let key = (front.trim().to_lowercase(), to);
    if let Some(set) = cache().lock().as_mut() {
        set.remove(&key);
    }
    Ok(())
}

#[tauri::command]
pub fn vocab_has(app: AppHandle, front: String, to: String) -> Result<bool, String> {
    let key = (front.trim().to_lowercase(), to);
    {
        let cache_guard = cache().lock();
        if let Some(set) = cache_guard.as_ref() {
            return Ok(set.contains(&key));
        }
    }
    // Cache miss on first call: warm from disk
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cards = read(&app)?;
    warm_cache(&cards);
    Ok(cards.iter().any(|c| same(c, &key.0, &key.1)))
}

#[tauri::command]
pub fn vocab_clear_matches(app: AppHandle, query: String) -> Result<usize, String> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(0);
    }
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut cards = read(&app)?;
    let original_len = cards.len();
    cards.retain(|c| !c.front.to_lowercase().contains(&needle) && !c.back.to_lowercase().contains(&needle));
    let removed = original_len - cards.len();
    if removed > 0 {
        write(&app, &cards)?;
        warm_cache(&cards);
    }
    Ok(removed)
}

/// Exports cards to CSV format suitable for Anki, Quizlet, or Excel.
#[tauri::command]
pub fn vocab_export_csv(app: AppHandle) -> Result<String, String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let cards = read(&app)?;
    let mut csv = String::from("Front,Back,Detail,From,To,Date\n");
    for card in cards {
        let escape = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        let date = chrono_like_date(card.at);
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            escape(&card.front),
            escape(&card.back),
            escape(&card.detail),
            escape(&card.from),
            escape(&card.to),
            escape(&date)
        ));
    }
    Ok(csv)
}

fn chrono_like_date(unix_secs: u64) -> String {
    let days = unix_secs / 86400;
    // Approximating YYYY-MM-DD from Unix epoch days without extra chrono crate
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[tauri::command]
pub fn vocab_import_cards(app: AppHandle, new_cards: Vec<Card>) -> Result<usize, String> {
    if new_cards.is_empty() {
        return Ok(0);
    }
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut cards = read(&app)?;
    let mut added = 0;
    for card in new_cards {
        let front = card.front.trim().to_string();
        if front.is_empty() || cards.iter().any(|c| same(c, &front, &card.to)) {
            continue;
        }
        let at = if card.at > 0 { card.at } else { SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) };
        cards.insert(0, Card { front, at, ..card });
        added += 1;
        if cards.len() >= MAX_CARDS {
            break;
        }
    }
    if added > 0 {
        write(&app, &cards)?;
        warm_cache(&cards);
    }
    Ok(added)
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
        push(&mut list, card("word", 1));
        push(&mut list, card("other", 2));
        push(&mut list, card("Word", 3));
        assert_eq!(list.iter().map(|c| c.front.as_str()).collect::<Vec<_>>(), ["Word", "other"]);
        assert_eq!(list[0].at, 3);
    }

    #[test]
    fn old_files_without_detail_still_load() {
        let raw = r#"[{"at": 1, "front": "hello", "back": "xin chào", "from": "en", "to": "vi"}]"#;
        let cards: Vec<Card> = serde_json::from_str(raw).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].detail, "");
    }

    #[test]
    fn formats_date_correctly() {
        // 1792022400 is 2026-10-15 00:00:00 UTC
        let date = chrono_like_date(1792022400);
        assert_eq!(date, "2026-10-15");
    }
}
