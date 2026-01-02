use crate::launcher::Launcher;

mod app;
mod launcher;

use iced::Font;
use iced_layershell::{
    Appearance,
    build_pattern::MainSettings,
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::LayerShellSettings,
};

pub const SF_PRO_FONT: &[u8] = include_bytes!("../fonts/SF-Pro-Display-Regular.otf");

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
            size: Some((700, 500)), // (700, 500)
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right | Anchor::Top,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            layer: Layer::Overlay,
            ..Default::default()
        },
        ..Default::default()
    })
    .font(SF_PRO_FONT)
    .default_font(Font::with_name("SF Pro Display"))
    .style(|_state, _theme| Appearance {
        background_color: iced::Color::TRANSPARENT,
        text_color: Default::default(),
    })
    .run_with(Launcher::init)
}
