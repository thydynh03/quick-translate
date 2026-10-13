//! OS-specific pieces: reading the selection, the global mouse hook and the
//! non-activating overlay window. Each OS module exposes the same functions.

use serde::Serialize;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

/// Screen rectangle in physical pixels.
#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn point(x: i32, y: i32) -> Self {
        Rect { left: x, top: y, right: x, bottom: y }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    pub fn union(&self, other: &Rect) -> Rect {
        Rect {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

/// Speech the system recognised, and where its captions are on screen.
#[derive(Clone, Debug)]
pub struct Caption {
    pub text: String,
    pub rect: Rect,
}

#[derive(Clone, Debug)]
pub struct Selection {
    pub text: String,
    /// Bounding box of the selected text, when the app exposes it.
    pub rect: Option<Rect>,
    /// The sentence around a short selection, for explaining a word in context.
    pub context: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum MouseEvent {
    LeftDown { x: i32, y: i32 },
    LeftUp { x: i32, y: i32 },
    OtherDown { x: i32, y: i32 },
    Wheel { x: i32, y: i32 },
}
