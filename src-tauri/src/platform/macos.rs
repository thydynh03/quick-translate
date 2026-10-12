#![allow(unsafe_op_in_unsafe_fn)]
//! macOS implementation. Reading the selection and the global mouse tap both
//! need the Accessibility permission (System Settings → Privacy & Security).
//!
//! Reading the screen (OCR through the Vision framework) needs the Screen
//! Recording permission.
//!
//! Not yet implemented here: a non-activating panel for the popup — showing
//! it briefly activates Quick Translate.

use std::ffi::{c_char, c_void, CStr};
use std::ptr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use accessibility_sys::{
    kAXBoundsForRangeParameterizedAttribute, kAXErrorSuccess, kAXFocusedUIElementAttribute,
    kAXRangeForPositionParameterizedAttribute, kAXSelectedTextAttribute, kAXSelectedTextRangeAttribute,
    kAXTrustedCheckOptionPrompt, kAXValueAttribute, kAXValueTypeCFRange, kAXValueTypeCGPoint, kAXValueTypeCGRect,
    AXIsProcessTrustedWithOptions, AXUIElementCopyAttributeValue, AXUIElementCopyElementAtPosition,
    AXUIElementCopyParameterizedAttributeValue, AXUIElementCreateSystemWide, AXUIElementRef, AXValueCreate,
    AXValueGetValue, AXValueRef,
};
use core_foundation::array::{CFArray, CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation::base::{CFGetTypeID, CFRange, CFRelease, CFType, CFTypeRef, TCFType};
use core_foundation::data::{CFDataGetBytePtr, CFDataGetLength, CFDataRef};
use core_foundation::dictionary::{CFDictionaryGetValue, CFDictionaryRef};
use core_foundation::number::{kCFNumberSInt32Type, CFNumberGetValue, CFNumberRef};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::event::{
    CallbackResult, CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::{CGPoint, CGRect, CGSize};

use objc2::encode::{Encode, Encoding, RefEncode};
use objc2::rc::autoreleasepool;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send, sel};

use super::{MouseEvent, Rect, Selection};

/// Backing scale factor, stored as f64 bits. macOS APIs speak points; the
/// rest of the app works in physical pixels.
static SCALE: AtomicU64 = AtomicU64::new(0);

fn scale() -> f64 {
    match SCALE.load(Ordering::Relaxed) {
        0 => 1.0,
        bits => f64::from_bits(bits),
    }
}

fn to_pixels(value: f64) -> i32 {
    (value * scale()).round() as i32
}

// ---------------------------------------------------------------- selection

unsafe fn copy_attribute(element: AXUIElementRef, name: &'static str) -> Option<CFTypeRef> {
    let mut value: CFTypeRef = ptr::null();
    let attribute = CFString::from_static_string(name);
    let status = AXUIElementCopyAttributeValue(element, attribute.as_concrete_TypeRef(), &mut value);
    (status == kAXErrorSuccess && !value.is_null()).then_some(value)
}

/// Reads the selected text of the focused UI element through the
/// Accessibility API, with its on-screen bounds when the app provides them.
pub fn selection_via_accessibility() -> Option<Selection> {
    unsafe {
        let system = AXUIElementCreateSystemWide();
        let focused = copy_attribute(system, kAXFocusedUIElementAttribute);
        CFRelease(system as CFTypeRef);
        let focused = focused?;
        let element = focused as AXUIElementRef;

        let text = copy_attribute(element, kAXSelectedTextAttribute).and_then(|value| {
            let text = (CFGetTypeID(value) == CFString::type_id())
                .then(|| CFString::wrap_under_get_rule(value as CFStringRef).to_string());
            CFRelease(value);
            text
        });

        let mut rect = None;
        let mut context = None;
        if let Some(range) = copy_attribute(element, kAXSelectedTextRangeAttribute) {
            rect = bounds_for_value(element, range);
            // A word or short phrase: also read the sentence it sits in.
            if let (Some(selected), Some(r)) = (&text, range_of(range)) {
                if selected.split_whitespace().count() <= 4 {
                    context = sentence_in_value(element, r.location).filter(|s| s.trim() != selected.trim());
                }
            }
            CFRelease(range);
        }
        CFRelease(focused);

        let text = text?.trim().to_string();
        (!text.is_empty()).then_some(Selection { text, rect, context })
    }
}

/// Screen bounds of an AXValue holding a CFRange of `element`'s text.
unsafe fn bounds_for_value(element: AXUIElementRef, range: CFTypeRef) -> Option<Rect> {
    let mut bounds: CFTypeRef = ptr::null();
    let attribute = CFString::from_static_string(kAXBoundsForRangeParameterizedAttribute);
    let status = AXUIElementCopyParameterizedAttributeValue(element, attribute.as_concrete_TypeRef(), range, &mut bounds);
    if status != kAXErrorSuccess || bounds.is_null() {
        return None;
    }
    let mut cg = CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(0.0, 0.0));
    let read = AXValueGetValue(bounds as AXValueRef, kAXValueTypeCGRect, &mut cg as *mut CGRect as *mut c_void);
    CFRelease(bounds);
    (read && cg.size.width > 0.0 && cg.size.height > 0.0).then(|| Rect {
        left: to_pixels(cg.origin.x),
        top: to_pixels(cg.origin.y),
        right: to_pixels(cg.origin.x + cg.size.width),
        bottom: to_pixels(cg.origin.y + cg.size.height),
    })
}

