use std::{collections::HashSet, fs, io, path::PathBuf};

#[derive(Debug, Default)]
pub struct Preferences {
    path: PathBuf,
    pub favorite_apps: HashSet<String>,
}

impl Preferences {
    pub fn load() -> io::Result<Self> {
        let package_name = env!("CARGO_PKG_NAME");
        let settings_file_name = "settings.conf";
        let xdg_dirs = xdg::BaseDirectories::with_prefix(&package_name);
        let settings_file_path = xdg_dirs.place_config_file(&settings_file_name)?;

        let settings_file = std::fs::read_to_string(&settings_file_path)?;
        let favorite_apps: HashSet<String> = settings_file.lines().map(String::from).collect();

        Ok(Self {
            path: settings_file_path,
            favorite_apps,
        })
    }

    fn save(&self) -> io::Result<()> {
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

        fs::write(&self.path, output)
    }

    pub fn toggle_favorite(&mut self, app_id: impl Into<String>) -> io::Result<()> {
        let id = app_id.into();
        if !self.favorite_apps.insert(id.clone()) {
            self.favorite_apps.remove(&id);
        }

        self.save()
    }
}
