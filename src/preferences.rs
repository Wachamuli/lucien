use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env,
    ops::Deref,
    path::PathBuf,
    sync::Arc,
};
use tokio::io;
use toml_edit::DocumentMut;

const DEFAULT_BACKGROUND_COLOR: &str = "#1F1F1FF2";
const DEFAULT_FOCUS_HIGHLIGHT_COLOR: &str = "#FFFFFF1F";
const DEFAULT_HOVER_HIGHLIGHT_COLOR: &str = "#FFFFFF14";
const DEFAULT_BORDER_COLOR: &str = "#A6A6A61A";

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Border {
    pub color: HexColor,
    pub width: f32,
    pub radius: [f32; 4],
}

impl Default for Border {
    fn default() -> Self {
        Self {
            color: DEFAULT_BORDER_COLOR.into(),
            width: 1.0,
            radius: [20.0, 20.0, 20.0, 20.0],
        }
    }
}

impl From<&Border> for iced::Border {
    fn from(value: &Border) -> iced::Border {
        iced::Border {
            color: *value.color,
            width: value.width,
            radius: iced::border::Radius {
                top_left: value.radius[0],
                top_right: value.radius[1],
                bottom_right: value.radius[2],
                bottom_left: value.radius[3],
            },
        }
    }
}

#[derive(Debug)]
pub struct HexColor(pub iced::Color);

impl From<&str> for HexColor {
    // FIXME: Not very idiomatic because it might fail.
    fn from(value: &str) -> HexColor {
        HexColor(
            iced::Color::parse(value)
                .expect("Invalid color. Use #RGB, #RRGGBB, #RGBA, or #RRGGBBAA format instead."),
        )
    }
}

impl Deref for HexColor {
    type Target = iced::Color;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for HexColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex_color = {
            let color = self.0;
            let r = (color.r * 255.0) as u8;
            let g = (color.g * 255.0) as u8;
            let b = (color.b * 255.0) as u8;
            let a = (color.a * 255.0) as u8;
            format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
        };
        serializer.serialize_str(&hex_color)
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let color = String::deserialize(deserializer)?;
        let converted_color = iced::Color::parse(&color).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "Invalid color. Use #RGB, #RRGGBB, #RGBA, or #RRGGBBAA format instead."
            ))
        })?;

        Ok(HexColor(converted_color))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub background: HexColor,
    pub focus_highlight: HexColor,
    pub hover_highlight: HexColor,
    pub border: Border,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: DEFAULT_BACKGROUND_COLOR.into(),
            focus_highlight: DEFAULT_FOCUS_HIGHLIGHT_COLOR.into(),
            hover_highlight: DEFAULT_HOVER_HIGHLIGHT_COLOR.into(),
            border: Border::default(),
        }
    }
}

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
pub struct Keystroks {
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

impl Keystroks {
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
                return false;
            }
            _ => false,
        }
    }
}
type Keybindings = HashMap<Action, Keystroks>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    #[serde(skip)]
    pub path: Option<PathBuf>,
    pub favorite_apps: HashSet<String>,
    pub theme: Theme,
    pub keybindings: Keybindings,
}

fn default_keybindings() -> Keybindings {
    let mut kb = HashMap::new();

    kb.insert(
        Action::Exit,
        Keystroks {
            key: Key::Escape,
            modifiers: [].into(),
        },
    );
    kb.insert(
        Action::Mark,
        Keystroks {
            key: Key::Char('f'),
            modifiers: [Modifier::Control].into(),
        },
    );
    kb.insert(
        Action::GoNextEntry,
        Keystroks {
            key: Key::Tab,
            modifiers: [].into(),
        },
    );
    kb.insert(
        Action::GoPreviousEntry,
        Keystroks {
            key: Key::Tab,
            modifiers: [Modifier::Shift].into(),
        },
    );

    kb
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            path: None,
            favorite_apps: HashSet::new(),
            theme: Theme::default(),
            keybindings: default_keybindings(),
        }
    }
}

impl Preferences {
    pub async fn load() -> io::Result<Self> {
        let package_name = env!("CARGO_PKG_NAME");
        let settings_file_name = "preferences.toml";
        let xdg_dirs = xdg::BaseDirectories::with_prefix(&package_name);
        let settings_file_path = xdg_dirs.place_config_file(&settings_file_name)?;

        let settings_file_string = tokio::fs::read_to_string(&settings_file_path)
            .await
            .unwrap_or_default();

        let mut preferences = toml::from_str::<Preferences>(&settings_file_string)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.message()))?;

        let mut extended_keybindings = default_keybindings();
        extended_keybindings.extend(preferences.keybindings);

        preferences.path = Some(settings_file_path);
        preferences.keybindings = extended_keybindings;
        Ok(preferences)
    }

    pub fn toggle_favorite(&mut self, app_id: impl Into<String>) -> toml_edit::Array {
        let id = app_id.into();
        if !self.favorite_apps.insert(id.clone()) {
            self.favorite_apps.remove(&id);
        }

        toml_edit::Array::from_iter(&self.favorite_apps)
    }
}

pub async fn save_into_disk(
    path: PathBuf,
    key: &str,
    value: impl Into<toml_edit::Value>,
) -> Result<PathBuf, Arc<io::Error>> {
    let settings_file_string = tokio::fs::read_to_string(&path).await.unwrap_or_default();
    let mut preferences = settings_file_string
        .parse::<DocumentMut>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.message()))?;
    preferences[key] = toml_edit::value(value);
    tokio::fs::write(&path, preferences.to_string()).await?;
    Ok(path)
}
