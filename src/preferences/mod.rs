use std::{collections::HashSet, env, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::io;
use toml_edit::DocumentMut;

pub mod keybindings;
pub mod theme;

use keybindings::{Keybindings, default_keybindings};
use theme::CustomTheme;

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
    pub fn load() -> io::Result<Self> {
        let package_name = env!("CARGO_PKG_NAME");
        let settings_file_name = "preferences.toml";
        let xdg_dirs = xdg::BaseDirectories::with_prefix(package_name);
        let settings_file_path = xdg_dirs.place_config_file(settings_file_name)?;

        let settings_file_string = std::fs::read_to_string(&settings_file_path).unwrap_or_default();

        let mut preferences = toml::from_str::<Preferences>(&settings_file_string)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

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
