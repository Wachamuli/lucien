use std::{collections::HashMap, str::FromStr};

use gio::glib::bitflags::bitflags;
use serde::{self, Deserializer, Serializer};
use serde::{Deserialize, Serialize};

const KEYSTROKE_SEPARATOR: &str = "-";

bitflags! {
    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    pub struct Modifiers: u8 {
        const SUPER   = 0b0001;
        const CONTROL = 0b0010;
        const ALT     = 0b0100;
        const SHIFT   = 0b1000;
    }
}

impl std::fmt::Display for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut modifier_names = self.iter_names().map(|(name, _)| match name {
            "SUPER" => "super",
            "CONTROL" => "control",
            "ALT" => "alt",
            "SHIFT" => "shift",
            name => name,
        });

        if let Some(first) = modifier_names.next() {
            write!(f, "{first}")?;
            for modifier in modifier_names {
                write!(f, "{KEYSTROKE_SEPARATOR}{modifier}")?;
            }
        };

        Ok(())
    }
}

impl FromStr for Modifiers {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Modifiers::empty());
        }

        s.split(KEYSTROKE_SEPARATOR)
            .try_fold(Modifiers::empty(), |mut acc, part| {
                acc |= match part {
                    "control" => Modifiers::CONTROL,
                    "shift" => Modifiers::SHIFT,
                    "alt" => Modifiers::ALT,
                    "super" => Modifiers::SUPER,
                    _ => {
                        return Err(format!(
                            "'{part}' is not a valid modifier. Use \
                            'logo', 'control', 'alt', or 'shift' instead"
                        ));
                    }
                };
                Ok(acc)
            })
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
    Unidentified,
    #[serde(untagged)]
    Character(char),
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
            Key::Character(c) => write!(f, "{c}"),
            Key::Unidentified => write!(f, "unidentified"),
        }
    }
}

impl FromStr for Key {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();

        if let (Some(c), None) = (chars.next(), chars.next()) {
            if c.is_alphanumeric() {
                return Ok(Key::Character(c));
            }
        }

        match s {
            "tab" => Ok(Key::Tab),
            "escape" => Ok(Key::Escape),
            "up" => Ok(Key::Up),
            "down" => Ok(Key::Down),
            "left" => Ok(Key::Left),
            "right" => Ok(Key::Right),
            _ => Err(format!(
                "'{s}' is not a valid key. It must be a named key \
                    ('tab', 'escape', 'up', 'down', 'left', 'right') or \
                    a single alphanumeric character (A-Z, 0-9)"
            )),
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

fn extract_parameter<T: FromStr>(parameter_part: &str) -> Result<T, String> {
    parameter_part
        .trim_end_matches(')')
        .trim()
        .parse::<T>()
        .map_err(|_| {
            format!(
                "incorrect type assigned to parameter: expected {}",
                std::any::type_name::<T>()
            )
        })
}

impl FromStr for Action {
    type Err = String;

    fn from_str(action: &str) -> Result<Self, Self::Err> {
        let (identifier, param) = action.split_once("(").unwrap_or((action, ""));
        match identifier {
            "toggle_favorite" => Ok(Action::ToggleFavorite),
            "close" => Ok(Action::Close),
            "next_entry" => Ok(Action::NextEntry),
            "previous_entry" => Ok(Action::PreviousEntry),
            "launch_entry" if param.ends_with(")") => {
                let index: usize = extract_parameter(param)?;
                Ok(Action::LaunchEntry(index))
            }
            _ => Err(format!(
                "unknown action '{action}'. Available actions are: 'toggle_favorite', \
                'close', 'next_entry', 'previous_entry', 'launch_entry(index)'"
            )),
        }
    }
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
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Keystrokes {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl<'de> Deserialize<'de> for Keystrokes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Keystrokes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl FromStr for Keystrokes {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.rsplit_once(KEYSTROKE_SEPARATOR) {
            Some((modifiers_str, key_str)) => {
                let key = Key::from_str(key_str)?;
                let modifiers = Modifiers::from_str(modifiers_str)?;
                Ok(Keystrokes { modifiers, key })
            }
            None => {
                let key = Key::from_str(s)?;
                let modifiers = Modifiers::empty();
                Ok(Keystrokes { modifiers, key })
            }
        }
    }
}

impl std::fmt::Display for Keystrokes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let modifiers_str = self.modifiers.to_string();

        if self.key == Key::Unidentified {
            return write!(f, "{modifiers_str}");
        }

        let key_str = self.key.to_string();

        if modifiers_str.is_empty() {
            return write!(f, "{key_str}");
        }

        write!(f, "{modifiers_str}{KEYSTROKE_SEPARATOR}{key_str}")
    }
}

impl Keystrokes {
    fn new<I>(modifiers: I, key: Key) -> Self
    where
        I: IntoIterator<Item = Modifiers>,
    {
        Self {
            key,
            modifiers: modifiers.into_iter().collect(),
        }
    }

