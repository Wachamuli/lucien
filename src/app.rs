use gio::{Icon, prelude::IconExt};
use std::path::PathBuf;

use gio::{AppInfo, AppLaunchContext, prelude::AppInfoExt};

#[derive(Debug)]
pub struct App {
    pub info: AppInfo,
    pub name: String,
    pub description: String,
    pub icon: Option<PathBuf>,
}

pub fn all_apps() -> Vec<App> {
    gio::AppInfo::all()
        .iter()
        .filter(|app| app.should_show())
        .map(|app| App {
            info: app.clone(),
            name: app.name().to_string(),
            description: app.description().unwrap_or_default().to_string(),
            icon: get_icon(app.icon()),
        })
        .collect()
}

fn get_icon_path_from_xdgicon(iconname: &str) -> Option<PathBuf> {
    dbg!(&iconname);

    if iconname.contains("/") || iconname.contains("\\") {
        return Some(PathBuf::from(iconname));
    }

    let scalable_icon_path = xdg::BaseDirectories::with_prefix("icons/hicolor/scalable/apps");
    if let Some(iconpath) = scalable_icon_path.find_data_file(format!("{iconname}.svg")) {
        return Some(iconpath);
    
    for prefix in &[
        "256x256", "128x128", "96x96", "64x64", "48x48", "32x32", "24x24", "16x16",
    ] {
        let iconpath = xdg::BaseDirectories::with_prefix(&format!("icons/hicolor/{prefix}/apps"));
        if let Some(iconpath) = iconpath.find_data_file(format!("{iconname}.png")) {
            return Some(iconpath);
        }
    }
    let pixmappath = xdg::BaseDirectories::with_prefix("pixmaps");
    if let Some(iconpath) = pixmappath.find_data_file(format!("{iconname}.svg")) {
        return Some(iconpath);
    }
    if let Some(iconpath) = pixmappath.find_data_file(format!("{iconname}.png")) {
        return Some(iconpath);
    }

    None
    // for themes in THEMES_LIST {
    //     let iconpath =
    //         xdg::BaseDirectories::with_prefix(&format!("icons/{themes}/apps/48"));
    //     if let Some(iconpath) = iconpath.find_data_file(format!("{iconname}.svg")) {
    //         return Some(iconpath);

    //         let iconpath =
    //             xdg::BaseDirectories::with_prefix(&format!("icons/{themes}/apps/64"));
    //         if let Some(iconpath) = iconpath.find_data_file(format!("{iconname}.svg")) {
    //             return Some(iconpath);
    //         }
    //     }
    //     None
    // } None
}

// fn get_icon_path_from_xdgicon(icon_name: &str) -> Option<PathBuf> {
//     let icon_path = xdg::BaseDirectories::with_prefix("icons/");

//     if let Some(icon_abs_path) = icon_path.find_data_file(format!("{icon_name}.svg")) {
//         return Some(icon_abs_path);
//     }

//     if let Some(icon_abs_path) = icon_path.find_data_file(format!("{icon_name}.png")) {
//         return Some(icon_abs_path);
//     }

//     let pixmap_path= xdg::BaseDirectories::with_prefix("pixmaps/");

//     if let Some(icon_abs_path) = pixmap_path.find_data_file(format!("{icon_name}.svg")) {
//         return Some(icon_abs_path);
//     }

//     if let Some(icon_abs_path) = pixmap_path.find_data_file(format!("{icon_name}.png")) {
//         return Some(icon_abs_path);
//     }

//     None
// }

pub fn get_icon(icon: Option<Icon>) -> Option<PathBuf> {
    let path = icon?.to_string()?;
    if let Some(xdg_icon_path) = get_icon_path_from_xdgicon(&path) {
        dbg!(&xdg_icon_path);
        return Some(xdg_icon_path);
    }

    None
}

impl App {
    pub fn launch(&self) {
        if let Err(e) = self.info.launch(&[], AppLaunchContext::NONE) {
            dbg!(e);
        }
    }
}
