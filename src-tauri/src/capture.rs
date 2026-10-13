//! Screen-region translation: a full-screen transparent window lets the user
//! drag a rectangle, which is then captured, read with OCR and translated.
//! The same window shows whole-window translations, once or continuously.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::platform::{self, Rect};
use crate::settings::Settings;
use crate::{log, popup};

pub const LABEL: &str = "capture";

/// Top-left corner (physical pixels) of the monitor the overlay covers.
static ORIGIN: Mutex<(i32, i32)> = Mutex::new((0, 0));

/// The monitor the overlay covers.
static SCREEN: Mutex<Rect> = Mutex::new(Rect { left: 0, top: 0, right: 0, bottom: 0 });

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
    crate::remember_app();
    let Some(window) = app.get_webview_window(LABEL) else { return };
    popup::hide(app);
    end_live(app);
    SESSION.fetch_add(1, Ordering::SeqCst);
    let Some(screen) = monitor_rect(app) else { return };
    cover(&window, screen);
    let _ = app.emit_to(LABEL, "capture-start", ());
    let _ = window.show();
    let _ = window.set_focus();
}

#[tauri::command]
pub fn capture_cancel(app: AppHandle) {
    end_live(&app);
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

#[derive(Serialize, Clone, PartialEq)]
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
    // Twice the centre of the last line added, to compare without halving.
    let mut last_centre = 0;
    for (rect, text) in lines {
        let height = rect.bottom - rect.top;
        if let Some(last) = blocks.last_mut() {
            let gap = rect.top - last.rect.bottom;
            let similar = (height - last.line_height).abs() * 3 <= last.line_height;
            // Lines of one paragraph start at the same place, or share a centre (subtitles, headings).
            let centre = |r: &Rect| r.left + r.right;
            let aligned = (rect.left - last.rect.left).abs() * 2 < height * 3 || (centre(&rect) - last_centre).abs() < height * 3;
            if similar && gap * 10 < height * 7 && gap * 2 > -height && aligned {
                last_centre = centre(&rect);
                last.rect = last.rect.union(&rect);
                last.text.push(' ');
                last.text.push_str(&text);
                continue;
            }
        }
        last_centre = rect.left + rect.right;
        blocks.push(Block { rect, text, line_height: height });
    }
    blocks
}

/// The monitor under the pointer.
fn monitor_rect(app: &AppHandle) -> Option<Rect> {
    let (x, y) = platform::cursor_pos();
    let monitor = app
        .monitor_from_point(x as f64, y as f64)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let (position, size) = (*monitor.position(), *monitor.size());
    Some(Rect {
        left: position.x,
        top: position.y,
        right: position.x + size.width as i32,
        bottom: position.y + size.height as i32,
    })
}

/// The active window, clipped to `screen`; the whole screen otherwise.
fn target_in(screen: Rect) -> Rect {
    platform::foreground_rect()
        .map(|w| Rect {
            left: w.left.max(screen.left),
            top: w.top.max(screen.top),
            right: w.right.min(screen.right),
            bottom: w.bottom.min(screen.bottom),
        })
        .filter(|w| w.right - w.left > 80 && w.bottom - w.top > 40)
        .unwrap_or(screen)
}

fn cover(window: &WebviewWindow, screen: Rect) {
    *ORIGIN.lock() = (screen.left, screen.top);
    *SCREEN.lock() = screen;
    let _ = window.set_position(PhysicalPosition::new(screen.left, screen.top));
    let size = PhysicalSize::new((screen.right - screen.left) as u32, (screen.bottom - screen.top) as u32);
    let _ = window.set_size(size);
}

/// The paragraphs of `target` worth translating: those with words that are
/// not already in the reader's language.
fn read_blocks(target: Rect, settings: &Settings, models: &Path) -> anyhow::Result<Vec<Block>> {
    // The Vietnamese recogniser is slow and only matters when the source can be Vietnamese.
    let vietnamese = (settings.primary_lang != "vi" && crate::engine::ocr_vi::installed(models)).then_some(models);
    let lines = platform::ocr_lines(target, vietnamese)?;
    Ok(paragraphs(lines)
        .into_iter()
        .filter(|block| block.text.chars().filter(|c| c.is_alphabetic()).count() >= 2)
        .filter(|block| crate::engine::lang::guess(&block.text).as_deref() != Some(settings.primary_lang.as_str()))
        .collect())
}

