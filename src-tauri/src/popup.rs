//! The floating popup: a small icon after a selection gesture, then a card
//! with the translation, anchored to the selected text.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::platform::{self, Rect};
use crate::{engine, history, log, AppState};

pub const LABEL: &str = "popup";
const CARD_WIDTH: f64 = 400.0;
const LOADING_HEIGHT: f64 = 96.0;
const MAX_CARD_HEIGHT: f64 = 460.0;
const ICON_SIZE: f64 = 34.0;
const GAP: i32 = 8;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Hidden,
    Icon,
    Card,
}

/// What the icon will translate once it is clicked.
#[derive(Clone, Debug)]
pub enum Pending {
    Text(String),
    /// The app did not expose its selection; copy it on click.
    NeedCopy,
}

pub struct PopupState {
    pub mode: Mode,
    pub pending: Option<Pending>,
    pub anchor: Rect,
    pub pinned: bool,
    pub payload: Value,
    /// Bumped on every show/hide so stale async results are dropped.
    pub generation: u64,
}

impl Default for PopupState {
    fn default() -> Self {
        PopupState {
            mode: Mode::Hidden,
            pending: None,
            anchor: Rect::default(),
            pinned: false,
            payload: json!({ "mode": "hidden" }),
            generation: 0,
        }
    }
}

/// Creates the hidden popup window. Done in code, after the settings window,
/// because WebView2 refuses to create both at once from the config.
pub fn create_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::App("index.html".into()))
        .title("Quick Translate Popup")
        .inner_size(CARD_WIDTH, LOADING_HEIGHT)
        .visible(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .build()
}

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

fn state(app: &AppHandle) -> &AppState {
    app.state::<AppState>().inner()
}

/// Screen rectangle of the popup while it is visible.
pub fn bounds(app: &AppHandle) -> Option<Rect> {
    if state(app).popup.lock().mode == Mode::Hidden {
        return None;
    }
    let window = window(app)?;
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some(Rect {
        left: position.x,
        top: position.y,
        right: position.x + size.width as i32,
        bottom: position.y + size.height as i32,
    })
}

pub fn mode(app: &AppHandle) -> Mode {
    state(app).popup.lock().mode
}

/// Hides the popup unless the user pinned the card.
pub fn dismiss(app: &AppHandle) {
    {
        let popup = state(app).popup.lock();
        if popup.mode == Mode::Hidden || (popup.mode == Mode::Card && popup.pinned) {
            return;
        }
    }
    hide(app);
}

pub fn hide(app: &AppHandle) {
    {
        let mut popup = state(app).popup.lock();
        popup.mode = Mode::Hidden;
        popup.pending = None;
        popup.pinned = false;
        popup.generation += 1;
        popup.payload = json!({ "mode": "hidden" });
    }
    if let Some(window) = window(app) {
        let _ = window.hide();
    }
    emit(app);
}

fn emit(app: &AppHandle) {
    let payload = state(app).popup.lock().payload.clone();
    let _ = app.emit_to(LABEL, "popup-state", payload);
}

/// Work area (physical pixels) and scale factor of the monitor under a point.
fn monitor_at(app: &AppHandle, x: i32, y: i32) -> (Rect, f64) {
    let monitor = app
        .monitor_from_point(x as f64, y as f64)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    match monitor {
        Some(m) => {
            let area = m.work_area();
            (
                Rect {
                    left: area.position.x,
                    top: area.position.y,
                    right: area.position.x + area.size.width as i32,
                    bottom: area.position.y + area.size.height as i32,
                },
                m.scale_factor(),
            )
        }
        None => (Rect { left: 0, top: 0, right: 1920, bottom: 1080 }, 1.0),
    }
}

/// Where a popup of `width` x `height` physical pixels goes for an anchor:
/// below it, or above when there is no room, always inside the work area.
pub fn placement(anchor: Rect, area: Rect, width: i32, height: i32) -> (i32, i32) {
    let max_x = (area.right - width - GAP).max(area.left + GAP);
    let x = anchor.left.clamp(area.left + GAP, max_x);

    let below = anchor.bottom + GAP;
    let above = anchor.top - GAP - height;
    let y = if below + height <= area.bottom - GAP {
        below
    } else if above >= area.top + GAP {
        above
    } else {
        (area.bottom - GAP - height).max(area.top + GAP)
    };
    (x, y)
}