unsafe fn range_of(value: CFTypeRef) -> Option<CFRange> {
    let mut range = CFRange { location: 0, length: 0 };
    AXValueGetValue(value as AXValueRef, kAXValueTypeCFRange, &mut range as *mut CFRange as *mut c_void).then_some(range)
}

/// Longest text read from one element; bigger documents are skipped.
const MAX_VALUE_CHARS: usize = 200_000;

/// The element's whole text (AXValue), when it is a string of sensible size.
unsafe fn value_text(element: AXUIElementRef) -> Option<String> {
    let value = copy_attribute(element, kAXValueAttribute)?;
    let text = (CFGetTypeID(value) == CFString::type_id()).then(|| CFString::wrap_under_get_rule(value as CFStringRef).to_string());
    CFRelease(value);
    text.filter(|t| t.len() <= MAX_VALUE_CHARS * 4)
}

/// Accessibility ranges count UTF-16 units; the sentence code counts chars.
fn utf16_to_char_offset(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (index, c) in text.chars().enumerate() {
        if units >= utf16 {
            return index;
        }
        units += c.len_utf16();
    }
    text.chars().count()
}

/// The sentence of the element's text around a UTF-16 position.
unsafe fn sentence_in_value(element: AXUIElementRef, location: isize) -> Option<String> {
    let text = value_text(element)?;
    let offset = utf16_to_char_offset(&text, location.max(0) as usize);
    let paragraph_start = text.chars().take(offset).collect::<Vec<_>>().iter().rposition(|c| *c == '\n').map_or(0, |i| i + 1);
    let paragraph: String = text.chars().skip(paragraph_start).take_while(|c| *c != '\n').collect();
    crate::sentence::around(&paragraph, offset - paragraph_start)
}

/// The word of `text` around a char offset.
fn word_around(text: &str, offset: usize) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let is_word = |c: char| c.is_alphanumeric() || c == '\'' || c == '-';
    if offset >= chars.len() || !is_word(chars[offset]) {
        return None;
    }
    let start = (0..offset).rev().take_while(|&i| is_word(chars[i])).last().unwrap_or(offset);
    let end = (offset..chars.len()).take_while(|&i| is_word(chars[i])).last().unwrap_or(offset) + 1;
    let word: String = chars[start..end].iter().collect();
    let word = word.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
    (!word.is_empty()).then_some(word)
}