/// Translates `texts`, handing each result to `done` with its index.
/// Stops quietly when the overlay session ends.
async fn translate_each(settings: &Settings, models: &Path, texts: Vec<String>, session: u64, mut done: impl FnMut(usize, String)) {
    // One request for everything keeps an online service from rate-limiting
    // us; it only counts if every text comes back.
    if settings.engine == "google" && texts.len() > 1 {
        let joined = texts.join("\n");
        if let Ok(all) = crate::engine::google::translate(&joined, "auto", &settings.primary_lang).await {
            let parts: Vec<&str> = all.text.split('\n').collect();
            if parts.len() == texts.len() {
                if SESSION.load(Ordering::SeqCst) == session {
                    for (index, part) in parts.into_iter().enumerate() {
                        done(index, part.to_string());
                    }
                }
                return;
            }
        }
    }
    let mut pending = futures_util::stream::iter(texts.into_iter().enumerate().map(|(index, text)| async move {
        (index, crate::engine::translate_text(settings, models, &text).await)
    }))
    .buffer_unordered(PARALLEL);
    while let Some((index, translated)) = pending.next().await {
        if SESSION.load(Ordering::SeqCst) != session {
            return;
        }
        if let Ok(translated) = translated {
            done(index, translated.text);
        }
    }
}

/// What the overlay draws for `block`, or nothing when the translation adds nothing.
fn translated(block: &Block, text: &str, dx: i32, dy: i32) -> Option<Translated> {
    let text = text.trim();
    if text.is_empty() || text.eq_ignore_ascii_case(block.text.trim()) {
        return None;
    }
    Some(Translated {
        x: block.rect.left + dx,
        y: block.rect.top + dy,
        width: block.rect.right - block.rect.left,
        height: block.rect.bottom - block.rect.top,
        line_height: block.line_height,
        text: text.to_string(),
    })
}

/// Translates every text block of the active window and draws each
/// translation over its source text. A click or Esc closes the overlay.
pub fn start_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    popup::hide(app);
    end_live(app);
    let Some(screen) = monitor_rect(app) else { return };
    let target = target_in(screen);

    let session = SESSION.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        let settings = crate::effective(&app);
        let models = crate::engine::offline::models_dir(&app);
        let blocks = match read_blocks(target, &settings, &models) {
            Ok(blocks) => blocks,
            Err(error) => {
                log(format!("window ocr failed: {error:#}"));
                popup::show_error(&app, &format!("OCR lỗi: {error}"), None);
                return;
            }
        };
        log(format!("window: {} blocks in {target:?}, engine {} into {}", blocks.len(), settings.engine, settings.primary_lang));

        cover(&window, screen);
        let _ = app.emit_to(LABEL, "window-start", blocks.len());
        let _ = window.show();
        let _ = window.set_focus();

        let (dx, dy) = (target.left - screen.left, target.top - screen.top);
        let texts = blocks.iter().map(|block| block.text.clone()).collect();
        tauri::async_runtime::block_on(translate_each(&settings, &models, texts, session, |index, text| {
            if let Some(payload) = translated(&blocks[index], &text, dx, dy) {
                let _ = app.emit_to(LABEL, "window-block", payload);
            }
        }));
        if SESSION.load(Ordering::SeqCst) == session {
            let _ = app.emit_to(LABEL, "window-done", ());
        }
    });
}

// -------------------------------------------------------------------- live

