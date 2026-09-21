use crate::config::{Action, ResolvedBinding};
use rdev::{Button, Key};
use std::time::{Duration, Instant};

struct Playback {
    binding: ResolvedBinding,
    active: bool,
    cursor: usize,
    due: Instant,
    held: Vec<Key>,
}

/// One scheduler owns all playback state. No detached worker can survive a release.
pub struct Engine {
    playbacks: Vec<Playback>,
}
impl Engine {
    pub fn new(bindings: Vec<ResolvedBinding>, now: Instant) -> Self {
        Self {
            playbacks: bindings
                .into_iter()
                .map(|binding| Playback {
                    binding,
                    active: false,
                    cursor: 0,
                    due: now,
                    held: vec![],
                })
                .collect(),
        }
    }
    pub fn button(
        &mut self,
        button: Button,
        down: bool,
        now: Instant,
        emit: &mut impl FnMut(Key, bool) -> Result<(), String>,
    ) -> Result<(), String> {
        for i in 0..self.playbacks.len() {
            if self.playbacks[i].binding.button != button {
                continue;
            }
            if down && !self.playbacks[i].active {
                self.playbacks[i].active = true;
                self.playbacks[i].cursor = 0;
                self.playbacks[i].due = now;
            } else if !down {
                self.playbacks[i].active = false;
                self.release(i, emit)?;
            }
        }
        Ok(())
    }
    fn release(
        &mut self,
        i: usize,
        emit: &mut impl FnMut(Key, bool) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut first_error = None;
        for key in std::mem::take(&mut self.playbacks[i].held) {
            if !self.playbacks.iter().any(|p| p.held.contains(&key)) {
                if let Err(e) = emit(key, false) {
                    self.playbacks[i].held.push(key);
                    first_error.get_or_insert(e);
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
    pub fn stop(
        &mut self,
        emit: &mut impl FnMut(Key, bool) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut first_error = None;
        for i in 0..self.playbacks.len() {
            self.playbacks[i].active = false;
            if let Err(e) = self.release(i, emit) {
                first_error.get_or_insert(e);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
    pub fn tick(
        &mut self,
        now: Instant,
        emit: &mut impl FnMut(Key, bool) -> Result<(), String>,
    ) -> Result<(), String> {
        for i in 0..self.playbacks.len() {
            // Bound work per tick so long zero-delay recordings cannot starve mouse releases.
            for _ in 0..128 {
                let p = &mut self.playbacks[i];
                if !p.active || p.due > now {
                    break;
                }
                if p.cursor == p.binding.actions.len() {
                    p.cursor = 0;
                }
                let action = p.binding.actions[p.cursor].clone();
                p.cursor += 1;
                match action {
                    Action::Wait(ms) => {
                        p.due = now + Duration::from_millis(ms);
                    }
                    Action::Key(key, down) => {
                        let already_held = self.playbacks.iter().any(|p| p.held.contains(&key));
                        if down {
                            self.playbacks[i].held.push(key);
                            if !already_held {
                                emit(key, true)?;
                            }
                        } else {
                            self.playbacks[i].held.retain(|k| *k != key);
                            if !self.playbacks.iter().any(|p| p.held.contains(&key)) {
                                if let Err(e) = emit(key, false) {
                                    self.playbacks[i].held.push(key);
                                    return Err(e);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding(button: Button) -> ResolvedBinding {
        ResolvedBinding {
            button,
            actions: vec![
                Action::Key(Key::KeyA, true),
                Action::Wait(500),
                Action::Key(Key::KeyA, false),
                Action::Wait(100),
            ],
        }
    }
    #[test]
    fn release_and_repress_cancels_old_cycle() {
        let now = Instant::now();
        let mut e = Engine::new(vec![binding(Button::Left)], now);
        let mut events = vec![];
        let mut out = |k, d| {
            events.push((k, d));
            Ok(())
        };
        e.button(Button::Left, true, now, &mut out).unwrap();
        e.tick(now, &mut out).unwrap();
        e.button(Button::Left, false, now, &mut out).unwrap();
        e.button(Button::Left, true, now, &mut out).unwrap();
        e.tick(now, &mut out).unwrap();
        e.stop(&mut out).unwrap();
        assert_eq!(
            events,
            vec![
                (Key::KeyA, true),
                (Key::KeyA, false),
                (Key::KeyA, true),
                (Key::KeyA, false)
            ]
        );
    }
    #[test]
    fn overlapping_bindings_do_not_release_each_others_keys() {
        let now = Instant::now();
        let mut e = Engine::new(vec![binding(Button::Left), binding(Button::Right)], now);
        let mut events = vec![];
        let mut out = |k, d| {
            events.push((k, d));
            Ok(())
        };
        e.button(Button::Left, true, now, &mut out).unwrap();
        e.button(Button::Right, true, now, &mut out).unwrap();
        e.tick(now, &mut out).unwrap();
        e.button(Button::Left, false, now, &mut out).unwrap();
        e.button(Button::Right, false, now, &mut out).unwrap();
        assert_eq!(events, vec![(Key::KeyA, true), (Key::KeyA, false)]);
    }
}
