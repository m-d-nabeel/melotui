use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use melotui_ipc::{channel::IpcWorker, command::DownloadCommand, event::IpcEvent};

use crate::progress::parse_progress;

enum InternalEvent {
    Progress(u8),
    Finished,
    Failed(String),
}

pub fn run(worker: impl IpcWorker<DownloadCommand>) {
    tracing::info!("Download worker started");

    // Bridge between the download thread and this IPC loop
    let (internal_tx, internal_rx) = mpsc::channel::<InternalEvent>();

    // Cancellation flag shared with the download thread
    let mut cancel_flag: Option<Arc<AtomicBool>> = None;

    loop {
        if let Some(cmd) = worker.try_recv() {
            tracing::debug!("Worker received command: {:?}", cmd);

            match cmd {
                DownloadCommand::Request {
                    query,
                    download_path,
                } => {
                    if cancel_flag.is_some() {
                        tracing::warn!("Download already running, ignoring");
                        continue;
                    }

                    tracing::info!("Starting download for: {}", query);
                    worker.send(IpcEvent::DownloadStarted {
                        query: query.clone(),
                    });

                    let flag = Arc::new(AtomicBool::new(false));
                    cancel_flag = Some(flag.clone());

                    let tx = internal_tx.clone();

                    thread::spawn(move || {
                        download_task(query, download_path, tx, flag);
                    });
                }

                DownloadCommand::Cancel => {
                    if let Some(flag) = &cancel_flag {
                        tracing::info!("Signaling cancellation...");
                        flag.store(true, Ordering::SeqCst);
                    }
                }

                DownloadCommand::Shutdown => {
                    tracing::info!("Worker: shutdown requested");
                    if let Some(flag) = &cancel_flag {
                        flag.store(true, Ordering::SeqCst);
                    }
                    break;
                }
            }
        }

        // Check Download Progress
        while let Ok(evt) = internal_rx.try_recv() {
            match evt {
                InternalEvent::Progress(percent) => {
                    worker.send(IpcEvent::DownloadProgress { percent });
                }
                InternalEvent::Finished => {
                    worker.send(IpcEvent::DownloadFinished);
                    cancel_flag = None;
                }
                InternalEvent::Failed(error) => {
                    worker.send(IpcEvent::DownloadFailed { error });
                    cancel_flag = None;
                }
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    tracing::info!("Download worker exited");
}

fn download_task(
    query: String,
    dl_path: PathBuf,
    tx: mpsc::Sender<InternalEvent>,
    cancel: Arc<AtomicBool>,
) {
    tracing::debug!("YT-DLP Thread Spawned, root: {:?}", dl_path);

    let cmd = Command::new("yt-dlp")
        .arg("-x")
        .arg("-f")
        .arg("bestaudio/best")
        .arg("--extract-audio")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("0")
        .arg("--embed-thumbnail")
        .arg("--add-metadata")
        .arg("--output")
        .arg(format!("{}/%(title)s.%(ext)s", dl_path.display()))
        .arg(format!("ytsearch1:{}", query))
        .arg("--newline") // Crucial for progress parsing
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) // Progress might come on stderr
        .spawn();

    let mut child = match cmd {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(InternalEvent::Failed(format!(
                "Failed to start yt-dlp: {}",
                e
            )));
            return;
        }
    };

    // We need to read stdout/stderr without blocking the cancellation check.
    // The easiest way is two small threads that pump lines to the main thread?
    // OR just use the reference implementation logic which spawns threads for IO.

    let stderr = child.stderr.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    let tx_err = tx.clone();
    let cancel_err = cancel.clone();

    // Stderr reader
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            if cancel_err.load(Ordering::SeqCst) {
                return;
            }
            // yt-dlp sends progress to stdout or stderr depending on version/flags
            if let Some(pct) = parse_progress(&line) {
                let _ = tx_err.send(InternalEvent::Progress(pct));
            }
        }
    });

    let tx_out = tx.clone();
    let cancel_out = cancel.clone();

    // Stdout reader
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if cancel_out.load(Ordering::SeqCst) {
                return;
            }
            tracing::debug!("yt-dlp: {}", line);
            // Also check stdout for progress just in case
            if let Some(pct) = parse_progress(&line) {
                let _ = tx_out.send(InternalEvent::Progress(pct));
            }
        }
    });

    // Monitor Loop
    loop {
        if cancel.load(Ordering::SeqCst) {
            tracing::info!("Killing yt-dlp process...");
            let _ = child.kill();
            let _ = child.wait();
            let _ = tx.send(InternalEvent::Failed("Cancelled".into()));
            return;
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    let _ = tx.send(InternalEvent::Finished);
                } else {
                    let _ = tx.send(InternalEvent::Failed(format!("Exit code: {}", status)));
                }
                return;
            }
            Ok(None) => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                let _ = tx.send(InternalEvent::Failed(e.to_string()));
                return;
            }
        }
    }
}