/// The element under a screen point (pixels) and the text position under it.
unsafe fn text_at_point(x: i32, y: i32) -> Option<(AXUIElementRef, CFRange)> {
    let (px, py) = (x as f64 / scale(), y as f64 / scale());
    let system = AXUIElementCreateSystemWide();
    let mut element: AXUIElementRef = ptr::null_mut();
    let status = AXUIElementCopyElementAtPosition(system, px as f32, py as f32, &mut element);
    CFRelease(system as CFTypeRef);
    if status != kAXErrorSuccess || element.is_null() {
        return None;
    }
    let point = CGPoint::new(px, py);
    let value = AXValueCreate(kAXValueTypeCGPoint, &point as *const CGPoint as *const c_void);
    let mut found: CFTypeRef = ptr::null();
    let attribute = CFString::from_static_string(kAXRangeForPositionParameterizedAttribute);
    let status = AXUIElementCopyParameterizedAttributeValue(element, attribute.as_concrete_TypeRef(), value as CFTypeRef, &mut found);
    CFRelease(value as CFTypeRef);
    if status != kAXErrorSuccess || found.is_null() {
        CFRelease(element as CFTypeRef);
        return None;
    }
    let range = range_of(found);
    CFRelease(found);
    match range {
        Some(range) => Some((element, range)),
        None => {
            CFRelease(element as CFTypeRef);
            None
        }
    }
}

/// Bounds of one character, to check the pointer is really over text.
unsafe fn char_bounds(element: AXUIElementRef, location: isize) -> Option<Rect> {
    let range = CFRange { location, length: 1 };
    let value = AXValueCreate(kAXValueTypeCFRange, &range as *const CFRange as *const c_void);
    let rect = bounds_for_value(element, value as CFTypeRef);
    CFRelease(value as CFTypeRef);
    rect
}

/// A rectangle a few pixels around the pointer, for placing the card.
fn pointer_rect(x: i32, y: i32) -> Rect {
    Rect { left: x, top: y - 10, right: x, bottom: y + 12 }
}

/// Shared by word_at and sentence_at: reads the text under the pointer.
fn read_at(x: i32, y: i32, sentence: bool) -> Option<Selection> {
    unsafe {
        let (element, range) = text_at_point(x, y)?;
        let near = char_bounds(element, range.location).is_some_and(|r| r.left - 4 <= x && x <= r.right + 4 && r.top - 4 <= y && y <= r.bottom + 4);
        let found = near
            .then(|| value_text(element))
            .flatten()
            .and_then(|text| {
                let offset = utf16_to_char_offset(&text, range.location.max(0) as usize);
                if sentence {
                    sentence_in_value(element, range.location)
                } else {
                    word_around(&text, offset)
                }
            });
        let rect = char_bounds(element, range.location).map(|r| Rect { left: x, right: x, ..r });
        CFRelease(element as CFTypeRef);
        let text = found?.trim().to_string();
        (!text.is_empty()).then(|| Selection { text, rect: rect.or(Some(pointer_rect(x, y))), context: None })
    }
}

const KEY_C: u16 = 8;

/// Fallback: press Cmd+C, read the clipboard, then restore its previous text.
pub fn selection_via_copy() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let saved = clipboard.get_text().ok();
    // An empty clipboard makes the copy easy to detect.
    let _ = clipboard.clear();

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    for down in [true, false] {
        let event = CGEvent::new_keyboard_event(source.clone(), KEY_C, down).ok()?;
        event.set_flags(CGEventFlags::CGEventFlagCommand);
        event.post(CGEventTapLocation::HID);
    }

    let deadline = Instant::now() + Duration::from_millis(500);
    let mut text = None;
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
        if let Ok(copied) = clipboard.get_text() {
            if !copied.is_empty() {
                text = Some(copied);
                break;
            }
        }
    }
    if let Some(previous) = saved {
        let _ = clipboard.set_text(previous);
    }
    text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

// ------------------------------------------------------------------ cursor

pub fn cursor_pos() -> (i32, i32) {
    CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .and_then(CGEvent::new)
        .map(|event| {
            let point = event.location();
            (to_pixels(point.x), to_pixels(point.y))
        })
        .unwrap_or((0, 0))
}

/// The pointer shape is not readable across apps on macOS, so the icon is
/// only offered when the Accessibility API reports a selection.
pub fn cursor_is_text() -> bool {
    false
}

pub fn double_click_ms() -> u64 {
    500
}

/// Name of the application in front, lower-case, for example "safari".
pub fn foreground_app() -> Option<String> {
    front_window().map(|(name, _)| name).filter(|name| !name.is_empty())
}

// -------------------------------------------------------------- mouse hook

