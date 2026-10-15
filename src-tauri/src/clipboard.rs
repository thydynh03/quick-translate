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
    /// True when this entry represents an image copied to the clipboard
    /// (e.g. a screenshot). The text is a display-only placeholder.
    #[serde(default)]
    pub image: bool,
    /// Base64 data URL (data:image/png;base64,...) for previewing screenshots/images.
    #[serde(default)]
    pub image_data: Option<String>,
    /// Rich text HTML format when available, preserving fonts, styles, and colors.
    #[serde(default)]
    pub html: Option<String>,
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

fn rgba_to_png_data_url(width: u32, height: u32, rgba: &[u8]) -> Option<String> {
    use image::{ImageBuffer, Rgba};
    use std::io::Cursor;
    use base64::Engine;

    let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, rgba.to_vec())?;
    let max_dim = 960;
    let preview = if width > max_dim || height > max_dim {
        let (nw, nh) = if width > height {
            (max_dim, (height * max_dim / width).max(1))
        } else {
            ((width * max_dim / height).max(1), max_dim)
        };
        image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let mut buf = Cursor::new(Vec::new());
    preview.write_to(&mut buf, image::ImageFormat::Png).ok()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
    Some(format!("data:image/png;base64,{b64}"))
}

fn set_clipboard_image_from_data_url(data_url: &str) -> anyhow::Result<()> {
    use base64::Engine;
    let b64 = data_url.strip_prefix("data:image/png;base64,").unwrap_or(data_url);
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
    let dynamic = image::load_from_memory(&bytes)?;
    let rgba = dynamic.to_rgba8();
    let (width, height) = (rgba.width() as usize, rgba.height() as usize);
    let raw = rgba.into_raw();
    let img_data = arboard::ImageData {
        width,
        height,
        bytes: std::borrow::Cow::Owned(raw),
    };
    let mut cb = arboard::Clipboard::new()?;
    cb.set_image(img_data)?;
    Ok(())
}

/// Copied texts, newest first.
#[derive(Default)]
pub struct Store {
    items: Vec<Item>,
    next: u64,
}

impl Store {
    /// Puts a copied text on top; a text already known moves up instead.
    pub fn add(&mut self, text: &str, html: Option<String>, app: Option<String>) -> bool {
        self.add_item(text, html, false, None, app)
    }

    /// Puts an image placeholder on top with preview image data.
    pub fn add_image(&mut self, data_url: Option<String>, width: usize, height: usize, app: Option<String>) -> bool {
        let label = if width > 0 && height > 0 {
            format!("[Ảnh {width}×{height}]")
        } else {
            "[Ảnh 📷]".to_string()
        };
        if let Some(newest) = self.items.first_mut().filter(|item| item.image) {
            newest.text = label;
            if data_url.is_some() {
                newest.image_data = data_url;
            }
            newest.app = app;
            newest.at = now();
            return false;
        }
        self.add_item(&label, None, true, data_url, app)
    }

