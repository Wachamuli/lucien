use crate::launcher::Launcher;

mod app;
mod launcher;

use iced_layershell::{
    Appearance,
    build_pattern::MainSettings,
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::LayerShellSettings,
};

pub fn main() -> iced_layershell::Result {
    iced_layershell::build_pattern::application(
        "application_launcher",
        Launcher::update,
        Launcher::view,
    )
    .subscription(Launcher::subscription)
    .settings(MainSettings {
        antialiasing: true,
        layer_settings: LayerShellSettings {
            size: Some((500, 500)), // (700, 500)
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right | Anchor::Top,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            layer: Layer::Overlay,
            ..Default::default()
        },
        ..Default::default()
    })
    .style(|_state, _theme| Appearance {
        background_color: iced::Color::TRANSPARENT,
        text_color: Default::default(),
    })
    .run_with(Launcher::init)
}
