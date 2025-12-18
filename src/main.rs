use std::path::PathBuf;

use crate::launcher::Launcher;

mod app;
mod launcher;

// use iced_layershell::{
//     build_pattern::MainSettings,
//     reexport::{Anchor, KeyboardInteractivity},
//     settings::LayerShellSettings,
// };

pub fn main() -> iced::Result {
    iced::application(
        "application_launcher",
        Launcher::update,
        Launcher::view,
    )
    .window_size((500.0, 500.0))
    .antialiasing(true)
    .run_with(Launcher::init)

    // .settings(MainSettings {
    //     antialiasing: true,
    //     layer_settings: LayerShellSettings {
    //         size: Some((500, 500)),
    //         anchor: Anchor::Bottom | Anchor::Left | Anchor::Right | Anchor::Top,
    //         // keyboard_interactivity: KeyboardInteractivity::Exclusive,
    //         ..Default::default()
    //     },
    //     ..Default::default()
    // })
}
