use melotui_core::AppMsg;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use crate::progress::parse_progress;

pub fn spawn_download(query: String, music_root: PathBuf, tx: Sender<AppMsg>) {
    thread::spawn(move || {
        tracing::debug!("YT-DLP Thread Spawned");
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
            .arg(format!("{}/%(title)s.%(ext)s", music_root.display()))
            .arg(format!("ytsearch1:{}", query))
            .arg("--newline")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        tracing::info!("CMD Created: {:?}", cmd);

        let mut child = match cmd {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(AppMsg::DownloadFailed {
                    error: e.to_string(),
                });
                return;
            }
        };

        let tx_err = tx.clone();
        let stderr = child.stderr.take().unwrap();

        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                tracing::debug!("yt-dlp stderr: {}", line);
                if let Some(pct) = parse_progress(&line) {
                    let _ = tx_err.send(AppMsg::DownloadProgress {
                        message: line,
                        percent: pct,
                    });
                }
            }
        });

        let tx_out = tx.clone();
        let stdout = child.stdout.take().unwrap();

        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                tracing::debug!("yt-dlp stderr: {}", line);
                if let Some(pct) = parse_progress(&line) {
                    let _ = tx_out.send(AppMsg::DownloadProgress {
                        message: line,
                        percent: pct,
                    });
                }
            }
        });

        match child.wait() {
            Ok(status) if status.success() => {
                let _ = tx.send(AppMsg::DownloadFinished);
                tracing::info!("Download Success: {:?}", status);
            }
            Ok(status) => {
                let _ = tx.send(AppMsg::DownloadFailed {
                    error: format!("yt-dlp exited with {}", status),
                });
                tracing::info!("Download Failed with status: {:?}", status);
            }
            Err(e) => {
                let _ = tx.send(AppMsg::DownloadFailed {
                    error: e.to_string(),
                });
                tracing::info!("Download Failed with error: {:?}", e);
            }
        }
    });
}