fn place(app: &AppHandle, anchor: Rect, width: f64, height: f64) {
    let Some(window) = window(app) else { return };
    let (area, scale) = monitor_at(app, anchor.left, anchor.bottom);
    let w = (width * scale).round() as i32;
    let h = (height * scale).round() as i32;
    let (x, y) = placement(anchor, area, w, h);
    let _ = window.set_size(PhysicalSize::new(w as u32, h as u32));
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

fn show(app: &AppHandle) {
    if let Some(window) = window(app) {
        platform::show_overlay(&window);
    }
}

/// Shows the small icon next to the pointer after a selection gesture.
pub fn show_icon(app: &AppHandle, pending: Pending, x: i32, y: i32, rect: Option<Rect>) {
    {
        let mut popup = state(app).popup.lock();
        if popup.mode == Mode::Card && popup.pinned {
            return;
        }
        popup.mode = Mode::Icon;
        popup.pending = Some(pending);
        // Keep the pointer as anchor when the reported rectangle is far away
        // from it (some apps report the whole document).
        popup.anchor = match rect {
            Some(r) if r.left - 40 <= x && x <= r.right + 40 && r.top - 40 <= y && y <= r.bottom + 40 => r,
            _ => Rect { left: x, top: y - 10, right: x, bottom: y + 12 },
        };
        popup.generation += 1;
        popup.payload = json!({ "mode": "icon" });
    }
    let icon_anchor = Rect { left: x + 10, top: y, right: x + 10, bottom: y + 6 };
    place(app, icon_anchor, ICON_SIZE, ICON_SIZE);
    emit(app);
    show(app);
}

/// Opens the card in its loading state and translates `text` into it.
pub fn translate_text(app: &AppHandle, text: String, anchor: Option<Rect>) {
    let (generation, anchor) = {
        let mut popup = state(app).popup.lock();
        popup.mode = Mode::Card;
        popup.pending = None;
        if let Some(anchor) = anchor {
            popup.anchor = anchor;
        }
        popup.generation += 1;
        popup.payload = json!({ "mode": "loading", "source": text });
        (popup.generation, popup.anchor)
    };
    place(app, anchor, CARD_WIDTH, LOADING_HEIGHT);
    emit(app);
    show(app);

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let settings = state(&app).settings.read().clone();
        let key = format!("{}|{}|{}|{}", settings.engine, settings.primary_lang, settings.secondary_lang, text);
        let cached = state(&app).cache.lock().get(&key).cloned();
        let result = match cached {
            Some(hit) => Ok(hit),
            None => engine::translate(&settings, &engine::offline::models_dir(&app), &text).await,
        };
        let payload = match &result {
            Ok(translation) => {
                state(&app).cache.lock().insert(key, translation.clone());
                if settings.save_history {
                    history::add(&app, &text, translation);
                }
                json!({ "mode": "result", "source": text, "result": translation })
            }
            Err(error) => {
                log(format!("translate failed: {error:#}"));
                json!({ "mode": "error", "source": text, "message": error.to_string() })
            }
        };
        {
            let mut popup = state(&app).popup.lock();
            if popup.generation != generation {
                return;
            }
            popup.payload = payload;
        }
        emit(&app);
    });
}

pub fn show_error(app: &AppHandle, message: &str, anchor: Option<Rect>) {
    let anchor = {
        let mut popup = state(app).popup.lock();
        popup.mode = Mode::Card;
        popup.pending = None;
        if let Some(anchor) = anchor {
            popup.anchor = anchor;
        }
        popup.generation += 1;
        popup.payload = json!({ "mode": "error", "source": "", "message": message });
        popup.anchor
    };
    place(app, anchor, CARD_WIDTH, LOADING_HEIGHT);
    emit(app);
    show(app);
}

