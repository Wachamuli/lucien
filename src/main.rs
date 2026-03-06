use std::{
    io::Read,
    os::fd::{AsRawFd, OwnedFd},
    path::PathBuf,
};

use crate::launcher::Lucien;

mod launcher;
mod preferences;
mod providers;
mod ui;

use anyhow::Context;
use nix::sys::socket::{self, AddressFamily, SockFlag, SockType, UnixAddr};
use sqlx::Connection;
use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"clipboard-listener".to_string()) {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        let content = buffer.trim().to_string();

        if !content.is_empty() {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;

            rt.block_on(handle_clipboard_insertion(&content))?;
        }
        std::process::exit(0);
    }

    std::panic::set_hook(Box::new(|info| {
        tracing::error!("Application panicked: {info}");
    }));

    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let package_name = env!("CARGO_PKG_NAME");
    let package_version = env!("CARGO_PKG_VERSION");

    let _single_instance_lock = get_single_instance(package_name)?;

    let xdg_cache_directory = xdg::BaseDirectories::with_prefix(package_name)
        .get_cache_home()
        .context("Failed to find cache directory: check that $HOME is set.")?;

    let _log_guard = setup_tracing_subscriber(xdg_cache_directory, "logs")?;
    tracing::info!("Running {package_name} v.{package_version}...");

    if std::env::var("BENCHMARK").is_ok() {
        std::process::exit(0);
    }

    iced::application(Lucien::new, Lucien::update, Lucien::view)
        .subscription(Lucien::subscription)
        .theme(Lucien::theme)
        .scale_factor(Lucien::scale_factor)
        .level(iced::window::Level::AlwaysOnTop)
        .centered()
        .window_size((700, 500))
        .decorations(false)
        .resizable(false)
        .exit_on_close_request(true)
        .transparent(true)
        .antialiasing(false)
        .run()?;

    Ok(())
}

fn setup_tracing_subscriber(
    cache_dir: PathBuf,
    filename: &str,
) -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    let file_appender = tracing_appender::rolling::daily(cache_dir, filename);
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = EnvFilter::from_default_env()
        .add_directive(Level::INFO.into())
        .add_directive("iced_wgpu=error".parse()?)
        .add_directive("usvg=error".parse()?)
        .add_directive("wgpu_hal=error".parse()?)
        .add_directive("wgpu_core=error".parse()?)
        .add_directive("iced_winit=error".parse()?)
        .add_directive("resvg=error".parse()?)
        .add_directive("calloop=error".parse()?);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_ansi(false).with_writer(non_blocking_file))
        .init();

    Ok(guard)
}

fn get_single_instance(name: &str) -> anyhow::Result<OwnedFd> {
    let sock = socket::socket(
        AddressFamily::Unix,
        SockType::Stream,
        SockFlag::SOCK_CLOEXEC,
        None,
    )
    .context("Unable to create socket.")?;
    let address =
        UnixAddr::new_abstract(name.as_bytes()).context("Invalid name for an abstract socket.")?;
    socket::bind(sock.as_raw_fd(), &address).context("An Instance is already running.")?;
    Ok(sock)
}

async fn handle_clipboard_insertion(content: &str) -> anyhow::Result<()> {
    let mut conn = sqlx::SqliteConnection::connect("/home/wachamuli/clipboard.db").await?;
    let mut transaction = conn.begin().await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS entries (
                    id TEXT PRIMARY KEY,
                    main TEXT NOT NULL,
                    secondary TEXT,
                    icon BLOB
                )
        "#,
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query("INSERT OR IGNORE INTO entries (id, main, secondary, icon) VALUES (?, ?, ?, NULL)")
        .bind(content)
        .bind(content)
        .bind("Text")
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;
    Ok(())
}
