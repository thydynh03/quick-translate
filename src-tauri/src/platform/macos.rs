#![allow(unsafe_op_in_unsafe_fn)]
//! macOS implementation. Reading the selection and the global mouse tap both
//! need the Accessibility permission (System Settings → Privacy & Security).
//!
//! Not yet implemented here: OCR (Vision), hover lookup, and a non-activating
//! panel for the popup — showing it briefly activates Quick Translate.

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use accessibility_sys::{
    kAXBoundsForRangeParameterizedAttribute, kAXErrorSuccess, kAXFocusedUIElementAttribute,
    kAXSelectedTextAttribute, kAXSelectedTextRangeAttribute, kAXTrustedCheckOptionPrompt, kAXValueTypeCGRect,
    AXIsProcessTrustedWithOptions, AXUIElementCopyAttributeValue, AXUIElementCopyParameterizedAttributeValue,
    AXUIElementCreateSystemWide, AXUIElementRef, AXValueGetValue, AXValueRef,
};
use core_foundation::base::{CFGetTypeID, CFRelease, CFTypeRef, TCFType};
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
        if let Some(range) = copy_attribute(element, kAXSelectedTextRangeAttribute) {
            let mut bounds: CFTypeRef = ptr::null();
            let attribute = CFString::from_static_string(kAXBoundsForRangeParameterizedAttribute);
            let status = AXUIElementCopyParameterizedAttributeValue(
                element,
                attribute.as_concrete_TypeRef(),
                range,
                &mut bounds,
            );
            if status == kAXErrorSuccess && !bounds.is_null() {
                let mut cg = CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(0.0, 0.0));
                let read = AXValueGetValue(bounds as AXValueRef, kAXValueTypeCGRect, &mut cg as *mut CGRect as *mut c_void);
                if read && cg.size.width > 0.0 && cg.size.height > 0.0 {
                    rect = Some(Rect {
                        left: to_pixels(cg.origin.x),
                        top: to_pixels(cg.origin.y),
                        right: to_pixels(cg.origin.x + cg.size.width),
                        bottom: to_pixels(cg.origin.y + cg.size.height),
                    });
                }
                CFRelease(bounds);
            }
            CFRelease(range);
        }
        CFRelease(focused);

        let text = text?.trim().to_string();
        (!text.is_empty()).then_some(Selection { text, rect })
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

pub fn foreground_app() -> Option<String> {
    None
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

// ------------------------------------------------------- not yet supported

pub fn ocr_lines(_rect: Rect, _vietnamese: Option<&std::path::Path>) -> anyhow::Result<Vec<(Rect, String)>> {
    anyhow::bail!("OCR chưa hỗ trợ trên macOS")
}

pub fn foreground_rect() -> Option<Rect> {
    None
}

pub fn word_at(_x: i32, _y: i32) -> Option<Selection> {
    None
}

pub fn hover_modifier_down() -> bool {
    false
}
