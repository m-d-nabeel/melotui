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
    /// The next track of the listening context (album, artist, playlist…),
    /// known ahead of time.
    Context(TrackId),
    /// Something in the context will play next — `commit` will produce a
    /// track — but which one isn't knowable until it happens: this is a
    /// shuffle wrap, which deals a fresh permutation as part of advancing.
    /// Distinct from `Exhausted`, which this must never be treated as:
    /// `commit` still needs to run for this variant, it just can't be
    /// previewed for a label first.
    ContextUnknown,
    /// The context is finished and nothing is queued.
    Exhausted,
}

impl NextUp {
    pub fn track(&self) -> Option<TrackId> {
        match self {
            NextUp::Queued { track, .. } => Some(*track),
            NextUp::Context(track) => Some(*track),
            NextUp::ContextUnknown | NextUp::Exhausted => None,
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
///
/// The existence check (`has_next`) and the label lookup (`peek_next`) are
/// deliberately separate calls against the context — see their docs. Using
/// `peek_next` alone here previously meant a shuffle wrap, which `peek_next`
/// can't preview a track for, was indistinguishable from there being nothing
/// left at all, so `commit` never ran and Next silently stopped working the
/// moment a shuffled pass reached its end.
pub fn peek(session: &SessionState) -> NextUp {
    if let Some((queue_id, track)) =
        next_queued(&session.playback_queue, session.playback.active_queue_id)
    {
        return NextUp::Queued { queue_id, track };
    }

    let context = &session.playback.context;
    if !context.has_next(&session.repeat_mode) {
        return NextUp::Exhausted;
    }
    match context.peek_next(&session.repeat_mode) {
        Some(track) => NextUp::Context(track),
        None => NextUp::ContextUnknown,
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
            session.playback_queue.items.retain(|e| e.id != *queue_id);
            session.playback.active_queue_id = Some(*queue_id);
            Some(*track)
        }
        NextUp::Context(_) | NextUp::ContextUnknown => {
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
        // A shuffled pass is ending: something will play, but which track is
        // dealt fresh as part of advancing, so there is nothing honest to
        // resolve a title for yet.
        NextUp::ContextUnknown => "Shuffling…".into(),
        NextUp::Exhausted => "None".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playback::context::PlaybackContext;

    fn session_with_shuffled_context(ids: &[u64]) -> SessionState {
        let mut session = SessionState::default();
        let tracks: Vec<TrackId> = ids.iter().copied().map(TrackId).collect();
        session.playback.context = PlaybackContext::new(tracks, 0, "Test".into(), true);
        session.playback.current = session.playback.context.current();
        session.repeat_mode = RepeatMode::Playlist;
        session
    }

    #[test]
    fn peek_then_commit_wraps_a_shuffled_playlist_instead_of_reporting_exhausted() {
        // `peek_next` legitimately can't preview a track across a shuffle
        // wrap — but `peek`/`commit` still must produce one, because
        // RepeatMode::Playlist means the run never really ends. Using
        // `peek_next` as the *existence* check instead of `has_next` made
        // this indistinguishable from there being nothing left, so Next
        // stopped working the instant a shuffled pass reached its end.
        let mut session = session_with_shuffled_context(&[1, 2, 3, 4, 5]);

        // Walk to the end of the one pass.
        for _ in 0..4 {
            let decision = peek(&session);
            assert_ne!(decision, NextUp::Exhausted, "must not exhaust mid-pass");
            let track = commit(&mut session, &decision).expect("a track within the pass");
            session.playback.current = Some(track);
        }

        // One more step: this is the wrap. peek_next cannot name the track,
        // but peek/commit must still produce one.
        let decision = peek(&session);
        assert_eq!(
            decision,
            NextUp::ContextUnknown,
            "the wrap target is genuinely unpredictable, which must read as \
             'something plays, unknown what' — not as Exhausted"
        );
        let track = commit(&mut session, &decision);
        assert!(
            track.is_some(),
            "commit must actually deal a track at the wrap, not silently do nothing"
        );
    }
}
