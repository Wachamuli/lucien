use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env,
    path::PathBuf,
    sync::Arc,
};
use tokio::io;
use toml_edit::DocumentMut;

use crate::theme::CustomTheme;

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
                return false;
            }
            _ => false,
        }
    }
}
type Keybindings = HashMap<Action, Keystrokes>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    #[serde(skip)]
    pub path: Option<PathBuf>,
    pub leading_icon_count: usize,
    pub favorite_apps: HashSet<String>,
    pub theme: CustomTheme,
    pub keybindings: Keybindings,
}

fn default_keybindings() -> Keybindings {
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

impl Default for Preferences {
    fn default() -> Self {
        Self {
            path: None,
            leading_icon_count: 10,
            favorite_apps: HashSet::new(),
            theme: CustomTheme::default(),
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

    let tmp_path = {
        let mut t = path.clone();
        t.set_extension("tmp");
        t
    };
    tokio::fs::write(&tmp_path, preferences.to_string()).await?;
    tokio::fs::rename(&tmp_path, &path).await?;
    Ok(path)
}
