//! Turns raw mouse events into "the user just selected text" and decides
//! when the popup should appear or go away.

use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::platform::{self, MouseEvent};
use crate::popup::{self, Mode, Pending};
use crate::{log, AppState};

const DRAG_THRESHOLD: i32 = 8;
/// Time the source app gets to update its selection after the mouse is released.
const SETTLE: Duration = Duration::from_millis(140);

/// Tracks presses to recognise drags and double/triple clicks.
#[derive(Default)]
pub struct Gesture {
    down: Option<(i32, i32)>,
    last_down: Option<(i32, i32, Instant)>,
    multi_click: bool,
}

impl Gesture {
    pub fn press(&mut self, x: i32, y: i32, now: Instant, double_click: Duration) {
        self.multi_click = matches!(self.last_down, Some((lx, ly, at))
            if now.duration_since(at) <= double_click && (x - lx).abs() <= 6 && (y - ly).abs() <= 6);
        self.down = Some((x, y));
        self.last_down = Some((x, y, now));
    }

    /// True when the release ends a gesture that normally selects text.
    pub fn release(&mut self, x: i32, y: i32) -> bool {
        let Some((dx, dy)) = self.down.take() else { return false };
        let dragged = (x - dx).abs() > DRAG_THRESHOLD || (y - dy).abs() > DRAG_THRESHOLD;
        dragged || self.multi_click
    }

    pub fn cancel(&mut self) {
        self.down = None;
    }
}

pub fn run(app: AppHandle, events: Receiver<MouseEvent>) {
    let double_click = Duration::from_millis(platform::double_click_ms());
    let mut gesture = Gesture::default();

    for event in events {
        match event {
            MouseEvent::LeftDown { x, y } => {
                if inside_popup(&app, x, y) {
                    gesture.cancel();
                    continue;
                }
                popup::dismiss(&app);
                app.state::<AppState>().gesture.fetch_add(1, Ordering::SeqCst);
                gesture.press(x, y, Instant::now(), double_click);
            }
            MouseEvent::LeftUp { x, y } => {
                if gesture.release(x, y) {
                    on_selection_gesture(&app, x, y);
                }
            }
            MouseEvent::OtherDown { x, y } => {
                if !inside_popup(&app, x, y) {
                    popup::dismiss(&app);
                }
            }
            MouseEvent::Wheel { x, y } => {
                // Scrolling moves the text away from the popup.
                if !inside_popup(&app, x, y) && popup::mode(&app) != Mode::Hidden {
                    popup::dismiss(&app);
                }
            }
        }
    }
}

/// Hover translation: while Ctrl is held and the pointer rests on a word,
/// translate that word. Off unless enabled in the settings.
pub fn run_hover(app: AppHandle) {
    const TICK: Duration = Duration::from_millis(120);
    const REST: Duration = Duration::from_millis(360);
    let mut last = (i32::MIN, i32::MIN);
    let mut since = Instant::now();
    let mut handled = false;
    loop {
        std::thread::sleep(TICK);
        let enabled = app.state::<AppState>().settings.read().hover_translate;
        if !enabled || !platform::hover_modifier_down() {
            handled = false;
            continue;
        }
        let position = platform::cursor_pos();
        if position != last {
            last = position;
            since = Instant::now();
            handled = false;
            continue;
        }
        if handled || since.elapsed() < REST || inside_popup(&app, position.0, position.1) {
            continue;
        }
        handled = true;
        if let Some(word) = platform::word_at(position.0, position.1) {
            log(format!("hover: {:?} rect={:?}", word.text, word.rect));
            popup::translate_text(&app, word.text, word.rect);
        }
    }
}

fn inside_popup(app: &AppHandle, x: i32, y: i32) -> bool {
    popup::bounds(app).is_some_and(|rect| rect.contains(x, y))
}

fn on_selection_gesture(app: &AppHandle, x: i32, y: i32) {
    let state = app.state::<AppState>();
    {
        let settings = state.settings.read();
        if !settings.auto_trigger {
            return;
        }
        if let Some(name) = platform::foreground_app() {
            if name == own_exe() || settings.excluded_apps.iter().any(|e| e.eq_ignore_ascii_case(&name)) {
                return;
            }
        }
    }
    let text_cursor = platform::cursor_is_text();
    let gesture_id = state.gesture.load(Ordering::SeqCst);
    let app = app.clone();

    std::thread::spawn(move || {
        std::thread::sleep(SETTLE);
        let state = app.state::<AppState>();
        if state.gesture.load(Ordering::SeqCst) != gesture_id {
            return;
        }
        match platform::selection_via_accessibility() {
            Some(selection) => {
                log(format!("gesture: accessibility text={:?} rect={:?}", popup::preview(&selection.text), selection.rect));
                popup::show_icon(&app, Pending::Text(selection.text), x, y, selection.rect);
            }
            // No accessibility data: only offer the icon where the pointer
            // says "text", so dragging windows or files stays quiet.
            None if text_cursor => {
                log("gesture: no accessibility text, text cursor -> copy on click");
                popup::show_icon(&app, Pending::NeedCopy, x, y, None);
            }
            None => {}
        }
    });
}

fn own_exe() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DC: Duration = Duration::from_millis(500);

    #[test]
    fn plain_click_is_not_a_selection() {
        let mut g = Gesture::default();
        g.press(100, 100, Instant::now(), DC);
        assert!(!g.release(102, 101));
    }

    #[test]
    fn drag_is_a_selection() {
        let mut g = Gesture::default();
        g.press(100, 100, Instant::now(), DC);
        assert!(g.release(180, 104));
    }

    #[test]
    fn double_click_is_a_selection() {
        let mut g = Gesture::default();
        let t = Instant::now();
        g.press(100, 100, t, DC);
        assert!(!g.release(100, 100));
        g.press(101, 100, t + Duration::from_millis(120), DC);
        assert!(g.release(101, 100));
    }

    #[test]
    fn slow_second_click_is_not_a_double_click() {
        let mut g = Gesture::default();
        let t = Instant::now();
        g.press(100, 100, t, DC);
        g.release(100, 100);
        g.press(100, 100, t + Duration::from_millis(900), DC);
        assert!(!g.release(100, 100));
    }

    #[test]
    fn release_without_press_is_ignored() {
        let mut g = Gesture::default();
        assert!(!g.release(500, 500));
    }
}