/// True when the app may use the Accessibility API. With `prompt`, macOS
/// shows its dialog pointing the user to System Settings.
fn accessibility_granted(prompt: bool) -> bool {
    unsafe {
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let value = if prompt { CFBoolean::true_value() } else { CFBoolean::false_value() };
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
    }
}

/// Listens to mouse buttons and the wheel with a listen-only event tap.
/// Waits for the Accessibility permission, so granting it needs no restart.
pub fn start_mouse_hook(tx: Sender<MouseEvent>) {
    std::thread::spawn(move || {
        if !accessibility_granted(true) {
            crate::log("mouse tap: waiting for the Accessibility permission");
            while !accessibility_granted(false) {
                std::thread::sleep(Duration::from_secs(2));
            }
        }
        let tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![
                CGEventType::LeftMouseDown,
                CGEventType::LeftMouseUp,
                CGEventType::RightMouseDown,
                CGEventType::ScrollWheel,
            ],
            move |_proxy, kind, event| {
                let point = event.location();
                let (x, y) = (to_pixels(point.x), to_pixels(point.y));
                let mouse = match kind {
                    CGEventType::LeftMouseDown => Some(MouseEvent::LeftDown { x, y }),
                    CGEventType::LeftMouseUp => Some(MouseEvent::LeftUp { x, y }),
                    CGEventType::RightMouseDown => Some(MouseEvent::OtherDown { x, y }),
                    CGEventType::ScrollWheel => Some(MouseEvent::Wheel { x, y }),
                    _ => None,
                };
                if let Some(mouse) = mouse {
                    let _ = tx.send(mouse);
                }
                CallbackResult::Keep
            },
        );
        match tap {
            Ok(tap) => unsafe {
                if let Ok(source) = tap.mach_port().create_runloop_source(0) {
                    CFRunLoop::get_current().add_source(&source, kCFRunLoopCommonModes);
                    tap.enable();
                    CFRunLoop::run_current();
                }
            },
            Err(()) => crate::log("mouse tap: could not be created"),
        }
    });
}

// ----------------------------------------------------------------- overlay

pub fn prepare_overlay(window: &tauri::WebviewWindow) {
    if let Ok(factor) = window.scale_factor() {
        SCALE.store(factor.to_bits(), Ordering::Relaxed);
    }
}

pub fn show_overlay(window: &tauri::WebviewWindow) {
    let _ = window.show();
}

pub fn hide_overlay(window: &tauri::WebviewWindow) {
    let _ = window.hide();
}

// ------------------------------------------------------------------ screen

#[repr(C)]
struct CGImageOpaque {
    _private: [u8; 0],
}

unsafe impl RefEncode for CGImageOpaque {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("CGImage", &[]));
}

/// `CGRect` as Objective-C methods return it.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VisionRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

unsafe impl Encode for VisionRect {
    const ENCODING: Encoding = Encoding::Struct(
        "CGRect",
        &[
            Encoding::Struct("CGPoint", &[Encoding::Double, Encoding::Double]),
            Encoding::Struct("CGSize", &[Encoding::Double, Encoding::Double]),
        ],
    );
}

const WINDOWS_ON_SCREEN: u32 = 1 << 0;
const WINDOWS_BELOW: u32 = 1 << 2;
const WINDOWS_NO_DESKTOP: u32 = 1 << 4;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCreateImage(bounds: CGRect, list: u32, window: u32, image: u32) -> *mut CGImageOpaque;
    fn CGWindowListCopyWindowInfo(list: u32, window: u32) -> CFArrayRef;
    fn CGImageGetWidth(image: *mut CGImageOpaque) -> usize;
    fn CGImageGetHeight(image: *mut CGImageOpaque) -> usize;
    fn CGImageGetBytesPerRow(image: *mut CGImageOpaque) -> usize;
    fn CGImageGetDataProvider(image: *mut CGImageOpaque) -> *mut c_void;
    fn CGDataProviderCopyData(provider: *mut c_void) -> CFDataRef;
    fn CGImageRelease(image: *mut CGImageOpaque);
    fn CGRectMakeWithDictionaryRepresentation(dictionary: CFDictionaryRef, rect: *mut CGRect) -> bool;
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
    static kCGWindowLayer: CFStringRef;
    static kCGWindowOwnerPID: CFStringRef;
    static kCGWindowOwnerName: CFStringRef;
    static kCGWindowBounds: CFStringRef;
}

