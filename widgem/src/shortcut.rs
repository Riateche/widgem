mod parse;

use {
    crate::{
        callback::Callback,
        event::KeyboardInputEvent,
        shortcut::parse::{parse_key, parse_keycode},
        RawWidgetId,
    },
    anyhow::{anyhow, bail},
    bitflags::bitflags,
    derive_more::From,
    once_cell::sync::OnceCell,
    winit::keyboard::{KeyCode, ModifiersState, NamedKey},
};

#[derive(PartialEq, Debug, Clone)]
pub struct KeyCombinations(pub Vec<KeyCombination>);

impl KeyCombinations {
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
        let text = text.trim();
        let mut iter = text.rsplitn(2, '+');
        let key_text = iter
            .next()
            .ok_or_else(|| anyhow!("no shortcut specified"))?
            .trim();
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
        if !event.info.state.is_pressed() {
            return false;
        }
        if Modifiers::from(event.modifiers) != self.modifiers {
            return false;
        }
        match &self.key {
            ShortcutKey::Logical(key) => &event.info.logical_key == key,
            ShortcutKey::Physical(key) => &event.info.physical_key == key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, From)]
pub enum ShortcutKey {
    Logical(NamedKey),
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
    pub move_to_next_char: KeyCombinations,
    pub move_to_previous_char: KeyCombinations,
    pub delete: KeyCombinations,
    pub backspace: KeyCombinations,
    pub cut: KeyCombinations,
    pub copy: KeyCombinations,
    pub paste: KeyCombinations,
    pub undo: KeyCombinations,
    pub redo: KeyCombinations,
    pub select_all: KeyCombinations,
    pub deselect: KeyCombinations,
    pub bold: KeyCombinations,
    pub italic: KeyCombinations,
    pub underline: KeyCombinations,
    pub move_to_next_word: KeyCombinations,
    pub move_to_previous_word: KeyCombinations,
    pub move_to_start_of_line: KeyCombinations,
    pub move_to_end_of_line: KeyCombinations,
    pub select_next_char: KeyCombinations,
    pub select_previous_char: KeyCombinations,
    pub select_next_word: KeyCombinations,
    pub select_previous_word: KeyCombinations,
    pub select_start_of_line: KeyCombinations,
    pub select_end_of_line: KeyCombinations,
    pub delete_start_of_word: KeyCombinations,
    pub delete_end_of_word: KeyCombinations,
    pub insert_paragraph_separator: KeyCombinations,
}

impl StandardShortcuts {
    pub fn new() -> Self {
        let s = |text| KeyCombinations::from_str_portable(text).unwrap();
        Self {
            #[cfg(not(target_os = "macos"))]
            move_to_next_char: s("Right"),
            #[cfg(target_os = "macos")]
            move_to_next_char: s("Right; MetaOrMacCtrl+F"),

            #[cfg(not(target_os = "macos"))]
            move_to_previous_char: s("Left"),
            #[cfg(target_os = "macos")]
            move_to_previous_char: s("Left; MetaOrMacCtrl+B"),

            delete: s("Delete; MetaOrMacCtrl+D"),

            #[cfg(not(target_os = "macos"))]
            backspace: s("Backspace"),
            #[cfg(target_os = "macos")]
            backspace: s("Backspace; MetaOrMacCtrl+H"),

            #[cfg(not(target_os = "macos"))]
            cut: s("Ctrl+X; Shift+Delete; F20"),
            #[cfg(target_os = "macos")]
            cut: s("CtrlOrMacCmd+X; MetaOrMacCtrl+K"),

            #[cfg(not(target_os = "macos"))]
            copy: s("Ctrl+C; Ctrl+Insert; F16"),
            #[cfg(target_os = "macos")]
            copy: s("CtrlOrMacCmd+C"),

            #[cfg(not(target_os = "macos"))]
            paste: s("Ctrl+V; Shift+Insert; F18"),
            #[cfg(target_os = "macos")]
            paste: s("CtrlOrMacCmd+V; MetaOrMacCtrl+Y"),

            #[cfg(not(target_os = "macos"))]
            undo: s("Ctrl+Z; Alt+Backspace; F14"),
            #[cfg(target_os = "macos")]
            undo: s("CtrlOrMacCmd+Z"),

            #[cfg(not(target_os = "macos"))]
            redo: s("Ctrl+Y; Shift+Ctrl+Z; Alt+Shift+Backspace"),
            #[cfg(target_os = "macos")]
            redo: s("Shift+CtrlOrMacCmd+Z"),

            select_all: s("CtrlOrMacCmd+A"),

            deselect: s("CtrlOrMacCmd+Shift+A"),

            bold: s("CtrlOrMacCmd+B"),

            italic: s("CtrlOrMacCmd+I"),

            underline: s("CtrlOrMacCmd+U"),

            #[cfg(not(target_os = "macos"))]
            move_to_next_word: s("Ctrl+Right"),
            #[cfg(target_os = "macos")]
            move_to_next_word: s("Alt+Right"),

            #[cfg(not(target_os = "macos"))]
            move_to_previous_word: s("Ctrl+Left"),
            #[cfg(target_os = "macos")]
            move_to_previous_word: s("Alt+Left"),

            #[cfg(not(target_os = "macos"))]
            move_to_start_of_line: s("Home"),
            #[cfg(target_os = "macos")]
            move_to_start_of_line: s("CtrlOrMacCmd+Left; MetaOrMacCtrl+Left"),

            #[cfg(not(target_os = "macos"))]
            move_to_end_of_line: s("End; Ctrl+E"),
            #[cfg(target_os = "macos")]
            move_to_end_of_line: s("CtrlOrMacCmd+Right; MetaOrMacCtrl+Right"),

            select_next_char: s("Shift+Right"),

            select_previous_char: s("Shift+Left"),

            #[cfg(not(target_os = "macos"))]
            select_next_word: s("Ctrl+Shift+Right"),
            #[cfg(target_os = "macos")]
            select_next_word: s("Alt+Shift+Right"),

            #[cfg(not(target_os = "macos"))]
            select_previous_word: s("Ctrl+Shift+Left"),
            #[cfg(target_os = "macos")]
            select_previous_word: s("Alt+Shift+Left"),

            #[cfg(not(target_os = "macos"))]
            select_start_of_line: s("Shift+Home"),
            #[cfg(target_os = "macos")]
            select_start_of_line: s("CtrlOrMacCmd+Shift+Left"),

            #[cfg(not(target_os = "macos"))]
            select_end_of_line: s("Shift+End"),
            #[cfg(target_os = "macos")]
            select_end_of_line: s("CtrlOrMacCmd+Shift+Right"),

            #[cfg(not(target_os = "macos"))]
            delete_start_of_word: s("Ctrl+Backspace"),
            #[cfg(target_os = "macos")]
            delete_start_of_word: s("Alt+Backspace"),

            delete_end_of_word: s("CtrlOrMacCmd+Delete"),

            insert_paragraph_separator: s("Enter"),
        }
    }
}

