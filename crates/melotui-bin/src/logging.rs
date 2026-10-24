use std::path::PathBuf;
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;

/// Starts logging to an hourly file in the data directory. If that
/// directory cannot be made — a read-only home, say — the app runs without a
/// log rather than not at all.
pub fn init() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let log_dir = log_dir();
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!(
            "melotui: not logging, cannot create {}: {e}",
            log_dir.display()
        );
        return None;
    }

    let file_appender = rolling::hourly(log_dir, "melotui.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,melotui=debug,symphonia=error".into()),
        )
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_line_number(true)
        .init();

    Some(guard)
}

pub fn log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("melotui")
        .join("logs")
}