#[link(name = "Vision", kind = "framework")]
unsafe extern "C" {}

/// Window number of the live overlay while it must stay out of captures.
static HIDDEN_FROM_CAPTURE: AtomicU32 = AtomicU32::new(0);

/// Reading the screen needs the Screen Recording permission. The first call
/// without it makes macOS show its dialog.
fn screen_access() -> anyhow::Result<()> {
    unsafe {
        if CGPreflightScreenCaptureAccess() {
            return Ok(());
        }
        CGRequestScreenCaptureAccess();
    }
    anyhow::bail!(
        "hãy cấp quyền Ghi màn hình (Screen Recording) cho Quick Translate trong System Settings → Privacy & Security, rồi mở lại ứng dụng"
    )
}

/// A picture of a screen rectangle (physical pixels). Release it with `CGImageRelease`.
unsafe fn screenshot(rect: Rect) -> anyhow::Result<*mut CGImageOpaque> {
    anyhow::ensure!(rect.right > rect.left && rect.bottom > rect.top, "vùng chọn rỗng");
    screen_access()?;
    let factor = scale();
    let bounds = CGRect::new(
        &CGPoint::new(rect.left as f64 / factor, rect.top as f64 / factor),
        &CGSize::new((rect.right - rect.left) as f64 / factor, (rect.bottom - rect.top) as f64 / factor),
    );
    let image = match HIDDEN_FROM_CAPTURE.load(Ordering::Relaxed) {
        0 => CGWindowListCreateImage(bounds, WINDOWS_ON_SCREEN, 0, 0),
        overlay => CGWindowListCreateImage(bounds, WINDOWS_BELOW, overlay, 0),
    };
    anyhow::ensure!(!image.is_null(), "không chụp được màn hình");
    Ok(image)
}

/// Reads the text in a screen rectangle, line by line, with each line's box
/// relative to the rectangle. The Vision framework reads Vietnamese itself,
/// so the separate Vietnamese recogniser is not used here.
pub fn ocr_lines(rect: Rect, _vietnamese: Option<&std::path::Path>) -> anyhow::Result<Vec<(Rect, String)>> {
    let (width, height) = ((rect.right - rect.left) as f64, (rect.bottom - rect.top) as f64);
    unsafe {
        let image = screenshot(rect)?;
        let lines = recognise(image, width, height);
        CGImageRelease(image);
        lines
    }
}

/// Reads the lines of text in a picture. Boxes are scaled to `width` × `height`.
unsafe fn recognise(image: *mut CGImageOpaque, width: f64, height: f64) -> anyhow::Result<Vec<(Rect, String)>> {
    {
        let lines = autoreleasepool(|_| {
            let options: *mut AnyObject = msg_send![class!(NSDictionary), dictionary];
            let handler: *mut AnyObject = msg_send![class!(VNImageRequestHandler), alloc];
            let handler: *mut AnyObject = msg_send![handler, initWithCGImage: image, options: options];
            let request: *mut AnyObject = msg_send![class!(VNRecognizeTextRequest), new];
            let _: () = msg_send![request, setUsesLanguageCorrection: true];
            // macOS 13 and later can tell the language of the text by themselves.
            let detects: bool = msg_send![request, respondsToSelector: sel!(setAutomaticallyDetectsLanguage:)];
            if detects {
                let _: () = msg_send![request, setAutomaticallyDetectsLanguage: true];
            }
            let requests = CFArray::from_CFTypes(&[CFType::wrap_under_get_rule(request as CFTypeRef)]);
            let failure: *mut *mut AnyObject = ptr::null_mut();
            let done: bool = msg_send![handler, performRequests: requests.as_concrete_TypeRef() as *mut AnyObject, error: failure];

            let mut lines = Vec::new();
            let results: *mut AnyObject = if done { msg_send![request, results] } else { ptr::null_mut() };
            let count: usize = if results.is_null() { 0 } else { msg_send![results, count] };
            for index in 0..count {
                let observation: *mut AnyObject = msg_send![results, objectAtIndex: index];
                let candidates: *mut AnyObject = msg_send![observation, topCandidates: 1usize];
                let found: usize = if candidates.is_null() { 0 } else { msg_send![candidates, count] };
                if found == 0 {
                    continue;
                }
                let candidate: *mut AnyObject = msg_send![candidates, objectAtIndex: 0usize];
                let string: *mut AnyObject = msg_send![candidate, string];
                let utf8: *const c_char = if string.is_null() { ptr::null() } else { msg_send![string, UTF8String] };
                if utf8.is_null() {
                    continue;
                }
                let text = CStr::from_ptr(utf8).to_string_lossy().trim().to_string();
                // Boxes are fractions of the picture, measured from its bottom-left corner.
                let bounds: VisionRect = msg_send![observation, boundingBox];
                let line = Rect {
                    left: (bounds.x * width).floor() as i32,
                    top: ((1.0 - bounds.y - bounds.height) * height).floor() as i32,
                    right: ((bounds.x + bounds.width) * width).ceil() as i32,
                    bottom: ((1.0 - bounds.y) * height).ceil() as i32,
                };
                if !text.is_empty() {
                    lines.push((line, text));
                }
            }
            let _: () = msg_send![request, release];
            let _: () = msg_send![handler, release];
            done.then_some(lines)
        });
        let mut lines = lines.ok_or_else(|| anyhow::anyhow!("Vision không đọc được ảnh"))?;
        lines.sort_by_key(|(line, _)| (line.top, line.left));
        Ok(lines)
    }
}

