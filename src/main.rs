use std::{
    os::fd::{AsRawFd, OwnedFd},
    path::PathBuf,
};

use crate::{launcher::Lucien, preferences::Preferences};

mod app;
mod launcher;
mod preferences;

use iced_layershell::{
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::LayerShellSettings,
};
use nix::sys::socket::{self, AddressFamily, SockFlag, SockType, UnixAddr};
use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> iced_layershell::Result {
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!("LAUNCHER CRASHED: {}", panic_info);
    }));

    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let package_name = env!("CARGO_PKG_NAME");
    let package_version = env!("CARGO_PKG_VERSION");

    let _single_instance_guard = match get_single_instance(package_name) {
        Ok(lock) => lock,
        Err(e) => {
            tracing::error!(
                "Another instance of {} is already running. {}",
                &package_name,
                e
            );
            return Ok(());
        }
    };

    let xdg_dirs = xdg::BaseDirectories::with_prefix(&package_name);
    let cache_dir = xdg_dirs.get_cache_home().expect(
        "Could not determine the user's Home directory. Ensure the $HOME environment variable is set."
    );

    let _log_guard = setup_tracing_subscriber(cache_dir, "logs");
    tracing::info!("Running {package_name} v.{package_version}...");

    let rt = tokio::runtime::Runtime::new()
        .expect("Unable to create async runtime to open Preferences file");

    let pref = match rt.block_on(Preferences::load()) {
        Ok(p) => {
            tracing::debug!("Running under user-defined preferences.");
            p
        }
        Err(e) => {
            if matches!(e.kind(), std::io::ErrorKind::InvalidInput) {
                tracing::error!(diagnostic = %e,"Syntax error detected");
            }

            tracing::warn!("Using in-memory defaults.");
            Preferences::default()
        }
    };

    let layershell_settings = LayerShellSettings {
        size: Some((700, 500)),
        anchor: Anchor::Bottom | Anchor::Left | Anchor::Right | Anchor::Top,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        layer: Layer::Overlay,
        ..Default::default()
    };

    iced_layershell::build_pattern::application(package_name, Lucien::update, Lucien::view)
        .subscription(Lucien::subscription)
        .theme(Lucien::theme)
        .layer_settings(layershell_settings)
        .antialiasing(true)
        .run_with(|| Lucien::init(pref))
}

fn setup_tracing_subscriber(
    cache_dir: PathBuf,
    filename: &str,
) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily(cache_dir, filename);
    let (non_blocking_file, _logger_guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = EnvFilter::from_default_env()
        .add_directive(Level::INFO.into())
        .add_directive("iced_wgpu=error".parse().unwrap())
        .add_directive("usvg=error".parse().unwrap())
        .add_directive("wgpu_hal=error".parse().unwrap())
        .add_directive("wgpu_core=error".parse().unwrap())
        .add_directive("calloop=error".parse().unwrap());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_ansi(false).with_writer(non_blocking_file))
        .init();

    return _logger_guard;
}

fn get_single_instance(name: &str) -> nix::Result<OwnedFd> {
    let sock = socket::socket(
        AddressFamily::Unix,
        SockType::Stream,
        SockFlag::SOCK_CLOEXEC,
        None,
    )?;
    let address = UnixAddr::new_abstract(name.as_bytes())?;
    socket::bind(sock.as_raw_fd(), &address)?;
    Ok(sock)
}
