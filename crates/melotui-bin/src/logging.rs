use std::path::PathBuf;
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;

pub fn init() -> tracing_appender::non_blocking::WorkerGuard {
    let log_dir = log_dir();
    std::fs::create_dir_all(&log_dir).expect("failed to create log dir");

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

    guard
}

fn log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join("melotui")
        .join("logs")
}
