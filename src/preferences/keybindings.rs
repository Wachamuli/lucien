use std::collections::{HashMap, HashSet};

use serde::{self, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Modifier {
    Alt,
    Shift,
    Control,
    Super,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Key {
    Tab,
    Escape,
    #[serde(untagged)]
    Char(char),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Keystrokes {
    pub key: Key,
    #[serde(default)]
    pub modifiers: HashSet<Modifier>,
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Mark,
    Exit,
    GoNextEntry,
    GoPreviousEntry,
}

impl Keystrokes {
    pub fn matches(
        &self,
        iced_key: &iced::keyboard::Key,
        iced_modifiers: iced::keyboard::Modifiers,
    ) -> bool {
        let alt = iced_modifiers.alt() == self.modifiers.contains(&Modifier::Alt);
        let shift = iced_modifiers.shift() == self.modifiers.contains(&Modifier::Shift);
        let control = iced_modifiers.control() == self.modifiers.contains(&Modifier::Control);
        let logo = iced_modifiers.logo() == self.modifiers.contains(&Modifier::Super);

        if !(alt && shift && control && logo) {
            return false;
        }

        match iced_key {
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) => {
                matches!(&self.key, Key::Tab)
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                matches!(&self.key, Key::Escape)
            }
            iced::keyboard::Key::Character(c) => {
                if let Key::Char(d) = &self.key {
                    return c.to_string() == d.to_string();
                }

                false
            }
            _ => false,
        }
    }
}

pub type Keybindings = HashMap<Action, Keystrokes>;

pub fn default_keybindings() -> Keybindings {
    let mut kb = HashMap::new();

    kb.insert(
        Action::Exit,
        Keystrokes {
            key: Key::Escape,
            modifiers: [].into(),
        },
    );
    kb.insert(
        Action::Mark,
        Keystrokes {
            key: Key::Char('f'),
            modifiers: [Modifier::Control].into(),
        },
    );
    kb.insert(
        Action::GoNextEntry,
        Keystrokes {
            key: Key::Tab,
            modifiers: [].into(),
        },
    );
    kb.insert(
        Action::GoPreviousEntry,
        Keystrokes {
            key: Key::Tab,
            modifiers: [Modifier::Shift].into(),
        },
    );

    kb
}
