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
use windows::Win32::UI::Accessibility::{
    IUIAutomationTextRange, TextPatternRangeEndpoint_End, TextPatternRangeEndpoint_Start, TextUnit_Line, TextUnit_Paragraph,
};
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
    let mut context = None;
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
        // A word or short phrase: also read the sentence it sits in.
        if ranges.len() == 1 && part.split_whitespace().count() <= 4 {
            context = unsafe { sentence_of(raw) }.filter(|sentence| sentence.trim() != part.trim());
        }
        if let Some(r) = unsafe { bounding_rect(raw) } {
            rect = Some(rect.map_or(r, |acc| acc.union(&r)));
        }
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(Selection { text, rect, context })
    }
}

/// The sentence that contains `range`: the enclosing paragraph (or line, when
/// the app has no paragraphs) cut at the sentence ends around the range start.
unsafe fn sentence_of(range: &IUIAutomationTextRange) -> Option<String> {
    let paragraph = range.Clone().ok()?;
    paragraph.ExpandToEnclosingUnit(TextUnit_Paragraph).ok()?;
    let text = paragraph.GetText(MAX_SELECTION_CHARS).ok()?.to_string();
    let before = paragraph.Clone().ok()?;
    before.MoveEndpointByRange(TextPatternRangeEndpoint_End, range, TextPatternRangeEndpoint_Start).ok()?;
    let head = before.GetText(MAX_SELECTION_CHARS).ok()?.to_string();
    // Both reads are capped; past the cap the offset no longer matches the text.
    let cap = MAX_SELECTION_CHARS as usize;
    if text.chars().count() >= cap || head.chars().count() >= cap {
        return None;
    }
    crate::sentence::around(&text, head.chars().count())
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

/// Hides a window that `show_overlay` put on screen. The windowing library
/// does not know about that, and its own `hide` does nothing for a window it
/// believes is hidden: the window would stay, see-through, swallowing clicks.
pub fn hide_overlay(window: &tauri::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    if let Some(hwnd) = hwnd_of(window) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
    let _ = window.hide();
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

/// Windows' recogniser finds nothing in a small picture, or in one where the
/// text touches the edges: a region dragged around a single word. Such a
/// picture is magnified and given a margin in its own background colour.
const OCR_COMFORT: i32 = 160;
const OCR_MARGIN: i32 = 32;
/// The recogniser refuses pictures larger than this on a side.
const OCR_LIMIT: i32 = 8000;

/// How many times a picture of this size is magnified before it is read.
fn magnification(width: i32, height: i32) -> i32 {
    let shorter = width.min(height).max(1);
    if shorter >= OCR_COMFORT {
        return 1;
    }
    let wanted = (OCR_COMFORT + shorter - 1) / shorter;
    let room = (OCR_LIMIT - 2 * OCR_MARGIN) / width.max(height).max(1);
    wanted.min(4).min(room).max(1)
}

/// BGRA pixels magnified `scale` times (smoothly) inside a margin of the
/// colour found in the corner. Returns the pixels and their width and height.
fn enlarge(pixels: &[u8], width: i32, height: i32, scale: i32) -> (Vec<u8>, i32, i32) {
    let (w, h) = (width as usize, height as usize);
    let (scale, margin) = (scale as usize, OCR_MARGIN as usize);
    let (out_w, out_h) = (w * scale + 2 * margin, h * scale + 2 * margin);
    let mut out: Vec<u8> = pixels[..4].iter().copied().cycle().take(out_w * out_h * 4).collect();
    for y in 0..h * scale {
        // The centre of each new pixel, in the old picture.
        let fy = ((y as f32 + 0.5) / scale as f32 - 0.5).clamp(0.0, (h - 1) as f32);
        let (y0, ty) = (fy as usize, fy.fract());
        let y1 = (y0 + 1).min(h - 1);
        for x in 0..w * scale {
            let fx = ((x as f32 + 0.5) / scale as f32 - 0.5).clamp(0.0, (w - 1) as f32);
            let (x0, tx) = (fx as usize, fx.fract());
            let x1 = (x0 + 1).min(w - 1);
            let at = ((y + margin) * out_w + x + margin) * 4;
            for channel in 0..4 {
                let sample = |px: usize, py: usize| pixels[(py * w + px) * 4 + channel] as f32;
                let top = sample(x0, y0) * (1.0 - tx) + sample(x1, y0) * tx;
                let bottom = sample(x0, y1) * (1.0 - tx) + sample(x1, y1) * tx;
                out[at + channel] = (top * (1.0 - ty) + bottom * ty).round() as u8;
            }
        }
    }
    (out, out_w as i32, out_h as i32)
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
    // What the recogniser sees may be a magnified copy; its answers are brought back to `pixels`.
    let scale = magnification(width, height);
    let (seen, seen_width, seen_height, margin) = if scale > 1 {
        let (seen, seen_width, seen_height) = enlarge(&pixels, width, height, scale);
        (seen, seen_width, seen_height, OCR_MARGIN)
    } else {
        (pixels.clone(), width, height, 0)
    };
    let back = |value: f32| (value - margin as f32) / scale as f32;
    let writer = DataWriter::new()?;
    writer.WriteBytes(&seen)?;
    let buffer = writer.DetachBuffer()?;
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        seen_width,
        seen_height,
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
                left: (back(bounds.X).floor() as i32).clamp(0, width),
                top: (back(bounds.Y).floor() as i32).clamp(0, height),
                right: (back(bounds.X + bounds.Width).ceil() as i32).clamp(0, width),
                bottom: (back(bounds.Y + bounds.Height).ceil() as i32).clamp(0, height),
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
            return (over_word && !text.is_empty()).then_some(Selection { text, rect: found.1, context: None });
        }
        element = walker.get_parent(&element).ok()?;
    }
    None
}

