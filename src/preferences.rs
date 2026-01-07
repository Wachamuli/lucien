use std::{
    collections::{HashMap, HashSet},
    io,
};

pub struct Preferences {
    pub favorite_apps: HashSet<String>,
}

impl Preferences {
    pub fn load() -> io::Result<Self> {
        let package_name = env!("CARGO_PKG_NAME");
        let settings_file_name = "settings.json";
        let xdg_dirs = xdg::BaseDirectories::with_prefix(&package_name);
        let settings_file_path = xdg_dirs.find_config_file(&settings_file_name);

        // CREATE FILE IF NOT
        let path = match settings_file_path {
            Some(path) => path,
            None => {
                let new_path = xdg_dirs.place_config_file(settings_file_name)?;

                let default_content = r#"favorite_apps": ["code.desktop"]"#;
                std::fs::write(&new_path, default_content)?;

                new_path
            }
        };

        // FILE PARSER
        let settings_file: HashMap<String, String> = std::fs::read_to_string(path)?
            .lines()
            .filter_map(|l| {
                let (key, value) = l.split_once(":")?;
                Some((key.trim().to_string(), value.trim().to_string()))
            })
            .collect();

        // PARSE
        let v: HashSet<String> = settings_file
            .get("favorite_apps")
            .map(|apps| {
                apps.trim()
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .map(|s| s.trim().trim_matches('"'))
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self { favorite_apps: v })
    }
}
