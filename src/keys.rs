use rdev::Key;

pub fn key_from_str(s: &str) -> Option<Key> {
    let lower = s.trim().to_lowercase();
    #[cfg(target_os = "macos")]
    if let Some(code) = lower.strip_prefix("keycode:") {
        return code.parse::<u16>().ok().map(crate::platform::key_from_code);
    }
    let lower = lower.as_str();

    if lower.len() == 1 {
        let ch = lower.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return match ch {
                'a' => Some(Key::KeyA),
                'b' => Some(Key::KeyB),
                'c' => Some(Key::KeyC),
                'd' => Some(Key::KeyD),
                'e' => Some(Key::KeyE),
                'f' => Some(Key::KeyF),
                'g' => Some(Key::KeyG),
                'h' => Some(Key::KeyH),
                'i' => Some(Key::KeyI),
                'j' => Some(Key::KeyJ),
                'k' => Some(Key::KeyK),
                'l' => Some(Key::KeyL),
                'm' => Some(Key::KeyM),
                'n' => Some(Key::KeyN),
                'o' => Some(Key::KeyO),
                'p' => Some(Key::KeyP),
                'q' => Some(Key::KeyQ),
                'r' => Some(Key::KeyR),
                's' => Some(Key::KeyS),
                't' => Some(Key::KeyT),
                'u' => Some(Key::KeyU),
                'v' => Some(Key::KeyV),
                'w' => Some(Key::KeyW),
                'x' => Some(Key::KeyX),
                'y' => Some(Key::KeyY),
                'z' => Some(Key::KeyZ),
                _ => None,
            };
        }
        if ch.is_ascii_digit() {
            return match ch {
                '0' => Some(Key::Num0),
                '1' => Some(Key::Num1),
                '2' => Some(Key::Num2),
                '3' => Some(Key::Num3),
                '4' => Some(Key::Num4),
                '5' => Some(Key::Num5),
                '6' => Some(Key::Num6),
                '7' => Some(Key::Num7),
                '8' => Some(Key::Num8),
                '9' => Some(Key::Num9),
                _ => None,
            };
        }
    }

    match lower {
        "return" | "enter" => Some(Key::Return),
        "space" => Some(Key::Space),
        "tab" => Some(Key::Tab),
        "escape" | "esc" => Some(Key::Escape),
        "backspace" => Some(Key::Backspace),
        "delete" | "del" => Some(Key::Delete),
        "up" => Some(Key::UpArrow),
        "down" => Some(Key::DownArrow),
        "left" => Some(Key::LeftArrow),
        "right" => Some(Key::RightArrow),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" | "pgup" => Some(Key::PageUp),
        "pagedown" | "pgdn" => Some(Key::PageDown),
        "control" | "ctrl" => Some(Key::ControlLeft),
        "shift" => Some(Key::ShiftLeft),
        "alt" | "option" => Some(Key::Alt),
        "meta" | "command" | "cmd" | "super" | "windows" => Some(Key::MetaLeft),
        "capslock" | "caps" => Some(Key::CapsLock),
        "minus" | "-" => Some(Key::Minus),
        "equal" | "=" => Some(Key::Equal),
        "leftbracket" | "[" => Some(Key::LeftBracket),
        "rightbracket" | "]" => Some(Key::RightBracket),
        "semicolon" | ";" => Some(Key::SemiColon),
        "quote" | "'" => Some(Key::Quote),
        "comma" | "," => Some(Key::Comma),
        "period" | "dot" | "." => Some(Key::Dot),
        "slash" | "/" => Some(Key::Slash),
        "backslash" | "\\" => Some(Key::BackSlash),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),

        "kp0" => Some(Key::Kp0),
        "kp1" => Some(Key::Kp1),
        "kp2" => Some(Key::Kp2),
        "kp3" => Some(Key::Kp3),
        "kp4" => Some(Key::Kp4),
        "kp5" => Some(Key::Kp5),
        "kp6" => Some(Key::Kp6),
        "kp7" => Some(Key::Kp7),
        "kp8" => Some(Key::Kp8),
        "kp9" => Some(Key::Kp9),
        "kpreturn" | "kpenter" => Some(Key::KpReturn),
        "kpminus" | "kp-" => Some(Key::KpMinus),
        "kpplus" | "kp+" => Some(Key::KpPlus),
        "kpmultiply" | "kp*" => Some(Key::KpMultiply),
        "kpdivide" | "kp/" => Some(Key::KpDivide),
        "kpdelete" | "kpdel" | "kp." => Some(Key::KpDelete),

        "rightcontrol" | "rctrl" => Some(Key::ControlRight),
        "rightshift" | "rshift" => Some(Key::ShiftRight),
        "rightmeta" | "rmeta" | "rightcommand" | "rcmd" | "rightsuper" => Some(Key::MetaRight),
        "altgr" => Some(Key::AltGr),

        "insert" | "ins" => Some(Key::Insert),
        "printscreen" | "prtsc" => Some(Key::PrintScreen),
        "scrolllock" => Some(Key::ScrollLock),
        "pause" | "break" => Some(Key::Pause),
        "numlock" | "numlk" => Some(Key::NumLock),
        "backquote" | "`" => Some(Key::BackQuote),

        _ => None,
    }
}