    #[rustfmt::skip]
    pub fn from_iced_keystrokes(
        iced_modifiers: iced::keyboard::Modifiers,
        iced_key: iced::keyboard::Key,
    ) -> Self {
        let mut modifiers = Modifiers::empty();

        // NOTE: `iced_modifiers` does not include the modifier bit for the key currently
        // being pressed, only for keys already held. So for lone modifier presses we
        // also check `iced_key` directly and OR in the flag ourselves.
        if iced_modifiers.logo()    { modifiers |= Modifiers::SUPER }
        if iced_modifiers.control() { modifiers |= Modifiers::CONTROL }
        if iced_modifiers.alt()     { modifiers |= Modifiers::ALT }
        if iced_modifiers.shift()   { modifiers |= Modifiers::SHIFT }

        use iced::keyboard::Key as IcedKey;
        use iced::keyboard::key::Named as IcedNamedKey;

        // Supplement the modifier state from the key itself to catch lone modifier presses
        // (see comment above).
        if let IcedKey::Named(ref named) = iced_key {
            match named {
                IcedNamedKey::Super       => modifiers |= Modifiers::SUPER,
                IcedNamedKey::Control     => modifiers |= Modifiers::CONTROL,
                IcedNamedKey::Alt         => modifiers |= Modifiers::ALT,
                IcedNamedKey::Shift       => modifiers |= Modifiers::SHIFT,
                _ => {}
            }
        }

        let key = match iced_key {
            IcedKey::Character(smol_str) => smol_str
                .chars()
                .next()
                .map(Key::Character)
                .unwrap_or(Key::Unidentified),
            IcedKey::Named(named) => match named {
                IcedNamedKey::Tab => Key::Tab,
                IcedNamedKey::Escape => Key::Escape,
                IcedNamedKey::ArrowUp => Key::Up,
                IcedNamedKey::ArrowDown => Key::Down,
                IcedNamedKey::ArrowLeft => Key::Left,
                IcedNamedKey::ArrowRight => Key::Right,
                _ => Key::Unidentified,
            },
            _ => Key::Unidentified,
        };

        Keystrokes { key, modifiers }
    }
}

pub type Keybindings = HashMap<Keystrokes, Action>;

pub fn default_keybindings() -> HashMap<Keystrokes, Action> {
    HashMap::from([
        (Keystrokes::new([], Key::Escape), Action::Close),
        (
            Keystrokes::new([Modifiers::CONTROL], Key::Character('f')),
            Action::ToggleFavorite,
        ),
        (Keystrokes::new([], Key::Tab), Action::NextEntry),
        (Keystrokes::new([], Key::Down), Action::NextEntry),
        (
            Keystrokes::new([Modifiers::SHIFT], Key::Tab),
            Action::PreviousEntry,
        ),
        (Keystrokes::new([], Key::Up), Action::PreviousEntry),
        (
            Keystrokes::new([Modifiers::CONTROL], Key::Character('1')),
            Action::LaunchEntry(1),
        ),
        (
            Keystrokes::new([Modifiers::CONTROL], Key::Character('2')),
            Action::LaunchEntry(2),
        ),
        (
            Keystrokes::new([Modifiers::CONTROL], Key::Character('3')),
            Action::LaunchEntry(3),
        ),
        (
            Keystrokes::new([Modifiers::CONTROL], Key::Character('4')),
            Action::LaunchEntry(4),
        ),
        (
            Keystrokes::new([Modifiers::CONTROL], Key::Character('5')),
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
