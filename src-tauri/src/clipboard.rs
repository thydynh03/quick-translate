//! Clipboard board: the texts copied lately, in a small window to paste one
//! of them again. Works like Win+V on Windows: the list lives in memory and
//! only pinned texts survive a restart.

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

use crate::{log, platform, popup, settings, AppState};

pub const LABEL: &str = "clipboard";
const WIDTH: f64 = 680.0;
const HEIGHT: f64 = 430.0;
/// Texts kept besides the pinned ones.
const LIMIT: usize = 50;
/// Longer texts are not kept.
const MAX_BYTES: usize = 100_000;
const TICK: Duration = Duration::from_millis(300);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub text: String,
    pub pinned: bool,
    /// The application it was copied in, when known.
    #[serde(default)]
    pub app: Option<String>,
    /// When it was copied, in milliseconds since 1970.
    #[serde(default)]
    pub at: u64,
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.as_millis() as u64).unwrap_or(0)
}

/// Copied texts, newest first.
#[derive(Default)]
pub struct Store {
    items: Vec<Item>,
    next: u64,
}

impl Store {
    /// Puts a copied text on top; a text already known moves up instead.
    pub fn add(&mut self, text: &str, app: Option<String>) -> bool {
        if text.trim().is_empty() || text.len() > MAX_BYTES {
            return false;
        }
        if self.items.first().is_some_and(|item| item.text == text) {
            return false;
        }
        let pinned = match self.items.iter().position(|item| item.text == text) {
            Some(index) => self.items.remove(index).pinned,
            None => false,
        };
        self.next += 1;
        self.items.insert(0, Item { id: self.next, text: text.to_string(), pinned, app, at: now() });
        // The oldest texts that are not pinned make room.
        let mut loose = 0;
        self.items.retain(|item| {
            item.pinned || {
                loose += 1;
                loose <= LIMIT
            }
        });
        true
    }

    pub fn pin(&mut self, id: u64, pinned: bool) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.pinned = pinned;
        }
    }

    pub fn remove(&mut self, id: u64) {
        self.items.retain(|item| item.id != id);
    }

    /// Empties the list; pinned texts stay.
    pub fn clear(&mut self) {
        self.items.retain(|item| item.pinned);
    }

    pub fn text(&self, id: u64) -> Option<String> {
        self.items.iter().find(|item| item.id == id).map(|item| item.text.clone())
    }

    fn pinned(&self) -> Vec<Item> {
        self.items.iter().filter(|item| item.pinned).cloned().collect()
    }
}

static STORE: LazyLock<Mutex<Store>> = LazyLock::new(Mutex::default);
/// The app itself is copying (reading a selection through Ctrl/Cmd+C).
static BUSY: AtomicBool = AtomicBool::new(false);
/// How many times the app copied by itself; the watcher skips those changes.
static OWN: AtomicU64 = AtomicU64::new(0);
/// The window that was in front when the board opened, to paste into.
static TARGET: AtomicIsize = AtomicIsize::new(0);

/// Runs a copy the app makes for itself, so it does not end up in the list.
pub fn own<T>(copy: impl FnOnce() -> T) -> T {
    BUSY.store(true, Ordering::SeqCst);
    let result = copy();
    OWN.fetch_add(1, Ordering::SeqCst);
    BUSY.store(false, Ordering::SeqCst);
    result
}

fn pinned_path(app: &AppHandle) -> std::path::PathBuf {
    settings::data_dir(app).join("clipboard.json")
}

fn save_pinned(app: &AppHandle) {
    let pinned = STORE.lock().pinned();
    if let Ok(json) = serde_json::to_string(&pinned) {
        let _ = std::fs::write(pinned_path(app), json);
    }
}

fn enabled(app: &AppHandle) -> bool {
    !app.state::<AppState>().settings.read().clipboard_hotkey.trim().is_empty()
}

fn changed(app: &AppHandle) {
    let _ = app.emit_to(LABEL, "clipboard-changed", ());
}

pub fn create_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::App("index.html".into()))
        .title("Quick Translate Clipboard")
        .inner_size(WIDTH, HEIGHT)
        .visible(false)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        // Glass: the system blurs what is behind the window. Each system takes the effect it knows.
        .effects(
            EffectsBuilder::new()
                .effects([Effect::Popover, Effect::Acrylic])
                .state(EffectState::Active)
                .radius(14.0)
                .build(),
        )
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()
}

/// Loads the pinned texts and starts watching the clipboard.
pub fn start(app: AppHandle) {
    if let Ok(json) = std::fs::read_to_string(pinned_path(&app)) {
        let mut pinned: Vec<Item> = serde_json::from_str(&json).unwrap_or_default();
        let mut store = STORE.lock();
        store.next = pinned.iter().map(|item| item.id).max().unwrap_or(0);
        pinned.retain(|item| item.pinned);
        store.items = pinned;
    }
    std::thread::spawn(move || watch(app));
}