/// The whole sentence under a screen point, from its start to its full stop.
/// The rectangle is the line under the pointer, so the card opens below it.
pub fn sentence_at(x: i32, y: i32) -> Option<Selection> {
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
            return unsafe {
                let word = raw.RangeFromPoint(POINT { x, y }).ok()?;
                word.ExpandToEnclosingUnit(TextUnit_Word).ok()?;
                // Same check as word_at: the pointer must be over text, not blank space.
                if !bounding_rect(&word).is_some_and(|r| r.contains(x, y)) {
                    return None;
                }
                let line = word.Clone().ok()?;
                line.ExpandToEnclosingUnit(TextUnit_Line).ok()?;
                let text = sentence_of(&word)?.trim().to_string();
                (!text.is_empty()).then(|| Selection { text, rect: bounding_rect(&line).or(bounding_rect(&word)), context: None })
            };
        }
        element = walker.get_parent(&element).ok()?;
    }
    None
}

/// Ctrl is the "translate what I point at" modifier.
pub fn hover_modifier_down() -> bool {
    key_down(VK_CONTROL)
}

// ---------------------------------------------------------- live captions

const CAPTIONS_WINDOW: &str = "LiveCaptionsDesktopWindow";

/// What Windows' own live captions (Win+Ctrl+L) show right now: the speech
/// it recognised in the sound the computer plays. None while the feature is
/// off or has nothing to show.
pub fn live_captions() -> Option<super::Caption> {
    use uiautomation::types::{TreeScope, UIProperty};
    use uiautomation::variants::Variant;

    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    let handle = unsafe { FindWindowW(&HSTRING::from(CAPTIONS_WINDOW), None) }.ok().filter(|window| !window.0.is_null())?;
    let automation = UIAutomation::new().ok()?;
    let window = automation.element_from_handle(uiautomation::types::Handle::from(handle.0 as isize)).ok()?;
    let condition = automation.create_property_condition(UIProperty::AutomationId, Variant::from("CaptionsTextBlock"), None).ok()?;
    let text = window.find_first(TreeScope::Descendants, &condition).ok()?.get_name().ok()?;
    // From the window itself: it may sit off the screen, where the accessibility tree reports no bounds.
    let mut bounds = windows::Win32::Foundation::RECT::default();
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowRect(handle, &mut bounds) }.ok()?;
    let rect = Rect { left: bounds.left, top: bounds.top, right: bounds.right, bottom: bounds.bottom };
    Some(super::Caption { text, rect })
}

fn captions_window() -> Option<HWND> {
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    unsafe { FindWindowW(&HSTRING::from(CAPTIONS_WINDOW), None) }.ok().filter(|window| !window.0.is_null())
}

/// Far enough to the left to be on no screen.
const CAPTIONS_AWAY: i32 = -30000;
/// Where the captions window was before `hide_live_captions` moved it away.
static CAPTIONS_HOME: std::sync::Mutex<Option<(i32, i32)>> = std::sync::Mutex::new(None);