/// A cheap summary of what a screen rectangle shows, to tell whether it changed.
pub fn fingerprint(rect: Rect) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    unsafe {
        let image = screenshot(rect).ok()?;
        let data = CGDataProviderCopyData(CGImageGetDataProvider(image));
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (CGImageGetWidth(image), CGImageGetHeight(image), CGImageGetBytesPerRow(image)).hash(&mut hasher);
        if !data.is_null() {
            std::slice::from_raw_parts(CFDataGetBytePtr(data), CFDataGetLength(data) as usize).hash(&mut hasher);
            CFRelease(data as CFTypeRef);
        }
        CGImageRelease(image);
        (!data.is_null()).then(|| hasher.finish())
    }
}

/// The frontmost ordinary window of another application: its owner's name and bounds.
fn front_window() -> Option<(String, Rect)> {
    unsafe {
        let windows = CGWindowListCopyWindowInfo(WINDOWS_ON_SCREEN | WINDOWS_NO_DESKTOP, 0);
        if windows.is_null() {
            return None;
        }
        let number = |window: CFDictionaryRef, key: CFStringRef| {
            let mut value = 0i32;
            let found = CFDictionaryGetValue(window, key as *const c_void) as CFNumberRef;
            (!found.is_null() && CFNumberGetValue(found, kCFNumberSInt32Type, &mut value as *mut i32 as *mut c_void))
                .then_some(value)
        };
        let mut front = None;
        // The list runs from the front of the screen to the back.
        for index in 0..CFArrayGetCount(windows) {
            let window = CFArrayGetValueAtIndex(windows, index) as CFDictionaryRef;
            if number(window, kCGWindowLayer) != Some(0) || number(window, kCGWindowOwnerPID) == Some(std::process::id() as i32) {
                continue;
            }
            let mut bounds = CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(0.0, 0.0));
            let shape = CFDictionaryGetValue(window, kCGWindowBounds as *const c_void) as CFDictionaryRef;
            if shape.is_null() || !CGRectMakeWithDictionaryRepresentation(shape, &mut bounds) {
                continue;
            }
            let owner = CFDictionaryGetValue(window, kCGWindowOwnerName as *const c_void) as CFStringRef;
            let name = if owner.is_null() { String::new() } else { CFString::wrap_under_get_rule(owner).to_string() };
            let rect = Rect {
                left: to_pixels(bounds.origin.x),
                top: to_pixels(bounds.origin.y),
                right: to_pixels(bounds.origin.x + bounds.size.width),
                bottom: to_pixels(bounds.origin.y + bounds.size.height),
            };
            front = Some((name.to_lowercase(), rect));
            break;
        }
        CFRelease(windows as CFTypeRef);
        front
    }
}

