//! Hotkeys and the mouse, polled: key presses with auto-repeat, key names for the UI, the cursor position.

use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyNameTextW, MapVirtualKeyW, MAPVK_VK_TO_VSC, VK_ADD, VK_DOWN, VK_HOME,
    VK_LEFT, VK_NEXT, VK_OEM_MINUS, VK_OEM_PLUS, VK_PRIOR, VK_RIGHT, VK_SUBTRACT, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// Keys edit mode itself uses, so they cannot be hotkeys.
pub const EDIT_MODE_KEYS: [u16; 11] = [
    VK_LEFT,
    VK_UP,
    VK_RIGHT,
    VK_DOWN,
    VK_PRIOR,
    VK_NEXT,
    VK_HOME,
    VK_OEM_PLUS,
    VK_OEM_MINUS,
    VK_ADD,
    VK_SUBTRACT,
];

/// Not offered as hotkeys when capturing: modifiers on their own, the Windows keys and IME keys.
pub const IGNORED_KEYS: [u16; 17] = [
    0x03, 0x10, 0x11, 0x12, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0x5B, 0x5C, 0x15, 0x17, 0x18, 0x19,
    0x1C,
];

/// A readable name for a virtual-key code, for the hotkey settings and the overlay's help text.
pub fn key_name(vk: u16) -> String {
    let fixed = match vk {
        0x04 => "Middle mouse",
        0x05 => "Mouse 4",
        0x06 => "Mouse 5",
        0x13 => "Pause",
        0x14 => "Caps Lock",
        0x2C => "Print Screen",
        0x2D => "Insert",
        0x2E => "Delete",
        0x23 => "End",
        0x90 => "Num Lock",
        0x91 => "Scroll Lock",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.to_string();
    }
    match vk {
        0x70..=0x87 => return format!("F{}", vk - 0x6F),
        0x60..=0x69 => return format!("Num {}", vk - 0x60),
        _ => {}
    }
    // letters, digits and punctuation: ask Windows for the name on this keyboard layout
    let mut buf = [0u16; 64];
    let len = unsafe {
        let scan = MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC);
        GetKeyNameTextW((scan << 16) as i32, buf.as_mut_ptr(), buf.len() as i32)
    };
    if len > 0 {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        format!("Key {vk}")
    }
}

/// Polled keyboard and mouse buttons: presses (edge-detected; bit 0 also catches taps shorter than a frame)
/// and auto-repeat for held keys.
#[derive(Default)]
pub struct Keys {
    held_since: std::collections::HashMap<u16, Instant>,
}

impl Keys {
    pub fn down(vk: u16) -> bool {
        unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
    }

    pub fn pressed(&mut self, vk: u16) -> bool {
        let state = unsafe { GetAsyncKeyState(vk as i32) } as u16;
        let down = state & 0x8000 != 0;
        let was_down = self.held_since.contains_key(&vk);
        if down && !was_down {
            self.held_since.insert(vk, Instant::now());
        } else if !down {
            self.held_since.remove(&vk);
        }
        (down && !was_down) || (!down && state & 1 != 0)
    }

    /// A press, then every frame while held for more than 0.3 s.
    pub fn repeating(&mut self, vk: u16) -> bool {
        let first = self.pressed(vk);
        first
            || self
                .held_since
                .get(&vk)
                .is_some_and(|t| t.elapsed() > Duration::from_millis(300))
    }
}

pub fn cursor() -> (i32, i32) {
    let mut p = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::settings;

    #[test]
    fn hotkey_names_and_reserved_keys() {
        assert_eq!(key_name(0x13), "Pause");
        assert_eq!(key_name(0x91), "Scroll Lock");
        assert_eq!(key_name(settings::DEFAULT_TOGGLE_KEY), "F3");
        assert_eq!(key_name(settings::DEFAULT_EDIT_KEY), "F4");
        assert_eq!(key_name(0x75), "F6");
        assert_eq!(key_name(0x05), "Mouse 4");
        assert!(!key_name(0x4B).is_empty()); // K, from the keyboard layout
                                             // the defaults must be bindable, and edit mode's own keys must not be
        for vk in [settings::DEFAULT_TOGGLE_KEY, settings::DEFAULT_EDIT_KEY] {
            assert!(!EDIT_MODE_KEYS.contains(&vk) && !IGNORED_KEYS.contains(&vk));
        }
        assert!(EDIT_MODE_KEYS.contains(&VK_HOME));
    }
}
