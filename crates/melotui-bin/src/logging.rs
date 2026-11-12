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

    let file_appender = match appender(&log_dir) {
        Ok(appender) => appender,
        Err(e) => {
            eprintln!(
                "melotui: not logging, cannot write in {}: {e}",
                log_dir.display()
            );
            return None;
        }
    };
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

/// How many log files are kept. There is one for each hour melotui ran in,
/// and nothing ever removed them: the folder only grew.
const LOG_FILES_KEPT: usize = 48;

/// The log file in `dir`, a new one each hour, the oldest removed past
/// [`LOG_FILES_KEPT`].
fn appender(dir: &std::path::Path) -> Result<rolling::RollingFileAppender, rolling::InitError> {
    rolling::Builder::new()
        .rotation(rolling::Rotation::HOURLY)
        .filename_prefix("melotui.log")
        .max_log_files(LOG_FILES_KEPT)
        .build(dir)
}

pub fn log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("melotui")
        .join("logs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn old_logs_are_removed_and_the_newest_kept() {
        let dir = tempfile::tempdir().unwrap();
        // A hundred hours of logs, as the folder was left to gather.
        for day in 1..=5 {
            for hour in 0..20 {
                let name = format!("melotui.log.2026-01-{day:02}-{hour:02}");
                std::fs::write(dir.path().join(name), "old").unwrap();
            }
        }
        std::fs::write(dir.path().join("something-else.txt"), "not a log").unwrap();

        let mut log = appender(dir.path()).unwrap();
        writeln!(log, "a line").unwrap();
        drop(log);

        let mut names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        let logs = names
            .iter()
            .filter(|n| n.starts_with("melotui.log"))
            .count();
        assert!(logs <= LOG_FILES_KEPT, "{logs} log files left");
        assert!(logs >= 2, "everything was removed");
        assert!(names.contains(&"something-else.txt".to_string()));
        // The one being written is among those kept. (Which of the others
        // go is by when each file was made, and these were all made now.)
        let written = names.iter().any(|n| {
            n.starts_with("melotui.log")
                && std::fs::read_to_string(dir.path().join(n)).is_ok_and(|t| t == "a line\n")
        });
        assert!(written, "the log being written was removed");
    }
}
