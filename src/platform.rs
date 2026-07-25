/// Platform-specific event listening and keyboard simulation.
///
/// On macOS we provide our own CGEvent tap that handles **all** mouse buttons
/// (Left, Right, Middle, and side buttons) and our own keyboard simulation
/// with a complete keycode table.  rdev's macOS support is incomplete for both.
///
/// On other platforms we fall back to `rdev::listen` and `rdev::simulate`.

use rdev::{Button, Event, EventType, Key};
use std::time::SystemTime;

// ── macOS implementation ────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    #![allow(improper_ctypes_definitions)]
    #![allow(improper_ctypes)]
    #![allow(static_mut_refs)]

    use super::*;
    use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, EventField};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use std::os::raw::c_void;

    // ── Event tap (listening) ─────────────────────────────────────────────

    type CFMachPortRef = *const c_void;
    type CFIndex = u64;
    type CFRunLoopSourceRef = *const c_void;
    type CFRunLoopRef = *const c_void;
    type CGEventTapProxy = *const c_void;
    type CGEventRef = CGEvent;

    const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;

    #[repr(u32)]
    #[allow(dead_code)]
    enum CGEventTapOption {
        Default = 0,
        ListenOnly = 1,
    }

    type QCallback = unsafe extern "C" fn(
        proxy: CGEventTapProxy,
        _type: CGEventType,
        cg_event: CGEventRef,
        user_info: *mut c_void,
    ) -> CGEventRef;

    #[link(name = "Cocoa", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: CGEventTapLocation,
            place: u32,
            options: CGEventTapOption,
            events_of_interest: u64,
            callback: QCallback,
            user_info: *mut c_void,
        ) -> CFMachPortRef;

        fn CFMachPortCreateRunLoopSource(
            allocator: *const c_void,
            tap: CFMachPortRef,
            order: CFIndex,
        ) -> CFRunLoopSourceRef;

        fn CFRunLoopAddSource(
            rl: CFRunLoopRef,
            source: CFRunLoopSourceRef,
            mode: *const c_void,
        );

        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
        fn CFRunLoopRun();

        static kCFRunLoopCommonModes: *const c_void;
    }

    static mut GLOBAL_CALLBACK: Option<Box<dyn FnMut(Event)>> = None;

    unsafe extern "C" fn raw_callback(
        _proxy: CGEventTapProxy,
        _type: CGEventType,
        cg_event: CGEventRef,
        _user_info: *mut c_void,
    ) -> CGEventRef {
        if let Some(event) = convert_event(_type, &cg_event) {
            if let Some(cb) = &mut GLOBAL_CALLBACK {
                cb(event);
            }
        }
        cg_event
    }

    fn convert_event(_type: CGEventType, cg_event: &CGEvent) -> Option<Event> {
        let event_type = match _type {
            CGEventType::LeftMouseDown => EventType::ButtonPress(Button::Left),
            CGEventType::LeftMouseUp => EventType::ButtonRelease(Button::Left),
            CGEventType::RightMouseDown => EventType::ButtonPress(Button::Right),
            CGEventType::RightMouseUp => EventType::ButtonRelease(Button::Right),
            CGEventType::OtherMouseDown => {
                let btn = cg_event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
                EventType::ButtonPress(button_from_i64(btn))
            }
            CGEventType::OtherMouseUp => {
                let btn = cg_event.get_integer_value_field(EventField::MOUSE_EVENT_BUTTON_NUMBER);
                EventType::ButtonRelease(button_from_i64(btn))
            }
            _ => return None,
        };

        Some(Event { event_type, time: SystemTime::now(), name: None })
    }

    fn button_from_i64(n: i64) -> Button {
        match n {
            0 => Button::Left,
            1 => Button::Right,
            2 => Button::Middle,
            _ => Button::Unknown(n as u8),
        }
    }

    fn mouse_event_mask() -> u64 {
        let m = |t: CGEventType| -> u64 { 1 << (t as u64) };
        m(CGEventType::LeftMouseDown)
            | m(CGEventType::LeftMouseUp)
            | m(CGEventType::RightMouseDown)
            | m(CGEventType::RightMouseUp)
            | m(CGEventType::OtherMouseDown)
            | m(CGEventType::OtherMouseUp)
            | m(CGEventType::MouseMoved)
            | m(CGEventType::LeftMouseDragged)
            | m(CGEventType::RightMouseDragged)
            | m(CGEventType::ScrollWheel)
    }

    pub fn listen<T>(callback: T) -> Result<(), String>
    where
        T: FnMut(Event) + 'static,
    {
        unsafe {
            GLOBAL_CALLBACK = Some(Box::new(callback));

            let tap = CGEventTapCreate(
                CGEventTapLocation::HID,
                K_CG_HEAD_INSERT_EVENT_TAP,
                CGEventTapOption::ListenOnly,
                mouse_event_mask(),
                raw_callback,
                std::ptr::null_mut(),
            );

            if tap.is_null() {
                return Err(
                    "Failed to create event tap.\n\
                     Make sure Accessibility permission is granted:\n\
                     System Settings → Privacy & Security → Accessibility"
                        .to_string(),
                );
            }

            let loop_source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if loop_source.is_null() {
                return Err("Failed to create run loop source.".to_string());
            }

            let current_loop = CFRunLoopGetCurrent();
            CFRunLoopAddSource(current_loop, loop_source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            CFRunLoopRun();
        }
        Ok(())
    }

    // ── Keyboard simulation ───────────────────────────────────────────────
    //
    // rdev's `code_from_key` is missing many keys on macOS (Kp*, Delete,
    // Home, End, PageUp/Down, Insert, PrintScreen, ScrollLock, Pause,
    // NumLock).  We provide a complete mapping here.

    /// Convert a Key to a macOS virtual keycode.
    /// Returns None only for keys that genuinely have no macOS equivalent.
    fn keycode_from_key(key: Key) -> Option<u16> {
        // Letters (QWERTY layout keycodes)
        const KEY_A: u16 = 0;    const KEY_B: u16 = 11;   const KEY_C: u16 = 8;
        const KEY_D: u16 = 2;    const KEY_E: u16 = 14;   const KEY_F: u16 = 3;
        const KEY_G: u16 = 5;    const KEY_H: u16 = 4;    const KEY_I: u16 = 34;
        const KEY_J: u16 = 38;   const KEY_K: u16 = 40;   const KEY_L: u16 = 37;
        const KEY_M: u16 = 46;   const KEY_N: u16 = 45;   const KEY_O: u16 = 31;
        const KEY_P: u16 = 35;   const KEY_Q: u16 = 12;   const KEY_R: u16 = 15;
        const KEY_S: u16 = 1;    const KEY_T: u16 = 17;   const KEY_U: u16 = 32;
        const KEY_V: u16 = 9;    const KEY_W: u16 = 13;   const KEY_X: u16 = 7;
        const KEY_Y: u16 = 16;   const KEY_Z: u16 = 6;

        // Number row
        const NUM0: u16 = 29; const NUM1: u16 = 18; const NUM2: u16 = 19;
        const NUM3: u16 = 20; const NUM4: u16 = 21; const NUM5: u16 = 23;
        const NUM6: u16 = 22; const NUM7: u16 = 26; const NUM8: u16 = 28;
        const NUM9: u16 = 25;

        // Symbols (US layout)
        const MINUS: u16 = 27;      const EQUAL: u16 = 24;
        const LEFT_BRACKET: u16 = 33;  const RIGHT_BRACKET: u16 = 30;
        const SEMI_COLON: u16 = 41; const QUOTE: u16 = 39;
        const COMMA: u16 = 43;      const DOT: u16 = 47;
        const SLASH: u16 = 44;      const BACK_SLASH: u16 = 42;
        const BACK_QUOTE: u16 = 50;

        // Modifiers
        const ALT: u16 = 58;           const ALT_GR: u16 = 61;
        const CONTROL_LEFT: u16 = 59;  const CONTROL_RIGHT: u16 = 62;
        const SHIFT_LEFT: u16 = 56;    const SHIFT_RIGHT: u16 = 60;
        const META_LEFT: u16 = 55;     const META_RIGHT: u16 = 54;
        const CAPS_LOCK: u16 = 57;     const FUNCTION: u16 = 63;

        // Whitespace / editing
        const RETURN: u16 = 36;     const TAB: u16 = 48;
        const SPACE: u16 = 49;      const BACKSPACE: u16 = 51;
        const ESCAPE: u16 = 53;     const DELETE_FWD: u16 = 117;

        // Arrows
        const UP: u16 = 126;    const DOWN: u16 = 125;
        const LEFT: u16 = 123;  const RIGHT: u16 = 124;

        // Navigation
        const HOME: u16 = 115;      const END: u16 = 119;
        const PAGE_UP: u16 = 116;   const PAGE_DOWN: u16 = 121;

        // Function keys
        const F1: u16 = 122;  const F2: u16 = 120;  const F3: u16 = 99;
        const F4: u16 = 118;  const F5: u16 = 96;   const F6: u16 = 97;
        const F7: u16 = 98;   const F8: u16 = 100;  const F9: u16 = 101;
        const F10: u16 = 109; const F11: u16 = 103; const F12: u16 = 111;
        const F13: u16 = 105; const F14: u16 = 107; const F15: u16 = 113;

        // Numpad
        const KP0: u16 = 82; const KP1: u16 = 83; const KP2: u16 = 84;
        const KP3: u16 = 85; const KP4: u16 = 86; const KP5: u16 = 87;
        const KP6: u16 = 88; const KP7: u16 = 89; const KP8: u16 = 91;
        const KP9: u16 = 92;
        const KP_RETURN: u16 = 76;  const KP_MINUS: u16 = 78;
        const KP_PLUS: u16 = 69;    const KP_MULTIPLY: u16 = 67;
        const KP_DIVIDE: u16 = 75;  const KP_DELETE: u16 = 65;

        // Misc
        const INSERT: u16 = 114;    const NUM_LOCK: u16 = 71;
        const INTL_BACKSLASH: u16 = 10;

        // ── Match ──────────────────────────────────────────────────────

        let code = match key {
            Key::KeyA => KEY_A, Key::KeyB => KEY_B, Key::KeyC => KEY_C,
            Key::KeyD => KEY_D, Key::KeyE => KEY_E, Key::KeyF => KEY_F,
            Key::KeyG => KEY_G, Key::KeyH => KEY_H, Key::KeyI => KEY_I,
            Key::KeyJ => KEY_J, Key::KeyK => KEY_K, Key::KeyL => KEY_L,
            Key::KeyM => KEY_M, Key::KeyN => KEY_N, Key::KeyO => KEY_O,
            Key::KeyP => KEY_P, Key::KeyQ => KEY_Q, Key::KeyR => KEY_R,
            Key::KeyS => KEY_S, Key::KeyT => KEY_T, Key::KeyU => KEY_U,
            Key::KeyV => KEY_V, Key::KeyW => KEY_W, Key::KeyX => KEY_X,
            Key::KeyY => KEY_Y, Key::KeyZ => KEY_Z,

            Key::Num0 => NUM0, Key::Num1 => NUM1, Key::Num2 => NUM2,
            Key::Num3 => NUM3, Key::Num4 => NUM4, Key::Num5 => NUM5,
            Key::Num6 => NUM6, Key::Num7 => NUM7, Key::Num8 => NUM8,
            Key::Num9 => NUM9,

            Key::Minus => MINUS, Key::Equal => EQUAL,
            Key::LeftBracket => LEFT_BRACKET, Key::RightBracket => RIGHT_BRACKET,
            Key::SemiColon => SEMI_COLON, Key::Quote => QUOTE,
            Key::Comma => COMMA, Key::Dot => DOT,
            Key::Slash => SLASH, Key::BackSlash => BACK_SLASH,
            Key::BackQuote => BACK_QUOTE, Key::IntlBackslash => INTL_BACKSLASH,

            Key::Alt => ALT, Key::AltGr => ALT_GR,
            Key::ControlLeft => CONTROL_LEFT, Key::ControlRight => CONTROL_RIGHT,
            Key::ShiftLeft => SHIFT_LEFT, Key::ShiftRight => SHIFT_RIGHT,
            Key::MetaLeft => META_LEFT, Key::MetaRight => META_RIGHT,
            Key::CapsLock => CAPS_LOCK, Key::Function => FUNCTION,

            Key::Return => RETURN, Key::Tab => TAB,
            Key::Space => SPACE, Key::Backspace => BACKSPACE,
            Key::Escape => ESCAPE, Key::Delete => DELETE_FWD,

            Key::UpArrow => UP, Key::DownArrow => DOWN,
            Key::LeftArrow => LEFT, Key::RightArrow => RIGHT,

            Key::Home => HOME, Key::End => END,
            Key::PageUp => PAGE_UP, Key::PageDown => PAGE_DOWN,

            Key::F1 => F1, Key::F2 => F2, Key::F3 => F3, Key::F4 => F4,
            Key::F5 => F5, Key::F6 => F6, Key::F7 => F7, Key::F8 => F8,
            Key::F9 => F9, Key::F10 => F10, Key::F11 => F11, Key::F12 => F12,

            Key::Kp0 => KP0, Key::Kp1 => KP1, Key::Kp2 => KP2,
            Key::Kp3 => KP3, Key::Kp4 => KP4, Key::Kp5 => KP5,
            Key::Kp6 => KP6, Key::Kp7 => KP7, Key::Kp8 => KP8,
            Key::Kp9 => KP9,
            Key::KpReturn => KP_RETURN, Key::KpMinus => KP_MINUS,
            Key::KpPlus => KP_PLUS, Key::KpMultiply => KP_MULTIPLY,
            Key::KpDivide => KP_DIVIDE, Key::KpDelete => KP_DELETE,

            Key::Insert => INSERT, Key::NumLock => NUM_LOCK,
            Key::PrintScreen => F13,   // F13 = Print Screen on Mac
            Key::ScrollLock => F14,    // F14 = Scroll Lock on Mac
            Key::Pause => F15,         // F15 = Pause on Mac

            Key::Unknown(raw) => {
                if raw <= u16::MAX as u32 {
                    raw as u16
                } else {
                    return None;
                }
            }
        };

        Some(code)
    }

    pub fn simulate_key(key: Key, press: bool) -> Result<(), String> {
        let code = keycode_from_key(key)
            .ok_or_else(|| format!("No macOS keycode for key {:?}", key))?;

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "Failed to create CGEventSource — check Accessibility permissions".to_string())?;

        let cg_event = CGEvent::new_keyboard_event(source, code, press)
            .map_err(|_| format!("Failed to create keyboard event for key {:?}", key))?;

        cg_event.post(CGEventTapLocation::HID);
        Ok(())
    }
}

// ── Public entry points ─────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
pub fn listen<T>(callback: T) -> Result<(), String>
where
    T: FnMut(Event) + 'static,
{
    macos::listen(callback)
}

#[cfg(not(target_os = "macos"))]
pub fn listen<T>(callback: T) -> Result<(), String>
where
    T: FnMut(Event) + 'static,
{
    rdev::listen(callback).map_err(|e| format!("{:?}", e))
}

#[cfg(target_os = "macos")]
pub fn simulate_key(key: Key, press: bool) -> Result<(), String> {
    macos::simulate_key(key, press)
}

#[cfg(not(target_os = "macos"))]
pub fn simulate_key(key: Key, press: bool) -> Result<(), String> {
    let event_type = if press {
        EventType::KeyPress(key)
    } else {
        EventType::KeyRelease(key)
    };
    rdev::simulate(&event_type).map_err(|e| format!("{:?}", e))
}