impl Default for StandardShortcuts {
    fn default() -> Self {
        Self::new()
    }
}

pub fn standard_shortcuts() -> &'static StandardShortcuts {
    static CELL: OnceCell<StandardShortcuts> = OnceCell::new();
    CELL.get_or_init(StandardShortcuts::new)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutScope {
    Widget,
    Window,
    Application,
    // TODO: support global shortcuts?
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShortcutId(RawWidgetId);

impl ShortcutId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(RawWidgetId::new_unique())
    }
}

#[derive(Debug, Clone)]
pub struct Shortcut {
    pub id: ShortcutId,
    pub key_combinations: KeyCombinations,
    pub scope: ShortcutScope,
    pub callback: Callback<()>,
}

impl Shortcut {
    pub fn new(
        key_combinations: KeyCombinations,
        scope: ShortcutScope,
        callback: Callback<()>,
    ) -> Self {
        Self {
            id: ShortcutId::new(),
            key_combinations,
            scope,
            callback,
        }
    }
}

#[test]
fn test_standard_shortcuts() {
    let shortcuts = StandardShortcuts::new();

    #[cfg(not(target_os = "macos"))]
    {
        let shortcut1 = KeyCombinations(vec![KeyCombination::new(
            Modifiers::empty(),
            NamedKey::ArrowRight,
        )]);
        assert_eq!(
            shortcuts.move_to_next_char, shortcut1,
            "standard_shortcuts: expected {:?}, got {:?}",
            shortcut1, shortcuts.move_to_next_char
        );

        let shortcut2 = KeyCombinations(vec![
            KeyCombination::new(Modifiers::CTRL_OR_MAC_CMD, KeyCode::KeyY),
            KeyCombination::new(Modifiers::CTRL_OR_MAC_CMD | Modifiers::SHIFT, KeyCode::KeyZ),
            KeyCombination::new(Modifiers::ALT | Modifiers::SHIFT, NamedKey::Backspace),
        ]);
        assert_eq!(
            shortcuts.redo, shortcut2,
            "standard_shortcuts: expected {:?}, got {:?}",
            shortcut1, shortcuts.redo
        );

        let shortcut3 = KeyCombinations(vec![KeyCombination::new(Modifiers::SHIFT, NamedKey::End)]);
        assert_eq!(
            shortcuts.select_end_of_line, shortcut3,
            "standard_shortcuts: expected {:?}, got {:?}",
            shortcut1, shortcuts.select_end_of_line
        );
    }

    #[cfg(target_os = "macos")]
    {
        let shortcut1 = KeyCombinations(vec![
            KeyCombination::new(Modifiers::empty(), NamedKey::ArrowRight),
            KeyCombination::new(Modifiers::META_OR_MAC_CTRL, KeyCode::KeyF),
        ]);
        assert_eq!(
            shortcuts.move_to_next_char, shortcut1,
            "standard_shortcuts: expected {:?}, got {:?}",
            shortcut1, shortcuts.move_to_next_char
        );

        let shortcut2 = KeyCombinations(vec![KeyCombination::new(
            Modifiers::CTRL_OR_MAC_CMD | Modifiers::SHIFT,
            KeyCode::KeyZ,
        )]);
        assert_eq!(
            shortcuts.redo, shortcut2,
            "standard_shortcuts: expected {:?}, got {:?}",
            shortcut1, shortcuts.redo
        );

        let shortcut3 = KeyCombinations(vec![KeyCombination::new(
            Modifiers::SHIFT | Modifiers::CTRL_OR_MAC_CMD,
            NamedKey::ArrowRight,
        )]);
        assert_eq!(
            shortcuts.select_end_of_line, shortcut3,
            "standard_shortcuts: expected {:?}, got {:?}",
            shortcut1, shortcuts.select_end_of_line
        );
    }
}
