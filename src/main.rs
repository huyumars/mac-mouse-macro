mod platform;

use rdev::{Button, Event, EventType, Key};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

// ── Config ──────────────────────────────────────────────────────────────────

/// A single key in the macro sequence.
///
/// Supports two formats in TOML:
///
/// **Simple** (just the key name, uses global defaults for timing):
/// ```toml
/// keys = ["a", "b", "c"]
/// ```
///
/// **Detailed** (per-key timing, overrides global defaults):
/// ```toml
/// keys = [
///   { key = "1", delay_ms = 20 },
///   { key = "2", delay_ms = 100 },
///   { key = "4" },
/// ]
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum KeyConfig {
    Simple(String),
    Detailed {
        key: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        press_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay_ms: Option<u64>,
    },
}

/// A fully-resolved key step ready for playback.
#[derive(Debug, Clone)]
struct KeyStep {
    key: Key,
    press_ms: u64,  // how long to hold the key down
    delay_ms: u64,  // delay after release, before the next key
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Config {
    #[serde(default = "default_mouse_button")]
    mouse_button: String,

    /// Key sequence.  Can be simple strings or detailed tables with per-key timing.
    #[serde(default = "default_keys")]
    keys: Vec<KeyConfig>,

    /// Global default: delay after releasing each key before pressing the next (ms).
    #[serde(default = "default_interval")]
    interval_ms: u64,

    /// Global default: how long each key is held down (ms).
    #[serde(default = "default_press_duration")]
    press_duration_ms: u64,
}

fn default_mouse_button() -> String { "Left".to_string() }
fn default_interval() -> u64 { 100 }
fn default_press_duration() -> u64 { 30 }

fn default_keys() -> Vec<KeyConfig> {
    vec![KeyConfig::Simple("a".to_string())]
}

impl Config {
    fn load() -> Self {
        let config_dir = config_dir();
        let config_path = config_dir.join("config.toml");

        if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(config) => {
                        println!("Loaded config from {:?}", config_path);
                        return config;
                    }
                    Err(e) => eprintln!(
                        "Failed to parse config at {:?}: {}. Using defaults.",
                        config_path, e
                    ),
                },
                Err(e) => eprintln!(
                    "Failed to read config at {:?}: {}. Using defaults.",
                    config_path, e
                ),
            }
        }

        let default = Config {
            mouse_button: default_mouse_button(),
            keys: default_keys(),
            interval_ms: default_interval(),
            press_duration_ms: default_press_duration(),
        };

        if let Err(e) = fs::create_dir_all(&config_dir) {
            eprintln!("Failed to create config dir {:?}: {}", config_dir, e);
        }
        match toml::to_string_pretty(&default) {
            Ok(content) => {
                if let Err(e) = fs::write(&config_path, &content) {
                    eprintln!(
                        "Failed to write default config to {:?}: {}",
                        config_path, e
                    );
                } else {
                    println!("Created default config at {:?}", config_path);
                }
            }
            Err(e) => eprintln!("Failed to serialize default config: {}", e),
        }

        default
    }

    fn mouse_button(&self) -> Option<Button> {
        match self.mouse_button.to_lowercase().as_str() {
            "left" => return Some(Button::Left),
            "right" => return Some(Button::Right),
            "middle" => return Some(Button::Middle),
            other => {
                if let Ok(id) = other.parse::<u8>() {
                    return Some(Button::Unknown(id));
                }
            }
        }

        eprintln!("Unknown mouse button: '{}'", self.mouse_button);
        eprintln!("Valid options: Left, Right, Middle, or a numeric ID (e.g. 4, 5)");
        eprintln!("Run with --identify to discover your mouse button IDs.");
        None
    }

    /// Resolve the key configs into `KeyStep`s, applying global defaults
    /// where per-key values are not specified.
    fn resolve_keys(&self) -> Vec<KeyStep> {
        self.keys
            .iter()
            .filter_map(|kc| {
                let (key_name, press_ms, delay_ms) = match kc {
                    KeyConfig::Simple(name) => {
                        (name.clone(), self.press_duration_ms, self.interval_ms)
                    }
                    KeyConfig::Detailed { key, press_ms, delay_ms } => {
                        (key.clone(),
                         press_ms.unwrap_or(self.press_duration_ms),
                         delay_ms.unwrap_or(self.interval_ms))
                    }
                };

                let key = key_from_str(&key_name).or_else(|| {
                    eprintln!("Warning: unknown key '{}' — skipping", key_name);
                    None
                })?;

                Some(KeyStep { key, press_ms, delay_ms })
            })
            .collect()
    }
}

