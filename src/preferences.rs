use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, fs, io, path::PathBuf};
use toml_edit::DocumentMut;

#[derive(Debug, Serialize, Deserialize)]
pub struct Border {
    #[serde(default = "default_border_color")]
    pub color: String,
    #[serde(default = "default_border_width")]
    pub width: u16,
}

fn default_border_color() -> String {
    "#A6A6A61A".to_string()
}
fn default_border_width() -> u16 {
    1
}

impl Default for Border {
    fn default() -> Self {
        Self {
            color: default_border_color(),
            width: default_border_width(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Theme {
    #[serde(default = "default_background_color")]
    pub background: String,
    #[serde(default = "default_foreground_color")]
    pub foreground: String,

    #[serde(default)]
    pub border: Border,
}

fn default_background_color() -> String {
    "#1F1F1FF2".to_string()
}

fn default_foreground_color() -> String {
    "#FFFFFF1F".to_string()
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: "#1F1F1FF2".to_string(),
            foreground: "#FFFFFF1F".to_string(),
            border: Border::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Preferences {
    path: Option<PathBuf>,

    #[serde(default)]
    pub favorite_apps: HashSet<String>,

    #[serde(default)]
    pub theme: Theme,
}

impl Default for Preferences {
    fn default() -> Self {
        tracing::warn!("Could not determine config path. Using in-memory defaults.");
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
            return Self::default();
        };

        // TODO: REMOVE
        let structure = toml::to_string_pretty(&Self::default());
        println!("preferences.toml should look like:\n{}", structure.unwrap());

        let settings_file_string = std::fs::read_to_string(&path).unwrap_or_default();
        let Ok(mut preferences): Result<Preferences, toml::de::Error> =
            toml::from_str(&settings_file_string)
        else {
            tracing::error!("Syntax error detected in {}", &settings_file_name);
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
        // At this point is safe to unwrap.
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