/// Moves the captions window off the screen, where it keeps listening, or
/// back to where it was. Returns whether the window is now where asked.
pub fn hide_live_captions(hide: bool) -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, SWP_NOZORDER};
    let Some(window) = captions_window() else { return false };
    let mut home = CAPTIONS_HOME.lock().unwrap();
    let mut rect = RECT::default();
    let target = unsafe {
        if GetWindowRect(window, &mut rect).is_err() {
            return false;
        }
        if hide {
            // Only the place it had before we first moved it counts.
            if home.is_none() {
                *home = Some((rect.left, rect.top));
            }
            (CAPTIONS_AWAY, rect.top)
        } else {
            match home.take() {
                Some(place) => place,
                None => return true,
            }
        }
    };
    unsafe { SetWindowPos(window, None, target.0, target.1, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE).is_ok() }
}

/// Closes the captions window.
pub fn close_live_captions() {
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
    if let Some(window) = captions_window() {
        unsafe {
            let _ = PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }
}

/// Opens Windows' live captions unless they are already on screen. Returns
/// whether they had to be opened.
pub fn start_live_captions() -> anyhow::Result<bool> {
    if captions_window().is_some() {
        return Ok(false);
    }
    let system = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    std::process::Command::new(format!("{system}\\System32\\LiveCaptions.exe"))
        .spawn()
        .map_err(|_| anyhow::anyhow!("Máy này không có Live Captions (cần Windows 11 22H2 trở lên)"))?;
    Ok(true)
}

// --------------------------------------------------------------- clipboard

/// A number that changes whenever the clipboard does.
pub fn clipboard_change() -> u64 {
    unsafe { GetClipboardSequenceNumber() as u64 }
}

/// Whether the app that copied asked clipboard histories to leave this out
/// (password managers do).
pub fn clipboard_concealed() -> bool {
    use windows::core::w;
    use windows::Win32::System::DataExchange::{IsClipboardFormatAvailable, RegisterClipboardFormatW};
    unsafe {
        let format = RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing"));
        format != 0 && IsClipboardFormatAvailable(format).is_ok()
    }
}

/// Whether the clipboard holds an image (bitmap, DIB, or PNG) rather than
/// text. The board saves a placeholder so the image can be pasted back.
pub fn clipboard_has_image() -> bool {
    use windows::Win32::System::DataExchange::IsClipboardFormatAvailable;
    use windows::Win32::System::Ole::{CF_BITMAP, CF_DIB, CF_DIBV5};
    // Also check PNG format registered by many apps (e.g. Snipping Tool).
    use windows::core::w;
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
    unsafe {
        if IsClipboardFormatAvailable(CF_BITMAP.0 as u32).is_ok()
            || IsClipboardFormatAvailable(CF_DIB.0 as u32).is_ok()
            || IsClipboardFormatAvailable(CF_DIBV5.0 as u32).is_ok()
        {
            return true;
        }
        let png = RegisterClipboardFormatW(w!("PNG"));
        png != 0 && IsClipboardFormatAvailable(png).is_ok()
    }
}

/// Copies a screen rectangle to the system clipboard as an image.
pub fn copy_screenshot(rect: Rect) -> anyhow::Result<()> {
    let (mut pixels, width, height) = grab(rect)?;
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    let img_data = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: std::borrow::Cow::Borrowed(&pixels),
    };
    let mut cb = arboard::Clipboard::new()?;
    cb.set_image(img_data)?;
    Ok(())
}