fn config_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("mouse-macro")
}

// ── Key mapping ─────────────────────────────────────────────────────────────

fn key_from_str(s: &str) -> Option<Key> {
    let lower = s.to_lowercase();
    let lower = lower.as_str();

    if lower.len() == 1 {
        let ch = lower.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return match ch {
                'a' => Some(Key::KeyA), 'b' => Some(Key::KeyB), 'c' => Some(Key::KeyC),
                'd' => Some(Key::KeyD), 'e' => Some(Key::KeyE), 'f' => Some(Key::KeyF),
                'g' => Some(Key::KeyG), 'h' => Some(Key::KeyH), 'i' => Some(Key::KeyI),
                'j' => Some(Key::KeyJ), 'k' => Some(Key::KeyK), 'l' => Some(Key::KeyL),
                'm' => Some(Key::KeyM), 'n' => Some(Key::KeyN), 'o' => Some(Key::KeyO),
                'p' => Some(Key::KeyP), 'q' => Some(Key::KeyQ), 'r' => Some(Key::KeyR),
                's' => Some(Key::KeyS), 't' => Some(Key::KeyT), 'u' => Some(Key::KeyU),
                'v' => Some(Key::KeyV), 'w' => Some(Key::KeyW), 'x' => Some(Key::KeyX),
                'y' => Some(Key::KeyY), 'z' => Some(Key::KeyZ),
                _ => None,
            };
        }
        if ch.is_ascii_digit() {
            return match ch {
                '0' => Some(Key::Num0), '1' => Some(Key::Num1), '2' => Some(Key::Num2),
                '3' => Some(Key::Num3), '4' => Some(Key::Num4), '5' => Some(Key::Num5),
                '6' => Some(Key::Num6), '7' => Some(Key::Num7), '8' => Some(Key::Num8),
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
        "f1" => Some(Key::F1), "f2" => Some(Key::F2), "f3" => Some(Key::F3),
        "f4" => Some(Key::F4), "f5" => Some(Key::F5), "f6" => Some(Key::F6),
        "f7" => Some(Key::F7), "f8" => Some(Key::F8), "f9" => Some(Key::F9),
        "f10" => Some(Key::F10), "f11" => Some(Key::F11), "f12" => Some(Key::F12),

        // Numpad
        "kp0" => Some(Key::Kp0), "kp1" => Some(Key::Kp1), "kp2" => Some(Key::Kp2),
        "kp3" => Some(Key::Kp3), "kp4" => Some(Key::Kp4), "kp5" => Some(Key::Kp5),
        "kp6" => Some(Key::Kp6), "kp7" => Some(Key::Kp7), "kp8" => Some(Key::Kp8),
        "kp9" => Some(Key::Kp9),
        "kpreturn" | "kpenter" => Some(Key::KpReturn),
        "kpminus" | "kp-" => Some(Key::KpMinus),
        "kpplus" | "kp+" => Some(Key::KpPlus),
        "kpmultiply" | "kp*" => Some(Key::KpMultiply),
        "kpdivide" | "kp/" => Some(Key::KpDivide),
        "kpdelete" | "kpdel" | "kp." => Some(Key::KpDelete),

        // Right-side modifiers
        "rightcontrol" | "rctrl" => Some(Key::ControlRight),
        "rightshift" | "rshift" => Some(Key::ShiftRight),
        "rightmeta" | "rmeta" | "rightcommand" | "rcmd" | "rightsuper" => Some(Key::MetaRight),
        "altgr" => Some(Key::AltGr),

        // Misc
        "insert" | "ins" => Some(Key::Insert),
        "printscreen" | "prtsc" => Some(Key::PrintScreen),
        "scrolllock" => Some(Key::ScrollLock),
        "pause" | "break" => Some(Key::Pause),
        "numlock" | "numlk" => Some(Key::NumLock),
        "backquote" | "`" => Some(Key::BackQuote),

        _ => None,
    }
}

// ── Identify mode ───────────────────────────────────────────────────────────

fn run_identify_mode() {
    println!("╔══════════════════════════════════════════╗");
    println!("║     Mouse Button Identification Mode     ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  Press any mouse button to see its ID.   ║");
    println!("║  Press Ctrl+C to exit.                  ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!("Example config entries:");
    println!("  mouse_button = \"Left\"     ← standard left button");
    println!("  mouse_button = \"Right\"    ← standard right button");
    println!("  mouse_button = \"Middle\"   ← standard middle button");
    println!("  mouse_button = \"4\"        ← side button (use the number shown below)");
    println!();
    println!("── Listening for mouse events ──");
    println!();

    let callback = |event: Event| match event.event_type {
        EventType::ButtonPress(button) => {
            let id_str = button_id_string(&button);
            println!(
                "  PRESS  │  {:?}  →  put this in your config: mouse_button = \"{}\"",
                button, id_str
            );
        }
        EventType::ButtonRelease(button) => {
            let id_str = button_id_string(&button);
            println!(
                "  RELEASE│  {:?}  →  config value: \"{}\"",
                button, id_str
            );
        }
        _ => {}
    };

    if let Err(error) = platform::listen(callback) {
        eprintln!();
        eprintln!("ERROR: {}", error);
        process::exit(1);
    }
}

fn button_id_string(button: &Button) -> String {
    match button {
        Button::Left => "Left".to_string(),
        Button::Right => "Right".to_string(),
        Button::Middle => "Middle".to_string(),
        Button::Unknown(id) => id.to_string(),
    }
}

// ── Macro mode ──────────────────────────────────────────────────────────────

fn run_macro_mode() {
    let config = Config::load();

    let mouse_button = match config.mouse_button() {
        Some(b) => b,
        None => process::exit(1),
    };

    let steps: Vec<KeyStep> = config.resolve_keys();

    if steps.is_empty() {
        eprintln!("No valid keys configured. Edit your config file to add keys.");
        eprintln!("Config dir: {:?}", config_dir());
        process::exit(1);
    }

    // Build a display string for the key sequence with timing info
    let keys_display: Vec<String> = steps
        .iter()
        .map(|s| format!("{:?}(hold {}ms, gap {}ms)", s.key, s.press_ms, s.delay_ms))
        .collect();

    println!("╔══════════════════════════════════════════╗");
    println!("║         Mouse Macro v0.2.0              ║");
    println!("╠══════════════════════════════════════════╣");
    println!(
        "║  Mouse button : {:<23} ║",
        format!("{:?}", mouse_button)
    );
    for (i, kd) in keys_display.iter().enumerate() {
        println!("║  Key {}        : {:<23} ║", i + 1, kd);
    }
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!(
        "Hold the {:?} mouse button to trigger the macro.",
        mouse_button
    );
    println!("Press Ctrl+C to exit.");
    println!();
    println!("NOTE: On macOS you must grant Accessibility permission to");
    println!("your terminal in System Settings → Privacy & Security → Accessibility.");

    let is_running = Arc::new(AtomicBool::new(false));

    let callback = {
        let is_running = Arc::clone(&is_running);
        let steps = steps.clone();

        move |event: Event| {
            match event.event_type {
                EventType::ButtonPress(button) if button == mouse_button => {
                    if !is_running.load(Ordering::SeqCst) {
                        is_running.store(true, Ordering::SeqCst);
                        let running_flag = Arc::clone(&is_running);
                        let steps = steps.clone();

                        thread::spawn(move || {
                            println!("▶ Macro started");
                            while running_flag.load(Ordering::SeqCst) {
                                for step in &steps {
                                    if !running_flag.load(Ordering::SeqCst) {
                                        break;
                                    }
                                    // Press
                                    if let Err(e) = platform::simulate_key(step.key, true) {
                                        eprintln!("⚠  Key press error for {:?}: {}", step.key, e);
                                    }
                                    thread::sleep(Duration::from_millis(step.press_ms));
                                    // Release
                                    if let Err(e) = platform::simulate_key(step.key, false) {
                                        eprintln!("⚠  Key release error for {:?}: {}", step.key, e);
                                    }
                                    thread::sleep(Duration::from_millis(step.delay_ms));
                                }
                            }
                            println!("■ Macro stopped");
                        });
                    }
                }
                EventType::ButtonRelease(button) if button == mouse_button => {
                    is_running.store(false, Ordering::SeqCst);
                }
                _ => {}
            }
        }
    };

    if let Err(error) = platform::listen(callback) {
        eprintln!();
        eprintln!("ERROR: {}", error);
        process::exit(1);
    }
}

// ── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--identify" || a == "-i") {
        run_identify_mode();
    } else {
        run_macro_mode();
    }
}
