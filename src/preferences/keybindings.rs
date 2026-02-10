use std::{collections::HashMap, str::FromStr};

use serde;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
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

impl FromStr for Modifier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "alt" => Ok(Modifier::Alt),
            "shift" => Ok(Modifier::Shift),
            "control" => Ok(Modifier::Control),
            "super" => Ok(Modifier::Super),
            c => Err(format!(
                "`{c}` is not valid modifier. Use `alt`, `shift`, `control`, or, `super` instead"
            )),
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

impl FromStr for Key {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tab" => Ok(Key::Tab),
            "escape" => Ok(Key::Escape),
            s => {
                let Some(character) = s.chars().next() else {
                    return Err("Empty character.".to_string());
                };

                if !character.is_alphanumeric() {
                    return Err(format!(
                        "{s} is not a alphanumeric value. A-Z, a-z, and 0-9 values are accepted"
                    ));
                }

                return Ok(Key::Char(character));
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    ToggleFavorite,
    Close,
    NextEntry,
    PreviousEntry,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct KeyStroke {
    pub key: Key,
    pub modifiers: Vec<Modifier>,
}

impl<'de> Deserialize<'de> for KeyStroke {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for KeyStroke {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl FromStr for KeyStroke {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<&str> = s.split('-').collect();

        let key_str = parts.pop().ok_or("missing key part")?;
        let key = Key::from_str(key_str)?;

        let mut modifiers = parts
            .into_iter()
            .map(Modifier::from_str)
            .collect::<Result<Vec<Modifier>, String>>()?;
        modifiers.sort_by_key(|m| m.to_string());
        Ok(KeyStroke { key, modifiers })
    }
}

impl std::fmt::Display for KeyStroke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut sorted_mods = self.modifiers.clone();
        sorted_mods.sort_by_key(|m| m.to_string());

        let mut parts: Vec<String> = sorted_mods.iter().map(|m| m.to_string()).collect();
        parts.push(self.key.to_string());
        write!(f, "{}", parts.join("-"))
    }
}

impl KeyStroke {
    fn new<I>(key: Key, modifiers: I) -> Self
    where
        I: IntoIterator<Item = Modifier>,
    {
        Self {
            key,
            modifiers: modifiers.into_iter().collect(),
        }
    }

    pub fn from_iced_keyboard(
        iced_key: iced::keyboard::Key,
        iced_modifiers: iced::keyboard::Modifiers,
    ) -> Self {
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

pub fn default_keybindings() -> HashMap<KeyStroke, Action> {
    HashMap::from([
        (KeyStroke::new(Key::Escape, []), Action::Close),
        (
            KeyStroke::new(Key::Char('f'), [Modifier::Control]),
            Action::ToggleFavorite,
        ),
        (KeyStroke::new(Key::Tab, []), Action::NextEntry),
        (
            KeyStroke::new(Key::Tab, [Modifier::Shift]),
            Action::PreviousEntry,
        ),
    ])
}

pub fn extend_keybindings(extended_keybindings: Keybindings) -> Keybindings {
    let mut base_keybindings = default_keybindings();

    for extended_keystroke in extended_keybindings.keys() {
        if base_keybindings.contains_key(extended_keystroke) {
            let old_action = base_keybindings[extended_keystroke];
            let new_action = extended_keybindings[extended_keystroke];
            tracing::warn!(
                "Overriding default keybinding '{extended_keystroke}': '{old_action:?}' -> '{new_action:?}'"
            );
        }
    }

    base_keybindings.extend(extended_keybindings);
    base_keybindings
}