    fn add_item(&mut self, text: &str, html: Option<String>, image: bool, image_data: Option<String>, app: Option<String>) -> bool {
        if !image && (text.trim().is_empty() || text.len() > MAX_BYTES) {
            return false;
        }
        if let Some(newest) = self.items.first_mut().filter(|item| item.text == text && item.image == image) {
            newest.app = app;
            newest.at = now();
            if html.is_some() {
                newest.html = html;
            }
            return false;
        }
        let pinned = match self.items.iter().position(|item| item.text == text && item.image == image) {
            Some(index) => self.items.remove(index).pinned,
            None => false,
        };
        self.next += 1;
        self.items.insert(0, Item { id: self.next, text: text.to_string(), image, image_data, html, pinned, app, at: now() });
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
        if let Err(error) = std::fs::write(pinned_path(app), json) {
            log(format!("clipboard: pinned texts not saved: {error}"));
        }
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
        .title("Swipelate Clipboard")
        .inner_size(WIDTH, HEIGHT)
        .min_inner_size(520.0, 340.0)
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
        .resizable(true)
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
        let source = platform::foreground_app().filter(|name| !name.starts_with("swipelate"));
        let excluded = source.as_ref().is_some_and(|name| {
            app.state::<AppState>().settings.read().excluded_apps.iter().any(|e| e.eq_ignore_ascii_case(name))
        });
        if excluded {
            continue;
        }
        // Try text first; fall back to an image placeholder when the clipboard holds a picture.
        let added = if let Some(text) = arboard::Clipboard::new().and_then(|mut cb| cb.get_text()).ok() {
            let html = platform::clipboard_html();
            STORE.lock().add(&text, html, source)
        } else if platform::clipboard_has_image() {
            let (data_url, w, h) = if let Ok(img) = arboard::Clipboard::new().and_then(|mut cb| cb.get_image()) {
                let url = rgba_to_png_data_url(img.width as u32, img.height as u32, &img.bytes);
                (url, img.width, img.height)
            } else {
                (None, 0, 0)
            };
            STORE.lock().add_image(data_url, w, h, source)
        } else {
            false
        };
        if added {
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
    let item = STORE.lock().items.iter().find(|i| i.id == id).cloned().ok_or("Mục này không còn trong danh sách")?;
    let target = TARGET.load(Ordering::SeqCst);
    if item.image {
        // If we have saved image data, restore it to clipboard in case clipboard was overwritten
        if let Some(ref data_url) = item.image_data {
            let _ = own(|| set_clipboard_image_from_data_url(data_url).map_err(|e| arboard::Error::Unknown { description: e.to_string() }));
        }
        std::thread::spawn(move || {
            platform::activate(target);
            hide(&app);
            std::thread::sleep(Duration::from_millis(150));
            if target != 0 && platform::foreground_handle() != target {
                log("clipboard: target not in front, paste left to the user");
                return;
            }
            platform::send_paste();
        });
    } else {
        // If rich text HTML was copied, restore HTML format so target apps receive font & style
        if let Some(ref html) = item.html {
            let _ = own(|| platform::set_clipboard_html(html, &item.text));
        } else {
            // As a copy of our own, so the list keeps its order.
            own(|| arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(item.text))).map_err(|error| error.to_string())?;
        }
        std::thread::spawn(move || {
            // In this order: once the board is gone, the system picks the next window by itself.
            platform::activate(target);
            hide(&app);
            std::thread::sleep(Duration::from_millis(150));
            // The system may refuse to bring the target back; pasting would then land elsewhere.
            // The text stays on the clipboard for the user to paste.
            if target != 0 && platform::foreground_handle() != target {
                log("clipboard: target not in front, paste left to the user");
                return;
            }
            platform::send_paste();
        });
    }
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
        assert!(store.add("one", None, None));
        assert!(store.add("two", None, None));
        assert!(!store.add("two", None, None));
        assert!(store.add("one", None, None));
        assert_eq!(texts(&store), ["one", "two"]);
        assert!(!store.add("  \n", None, None));
        assert!(!store.add(&"x".repeat(MAX_BYTES + 1), None, None));
    }

    #[test]
    fn pinned_texts_outlive_the_limit_and_a_clear() {
        let mut store = Store::default();
        store.add("keep", None, None);
        let id = store.items[0].id;
        store.pin(id, true);
        for number in 0..LIMIT + 5 {
            store.add(&number.to_string(), None, None);
        }
        assert_eq!(store.items.len(), LIMIT + 1);
        assert!(texts(&store).contains(&"keep"));
        // Copied again, it stays pinned.
        store.add("keep", None, None);
        assert!(store.items[0].pinned);
        store.clear();
        assert_eq!(texts(&store), ["keep"]);
        store.remove(store.items[0].id);
        assert!(store.items.is_empty());
    }
}
