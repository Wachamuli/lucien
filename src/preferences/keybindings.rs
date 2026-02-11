use std::{collections::HashMap, str::FromStr};

use serde::de::{self, Visitor};
use serde::{self, Deserializer, Serializer};
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

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Tab,
    Escape,
    #[serde(untagged)]
    Char(char),
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Up => write!(f, "up"),
            Key::Down => write!(f, "down"),
            Key::Left => write!(f, "left"),
            Key::Right => write!(f, "right"),
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
            "up" => Ok(Key::Up),
            "down" => Ok(Key::Down),
            "left" => Ok(Key::Left),
            "right" => Ok(Key::Right),
            s => {
                let Some(character) = s.chars().next() else {
                    return Err("missing character".to_string());
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

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum Action {
    ToggleFavorite,
    Close,
    NextEntry,
    PreviousEntry,
    LaunchEntry(usize),
}

impl Serialize for Action {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Action::ToggleFavorite => serializer.serialize_str("toggle_favorite"),
            Action::Close => serializer.serialize_str("close"),
            Action::NextEntry => serializer.serialize_str("next_entry"),
            Action::PreviousEntry => serializer.serialize_str("previous_entry"),
            Action::LaunchEntry(n) => serializer.serialize_str(&format!("launch_entry({})", n)),
        }
    }
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ActionVisitor;

        impl<'de> Visitor<'de> for ActionVisitor {
            type Value = Action;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid action string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Action, E>
            where
                E: de::Error,
            {
                match value {
                    "toggle_favorite" => Ok(Action::ToggleFavorite),
                    "close" => Ok(Action::Close),
                    "next_entry" => Ok(Action::NextEntry),
                    "previous_entry" => Ok(Action::PreviousEntry),
                    s if s.starts_with("launch_entry(") && s.ends_with(")") => {
                        let num_str = &s[13..s.len() - 1]; // Extract the number
                        let num = num_str.parse::<usize>().map_err(de::Error::custom)?;
                        Ok(Action::LaunchEntry(num))
                    }
                    _ => Err(de::Error::unknown_variant(
                        value,
                        &[
                            "toggle_favorite",
                            "close",
                            "next_entry",
                            "previous_entry",
                            "launch_entry(n)",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_str(ActionVisitor)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
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
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => Key::Up,
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => Key::Down,
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowLeft) => Key::Left,
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight) => Key::Right,
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
        (KeyStroke::new(Key::Down, []), Action::NextEntry),
        (
            KeyStroke::new(Key::Tab, [Modifier::Shift]),
            Action::PreviousEntry,
        ),
        (KeyStroke::new(Key::Up, []), Action::PreviousEntry),
        (
            KeyStroke::new(Key::Char('1'), [Modifier::Control]),
            Action::LaunchEntry(1),
        ),
        (
            KeyStroke::new(Key::Char('2'), [Modifier::Control]),
            Action::LaunchEntry(2),
        ),
        (
            KeyStroke::new(Key::Char('3'), [Modifier::Control]),
            Action::LaunchEntry(3),
        ),
        (
            KeyStroke::new(Key::Char('4'), [Modifier::Control]),
            Action::LaunchEntry(4),
        ),
        (
            KeyStroke::new(Key::Char('5'), [Modifier::Control]),
            Action::LaunchEntry(5),
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
