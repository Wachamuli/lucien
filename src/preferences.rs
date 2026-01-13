use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, fs, io, path::PathBuf};
use toml_edit::DocumentMut;

const DEFAULT_BACKGROUND_COLOR: &str = "#1F1F1FF2";
const DEFAULT_FOCUS_HIGHLIGHT_COLOR: &str = "#FFFFFF1F";
const DEFAULT_HOVER_HIGHLIGHT_COLOR: &str = "#FFFFFF14";
const DEFAULT_BORDER_COLOR: &str = "#A6A6A61A";

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Border {
    pub color: String,
    pub width: u16,
    pub radius: u16,
}

impl Default for Border {
    fn default() -> Self {
        Self {
            color: DEFAULT_BORDER_COLOR.to_string(),
            width: 1,
            radius: 20,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub background: String,
    pub focus_highlight: String,
    pub hover_highlight: String,
    pub border: Border,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Hex(DEFAULT_BACKGROUND_COLOR.to_string()),
            focus_highlight: DEFAULT_FOCUS_HIGHLIGHT_COLOR.to_string(),
            hover_highlight: DEFAULT_HOVER_HIGHLIGHT_COLOR.to_string(),
            border: Border::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    #[serde(skip)]
    path: Option<PathBuf>,
    pub favorite_apps: HashSet<String>,
    pub theme: Theme,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            path: None,
            favorite_apps: HashSet::new(),
            theme: Theme::default(),
        }
    }
}

impl Preferences {
    pub fn load() -> Self {
        let package_name = env!("CARGO_PKG_NAME");
        let settings_file_name = "preferences.toml";
        let xdg_dirs = xdg::BaseDirectories::with_prefix(&package_name);
        let settings_file_path = xdg_dirs.place_config_file(&settings_file_name);

        let Ok(path) = settings_file_path else {
            tracing::warn!("Could not determine config path. Using in-memory defaults.");
            return Self::default();
        };

        let settings_file_string = std::fs::read_to_string(&path).unwrap_or_default();
        let Ok(mut preferences): Result<Preferences, toml::de::Error> =
            toml::from_str(&settings_file_string)
        else {
            tracing::error!(path = ?path,"Syntax error detected in");
            return Self::default();
        };

        preferences.path = Some(path);
        tracing::debug!("Running under user-defined preferences.");
        preferences
    }

    fn save(&self, key: &str, value: impl Into<toml_edit::Value>) -> io::Result<()> {
        let Some(ref preferences_path) = self.path else {
            tracing::warn!("In-memory defaults. Settings will not be saved");
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No persistent path available",
            ));
        };

        let settings_file_string = std::fs::read_to_string(&preferences_path).unwrap_or_default();
        // At this point is safe to call `unwrap`. The path leads to a valid TOML
        // checked by the `load` function.
        let mut preferences = settings_file_string.parse::<DocumentMut>().unwrap();
        preferences[key] = toml_edit::value(value);
        fs::write(preferences_path, preferences.to_string())?;
        tracing::debug!(path = ?preferences_path, "Preference saved into disk.");
        Ok(())
    }

    pub fn toggle_favorite(&mut self, app_id: impl Into<String>) -> io::Result<()> {
        let id = app_id.into();
        if !self.favorite_apps.insert(id.clone()) {
            self.favorite_apps.remove(&id);
        }

        self.save(
            "favorite_apps",
            toml_edit::Array::from_iter(&self.favorite_apps),
        )
    }
}
