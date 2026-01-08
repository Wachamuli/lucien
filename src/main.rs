use crate::launcher::Lucien;

mod app;
mod launcher;
mod preferences;

use iced_layershell::{
    Appearance,
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::LayerShellSettings,
};
use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn main() -> iced_layershell::Result {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("LAUNCHER CRASHED: {}", panic_info);
    }));

    let package_name = env!("CARGO_PKG_NAME");
    let xdg_dirs = xdg::BaseDirectories::with_prefix(&package_name);
    let cache_dir = xdg_dirs.get_cache_home().expect(
        "Could not determine the user's Home directory. Ensure the $HOME environment variable is set."
    );
    let file_appender = tracing_appender::rolling::daily(cache_dir, "logs");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = EnvFilter::from_default_env()
        .add_directive(Level::INFO.into())
        .add_directive("iced_wgpu=error".parse().unwrap())
        .add_directive("usvg=error".parse().unwrap())
        .add_directive("wgpu_hal=error".parse().unwrap())
        .add_directive("wgpu_core=error".parse().unwrap())
        .add_directive("wgpu_core=error".parse().unwrap())
        .add_directive(format!("{package_name}=info").parse().unwrap());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_ansi(false).with_writer(non_blocking_file))
        .init();

    let package_version = env!("CARGO_PKG_VERSION");
    tracing::info!("Running {package_name} v. {package_version}...");

    let layershell_settings = LayerShellSettings {
        size: Some((700, 500)),
        anchor: Anchor::Bottom | Anchor::Left | Anchor::Right | Anchor::Top,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        layer: Layer::Overlay,
        ..Default::default()
    };

    iced_layershell::build_pattern::application(
        "application_launcher",
        Lucien::update,
        Lucien::view,
    )
    .subscription(Lucien::subscription)
    .layer_settings(layershell_settings)
    .antialiasing(true)
    .style(|_state, _theme| Appearance {
        background_color: iced::Color::TRANSPARENT,
        text_color: Default::default(),
    })
    .run_with(Lucien::init)
}
