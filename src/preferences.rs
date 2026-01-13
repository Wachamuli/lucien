use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, fs, io, ops::Deref, path::PathBuf};
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
        // TODO: Remove the line below
        println!("{}", toml::to_string_pretty(&Self::default()).unwrap());

        let mut preferences = match toml::from_str::<Preferences>(&settings_file_string) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(path = ?path, diagnostic = %e,"Syntax error detected in");
                tracing::warn!("Using in-memory defaults.");
                return Self::default();
            }
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
