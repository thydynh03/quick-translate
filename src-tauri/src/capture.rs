//! Screen-region translation: a full-screen transparent window lets the user
//! drag a rectangle, which is then captured, read with OCR and translated.

use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::StreamExt;
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::platform::{self, Rect};
use crate::{log, popup, AppState};

pub const LABEL: &str = "capture";

/// Top-left corner (physical pixels) of the monitor the overlay covers.
static ORIGIN: Mutex<(i32, i32)> = Mutex::new((0, 0));

pub fn create_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::App("index.html".into()))
        .title("Quick Translate Capture")
        .visible(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()
}

/// Covers the monitor under the pointer with the selection overlay.
pub fn start(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    popup::hide(app);
    let (x, y) = platform::cursor_pos();
    let monitor = app
        .monitor_from_point(x as f64, y as f64)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let (position, size) = (*monitor.position(), *monitor.size());
    *ORIGIN.lock() = (position.x, position.y);
    let _ = window.set_position(PhysicalPosition::new(position.x, position.y));
    let _ = window.set_size(PhysicalSize::new(size.width, size.height));
    let _ = app.emit_to(LABEL, "capture-start", ());
    let _ = window.show();
    let _ = window.set_focus();
}

#[tauri::command]
pub fn capture_cancel(app: AppHandle) {
    SESSION.fetch_add(1, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}

/// The user finished dragging. Coordinates are physical pixels relative to
/// the overlay window.
#[tauri::command]
pub fn capture_region(app: AppHandle, x: i32, y: i32, width: i32, height: i32) {
    capture_cancel(app.clone());
    let (ox, oy) = *ORIGIN.lock();
    let rect = Rect { left: ox + x, top: oy + y, right: ox + x + width, bottom: oy + y + height };
    if width < 8 || height < 8 {
        return;
    }
    std::thread::spawn(move || {
        // Let the overlay disappear from the screen before grabbing pixels.
        std::thread::sleep(std::time::Duration::from_millis(180));
        let models = crate::engine::offline::models_dir(&app);
        let vietnamese = crate::engine::ocr_vi::installed(&models).then_some(models.as_path());
        match platform::ocr_lines(rect, vietnamese).map(join_lines) {
            Ok(text) if !text.trim().is_empty() => {
                log(format!("ocr: {:?}", popup::preview(&text)));
                popup::translate_text(&app, text, Some(rect));
            }
            Ok(_) => popup::show_error(&app, "Không nhận ra chữ trong vùng đã chọn.", Some(rect)),
            Err(error) => {
                log(format!("ocr failed: {error:#}"));
                popup::show_error(&app, &format!("OCR lỗi: {error}"), Some(rect));
            }
        }
    });
}

/// OCR lines of one paragraph are wrapped sentences: join them for translation.
pub fn join_lines(lines: Vec<(Rect, String)>) -> String {
    lines.into_iter().map(|(_, text)| text).collect::<Vec<_>>().join(" ")
}

// ------------------------------------------------------------ whole window

/// Bumped whenever the overlay closes, so a running window translation stops.
static SESSION: AtomicU64 = AtomicU64::new(0);
const PARALLEL: usize = 4;

/// A paragraph found on screen, in pixels relative to the captured area.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub rect: Rect,
    pub text: String,
    pub line_height: i32,
}

#[derive(Serialize, Clone)]
struct Translated {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    line_height: i32,
    text: String,
}

/// Merges consecutive lines that sit right under each other into paragraphs,
/// so sentences are translated whole.
pub fn paragraphs(lines: Vec<(Rect, String)>) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    for (rect, text) in lines {
        let height = rect.bottom - rect.top;
        if let Some(last) = blocks.last_mut() {
            let gap = rect.top - last.rect.bottom;
            let similar = (height - last.line_height).abs() * 3 <= last.line_height;
            if similar && gap * 10 < height * 7 && gap * 2 > -height && (rect.left - last.rect.left).abs() * 2 < height * 3 {
                last.rect = last.rect.union(&rect);
                last.text.push(' ');
                last.text.push_str(&text);
                continue;
            }
        }
        blocks.push(Block { rect, text, line_height: height });
    }
    blocks
}