/// Whether the overlay is translating the screen continuously.
static LIVE: AtomicBool = AtomicBool::new(false);
/// Whether the overlay is asking what live mode should follow.
static CHOOSING: AtomicBool = AtomicBool::new(false);
/// How often the screen is compared with the last reading.
const LIVE_TICK: Duration = Duration::from_millis(120);
/// A picture that keeps changing (video, scrolling) is read anyway after this long.
const LIVE_RESTLESS: Duration = Duration::from_millis(400);
/// Translations kept while live mode runs, so text that returns is not sent again.
const LIVE_CACHE: usize = 2000;

/// Gives the overlay back its normal behaviour after live mode.
fn end_live(app: &AppHandle) {
    CHOOSING.store(false, Ordering::SeqCst);
    if LIVE.swap(false, Ordering::SeqCst) {
        SESSION.fetch_add(1, Ordering::SeqCst);
        if let Some(window) = app.get_webview_window(LABEL) {
            platform::hide_overlay(&window);
            let _ = window.set_ignore_cursor_events(false);
            platform::exclude_from_capture(&window, false);
        }
        log("live: off");
    }
}

/// The live shortcut: switches live translation off when it runs, otherwise
/// asks whether to follow the whole screen or a region the user drags.
pub fn toggle_live(app: &AppHandle) {
    if LIVE.load(Ordering::SeqCst) {
        end_live(app);
        return;
    }
    if CHOOSING.load(Ordering::SeqCst) {
        capture_cancel(app.clone());
        return;
    }
    crate::remember_app();
    let Some(window) = app.get_webview_window(LABEL) else { return };
    popup::hide(app);
    SESSION.fetch_add(1, Ordering::SeqCst);
    let Some(screen) = monitor_rect(app) else { return };
    cover(&window, screen);
    CHOOSING.store(true, Ordering::SeqCst);
    // The connection is opened while the user chooses.
    crate::engine::warm(&app.state::<crate::AppState>().settings.read());
    let _ = app.emit_to(LABEL, "live-choose", ());
    let _ = window.show();
    let _ = window.set_focus();
}

#[derive(Serialize, Clone)]
struct LiveStart {
    hotkey: String,
    /// The followed region as x, y, width, height relative to the overlay; none for the whole screen.
    region: Option<[i32; 4]>,
}

/// The user chose what to follow: the whole screen, or `region` (x, y, width,
/// height in physical pixels relative to the overlay).
#[tauri::command]
pub fn live_begin(app: AppHandle, region: Option<[i32; 4]>) {
    if !CHOOSING.swap(false, Ordering::SeqCst) {
        return;
    }
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let screen = *SCREEN.lock();
    let target = match region {
        Some([x, y, width, height]) if width >= 24 && height >= 12 => {
            Rect { left: screen.left + x, top: screen.top + y, right: screen.left + x + width, bottom: screen.top + y + height }
        }
        Some(_) => {
            let _ = window.hide();
            return;
        }
        None => screen,
    };
    let session = SESSION.fetch_add(1, Ordering::SeqCst) + 1;
    LIVE.store(true, Ordering::SeqCst);

    // From here on the overlay only shows: the mouse and the keyboard go to
    // the windows underneath, and our own text stays out of the pictures we read.
    platform::hide_overlay(&window);
    let _ = window.set_ignore_cursor_events(true);
    platform::exclude_from_capture(&window, true);
    let hotkey = app.state::<crate::AppState>().settings.read().live_hotkey.clone();
    let _ = app.emit_to(LABEL, "live-start", LiveStart { hotkey, region: region.filter(|_| target != screen) });
    platform::show_passive(&window);
    log(format!("live: on, following {target:?}"));

    std::thread::spawn(move || follow(app, session, screen, target));
}

