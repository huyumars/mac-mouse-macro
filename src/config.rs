use crate::keys::key_from_str;
use rdev::{Button, Key};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub mouse_button: String,
    #[serde(default)]
    pub keys: Vec<KeyConfig>,
    pub interval_ms: Option<u64>,
    pub press_duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum KeyConfig {
    Simple(String),
    Recorded {
        key: String,
        action: KeyAction,
        wait_ms: u64,
    },
    Detailed {
        key: String,
        press_ms: Option<u64>,
        delay_ms: Option<u64>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum KeyAction {
    Down,
    Up,
}

#[derive(Debug, Clone)]
pub enum Action {
    Wait(u64),
    Key(Key, bool),
}

#[derive(Debug, Clone)]
pub struct ResolvedBinding {
    pub button: Button,
    pub actions: Vec<Action>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub mouse_button: Option<String>,
    pub keys: Option<Vec<KeyConfig>>,
    #[serde(default = "default_interval")]
    pub interval_ms: u64,
    #[serde(default = "default_press")]
    pub press_duration_ms: u64,
    #[serde(default)]
    pub bindings: Vec<Binding>,
}
fn default_interval() -> u64 {
    100
}
fn default_press() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mouse_button: None,
            keys: None,
            interval_ms: 100,
            press_duration_ms: 30,
            bindings: vec![Binding {
                name: None,
                mouse_button: "Left".into(),
                keys: vec![KeyConfig::Simple("a".into())],
                interval_ms: None,
                press_duration_ms: None,
            }],
        }
    }
}

pub fn config_path() -> PathBuf {
    PathBuf::from(env::var_os("HOME").unwrap_or_else(|| ".".into()))
        .join(".config/mouse-macro/config.toml")
}

pub fn parse_button(raw: &str) -> Result<Button, String> {
    match raw.trim().to_lowercase().as_str() {
        "left" | "0" => Ok(Button::Left),
        "right" | "1" => Ok(Button::Right),
        "middle" | "2" => Ok(Button::Middle),
        n => n
            .parse::<u8>()
            .map(Button::Unknown)
            .map_err(|_| format!("Invalid mouse button: {raw}")),
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("Cannot read {}: {e}", path.display())),
        }
    }
    pub fn parse(text: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        config.resolve()?;
        Ok(config)
    }
    pub fn normalized_bindings(&self) -> Vec<Binding> {
        let mut bindings = self.bindings.clone();
        if self.mouse_button.is_some() || self.keys.is_some() {
            bindings.insert(
                0,
                Binding {
                    name: None,
                    mouse_button: self.mouse_button.clone().unwrap_or_else(|| "Left".into()),
                    keys: self
                        .keys
                        .clone()
                        .unwrap_or_else(|| vec![KeyConfig::Simple("a".into())]),
                    interval_ms: None,
                    press_duration_ms: None,
                },
            );
        }
        bindings
    }
    pub fn resolve(&self) -> Result<Vec<ResolvedBinding>, String> {
        let bindings = self.normalized_bindings();
        let timing = |n: u64| {
            if n <= 600_000 {
                Ok(n)
            } else {
                Err("Timing must be between 0 and 600000 ms".to_string())
            }
        };
        timing(self.interval_ms)?;
        timing(self.press_duration_ms)?;
        let mut result: Vec<ResolvedBinding> = Vec::new();
        for binding in bindings {
            let button = parse_button(&binding.mouse_button)?;
            if result.iter().any(|b| b.button == button) {
                return Err(format!("Duplicate mouse binding: {}", binding.mouse_button));
            }
            if binding.keys.is_empty() {
                return Err(format!("Binding {} has no keys", binding.mouse_button));
            }
            let press = timing(binding.press_duration_ms.unwrap_or(self.press_duration_ms))?;
            let gap = timing(binding.interval_ms.unwrap_or(self.interval_ms))?;
            let mut actions = Vec::new();
            let mut held = Vec::new();
            let recorded = binding
                .keys
                .iter()
                .any(|k| matches!(k, KeyConfig::Recorded { .. }));
            for step in binding.keys {
                let (name, down, up) = match step {
                    KeyConfig::Recorded {
                        key,
                        action,
                        wait_ms,
                    } => {
                        let k = key_from_str(&key).ok_or_else(|| format!("Unknown key: {key}"))?;
                        let is_down = matches!(action, KeyAction::Down);
                        if is_down {
                            if held.contains(&k) {
                                return Err(format!("Repeated key down: {key}"));
                            }
                            held.push(k);
                        } else {
                            let index = held
                                .iter()
                                .position(|v| *v == k)
                                .ok_or_else(|| format!("Key up without down: {key}"))?;
                            held.remove(index);
                        }
                        actions.push(Action::Wait(timing(wait_ms)?));
                        actions.push(Action::Key(k, is_down));
                        continue;
                    }
                    KeyConfig::Simple(key) => (key, press, gap),
                    KeyConfig::Detailed {
                        key,
                        press_ms,
                        delay_ms,
                    } => (
                        key,
                        timing(press_ms.unwrap_or(press))?,
                        timing(delay_ms.unwrap_or(gap))?,
                    ),
                };
                let key = key_from_str(&name).ok_or_else(|| format!("Unknown key: {name}"))?;
                if held.contains(&key) {
                    return Err(format!("Key already held: {name}"));
                }
                actions.extend([
                    Action::Key(key, true),
                    Action::Wait(down),
                    Action::Key(key, false),
                    Action::Wait(up),
                ]);
            }
            if !held.is_empty() {
                return Err("Recording contains keys without a release".into());
            }
            // Yield between cycles even when all configured timings are zero.
            if recorded {
                actions.push(Action::Wait(gap.max(1)));
            } else if gap == 0 {
                actions.push(Action::Wait(1));
            }
            result.push(ResolvedBinding { button, actions });
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_and_timing() {
        let c = Config::parse("mouse_button='Left'\nkeys=[{key='a',press_ms=7}]\npress_duration_ms=50\ninterval_ms=12").unwrap();
        assert!(matches!(
            c.resolve().unwrap()[0].actions[1],
            Action::Wait(7)
        ));
        assert!(Config::parse("keys=['a']").unwrap().resolve().unwrap()[0].button == Button::Left);
    }
    #[test]
    fn invalid_configs_are_rejected() {
        for text in ["keys=['typo']", "keys=[]", "interval_ms=600001", "interval_mss=3", "[[bindings]]\nmouse_button='0'\nkeys=['a']\n[[bindings]]\nmouse_button='Left'\nkeys=['b']", "keys=[{key='a',action='up',wait_ms=0}]", "keys=[{key='a',action='down',wait_ms=0}]"] {
            assert!(Config::parse(text).is_err(), "{text}");
        }
    }
    #[test]
    fn invalid_file_is_not_overwritten() {
        let p = std::env::temp_dir().join(format!("mouse-macro-test-{}.toml", std::process::id()));
        fs::write(&p, "broken = [").unwrap();
        assert!(Config::load(&p).is_err());
        assert_eq!(fs::read_to_string(&p).unwrap(), "broken = [");
        fs::remove_file(p).unwrap();
    }
    #[test]
    fn recorded_chord() {
        let c = Config::parse("keys=[{key='ctrl',action='down',wait_ms=0},{key='a',action='down',wait_ms=10},{key='a',action='up',wait_ms=20},{key='ctrl',action='up',wait_ms=0}]").unwrap();
        assert_eq!(c.resolve().unwrap()[0].actions.len(), 9);
    }
}
