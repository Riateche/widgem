use anyhow::{anyhow, bail};
use bitflags::bitflags;
use derive_more::From;
use winit::keyboard::{Key, KeyCode, ModifiersState};

mod parse;

use crate::{
    event::KeyboardInputEvent,
    shortcut::parse::{parse_key, parse_keycode},
};

pub struct Shortcut(pub Vec<KeyCombination>);

impl Shortcut {
    pub fn new(modifiers: Modifiers, key: impl Into<ShortcutKey>) -> Self {
        Self(vec![KeyCombination::new(modifiers, key)])
    }

    pub fn or(mut self, modifiers: Modifiers, key: impl Into<ShortcutKey>) -> Self {
        self.0.push(KeyCombination::new(modifiers, key));
        self
    }

    pub fn matches(&self, event: &KeyboardInputEvent) -> bool {
        self.0.iter().any(|s| s.matches(event))
    }

    pub fn from_str_portable(text: &str) -> anyhow::Result<Self> {
        let mut r = Vec::new();
        for part in text.split(';') {
            let part = part.trim();
            if !part.is_empty() {
                r.push(KeyCombination::from_str_portable(part)?);
            }
        }
        if r.is_empty() {
            bail!("no shortcut specified");
        }
        Ok(Self(r))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombination {
    pub key: ShortcutKey,
    pub modifiers: Modifiers,
}

impl KeyCombination {
    pub fn new(modifiers: Modifiers, key: impl Into<ShortcutKey>) -> Self {
        KeyCombination {
            key: key.into(),
            modifiers,
        }
    }

    pub fn from_str_portable(text: &str) -> anyhow::Result<Self> {
        let text = text.to_ascii_lowercase();
        let mut iter = text.rsplitn(2, '+');
        let key_text = iter
            .next()
            .ok_or_else(|| anyhow!("no shortcut specified"))?;
        let mut modifiers = Modifiers::empty();
        if let Some(modifiers_text) = iter.next() {
            for modifier_text in modifiers_text.split('+') {
                match modifier_text.trim() {
                    "" => bail!("invalid format"),
                    "shift" => modifiers |= Modifiers::SHIFT,
                    "alt" => modifiers |= Modifiers::ALT,
                    "ctrl" | "ctrlormaccmd" => modifiers |= Modifiers::CTRL_OR_MAC_CMD,
                    "meta" | "metaormacctrl" => modifiers |= Modifiers::META_OR_MAC_CTRL,
                    _ => bail!("unknown modifier"),
                }
            }
        }
        let key = if let Some(key) = parse_key(key_text) {
            ShortcutKey::Logical(key)
        } else if let Some(key) = parse_keycode(key_text) {
            ShortcutKey::Physical(key)
        } else {
            bail!("unknown key");
        };
        Ok(Self { key, modifiers })
    }

    pub fn matches(&self, event: &KeyboardInputEvent) -> bool {
        if !event.event.state.is_pressed() {
            return false;
        }
        if Modifiers::from(event.modifiers) != self.modifiers {
            return false;
        }
        match &self.key {
            ShortcutKey::Logical(key) => &event.event.logical_key == key,
            ShortcutKey::Physical(key) => &event.event.physical_key == key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, From)]
pub enum ShortcutKey {
    Logical(Key),
    Physical(KeyCode),
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Modifiers: u8 {
        const SHIFT = 0b1;
        const ALT = 0b10;
        const CTRL_OR_MAC_CMD = 0b100;
        const META_OR_MAC_CTRL = 0b1000;


    }
}

#[cfg(not(target_os = "macos"))]
impl From<ModifiersState> for Modifiers {
    fn from(value: ModifiersState) -> Self {
        let mut r = Self::empty();
        if value.shift_key() {
            r |= Self::SHIFT;
        }
        if value.alt_key() {
            r |= Self::ALT;
        }
        if value.control_key() {
            r |= Self::CTRL_OR_MAC_CMD;
        }
        if value.super_key() {
            r |= Self::META_OR_MAC_CTRL;
        }
        r
    }
}

#[cfg(target_os = "macos")]
impl From<ModifiersState> for Modifiers {
    fn from(value: ModifiersState) -> Self {
        let mut r = Self::empty();
        if value.shift_key() {
            r |= Self::SHIFT;
        }
        if value.alt_key() {
            r |= Self::ALT;
        }
        if value.control_key() {
            r |= Self::META_OR_MAC_CTRL;
        }
        if value.super_key() {
            r |= Self::CTRL_OR_MAC_CMD;
        }
        r
    }
}

pub struct StandardShortcuts {
    pub move_to_next_char: Shortcut,
    pub move_to_previous_char: Shortcut,
}

impl StandardShortcuts {
    pub fn new() -> Self {
        let s = |text| Shortcut::from_str_portable(text).unwrap();
        Self {
            #[cfg(not(target_os = "macos"))]
            move_to_next_char: s("Right"),
            #[cfg(target_os = "macos")]
            move_to_next_char: s("Right; MetaOrMacCtrl+F"),
            #[cfg(not(target_os = "macos"))]
            move_to_previous_char: s("Left"),
            #[cfg(target_os = "macos")]
            move_to_previous_char: Shortcut::new(Modifiers::empty(), Key::ArrowLeft)
                .or(Modifiers::META_OR_MAC_CTRL, KeyCode::KeyB),
        }
    }
}

impl Default for StandardShortcuts {
    fn default() -> Self {
        Self::new()
    }
}
