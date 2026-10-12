#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_void;
use std::sync::mpsc::Sender;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use uiautomation::patterns::UITextPattern;
use uiautomation::{UIAutomation, UIElement};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Com::SAFEARRAY;
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows::Win32::System::Ole::{
    SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetLBound, SafeArrayGetUBound,
    SafeArrayUnaccessData,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::IUIAutomationTextRange;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetDoubleClickTime, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetCursorInfo, GetCursorPos, GetForegroundWindow,
    GetMessageW, GetWindowLongPtrW, GetWindowThreadProcessId, LoadCursorW, SetWindowLongPtrW,
    SetWindowPos, SetWindowsHookExW, ShowWindow, CURSORINFO, GWL_EXSTYLE, HWND_TOPMOST, IDC_IBEAM,
    MSG, MSLLHOOKSTRUCT, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};

use super::{MouseEvent, Rect, Selection};

const MAX_SELECTION_CHARS: i32 = 5000;

// ---------------------------------------------------------------- selection

/// Reads the selection through UI Automation. Gives the text and, when the
/// app reports it, where the text is on screen. Never touches the clipboard.
pub fn selection_via_accessibility() -> Option<Selection> {
    let automation = UIAutomation::new().ok()?;
    let focused = automation.get_focused_element().ok()?;
    if focused.is_password().unwrap_or(false) {
        return None;
    }
    let walker = automation.get_control_view_walker().ok()?;

    let mut element: UIElement = focused;
    for _ in 0..8 {
        if let Ok(pattern) = element.get_pattern::<UITextPattern>() {
            if let Some(selection) = read_text_pattern(&pattern) {
                return Some(selection);
            }
        }
        element = walker.get_parent(&element).ok()?;
    }
    None
}

fn read_text_pattern(pattern: &UITextPattern) -> Option<Selection> {
    let ranges = pattern.get_selection().ok()?;
    let mut text = String::new();
    let mut rect: Option<Rect> = None;
    for range in &ranges {
        let part = range.get_text(MAX_SELECTION_CHARS).unwrap_or_default();
        if part.trim().is_empty() {
            continue;
        }
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&part);
        let raw: &IUIAutomationTextRange = range.as_ref();
        if let Some(r) = unsafe { bounding_rect(raw) } {
            rect = Some(rect.map_or(r, |acc| acc.union(&r)));
        }
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(Selection { text, rect })
    }
}

/// Union of the per-line rectangles of a text range.
unsafe fn bounding_rect(range: &IUIAutomationTextRange) -> Option<Rect> {
    let array: *mut SAFEARRAY = range.GetBoundingRectangles().ok()?;
    if array.is_null() {
        return None;
    }
    let mut result: Option<Rect> = None;
    if let (Ok(lower), Ok(upper)) = (SafeArrayGetLBound(array, 1), SafeArrayGetUBound(array, 1)) {
        let count = (upper - lower + 1).max(0) as usize;
        let mut data: *mut c_void = std::ptr::null_mut();
        if count >= 4 && SafeArrayAccessData(array, &mut data).is_ok() {
            let values = std::slice::from_raw_parts(data as *const f64, count);
            for quad in values.chunks_exact(4) {
                let (left, top, width, height) = (quad[0], quad[1], quad[2], quad[3]);
                if width <= 0.0 || height <= 0.0 {
                    continue;
                }
                let r = Rect {
                    left: left as i32,
                    top: top as i32,
                    right: (left + width) as i32,
                    bottom: (top + height) as i32,
                };
                result = Some(result.map_or(r, |acc| acc.union(&r)));
            }
            let _ = SafeArrayUnaccessData(array);
        }
    }
    let _ = SafeArrayDestroy(array);
    result
}

/// Fallback: press Ctrl+C in the foreground app, read the clipboard, then put
/// the previous clipboard text or image back.
pub fn selection_via_copy() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let saved_text = clipboard.get_text().ok();
    let saved_image = if saved_text.is_none() { clipboard.get_image().ok() } else { None };
    let before = unsafe { GetClipboardSequenceNumber() };

    release_held_modifiers();
    send_copy();

    let deadline = Instant::now() + Duration::from_millis(500);
    let mut changed = false;
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(15));
        if unsafe { GetClipboardSequenceNumber() } != before {
            changed = true;
            break;
        }
    }
    if !changed {
        return None;
    }
    // Some apps fill the clipboard in several steps.
    std::thread::sleep(Duration::from_millis(30));
    let text = clipboard.get_text().ok();

    if let Some(previous) = saved_text {
        let _ = clipboard.set_text(previous);
    } else if let Some(image) = saved_image {
        let _ = clipboard.set_image(image);
    }

    text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

fn key_down(key: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(key.0 as i32) as u16 & 0x8000) != 0 }
}

