// use melotui_core::AppMsg;
// use melotui_core::state_registry::DownloadControl;
use melotui_ipc::channel::inprocess::WorkerEndpoint;
// use std::io::{BufRead, BufReader};
// use std::path::PathBuf;
// use std::process::{Command, Stdio};
// use std::sync::atomic::Ordering;
// use std::sync::mpsc::Sender;
// use std::thread;

// use crate::progress::parse_progress;

pub fn run(worker_ipc: WorkerEndpoint) {
    tracing::info!("Worker Run called for yt-dlp: {:?}", worker_ipc);
}
// pub fn spawn_download(
//     query: String,
//     music_root: PathBuf,
//     tx: Sender<AppMsg>,
//     control: DownloadControl,
// ) {
//     thread::spawn(move || {
//         tracing::debug!("YT-DLP Thread Spawned");
//         let cmd = Command::new("yt-dlp")
//             .arg("-x")
//             .arg("-f")
//             .arg("bestaudio/best")
//             .arg("--extract-audio")
//             .arg("--audio-format")
//             .arg("mp3")
//             .arg("--audio-quality")
//             .arg("0")
//             .arg("--embed-thumbnail")
//             .arg("--add-metadata")
//             .arg("--output")
//             .arg(format!("{}/%(title)s.%(ext)s", music_root.display()))
//             .arg(format!("ytsearch1:{}", query))
//             .arg("--newline")
//             .stdout(Stdio::piped())
//             .stderr(Stdio::piped())
//             .spawn();
//
//         tracing::info!("CMD Created: {:?}", cmd);
//
//         let cancel_flag = control.cancel;
//         let mut child = match cmd {
//             Ok(c) => c,
//             Err(e) => {
//                 let _ = tx.send(AppMsg::DownloadFailed {
//                     error: e.to_string(),
//                 });
//                 return;
//             }
//         };
//
//         let tx_err = tx.clone();
//         let cancel_err = cancel_flag.clone();
//         let stderr = child.stderr.take().unwrap();
//
//         std::thread::spawn(move || {
//             let reader = BufReader::new(stderr);
//             for line in reader.lines().map_while(Result::ok) {
//                 if cancel_err.load(Ordering::SeqCst) {
//                     tracing::debug!("stderr reader exiting due to cancel");
//                     return;
//                 }
//
//                 tracing::debug!("yt-dlp stderr: {}", line);
//
//                 if let Some(pct) = parse_progress(&line) {
//                     let _ = tx_err.send(AppMsg::DownloadProgress { percent: pct });
//                 }
//             }
//         });
//
//         let tx_out = tx.clone();
//         let cancel_out = cancel_flag.clone();
//         let stdout = child.stdout.take().unwrap();
//
//         std::thread::spawn(move || {
//             let reader = BufReader::new(stdout);
//             for line in reader.lines().map_while(Result::ok) {
//                 if cancel_out.load(Ordering::SeqCst) {
//                     tracing::debug!("stdout reader exiting due to cancel");
//                     return;
//                 }
//
//                 tracing::debug!("yt-dlp stdout: {}", line);
//
//                 if let Some(pct) = parse_progress(&line) {
//                     let _ = tx_out.send(AppMsg::DownloadProgress { percent: pct });
//                 }
//             }
//         });
//
//         loop {
//             // Check cancellation cooperatively
//             if cancel_flag.load(Ordering::SeqCst) {
//                 tracing::info!("Cancellation requested, killing yt-dlp");
//
//                 let _ = child.kill();
//                 let _ = child.wait(); // reap process
//
//                 let _ = tx.send(AppMsg::DownloadFailed {
//                     error: "Download cancelled".into(),
//                 });
//
//                 return;
//             }
//
//             match child.try_wait() {
//                 Ok(Some(status)) => {
//                     if status.success() {
//                         let _ = tx.send(AppMsg::DownloadFinished);
//                         tracing::info!("Download Success: {:?}", status);
//                     } else {
//                         let _ = tx.send(AppMsg::DownloadFailed {
//                             error: format!("yt-dlp exited with {}", status),
//                         });
//                         tracing::info!("Download Failed with status: {:?}", status);
//                     }
//                     return;
//                 }
//                 Ok(None) => {
//                     std::thread::sleep(std::time::Duration::from_millis(100));
//                 }
//                 Err(e) => {
//                     let _ = tx.send(AppMsg::DownloadFailed {
//                         error: e.to_string(),
//                     });
//                     tracing::info!("Download Failed with error: {:?}", e);
//                     return;
//                 }
//             }
//         }
//     });
// }
