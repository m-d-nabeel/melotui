use std::path::PathBuf;
use std::sync::Mutex;
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;

/// Starts logging to an hourly file in the data directory. If that
/// directory cannot be made — a read-only home, say — the app runs without a
/// log rather than not at all.
pub fn init() {
    if let Some(guard) = start()
        && let Ok(mut held) = WRITER.lock()
    {
        *held = Some(guard);
    }
}

/// The log's writer, which runs on a thread of its own and writes what it
/// has been given once this is dropped. Kept here rather than in `main`, so
/// that a panic on any thread can have the log written before the process
/// ends.
static WRITER: Mutex<Option<tracing_appender::non_blocking::WorkerGuard>> = Mutex::new(None);

/// Writes out whatever has been logged and not yet written, and ends
/// logging. For the way out of the process, whichever way that is.
pub fn flush() {
    // `try_lock`, not `lock`: called from the panic hook, which must not
    // wait on anything.
    if let Ok(mut held) = WRITER.try_lock() {
        drop(held.take());
    }
}

fn start() -> Option<tracing_appender::non_blocking::WorkerGuard> {
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