fn key_input(key: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// The hotkey's own modifiers may still be down; Ctrl+C would then arrive as
/// Ctrl+Alt+C. Wait briefly for the user to let go, then force them up.
fn release_held_modifiers() {
    let others = [VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN];
    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline && others.iter().any(|k| key_down(*k)) {
        std::thread::sleep(Duration::from_millis(10));
    }
    let stuck: Vec<INPUT> =
        others.iter().filter(|k| key_down(**k)).map(|k| key_input(*k, true)).collect();
    if !stuck.is_empty() {
        send(&stuck);
    }
}

fn send_copy() {
    let ctrl_held = key_down(VK_CONTROL);
    let mut inputs = Vec::with_capacity(4);
    if !ctrl_held {
        inputs.push(key_input(VK_CONTROL, false));
    }
    inputs.push(key_input(VK_C, false));
    inputs.push(key_input(VK_C, true));
    if !ctrl_held {
        inputs.push(key_input(VK_CONTROL, true));
    }
    send(&inputs);
}

// ------------------------------------------------------------------ cursor

pub fn cursor_pos() -> (i32, i32) {
    let mut point = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut point);
    }
    (point.x, point.y)
}

/// True when the pointer is the text I-beam, i.e. it is over selectable text.
pub fn cursor_is_text() -> bool {
    unsafe {
        let mut info = CURSORINFO { cbSize: std::mem::size_of::<CURSORINFO>() as u32, ..Default::default() };
        if GetCursorInfo(&mut info).is_err() {
            return false;
        }
        match LoadCursorW(None, IDC_IBEAM) {
            Ok(ibeam) => info.hCursor == ibeam,
            Err(_) => false,
        }
    }
}

pub fn double_click_ms() -> u64 {
    unsafe { GetDoubleClickTime() as u64 }
}

/// File name of the foreground app's executable, lower-cased ("chrome.exe").
pub fn foreground_app() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 512];
        let mut len = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        ok.ok()?;
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        path.rsplit(['\\', '/']).next().map(|name| name.to_lowercase())
    }
}

// -------------------------------------------------------------- mouse hook

static MOUSE_TX: OnceLock<Sender<MouseEvent>> = OnceLock::new();

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        if let Some(tx) = MOUSE_TX.get() {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let (x, y) = (info.pt.x, info.pt.y);
            let event = match wparam.0 as u32 {
                WM_LBUTTONDOWN => Some(MouseEvent::LeftDown { x, y }),
                WM_LBUTTONUP => Some(MouseEvent::LeftUp { x, y }),
                WM_RBUTTONDOWN | WM_MBUTTONDOWN => Some(MouseEvent::OtherDown { x, y }),
                WM_MOUSEWHEEL => Some(MouseEvent::Wheel { x, y }),
                _ => None,
            };
            if let Some(event) = event {
                let _ = tx.send(event);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Installs a low-level mouse hook on its own thread and forwards events.
pub fn start_mouse_hook(tx: Sender<MouseEvent>) {
    if MOUSE_TX.set(tx).is_err() {
        return;
    }
    std::thread::spawn(|| unsafe {
        if SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0).is_err() {
            crate::log("mouse hook: SetWindowsHookExW failed");
            return;
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
    });
}

// ----------------------------------------------------------------- overlay

fn hwnd_of(window: &tauri::WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|h| HWND(h.0 as *mut c_void))
}

/// Makes the popup a tool window that never takes keyboard focus, so the
/// selection in the source app stays intact.
pub fn prepare_overlay(window: &tauri::WebviewWindow) {
    if let Some(hwnd) = hwnd_of(window) {
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                style | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize,
            );
        }
    }
}

pub fn show_overlay(window: &tauri::WebviewWindow) {
    if let Some(hwnd) = hwnd_of(window) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

/// Shows a window without taking the keyboard focus.
pub fn show_passive(window: &tauri::WebviewWindow) {
    show_overlay(window);
}

// --------------------------------------------------------------------- OCR

/// Copies a screen rectangle as top-down BGRA pixels.
fn grab(rect: Rect) -> anyhow::Result<(Vec<u8>, i32, i32)> {
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
        ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, SRCCOPY,
    };
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    anyhow::ensure!(width > 0 && height > 0, "vùng chọn rỗng");
    unsafe {
        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        let previous = SelectObject(memory, bitmap.into());
        let copied = BitBlt(memory, 0, 0, width, height, Some(screen), rect.left, rect.top, SRCCOPY);

        let mut info = BITMAPINFO::default();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        };
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let rows = GetDIBits(memory, bitmap, 0, height as u32, Some(pixels.as_mut_ptr() as *mut c_void), &mut info, DIB_RGB_COLORS);

        SelectObject(memory, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        anyhow::ensure!(copied.is_ok() && rows > 0, "không chụp được màn hình");
        Ok((pixels, width, height))
    }
}

