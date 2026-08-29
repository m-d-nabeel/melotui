//! The one place that decides what plays next.
//!
//! There used to be three: `handle_request_next` walked the context,
//! `handle_audio_finished` checked the play queue first and then the context,
//! and `predict_next_song` reimplemented a third variant for the "up next"
//! label. They disagreed — pressing Next skipped queued tracks that letting the
//! song end would have played. Anything that needs to know what comes next now
//! asks [`peek`], and anything that moves calls [`peek`] then [`commit`], so
//! the prediction and the action cannot drift apart.

use crate::domain::TrackId;
use crate::playback::state::{PlaybackQueueState, QueueId, RepeatMode};
use crate::state::SessionState;

/// What should play after the current track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextUp {
    /// A track the user explicitly queued. Queued tracks always win: they are a
    /// deliberate interjection into whatever is playing.
    Queued { queue_id: QueueId, track: TrackId },
    /// The next track of the listening context (album, artist, playlist…).
    Context(TrackId),
    /// The context is finished and nothing is queued.
    Exhausted,
}

impl NextUp {
    pub fn track(&self) -> Option<TrackId> {
        match self {
            NextUp::Queued { track, .. } => Some(*track),
            NextUp::Context(track) => Some(*track),
            NextUp::Exhausted => None,
        }
    }
}

/// The first queued entry that is not the one already playing.
///
/// A playing queue entry stays in the list so the queue view can highlight it,
/// so "the next queued track" means the first entry that is not `active`.
fn next_queued(queue: &PlaybackQueueState, active: Option<QueueId>) -> Option<(QueueId, TrackId)> {
    queue
        .items
        .iter()
        .find(|e| Some(e.id) != active)
        .map(|e| (e.id, e.track_id))
}

/// Decides what comes next without changing anything.
pub fn peek(session: &SessionState) -> NextUp {
    if let Some((queue_id, track)) =
        next_queued(&session.playback_queue, session.playback.active_queue_id)
    {
        return NextUp::Queued { queue_id, track };
    }

    match session.playback.context.peek_next(&session.repeat_mode) {
        Some(track) => NextUp::Context(track),
        None => NextUp::Exhausted,
    }
}

/// Applies the decision from [`peek`], moving the cursor and consuming the
/// queue entry. Returns the track to start, if any.
///
/// `peek` must be called immediately before this; passing a stale decision
/// would advance the cursor toward a track that is no longer next.
pub fn commit(session: &mut SessionState, decision: &NextUp) -> Option<TrackId> {
    match decision {
        NextUp::Queued { queue_id, track } => {
            session
                .playback_queue
                .items
                .retain(|e| e.id != *queue_id);
            session.playback.active_queue_id = Some(*queue_id);
            Some(*track)
        }
        NextUp::Context(_) => {
            session.playback.active_queue_id = None;
            session.playback.context.advance(&session.repeat_mode)
        }
        NextUp::Exhausted => None,
    }
}

/// Drops the queue entry that just finished playing.
///
/// Called before advancing so a finished interjection does not linger in the
/// list and get chosen again.
pub fn retire_active_queue_entry(session: &mut SessionState) {
    if let Some(active) = session.playback.active_queue_id.take() {
        session.playback_queue.items.retain(|e| e.id != active);
    }
}

/// Steps backwards. The play queue is forward-only — it is a list of things
/// still to come — so Previous always walks the listening context.
pub fn retreat(session: &mut SessionState) -> Option<TrackId> {
    let repeat = session.repeat_mode.clone();
    let track = session.playback.context.retreat(&repeat)?;
    session.playback.active_queue_id = None;
    Some(track)
}

/// Human-readable label for what plays next, for the "up next" display.
pub fn describe_next(session: &SessionState, resolve: impl Fn(TrackId) -> String) -> String {
    // Repeat-one overrides everything on natural end, so report it honestly.
    if session.repeat_mode == RepeatMode::Track
        && let Some(current) = session.playback.current
    {
        return resolve(current);
    }
    if session.repeat_mode == RepeatMode::PlayOnce {
        return "End of playback".into();
    }
    match peek(session) {
        NextUp::Queued { track, .. } | NextUp::Context(track) => resolve(track),
        NextUp::Exhausted => "None".into(),
    }
}