/// Reads `target` whenever it changes and draws the translations over it.
fn follow(app: AppHandle, session: u64, screen: Rect, target: Rect) {
    let models = crate::engine::offline::models_dir(&app);
    let mut cache: HashMap<String, String> = HashMap::new();
    // Texts of the previous reading. While the picture keeps moving, only text
    // seen twice in a row is translated, so scrolling does not cause requests.
    let mut previous: HashSet<String> = HashSet::new();
    // The picture of the last reading, the last picture seen, and since when it moves.
    let mut read = None;
    let mut seen = None;
    let mut moving: Option<Instant> = None;
    let mut settled = false;
    let mut shown: Option<Vec<Translated>> = None;
    let (dx, dy) = (target.left - screen.left, target.top - screen.top);
    let alive = || SESSION.load(Ordering::SeqCst) == session;
    // A region is small and usually holds subtitles over a moving picture:
    // it is read sooner and its text is translated at first sight.
    let eager = target != screen;
    let restless = if eager { LIVE_TICK } else { LIVE_RESTLESS };

    while alive() {
        let now = platform::fingerprint(target);
        let mut stable = true;
        match now {
            Some(_) if settled && now == read => {
                std::thread::sleep(LIVE_TICK);
                continue;
            }
            Some(_) if now != seen => {
                // Still changing: give it a moment to come to rest.
                seen = now;
                if moving.get_or_insert_with(Instant::now).elapsed() < restless {
                    std::thread::sleep(LIVE_TICK);
                    continue;
                }
                stable = false;
            }
            Some(_) => {}
            // No way to tell whether the screen changed: read it at a slower pace.
            None => std::thread::sleep(LIVE_TICK * 4),
        }
        moving = None;
        read = now;
        let started = Instant::now();
        let settings = crate::effective(&app);
        let blocks = match read_blocks(target, &settings, &models) {
            Ok(blocks) => blocks,
            Err(error) => {
                log(format!("live ocr failed: {error:#}"));
                popup::show_error(&app, &format!("OCR lỗi: {error}"), None);
                end_live(&app);
                return;
            }
        };
        let read_ms = started.elapsed().as_millis();
        let mut show = |cache: &HashMap<String, String>| {
            let frame: Vec<Translated> =
                blocks.iter().filter_map(|block| translated(block, cache.get(&block.text)?, dx, dy)).collect();
            if alive() && shown.as_ref() != Some(&frame) {
                log(format!("live: showing {} of {} blocks", frame.len(), blocks.len()));
                let _ = app.emit_to(LABEL, "live-frame", frame.clone());
                shown = Some(frame);
            }
        };

        let mut fresh: Vec<String> = Vec::new();
        for block in &blocks {
            let wanted = stable || eager || previous.contains(&block.text);
            if wanted && !cache.contains_key(&block.text) && !fresh.contains(&block.text) {
                fresh.push(block.text.clone());
            }
        }
        if !fresh.is_empty() {
            // What is already known moves to its new place without waiting for the rest.
            show(&cache);
            if cache.len() > LIVE_CACHE {
                cache.clear();
            }
            let count = fresh.len();
            let mut results: Vec<Option<String>> = vec![None; count];
            tauri::async_runtime::block_on(translate_each(&settings, &models, fresh.clone(), session, |index, text| {
                results[index] = Some(text);
            }));
            // A failed text is remembered as empty, so it is not sent again on every reading.
            for (text, result) in fresh.into_iter().zip(results) {
                cache.insert(text, result.unwrap_or_default());
            }
            log(format!(
                "live: {count} new blocks by {} in {} ms (reading {read_ms} ms)",
                settings.engine,
                started.elapsed().as_millis()
            ));
        }
        settled = blocks.iter().all(|block| cache.contains_key(&block.text));
        previous = blocks.iter().map(|block| block.text.clone()).collect();
        show(&cache);
        std::thread::sleep(LIVE_TICK);
    }
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
    fn centred_lines_become_one_paragraph() {
        let wide = (Rect { left: 300, top: 900, right: 900, bottom: 930 }, "I never said that".to_string());
        let narrow = (Rect { left: 450, top: 936, right: 750, bottom: 966 }, "to anyone.".to_string());
        let blocks = paragraphs(vec![wide, narrow]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "I never said that to anyone.");
    }

    #[test]
    fn distant_or_shifted_lines_stay_separate() {
        let blocks = paragraphs(vec![line(10, 10, "File"), line(10, 120, "Body text"), line(400, 144, "Side note")]);
        assert_eq!(blocks.len(), 3);
    }
}
