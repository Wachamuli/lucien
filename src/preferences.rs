use std::{collections::HashSet, env, fs, io, path::PathBuf};

#[derive(Debug)]
pub struct Preferences {
    path: Option<PathBuf>,
    pub favorite_apps: HashSet<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            path: None,
            favorite_apps: HashSet::new(),
        }
    }
}

impl Preferences {
    pub fn load() -> Self {
        let package_name = env!("CARGO_PKG_NAME");
        let settings_file_name = "preferences.conf";
        let xdg_dirs = xdg::BaseDirectories::with_prefix(&package_name);
        let settings_file_path = xdg_dirs.place_config_file(&settings_file_name);

        let Ok(path) = settings_file_path else {
            tracing::warn!("Could not determine config path. Using in-memory defaults.");
            return Self::default();
        };

        let settings_file = std::fs::read_to_string(&path).unwrap_or_default();
        let favorite_apps: HashSet<String> = settings_file.lines().map(String::from).collect();

        Self {
            path: Some(path),
            favorite_apps,
        }
    }

    fn save(&self) -> io::Result<()> {
        let Some(ref preferences_path) = self.path else {
            tracing::warn!("In-memory defaults. Settings will not be saved");
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No persistent path available",
            ));
        };

        let content = self
            .favorite_apps
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        let output = if content.is_empty() {
            content
        } else {
            format!("{}\n", content)
        };

        fs::write(preferences_path, output)?;
        tracing::debug!(path = ?preferences_path, "Preference saved into disk.");
        Ok(())
    }

    pub fn toggle_favorite(&mut self, app_id: impl Into<String>) -> io::Result<()> {
        let id = app_id.into();
        if !self.favorite_apps.insert(id.clone()) {
            self.favorite_apps.remove(&id);
        }

        self.save()
    }
}