/// Reads the text in a screen rectangle, line by line, with each line's box
/// relative to the rectangle. `vietnamese` is the models folder when the
/// Vietnamese recogniser is installed: Windows has no Vietnamese OCR, so its
/// engine only locates the words and that model reads them.
pub fn ocr_lines(rect: Rect, vietnamese: Option<&std::path::Path>) -> anyhow::Result<Vec<(Rect, String)>> {
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;

    let (mut pixels, width, height) = grab(rect)?;
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    let writer = DataWriter::new()?;
    writer.WriteBytes(&pixels)?;
    let buffer = writer.DetachBuffer()?;
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        width,
        height,
        BitmapAlphaMode::Premultiplied,
    )?;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|_| anyhow::anyhow!("máy chưa cài gói ngôn ngữ OCR nào của Windows"))?;
    let tag = engine.RecognizerLanguage().and_then(|l| l.LanguageTag()).map(|t| t.to_string()).unwrap_or_default();
    let latin = !["ja", "zh", "ko"].iter().any(|prefix| tag.starts_with(prefix));
    let result = engine.RecognizeAsync(&bitmap)?.join()?;
    let mut lines = Vec::new();
    for line in result.Lines()? {
        let mut text = line.Text()?.to_string();
        let mut words = Vec::new();
        for word in line.Words()? {
            let bounds = word.BoundingRect()?;
            words.push(Rect {
                left: bounds.X.floor() as i32,
                top: bounds.Y.floor() as i32,
                right: (bounds.X + bounds.Width).ceil() as i32,
                bottom: (bounds.Y + bounds.Height).ceil() as i32,
            });
        }
        let Some(bounds) = words.iter().copied().reduce(|a, b| a.union(&b)) else { continue };
        if let Some(root) = vietnamese.filter(|_| latin) {
            match crate::engine::ocr_vi::read_line(root, &pixels, width, height, &words) {
                Ok(read) if !read.is_empty() => text = read,
                Ok(_) => {}
                Err(error) => crate::log(format!("ocr-vi: {error:#}")),
            }
        }
        lines.push((bounds, text));
    }
    Ok(lines)
}

/// A cheap summary of what a screen rectangle shows, to tell whether it changed.
pub fn fingerprint(rect: Rect) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    let (pixels, _, _) = grab(rect).ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pixels.hash(&mut hasher);
    Some(hasher.finish())
}

/// Leaves a window out of screen captures (or puts it back), so the screen
/// under it can still be read.
pub fn exclude_from_capture(window: &tauri::WebviewWindow, exclude: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE};
    if let Some(hwnd) = hwnd_of(window) {
        unsafe {
            let _ = SetWindowDisplayAffinity(hwnd, if exclude { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE });
        }
    }
}

/// Bounds of the window the user is working in.
pub fn foreground_rect() -> Option<Rect> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};
    unsafe {
        let window = GetForegroundWindow();
        if window.0.is_null() {
            return None;
        }
        let mut rect = RECT::default();
        GetWindowRect(window, &mut rect).ok()?;
        Some(Rect { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom })
    }
}

// ------------------------------------------------------------------- hover

/// The word under a screen point, for apps that expose text through UI
/// Automation (Notepad, Word, browsers with accessibility on).
pub fn word_at(x: i32, y: i32) -> Option<Selection> {
    use windows::Win32::UI::Accessibility::{IUIAutomationTextPattern, TextUnit_Word};

    let automation = UIAutomation::new().ok()?;
    let walker = automation.get_control_view_walker().ok()?;
    let mut element = automation.element_from_point(uiautomation::types::Point::new(x, y)).ok()?;
    if element.is_password().unwrap_or(false) {
        return None;
    }
    for _ in 0..4 {
        if let Ok(pattern) = element.get_pattern::<UITextPattern>() {
            let raw: &IUIAutomationTextPattern = pattern.as_ref();
            let found = unsafe {
                let range = raw.RangeFromPoint(POINT { x, y }).ok()?;
                range.ExpandToEnclosingUnit(TextUnit_Word).ok()?;
                let text = range.GetText(80).ok()?.to_string();
                (text, bounding_rect(&range))
            };
            let text = found.0.trim().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
            // The range snaps to the nearest word even over blank space.
            let over_word = found.1.is_some_and(|r| r.contains(x, y));
            return (over_word && !text.is_empty()).then_some(Selection { text, rect: found.1 });
        }
        element = walker.get_parent(&element).ok()?;
    }
    None
}

/// Ctrl is the "translate what I point at" modifier.
pub fn hover_modifier_down() -> bool {
    key_down(VK_CONTROL)
}