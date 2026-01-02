use crate::launcher::Launcher;

mod app;
mod launcher;

use iced_layershell::{
    Appearance,
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::LayerShellSettings,
};

pub fn main() -> iced_layershell::Result {
    let layershell_settings = LayerShellSettings {
        size: Some((700, 500)),
        anchor: Anchor::Bottom | Anchor::Left | Anchor::Right | Anchor::Top,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        layer: Layer::Overlay,
        ..Default::default()
    };

    iced_layershell::build_pattern::application(
        "application_launcher",
        Launcher::update,
        Launcher::view,
    )
    .subscription(Launcher::subscription)
    .layer_settings(layershell_settings)
    .antialiasing(true)
    .style(|_state, _theme| Appearance {
        background_color: iced::Color::TRANSPARENT,
        text_color: Default::default(),
    })
    .run_with(Launcher::init)
}
