use std::collections::HashMap;

use serde::{self, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Modifier {
    Alt,
    Shift,
    Control,
    Super,
}

impl std::fmt::Display for Modifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Modifier::Alt => write!(f, "alt"),
            Modifier::Shift => write!(f, "shift"),
            Modifier::Control => write!(f, "control"),
            Modifier::Super => write!(f, "super"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Key {
    Tab,
    Escape,
    #[serde(untagged)]
    Char(char),
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Tab => write!(f, "tab"),
            Key::Escape => write!(f, "escape"),
            Key::Char(c) => write!(f, "{c}"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct KeyStroke {
    pub key: Key,
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
}

impl std::fmt::Display for KeyStroke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();

        if self.modifiers.contains(&Modifier::Control) {
            parts.push(Modifier::Control.to_string());
        }
        if self.modifiers.contains(&Modifier::Super) {
            parts.push(Modifier::Super.to_string());
        }
        if self.modifiers.contains(&Modifier::Alt) {
            parts.push(Modifier::Alt.to_string());
        }
        if self.modifiers.contains(&Modifier::Shift) {
            parts.push(Modifier::Shift.to_string());
        }

        parts.push(self.key.to_string());
        write!(f, "{}", parts.join("-"))
    }
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Mark,
    Exit,
    GoNextEntry,
    GoPreviousEntry,
}

impl KeyStroke {
    pub fn new(iced_key: iced::keyboard::Key, iced_modifiers: iced::keyboard::Modifiers) -> Self {
        let mut modifiers = Vec::new();

        if iced_modifiers.control() {
            modifiers.push(Modifier::Control)
        };
        if iced_modifiers.logo() {
            modifiers.push(Modifier::Super);
        }
        if iced_modifiers.alt() {
            modifiers.push(Modifier::Alt)
        };
        if iced_modifiers.shift() {
            modifiers.push(Modifier::Shift);
        }

        let key = match iced_key {
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) => Key::Tab,
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => Key::Escape,
            iced::keyboard::Key::Character(c) => Key::Char(c.chars().next().unwrap_or(' ')),
            _ => Key::Char(' '),
        };

        KeyStroke { key, modifiers }
    }
}

pub type Keybindings = HashMap<KeyStroke, Action>;

pub fn default_keybindings() -> Keybindings {
    let mut kb = HashMap::new();

    kb.insert(
        KeyStroke {
            key: Key::Escape,
            modifiers: [].into(),
        },
        Action::Exit,
    );
    kb.insert(
        KeyStroke {
            key: Key::Char('f'),
            modifiers: [Modifier::Control].into(),
        },
        Action::Mark,
    );
    kb.insert(
        KeyStroke {
            key: Key::Tab,
            modifiers: [].into(),
        },
        Action::GoNextEntry,
    );
    kb.insert(
        KeyStroke {
            key: Key::Tab,
            modifiers: [Modifier::Shift].into(),
        },
        Action::GoPreviousEntry,
    );

    kb
}