/// Bounds of the window the user is working in.
pub fn foreground_rect() -> Option<Rect> {
    front_window().map(|(_, rect)| rect)
}

/// Keeps a window out of the pictures taken here (or lets it back in), so
/// the screen under it can still be read.
pub fn exclude_from_capture(window: &tauri::WebviewWindow, exclude: bool) {
    let number = match window.ns_window() {
        Ok(handle) if exclude && !handle.is_null() => unsafe {
            let number: isize = msg_send![handle as *mut AnyObject, windowNumber];
            number.max(0) as u32
        },
        _ => 0,
    };
    HIDDEN_FROM_CAPTURE.store(number, Ordering::Relaxed);
}

/// Shows a window without making Quick Translate the active application.
pub fn show_passive(window: &tauri::WebviewWindow) {
    let shown = window.clone();
    let _ = window.run_on_main_thread(move || {
        if let Ok(handle) = shown.ns_window() {
            if !handle.is_null() {
                unsafe {
                    let _: () = msg_send![handle as *mut AnyObject, orderFrontRegardless];
                }
            }
        }
    });
}

// ------------------------------------------------------------------- hover

/// The word under a screen point, in apps that expose their text through
/// the Accessibility API (TextEdit, Notes, Safari, Mail…).
pub fn word_at(x: i32, y: i32) -> Option<Selection> {
    read_at(x, y, false)
}

/// The whole sentence under a screen point, from its start to its full stop.
pub fn sentence_at(x: i32, y: i32) -> Option<Selection> {
    read_at(x, y, true)
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceFlagsState(state: i32) -> u64;
}

/// Control is the "translate what I point at" modifier, as on Windows.
pub fn hover_modifier_down() -> bool {
    const COMBINED_SESSION_STATE: i32 = 0;
    const CONTROL: u64 = 1 << 18;
    unsafe { CGEventSourceFlagsState(COMBINED_SESSION_STATE) & CONTROL != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_offsets_become_char_offsets() {
        // "😀" is two UTF-16 units but one char.
        assert_eq!(utf16_to_char_offset("a😀b", 3), 2);
        assert_eq!(utf16_to_char_offset("Việt Nam", 5), 5);
        assert_eq!(utf16_to_char_offset("ab", 10), 2);
    }

    #[test]
    fn word_around_an_offset() {
        let text = "Please confirm whether you can't attend.";
        assert_eq!(word_around(text, 9).as_deref(), Some("confirm"));
        assert_eq!(word_around(text, 29).as_deref(), Some("can't"));
        assert_eq!(word_around(text, 6), None);
    }

    #[link(name = "ImageIO", kind = "framework")]
    unsafe extern "C" {
        fn CGImageSourceCreateWithData(data: CFDataRef, options: CFDictionaryRef) -> *mut c_void;
        fn CGImageSourceCreateImageAtIndex(source: *mut c_void, index: usize, options: CFDictionaryRef) -> *mut CGImageOpaque;
    }

    /// Runs the real Vision recogniser on a picture with one English and one
    /// Vietnamese line, the way a screenshot would be read.
    #[test]
    fn vision_reads_both_lines_of_a_picture() {
        let png = core_foundation::data::CFData::from_buffer(include_bytes!("../../tests/data/ocr-sample.png"));
        let lines = unsafe {
            let source = CGImageSourceCreateWithData(png.as_concrete_TypeRef(), ptr::null());
            assert!(!source.is_null());
            let image = CGImageSourceCreateImageAtIndex(source, 0, ptr::null());
            assert!(!image.is_null());
            let lines = recognise(image, 900.0, 160.0).expect("vision");
            CGImageRelease(image);
            CFRelease(source as CFTypeRef);
            lines
        };
        println!("vision read: {lines:?}");
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[0].1, "The meeting has been moved to Thursday.");
        assert!(lines[0].0.top < lines[1].0.top && lines[0].0.left < 60 && lines[0].0.bottom < 90, "{lines:?}");
        // Vietnamese needs macOS 13 or later; without it the accents are lost.
        assert!(lines[1].1.contains("họp") && lines[1].1.contains("thứ"), "{lines:?}");
    }
}