/// Translates every text block of the active window and draws each
/// translation over its source text. A click or Esc closes the overlay.
pub fn start_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    popup::hide(app);
    let (x, y) = platform::cursor_pos();
    let monitor = app
        .monitor_from_point(x as f64, y as f64)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let (position, size) = (*monitor.position(), *monitor.size());
    let screen = Rect {
        left: position.x,
        top: position.y,
        right: position.x + size.width as i32,
        bottom: position.y + size.height as i32,
    };
    // The active window, clipped to this monitor; the whole monitor otherwise.
    let target = platform::foreground_rect()
        .map(|w| Rect {
            left: w.left.max(screen.left),
            top: w.top.max(screen.top),
            right: w.right.min(screen.right),
            bottom: w.bottom.min(screen.bottom),
        })
        .filter(|w| w.right - w.left > 80 && w.bottom - w.top > 40)
        .unwrap_or(screen);

    let session = SESSION.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        let settings = app.state::<AppState>().settings.read().clone();
        let models = crate::engine::offline::models_dir(&app);
        let vietnamese = crate::engine::ocr_vi::installed(&models).then_some(models.as_path());
        let lines = match platform::ocr_lines(target, vietnamese) {
            Ok(lines) => lines,
            Err(error) => {
                log(format!("window ocr failed: {error:#}"));
                popup::show_error(&app, &format!("OCR lỗi: {error}"), None);
                return;
            }
        };
        // Skip what is already in the reader's language and what has no words.
        let blocks: Vec<Block> = paragraphs(lines)
            .into_iter()
            .filter(|block| block.text.chars().filter(|c| c.is_alphabetic()).count() >= 2)
            .filter(|block| crate::engine::lang::guess(&block.text).as_deref() != Some(settings.primary_lang.as_str()))
            .collect();
        log(format!("window: {} blocks in {target:?}", blocks.len()));

        *ORIGIN.lock() = (screen.left, screen.top);
        let _ = window.set_position(PhysicalPosition::new(screen.left, screen.top));
        let _ = window.set_size(PhysicalSize::new(size.width, size.height));
        let _ = app.emit_to(LABEL, "window-start", blocks.len());
        let _ = window.show();
        let _ = window.set_focus();

        let (dx, dy) = (target.left - screen.left, target.top - screen.top);
        tauri::async_runtime::block_on(async {
            let mut pending = futures_util::stream::iter(blocks.into_iter().map(|block| {
                let (settings, models) = (&settings, &models);
                async move {
                    let translated = crate::engine::translate_text(settings, models, &block.text).await;
                    (block, translated)
                }
            }))
            .buffer_unordered(PARALLEL);
            while let Some((block, translated)) = pending.next().await {
                if SESSION.load(Ordering::SeqCst) != session {
                    return;
                }
                let Ok(translated) = translated else { continue };
                if translated.text.trim().is_empty() || translated.text.trim().eq_ignore_ascii_case(block.text.trim()) {
                    continue;
                }
                let payload = Translated {
                    x: block.rect.left + dx,
                    y: block.rect.top + dy,
                    width: block.rect.right - block.rect.left,
                    height: block.rect.bottom - block.rect.top,
                    line_height: block.line_height,
                    text: translated.text,
                };
                let _ = app.emit_to(LABEL, "window-block", payload);
            }
        });
        if SESSION.load(Ordering::SeqCst) == session {
            let _ = app.emit_to(LABEL, "window-done", ());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(left: i32, top: i32, text: &str) -> (Rect, String) {
        (Rect { left, top, right: left + 300, bottom: top + 20 }, text.to_string())
    }

    #[test]
    fn wrapped_lines_become_one_paragraph() {
        let blocks = paragraphs(vec![line(10, 10, "The meeting has been"), line(10, 34, "moved to Thursday.")]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "The meeting has been moved to Thursday.");
        assert_eq!(blocks[0].rect, Rect { left: 10, top: 10, right: 310, bottom: 54 });
    }

    #[test]
    fn distant_or_shifted_lines_stay_separate() {
        let blocks = paragraphs(vec![line(10, 10, "File"), line(10, 120, "Body text"), line(400, 144, "Side note")]);
        assert_eq!(blocks.len(), 3);
    }
}
