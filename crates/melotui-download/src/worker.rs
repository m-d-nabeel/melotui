use melotui_core::AppMsg;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

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
            .arg("--no-progress")
            .arg("--output")
            .arg(format!("{}/%(title)s.%(ext)s", music_root.display()))
            .arg(format!("ytsearch1:{}", query))
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

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        for line in stderr_reader.lines().map_while(Result::ok) {
            tracing::warn!("yt-dlp stderr: {}", line);
            let _ = tx.send(AppMsg::DownloadProgress { message: line });
        }

        for line in stdout_reader.lines().map_while(Result::ok) {
            tracing::warn!("yt-dlp stdout: {}", line);
            let _ = tx.send(AppMsg::DownloadProgress { message: line });
        }

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