/// Reads rich text (HTML) from the clipboard if available.
pub fn clipboard_html() -> Option<String> {
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
    };
    use windows::Win32::System::Memory::GlobalLock;
    use windows::core::w;
    unsafe {
        let format = RegisterClipboardFormatW(w!("HTML Format"));
        if format == 0 || !IsClipboardFormatAvailable(format).is_ok() {
            return None;
        }
        if !OpenClipboard(None).is_ok() {
            return None;
        }
        let handle = GetClipboardData(format);
        if handle.is_err() {
            let _ = CloseClipboard();
            return None;
        }
        let hglobal = windows::Win32::Foundation::HGLOBAL(handle.unwrap().0);
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            let _ = CloseClipboard();
            return None;
        }
        let cstr = std::ffi::CStr::from_ptr(ptr as *const std::ffi::c_char);
        let bytes = cstr.to_bytes().to_vec();
        let _ = windows::Win32::System::Memory::GlobalUnlock(hglobal);
        let raw = String::from_utf8_lossy(&bytes).to_string();
        let _ = CloseClipboard();
        if let (Some(start_idx), Some(end_idx)) = (raw.find("StartFragment:"), raw.find("EndFragment:")) {
            let start_pos: usize = raw[start_idx + 14..].lines().next().and_then(|l| l.trim().parse().ok()).unwrap_or(0);
            let end_pos: usize = raw[end_idx + 12..].lines().next().and_then(|l| l.trim().parse().ok()).unwrap_or(bytes.len());
            if start_pos < end_pos && end_pos <= bytes.len() {
                return String::from_utf8(bytes[start_pos..end_pos].to_vec()).ok();
            }
        }
        Some(raw)
    }
}

/// Sets rich text (HTML) and plain text fallback to the clipboard.
pub fn set_clipboard_html(_html: &str, text: &str) -> anyhow::Result<()> {
    let mut cb = arboard::Clipboard::new()?;
    cb.set_text(text)?;
    Ok(())
}


/// The window in front, to come back to with `activate`.
pub fn foreground_handle() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

/// Brings a window back in front. Windows only allows this while one of our
/// own windows is the one in front.
pub fn activate(handle: isize) {
    use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
    if handle != 0 {
        unsafe {
            let _ = SetForegroundWindow(HWND(handle as *mut c_void));
        }
    }
}

/// Presses Ctrl+V in the window in front.
pub fn send_paste() {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_V;
    std::thread::sleep(Duration::from_millis(120));
    release_held_modifiers();
    let ctrl_held = key_down(VK_CONTROL);
    let mut inputs = Vec::with_capacity(4);
    if !ctrl_held {
        inputs.push(key_input(VK_CONTROL, false));
    }
    inputs.push(key_input(VK_V, false));
    inputs.push(key_input(VK_V, true));
    if !ctrl_held {
        inputs.push(key_input(VK_CONTROL, true));
    }
    send(&inputs);
}

#[cfg(test)]
mod caption_tests {
    /// Prints what the live captions show; needs them open and something speaking.
    #[test]
    #[ignore]
    fn reads_the_live_captions() {
        for _ in 0..12 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            println!("{:?}", super::live_captions().map(|caption| (caption.text, caption.rect)));
        }
    }
}

#[cfg(test)]
mod ocr_tests {
    use super::*;

    #[test]
    fn small_pictures_are_magnified_within_the_limit() {
        assert_eq!(magnification(800, 600), 1);
        assert_eq!(magnification(90, 28), 4);
        assert_eq!(magnification(300, 100), 2);
        // A long thin strip may not grow past what the recogniser accepts.
        assert_eq!(magnification(3000, 30), 2);
        assert_eq!(magnification(7990, 20), 1);
    }

    #[test]
    fn enlarging_keeps_the_picture_inside_a_margin() {
        // Two pixels: black, then white, on a black corner.
        let pixels = [0, 0, 0, 255, 255, 255, 255, 255];
        let (out, width, height) = enlarge(&pixels, 2, 1, 4);
        assert_eq!((width, height), (8 + 2 * OCR_MARGIN, 4 + 2 * OCR_MARGIN));
        let at = |x: i32, y: i32| out[((y * width + x) * 4) as usize];
        assert_eq!(at(0, 0), 0);
        assert_eq!(at(OCR_MARGIN, OCR_MARGIN), 0);
        assert_eq!(at(OCR_MARGIN + 7, OCR_MARGIN + 3), 255);
        assert_eq!(at(width - 1, height - 1), 0);
    }

    /// Reads the screen rectangle given as `QT_OCR_RECT=left,top,right,bottom`.
    #[test]
    #[ignore]
    fn reads_a_screen_rectangle() {
        let numbers: Vec<i32> = std::env::var("QT_OCR_RECT").unwrap().split(',').map(|n| n.parse().unwrap()).collect();
        let rect = Rect { left: numbers[0], top: numbers[1], right: numbers[2], bottom: numbers[3] };
        println!("magnified {}x: {:?}", magnification(rect.right - rect.left, rect.bottom - rect.top), ocr_lines(rect, None).unwrap());
    }
}
