use melotui_bus::{
    channel::{IpcWorker, inprocess::WorkerEndpoint},
    command::CoverCommand,
    event::IpcEvent,
};
use melotui_core::message::cover::CoverContext;
use std::time::Instant;

pub fn run(worker: WorkerEndpoint<CoverCommand>, max_cover_resolution: u32) {
    tracing::info!("Cover worker started");
    let cache = crate::cache::CoverCache::new(max_cover_resolution);

    let mut pending_playback: Option<CoverCommand> = None;
    let mut pending_info: Option<CoverCommand> = None;

    while let Some(msg) = worker.recv() {
        if matches!(msg, CoverCommand::Shutdown) {
            break;
        }

        update_pending(&mut pending_playback, &mut pending_info, msg);

        // drain messages to skip ahead
        while let Some(msg) = worker.try_recv() {
            if matches!(msg, CoverCommand::Shutdown) {
                return;
            }
            update_pending(&mut pending_playback, &mut pending_info, msg);
        }

        // prioritize playback over info
        if let Some(cmd) = pending_playback.take() {
            process_load(&worker, cmd, &cache);
        }

        if let Some(cmd) = pending_info.take() {
            process_load(&worker, cmd, &cache);
        }
    }

    tracing::info!("Cover worker shutting down");
}

fn update_pending(
    playback: &mut Option<CoverCommand>,
    info: &mut Option<CoverCommand>,
    cmd: CoverCommand,
) {
    if let CoverCommand::Load { context, .. } = &cmd {
        match context {
            CoverContext::Playback => *playback = Some(cmd),
            CoverContext::Info => *info = Some(cmd),
        }
    }
}

fn process_load(
    worker: &WorkerEndpoint<CoverCommand>,
    cmd: CoverCommand,
    cache: &crate::cache::CoverCache,
) {
    if let CoverCommand::Load { id, path, context } = cmd {
        let start = Instant::now();
        let load_result = cache.get_or_load(&path);

        match load_result {
            Some(img) => {
                let cache_path = cache.resolve_cache_path(&path);
                tracing::debug!(
                    "Loaded cover for {:?} in {:?} ({:?})",
                    id,
                    start.elapsed(),
                    context
                );
                worker.send(IpcEvent::CoverLoaded {
                    id,
                    cover: Some(img),
                    context,
                    cache_path,
                });
            }
            None => {
                tracing::debug!("No cover found for {:?} ({:?})", path, context);
                worker.send(IpcEvent::CoverLoaded {
                    id,
                    cover: None,
                    context,
                    cache_path: None,
                });
            }
        }
    }
}