/// Translates the current selection: accessibility first, then copy.
pub fn translate_selection(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let (x, y) = platform::cursor_pos();
        let pointer = Rect { left: x, top: y - 10, right: x, bottom: y + 12 };
        let selection = platform::selection_via_accessibility();
        let (text, rect) = match selection {
            Some(s) => (Some(s.text), s.rect),
            None => (platform::selection_via_copy(), None),
        };
        log(format!("hotkey: text={:?} rect={:?}", text.as_deref().map(preview), rect));
        match text {
            Some(text) => translate_text(&app, text, Some(rect.unwrap_or(pointer))),
            None => show_error(&app, "KhĂ´ng tĂ¬m tháº¥y vÄƒn báº£n Ä‘ang bĂ´i Ä‘en.", Some(pointer)),
        }
    });
}

pub fn preview(text: &str) -> String {
    text.chars().take(60).collect()
}

// ---------------------------------------------------------------- commands

#[tauri::command]
pub fn popup_state(app: AppHandle) -> Value {
    state(&app).popup.lock().payload.clone()
}

/// The icon was clicked.
#[tauri::command]
pub fn popup_activate(app: AppHandle) {
    let pending = state(&app).popup.lock().pending.take();
    match pending {
        Some(Pending::Text(text)) => translate_text(&app, text, None),
        Some(Pending::NeedCopy) => {
            std::thread::spawn(move || match platform::selection_via_copy() {
                Some(text) => translate_text(&app, text, None),
                None => show_error(&app, "KhĂ´ng láº¥y Ä‘Æ°á»£c vÄƒn báº£n á»Ÿ á»©ng dá»¥ng nĂ y.", None),
            });
        }
        None => hide(&app),
    }
}

/// The card reports its rendered height so the window can fit it.
#[tauri::command]
pub fn popup_resize(app: AppHandle, height: f64) {
    let anchor = {
        let popup = state(&app).popup.lock();
        if popup.mode != Mode::Card {
            return;
        }
        popup.anchor
    };
    place(&app, anchor, CARD_WIDTH, height.clamp(60.0, MAX_CARD_HEIGHT));
}

#[tauri::command]
pub fn popup_close(app: AppHandle) {
    hide(&app);
}

#[tauri::command]
pub fn popup_pin(app: AppHandle, pinned: bool) {
    state(&app).popup.lock().pinned = pinned;
}

/// Translates a word or phrase picked inside the card, in the same card.
#[tauri::command]
pub fn popup_lookup(app: AppHandle, text: String) {
    let text = text.trim().to_string();
    if !text.is_empty() {
        translate_text(&app, text, None);
    }
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect { left: 0, top: 0, right: 1920, bottom: 1040 };

    #[test]
    fn goes_below_the_selection() {
        let anchor = Rect { left: 300, top: 200, right: 600, bottom: 220 };
        assert_eq!(placement(anchor, AREA, 400, 150), (300, 228));
    }

    #[test]
    fn flips_above_near_the_bottom() {
        let anchor = Rect { left: 300, top: 950, right: 600, bottom: 970 };
        assert_eq!(placement(anchor, AREA, 400, 150), (300, 950 - 8 - 150));
    }

    #[test]
    fn stays_inside_the_right_edge() {
        let anchor = Rect { left: 1800, top: 200, right: 1900, bottom: 220 };
        assert_eq!(placement(anchor, AREA, 400, 150).0, 1920 - 400 - 8);
    }

    #[test]
    fn works_on_a_monitor_left_of_the_primary() {
        let area = Rect { left: -1920, top: 0, right: 0, bottom: 1040 };
        let anchor = Rect { left: -1900, top: 100, right: -1800, bottom: 120 };
        assert_eq!(placement(anchor, area, 400, 150), (-1900, 128));
    }

    #[test]
    fn tall_anchor_with_no_room_is_pinned_to_the_bottom() {
        let anchor = Rect { left: 100, top: 20, right: 900, bottom: 1030 };
        assert_eq!(placement(anchor, AREA, 400, 150), (100, 1040 - 8 - 150));
    }
}
