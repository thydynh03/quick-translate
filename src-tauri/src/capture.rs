//! Screen-region translation: a full-screen transparent window lets the user
//! drag a rectangle, which is then captured, read with OCR and translated.

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::platform::{self, Rect};
use crate::{log, popup};

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
        match platform::ocr_region(rect) {
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
