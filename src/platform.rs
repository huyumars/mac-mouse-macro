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
    use super::*;
    use core_graphics::event::{CGEvent, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use std::os::raw::c_void;

    type Ref = *const c_void;
    type Callback = unsafe extern "C" fn(Ref, u32, Ref, *mut c_void) -> Ref;
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            mask: u64,
            callback: Callback,
            info: *mut c_void,
        ) -> Ref;
        fn CGEventTapEnable(tap: Ref, enable: bool);
        fn CGPreflightPostEventAccess() -> bool;
        fn CGPreflightListenEventAccess() -> bool;
        fn CGRequestPostEventAccess() -> bool;
        fn CGRequestListenEventAccess() -> bool;
        fn CGEventGetIntegerValueField(event: Ref, field: u32) -> i64;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFMachPortCreateRunLoopSource(allocator: Ref, tap: Ref, order: isize) -> Ref;
        fn CFRunLoopAddSource(rl: Ref, source: Ref, mode: Ref);
        fn CFRunLoopRemoveSource(rl: Ref, source: Ref, mode: Ref);
        fn CFRunLoopGetCurrent() -> Ref;
        fn CFRunLoopRun();
        fn CFRelease(value: Ref);
        static kCFRunLoopCommonModes: Ref;
    }
    struct Context {
        callback: Box<dyn FnMut(Event)>,
        tap: Ref,
    }
    unsafe extern "C" fn raw_callback(_: Ref, kind: u32, event: Ref, info: *mut c_void) -> Ref {
        // CGEventTap passes a borrowed event; never wrap it as an owned CGEvent.
        let context = &mut *(info as *mut Context);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if kind == u32::MAX || kind == u32::MAX - 1 {
                // A disabled tap may have missed mouse-up events. Cancel every binding.
                for id in 0..=255 {
                    (context.callback)(Event {
                        event_type: EventType::ButtonRelease(button(id)),
                        time: SystemTime::now(),
                        name: None,
                    });
                }
                CGEventTapEnable(context.tap, true);
                return;
            }
            let (button, down) = match kind {
                1 => (Button::Left, true),
                2 => (Button::Left, false),
                3 => (Button::Right, true),
                4 => (Button::Right, false),
                25 | 26 if !event.is_null() => {
                    let id = CGEventGetIntegerValueField(event, 3);
                    if !(0..=255).contains(&id) {
                        return;
                    }
                    (button(id as u8), kind == 25)
                }
                _ => return,
            };
            (context.callback)(Event {
                event_type: if down {
                    EventType::ButtonPress(button)
                } else {
                    EventType::ButtonRelease(button)
                },
                time: SystemTime::now(),
                name: None,
            });
        }));
        if result.is_err() {
            crate::STOP.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        event
    }
    fn button(id: u8) -> Button {
        match id {
            0 => Button::Left,
            1 => Button::Right,
            2 => Button::Middle,
            n => Button::Unknown(n),
        }
    }
    pub fn permissions(request: Option<&str>) -> super::Permissions {
        unsafe {
            match request {
                Some("keyboard") if !CGPreflightPostEventAccess() => {
                    CGRequestPostEventAccess();
                }
                Some("input") if !CGPreflightListenEventAccess() => {
                    CGRequestListenEventAccess();
                }
                _ => {}
            }
            super::Permissions {
                keyboard: CGPreflightPostEventAccess(),
                input_monitoring: CGPreflightListenEventAccess(),
            }
        }
    }
    pub fn check_playback_permission() -> Result<(), String> {
        if unsafe { CGPreflightPostEventAccess() } {
            Ok(())
        } else {
            Err("Accessibility permission is required to post keyboard events. Enable it for Mouse Macro (or your terminal), then restart.".into())
        }
    }
    pub fn listen<T: FnMut(Event) + 'static>(callback: T) -> Result<(), String> {
        let mut context = Box::new(Context {
            callback: Box::new(callback),
            tap: std::ptr::null(),
        });
        unsafe {
            let mask = [1, 2, 3, 4, 25, 26]
                .iter()
                .fold(0u64, |mask, n| mask | (1 << n));
            let tap = CGEventTapCreate(
                0,
                0,
                1,
                mask,
                raw_callback,
                (&mut *context as *mut Context).cast(),
            );
            if tap.is_null() {
                return Err("Cannot listen to mouse events. Enable Accessibility and Input Monitoring for Mouse Macro (or your terminal) in System Settings → Privacy & Security, then restart.".into());
            }
            context.tap = tap;
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                CFRelease(tap);
                return Err("Cannot create event run loop source".into());
            }
            let run_loop = CFRunLoopGetCurrent();
            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            CFRunLoopRun();
            CGEventTapEnable(tap, false);
            CFRunLoopRemoveSource(run_loop, source, kCFRunLoopCommonModes);
            CFRelease(source);
            CFRelease(tap);
        }
        Ok(())
    }

    const KEYCODES: &[(Key, u16)] = &[
        (Key::KeyA, 0),
        (Key::KeyB, 11),
        (Key::KeyC, 8),
        (Key::KeyD, 2),
        (Key::KeyE, 14),
        (Key::KeyF, 3),
        (Key::KeyG, 5),
        (Key::KeyH, 4),
        (Key::KeyI, 34),
        (Key::KeyJ, 38),
        (Key::KeyK, 40),
        (Key::KeyL, 37),
        (Key::KeyM, 46),
        (Key::KeyN, 45),
        (Key::KeyO, 31),
        (Key::KeyP, 35),
        (Key::KeyQ, 12),
        (Key::KeyR, 15),
        (Key::KeyS, 1),
        (Key::KeyT, 17),
        (Key::KeyU, 32),
        (Key::KeyV, 9),
        (Key::KeyW, 13),
        (Key::KeyX, 7),
        (Key::KeyY, 16),
        (Key::KeyZ, 6),
        (Key::Num0, 29),
        (Key::Num1, 18),
        (Key::Num2, 19),
        (Key::Num3, 20),
        (Key::Num4, 21),
        (Key::Num5, 23),
        (Key::Num6, 22),
        (Key::Num7, 26),
        (Key::Num8, 28),
        (Key::Num9, 25),
        (Key::Minus, 27),
        (Key::Equal, 24),
        (Key::LeftBracket, 33),
        (Key::RightBracket, 30),
        (Key::SemiColon, 41),
        (Key::Quote, 39),
        (Key::Comma, 43),
        (Key::Dot, 47),
        (Key::Slash, 44),
        (Key::BackSlash, 42),
        (Key::BackQuote, 50),
        (Key::IntlBackslash, 10),
        (Key::Alt, 58),
        (Key::AltGr, 61),
        (Key::ControlLeft, 59),
        (Key::ControlRight, 62),
        (Key::ShiftLeft, 56),
        (Key::ShiftRight, 60),
        (Key::MetaLeft, 55),
        (Key::MetaRight, 54),
        (Key::CapsLock, 57),
        (Key::Function, 63),
        (Key::Return, 36),
        (Key::Tab, 48),
        (Key::Space, 49),
        (Key::Backspace, 51),
        (Key::Escape, 53),
        (Key::Delete, 117),
        (Key::UpArrow, 126),
        (Key::DownArrow, 125),
        (Key::LeftArrow, 123),
        (Key::RightArrow, 124),
        (Key::Home, 115),
        (Key::End, 119),
        (Key::PageUp, 116),
        (Key::PageDown, 121),
        (Key::F1, 122),
        (Key::F2, 120),
        (Key::F3, 99),
        (Key::F4, 118),
        (Key::F5, 96),
        (Key::F6, 97),
        (Key::F7, 98),
        (Key::F8, 100),
        (Key::F9, 101),
        (Key::F10, 109),
        (Key::F11, 103),
        (Key::F12, 111),
        (Key::Kp0, 82),
        (Key::Kp1, 83),
        (Key::Kp2, 84),
        (Key::Kp3, 85),
        (Key::Kp4, 86),
        (Key::Kp5, 87),
        (Key::Kp6, 88),
        (Key::Kp7, 89),
        (Key::Kp8, 91),
        (Key::Kp9, 92),
        (Key::KpReturn, 76),
        (Key::KpMinus, 78),
        (Key::KpPlus, 69),
        (Key::KpMultiply, 67),
        (Key::KpDivide, 75),
        (Key::KpDelete, 65),
        (Key::Insert, 114),
        (Key::NumLock, 71),
        (Key::PrintScreen, 105),
        (Key::ScrollLock, 107),
        (Key::Pause, 113),
    ];
    fn keycode_from_key(key: Key) -> Option<u16> {
        if let Key::Unknown(raw) = key {
            return u16::try_from(raw).ok();
        }
        KEYCODES
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, code)| *code)
    }
    pub fn key_from_code(code: u16) -> Key {
        KEYCODES
            .iter()
            .find(|(_, c)| *c == code)
            .map_or(Key::Unknown(code as u32), |(key, _)| *key)
    }
    thread_local! { static MODIFIERS: std::cell::RefCell<Vec<u16>> = const { std::cell::RefCell::new(Vec::new()) }; }
    fn modifier_flags(codes: &[u16]) -> core_graphics::event::CGEventFlags {
        use core_graphics::event::CGEventFlags as F;
        codes.iter().fold(F::empty(), |flags, code| {
            flags
                | match code {
                    54 | 55 => F::CGEventFlagCommand,
                    56 | 60 => F::CGEventFlagShift,
                    58 | 61 => F::CGEventFlagAlternate,
                    59 | 62 => F::CGEventFlagControl,
                    63 => F::CGEventFlagSecondaryFn,
                    _ => F::empty(),
                }
        })
    }
    pub fn simulate_key(key: Key, press: bool) -> Result<(), String> {
        let code =
            keycode_from_key(key).ok_or_else(|| format!("No macOS keycode for key {:?}", key))?;

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|_| {
            "Failed to create CGEventSource — check Accessibility permissions".to_string()
        })?;

        let cg_event = CGEvent::new_keyboard_event(source, code, press)
            .map_err(|_| format!("Failed to create keyboard event for key {:?}", key))?;

        MODIFIERS.with(|state| {
            let mut codes = state.borrow_mut();
            let old = modifier_flags(&codes);
            if matches!(code, 54..=56 | 58..=63) {
                codes.retain(|c| *c != code);
                if press {
                    codes.push(code);
                }
            }
            cg_event.set_flags((cg_event.get_flags() & !old) | modifier_flags(&codes));
        });
        cg_event.post(CGEventTapLocation::HID);
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn keycode_roundtrip() {
            for (key, code) in KEYCODES {
                assert_eq!(key_from_code(*code), *key);
                assert_eq!(keycode_from_key(*key), Some(*code));
            }
        }
        #[test]
        fn right_modifier_remains_held() {
            assert_eq!(modifier_flags(&[56, 60]), modifier_flags(&[60]));
            assert_ne!(modifier_flags(&[60]), modifier_flags(&[]));
        }
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

#[cfg(target_os = "macos")]
pub fn key_from_code(code: u16) -> Key {
    macos::key_from_code(code)
}

pub fn check_playback_permission() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::check_playback_permission()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

#[derive(serde::Serialize)]
pub struct Permissions {
    pub keyboard: bool,
    pub input_monitoring: bool,
}

pub fn permissions(request: Option<&str>) -> Permissions {
    #[cfg(target_os = "macos")]
    {
        macos::permissions(request)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = request;
        Permissions {
            keyboard: true,
            input_monitoring: true,
        }
    }
}