fn watch(app: AppHandle) {
    let mut seen = platform::clipboard_change();
    let mut own = OWN.load(Ordering::SeqCst);
    loop {
        std::thread::sleep(TICK);
        if BUSY.load(Ordering::SeqCst) {
            continue;
        }
        let now = platform::clipboard_change();
        let own_now = OWN.load(Ordering::SeqCst);
        if now == seen {
            own = own_now;
            continue;
        }
        seen = now;
        if own_now != own {
            own = own_now;
            continue;
        }
        if !enabled(&app) || platform::clipboard_concealed() {
            continue;
        }
        // Texts the app itself puts there (a copied translation) have no source to show.
        let source = platform::foreground_app().filter(|name| !name.starts_with("quick-translate"));
        let excluded = source.as_ref().is_some_and(|name| {
            app.state::<AppState>().settings.read().excluded_apps.iter().any(|e| e.eq_ignore_ascii_case(name))
        });
        if excluded {
            continue;
        }
        let Some(text) = arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()).ok() else { continue };
        if STORE.lock().add(&text, source) {
            changed(&app);
        }
    }
}

fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}

/// The shortcut: opens the board on the screen the pointer is on, or closes it.
pub fn toggle(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    if window.is_visible().unwrap_or(false) {
        hide(app);
        return;
    }
    TARGET.store(platform::foreground_handle(), Ordering::SeqCst);
    let (x, y) = platform::cursor_pos();
    let monitor = app.monitor_from_point(x as f64, y as f64).ok().flatten().or_else(|| app.primary_monitor().ok().flatten());
    if let Some(monitor) = monitor {
        let area = monitor.work_area();
        let width = (WIDTH * monitor.scale_factor()) as i32;
        let height = (HEIGHT * monitor.scale_factor()) as i32;
        // Centred and a little above the middle, where macOS puts Spotlight.
        let left = area.position.x + (area.size.width as i32 - width) / 2;
        let top = area.position.y + ((area.size.height as i32 - height) / 3).max(0);
        let _ = window.set_position(PhysicalPosition::new(left, top));
    }
    let _ = app.emit_to(LABEL, "clipboard-open", ());
    let _ = window.show();
    let _ = window.set_focus();
    log("clipboard: board opened");
}

/// The board lost the keyboard: the user went elsewhere.
pub fn blurred(app: &AppHandle) {
    hide(app);
}

#[tauri::command]
pub fn clipboard_list() -> Vec<Item> {
    STORE.lock().items.clone()
}

/// Puts a text of the list back on the clipboard and pastes it where the user was.
#[tauri::command]
pub fn clipboard_paste(app: AppHandle, id: u64) -> Result<(), String> {
    let text = STORE.lock().text(id).ok_or("Mục này không còn trong danh sách")?;
    arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)).map_err(|error| error.to_string())?;
    let target = TARGET.load(Ordering::SeqCst);
    std::thread::spawn(move || {
        // In this order: once the board is gone, the system picks the next window by itself.
        platform::activate(target);
        hide(&app);
        platform::send_paste();
    });
    Ok(())
}

#[tauri::command]
pub fn clipboard_pin(app: AppHandle, id: u64, pinned: bool) {
    STORE.lock().pin(id, pinned);
    save_pinned(&app);
    changed(&app);
}

#[tauri::command]
pub fn clipboard_remove(app: AppHandle, id: u64) {
    STORE.lock().remove(id);
    save_pinned(&app);
    changed(&app);
}

#[tauri::command]
pub fn clipboard_clear(app: AppHandle) {
    STORE.lock().clear();
    changed(&app);
}

#[tauri::command]
pub fn clipboard_hide(app: AppHandle) {
    hide(&app);
}

/// Translates a text of the list in the usual popup.
#[tauri::command]
pub fn clipboard_translate(app: AppHandle, id: u64) {
    let Some(text) = STORE.lock().text(id) else { return };
    hide(&app);
    popup::translate_text(&app, text, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(store: &Store) -> Vec<&str> {
        store.items.iter().map(|item| item.text.as_str()).collect()
    }

    #[test]
    fn a_known_text_moves_to_the_top() {
        let mut store = Store::default();
        assert!(store.add("one", None));
        assert!(store.add("two", None));
        assert!(!store.add("two", None));
        assert!(store.add("one", None));
        assert_eq!(texts(&store), ["one", "two"]);
        assert!(!store.add("  \n", None));
        assert!(!store.add(&"x".repeat(MAX_BYTES + 1), None));
    }

    #[test]
    fn pinned_texts_outlive_the_limit_and_a_clear() {
        let mut store = Store::default();
        store.add("keep", None);
        let id = store.items[0].id;
        store.pin(id, true);
        for number in 0..LIMIT + 5 {
            store.add(&number.to_string(), None);
        }
        assert_eq!(store.items.len(), LIMIT + 1);
        assert!(texts(&store).contains(&"keep"));
        // Copied again, it stays pinned.
        store.add("keep", None);
        assert!(store.items[0].pinned);
        store.clear();
        assert_eq!(texts(&store), ["keep"]);
        store.remove(store.items[0].id);
        assert!(store.items.is_empty());
    }
}
