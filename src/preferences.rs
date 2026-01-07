use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
    path::PathBuf,
};

#[derive(Debug)]
pub struct Preferences {
    path: PathBuf,
    pub favorite_apps: HashSet<String>,
}

impl Preferences {
    pub fn load() -> io::Result<Self> {
        let package_name = env!("CARGO_PKG_NAME");
        let settings_file_name = "settings.conf";
        let xdg_dirs = xdg::BaseDirectories::with_prefix(&package_name);
        let settings_file_path = xdg_dirs.find_config_file(&settings_file_name).unwrap();

        // TODO: CREATE FILE IF IT DOESN'T EXIST

        // FILE PARSER
        let settings_file = std::fs::read_to_string(&settings_file_path)?;
        let favorite_apps: HashSet<String> = settings_file.lines().map(String::from).collect();

        Ok(Self {
            path: settings_file_path,
            favorite_apps,
        })
    }

    pub fn toggle_favorite(&mut self, app_id: impl Into<String>) -> io::Result<()> {
        let app_id = app_id.into();
        let preferences_file = std::fs::OpenOptions::new()
            .write(true)
            .create(false)
            .append(true)
            .open(&self.path)?;

        let exists = self.favorite_apps.contains(&app_id);

        if exists {
            let content = std::fs::read_to_string(&self.path)?;
            let new_content: String = content
                .lines()
                .filter(|line| line.trim() != app_id)
                .collect::<Vec<_>>()
                .join("\n");

            let final_output = if new_content.is_empty() {
                new_content
            } else {
                format!("{}\n", new_content)
            };

            std::fs::write(&self.path, final_output)?;

            self.favorite_apps.remove(&app_id);
        } else {
            writeln!(&preferences_file, "{}", app_id)?;
            self.favorite_apps.insert(app_id);
        }

        Ok(())
    }
}
