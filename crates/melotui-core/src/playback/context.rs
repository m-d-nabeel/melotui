//! The run of tracks the listener is moving through, and where they are in it.
//!
//! This type exists because a playback position cannot be represented by a
//! track id. A playlist may legitimately contain the same track more than once,
//! so "where am I" has to be an index, and every step has to be arithmetic on
//! that index rather than a search for the id that happens to be playing.
//!
//! Ordering is kept separate from content. [`tracks`](PlaybackContext::tracks)
//! holds the sequence in its natural order — playlist order, album order,
//! whatever the user picked. `order` is a permutation of indices into it: the
//! identity while playing straight through, a shuffled permutation otherwise.
//! `cursor` indexes `order`. Toggling shuffle rebuilds `order` without
//! disturbing `tracks`, so turning it off puts the listener back exactly where
//! they were in the real sequence.
//!
//! # Shuffle declustering
//!
//! `groups` is a third parallel array: an opaque bucket id per track, supplied
//! by the caller (typically "which artist", or "which album" when the whole
//! context is already one artist). A plain per-track random permutation
//! readily plays two or three tracks by the same artist back to back purely by
//! chance — the classic "this shuffle isn't shuffling" complaint. The shuffle
//! here is *group-aware*: it fills the play order the way you'd deal cards from
//! several separate decks, always preferring to draw from a deck other than the
//! one you just drew from, and only touching the same deck twice running when
//! every other deck has run out. A caller with nothing meaningful to group by
//! (or that does not care) passes one group id for every track, which makes
//! this degenerate exactly to a plain Fisher-Yates shuffle.

use rand::RngExt;
use rand::seq::SliceRandom;
use std::collections::HashMap;

use crate::domain::TrackId;
use crate::playback::state::RepeatMode;

/// An ordered listening context with a cursor.
#[derive(Debug, Clone, Default)]
pub struct PlaybackContext {
    /// Tracks in their natural order.
    tracks: Vec<TrackId>,
    /// Declustering bucket per track, parallel to `tracks`. See module docs.
    groups: Vec<u32>,
    /// Permutation of indices into `tracks`; the playback order.
    order: Vec<usize>,
    /// Position within `order`. Meaningless when `order` is empty.
    cursor: usize,
    shuffle: bool,
    /// Human-readable origin, e.g. "Artist: Radiohead".
    source_name: String,
}

impl PlaybackContext {
    /// Builds a context positioned on `start` (an index into `tracks`), with no
    /// shuffle declustering — every track shares one group, so shuffling is a
    /// plain permutation. Use [`with_groups`](Self::with_groups) when the
    /// caller can say what would feel repetitive if clustered (usually the
    /// artist).
    pub fn new(tracks: Vec<TrackId>, start: usize, source_name: String, shuffle: bool) -> Self {
        let groups = vec![0; tracks.len()];
        Self::with_groups(tracks, groups, start, source_name, shuffle)
    }

    /// Builds a context positioned on `start`, decluster-shuffling by `groups`
    /// (one bucket id per entry of `tracks`, same length) when `shuffle` is on.
    ///
    /// With shuffle on, `start` still plays first and everything else is dealt
    /// in behind it — picking a track should always play that track.
    pub fn with_groups(
        tracks: Vec<TrackId>,
        groups: Vec<u32>,
        start: usize,
        source_name: String,
        shuffle: bool,
    ) -> Self {
        // A caller passing a mismatched vector is a bug on their end, not
        // something to propagate as a panic mid-playback; fall back to
        // "everything its own group", same as `new`.
        let groups = if groups.len() == tracks.len() {
            groups
        } else {
            vec![0; tracks.len()]
        };
        let mut ctx = Self {
            order: (0..tracks.len()).collect(),
            cursor: start.min(tracks.len().saturating_sub(1)),
            tracks,
            groups,
            shuffle: false,
            source_name,
        };
        if shuffle {
            ctx.set_shuffle(true);
        }
        ctx
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// The sequence in its natural order.
    pub fn tracks(&self) -> &[TrackId] {
        &self.tracks
    }

    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    pub fn set_source_name(&mut self, name: String) {
        self.source_name = name;
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    /// The track the cursor is on.
    pub fn current(&self) -> Option<TrackId> {
        self.tracks.get(*self.order.get(self.cursor)?).copied()
    }

    /// Position in the *natural* sequence, which is what gets persisted and
    /// shown to the user. Unrelated to `cursor` once shuffle is on.
    pub fn natural_position(&self) -> usize {
        self.order.get(self.cursor).copied().unwrap_or(0)
    }

    /// Cursor position after one step forward, or `None` at the end.
    fn step(&self, forward: bool, repeat: &RepeatMode) -> Option<usize> {
        if self.order.is_empty() {
            return None;
        }
        let wraps = *repeat == RepeatMode::Playlist;
        if forward {
            if self.cursor + 1 < self.order.len() {
                Some(self.cursor + 1)
            } else if wraps {
                Some(0)
            } else {
                None
            }
        } else if self.cursor > 0 {
            Some(self.cursor - 1)
        } else if wraps {
            Some(self.order.len() - 1)
        } else {
            None
        }
    }

    /// Whether calling `advance` would produce a track at all.
    ///
    /// Deliberately separate from [`peek_next`](Self::peek_next): *whether*
    /// something plays next never depends on shuffle (a wrap under
    /// `RepeatMode::Playlist` always has something to deal, reshuffled or
    /// not) — only *which* track does. Conflating the two here previously
    /// made a real `advance()` call get skipped entirely at a shuffle wrap,
    /// because the caller mistook "I can't predict which track" for "there is
    /// no next track" and stopped instead of continuing.
    pub fn has_next(&self, repeat: &RepeatMode) -> bool {
        self.step(true, repeat).is_some()
    }

    /// What `advance` would return, without moving — for *display* only (the
    /// "up next" label). Keeps that label honest by construction when it can
    /// be: it cannot drift from what actually plays, because it is never
    /// asked to guess.
    ///
    /// Returns `None` both when nothing will play next and when a shuffle
    /// wrap makes the exact track genuinely unknowable ahead of time. Callers
    /// that need to know *whether* to advance at all — as opposed to what
    /// label to print — must use [`has_next`](Self::has_next) instead, never
    /// this method; see its docs for why.
    pub fn peek_next(&self, repeat: &RepeatMode) -> Option<TrackId> {
        let next = self.step(true, repeat)?;
        if self.shuffle && next == 0 && self.cursor + 1 >= self.order.len() {
            return None;
        }
        self.tracks.get(self.order[next]).copied()
    }

    pub fn peek_prev(&self, repeat: &RepeatMode) -> Option<TrackId> {
        let prev = self.step(false, repeat)?;
        self.tracks.get(self.order[prev]).copied()
    }

    /// Moves forward one track. Returns `None` at the end of the run when the
    /// repeat mode does not wrap.
    pub fn advance(&mut self, repeat: &RepeatMode) -> Option<TrackId> {
        let next = self.step(true, repeat)?;
        // Wrapping under shuffle deals a fresh permutation, otherwise repeat
        // would replay one fixed order forever.
        if next == 0 && self.cursor + 1 >= self.order.len() && self.shuffle {
            let last = self.order.get(self.cursor).copied();
            self.reshuffle_avoiding(last);
        }
        self.cursor = next;
        self.current()
    }

    pub fn retreat(&mut self, repeat: &RepeatMode) -> Option<TrackId> {
        let prev = self.step(false, repeat)?;
        self.cursor = prev;
        self.current()
    }

    /// Moves the cursor onto a natural index, keeping the current order.
    pub fn seek_to_natural(&mut self, natural: usize) -> Option<TrackId> {
        let pos = self.order.iter().position(|&i| i == natural)?;
        self.cursor = pos;
        self.current()
    }

    /// Turns shuffle on or off without interrupting what is playing.
    ///
    /// On: the current track is dealt to the front and the rest are dealt in
    /// behind it, decluster-shuffled by group and continuing the "don't repeat
    /// the last group" rule from the track that is already playing. Off: the
    /// natural order is restored and the cursor lands on wherever the current
    /// track really sits.
    pub fn set_shuffle(&mut self, shuffle: bool) {
        if shuffle == self.shuffle {
            return;
        }
        self.shuffle = shuffle;
        let current_natural = self.natural_position();

        if shuffle {
            let seed_group = self.groups.get(current_natural).copied();
            let (rest_indices, rest_groups): (Vec<usize>, Vec<u32>) = (0..self.tracks.len())
                .filter(|&i| i != current_natural)
                .map(|i| (i, self.groups[i]))
                .unzip();
            let dealt = Self::decluster_order(&rest_groups, seed_group);
            let rest = dealt.into_iter().map(|pos| rest_indices[pos]);
            self.order = std::iter::once(current_natural).chain(rest).collect();
            self.cursor = 0;
        } else {
            self.order = (0..self.tracks.len()).collect();
            self.cursor = current_natural.min(self.order.len().saturating_sub(1));
        }
    }

    /// Deals a fresh decluster-shuffled permutation for a wrap, trying not to
    /// open with the same group the run just ended on and, above all, never
    /// with the exact track that just played — an immediate repeat is the one
    /// thing shuffle must never do, regardless of grouping.
    fn reshuffle_avoiding(&mut self, just_played: Option<usize>) {
        let seed_group = just_played.and_then(|i| self.groups.get(i).copied());
        let mut order = Self::decluster_order(&self.groups, seed_group);
        if let Some(last) = just_played
            && order.len() > 1
            && order[0] == last
        {
            let swap_with = rand::rng().random_range(1..order.len());
            order.swap(0, swap_with);
        }
        self.order = order;
    }

    /// Deals every index of `groups` into a full permutation, preferring at
    /// each step a group other than the previous pick (starting from
    /// `seed_last_group`), and weighting the choice among eligible groups by
    /// how many of their tracks are still undealt — so a large bucket (an
    /// artist with a big discography) is spread across the whole run instead
    /// of being forced entirely to the tail once smaller buckets empty out.
    ///
    /// Falls back to dealing from the previous group when it is the only one
    /// left with anything undealt, which is also what makes this degenerate to
    /// a plain shuffle when every track shares one group.
    fn decluster_order(groups: &[u32], seed_last_group: Option<u32>) -> Vec<usize> {
        let mut rng = rand::rng();

        let mut buckets: HashMap<u32, Vec<usize>> = HashMap::new();
        for (i, &g) in groups.iter().enumerate() {
            buckets.entry(g).or_default().push(i);
        }
        for bucket in buckets.values_mut() {
            bucket.shuffle(&mut rng);
        }

        let mut order = Vec::with_capacity(groups.len());
        let mut last_group = seed_last_group;

        while order.len() < groups.len() {
            let mut eligible: Vec<u32> = buckets
                .iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(&g, _)| g)
                .collect();
            let different: Vec<u32> = eligible
                .iter()
                .copied()
                .filter(|g| Some(*g) != last_group)
                .collect();
            if !different.is_empty() {
                eligible = different;
            }

            let total: usize = eligible.iter().map(|g| buckets[g].len()).sum();
            let mut pick = rng.random_range(0..total);
            let mut chosen = eligible[0];
            for &g in &eligible {
                let n = buckets[&g].len();
                if pick < n {
                    chosen = g;
                    break;
                }
                pick -= n;
            }

            // Popping is safe: `chosen` was just confirmed non-empty above.
            order.push(buckets.get_mut(&chosen).unwrap().pop().unwrap());
            last_group = Some(chosen);
        }

        order
    }

    /// Rebuilds a context from persisted state, dropping tracks that have since
    /// left the library and keeping the cursor on `active` if it survived. No
    /// shuffle declustering; see [`restore_with_groups`](Self::restore_with_groups).
    pub fn restore(
        tracks: Vec<TrackId>,
        persisted_position: usize,
        active: Option<TrackId>,
        source_name: String,
        shuffle: bool,
    ) -> Self {
        let groups = vec![0; tracks.len()];
        Self::restore_with_groups(
            tracks,
            groups,
            persisted_position,
            active,
            source_name,
            shuffle,
        )
    }

    /// [`restore`](Self::restore), decluster-shuffling by `groups` when
    /// `shuffle` is on.
    pub fn restore_with_groups(
        tracks: Vec<TrackId>,
        groups: Vec<u32>,
        persisted_position: usize,
        active: Option<TrackId>,
        source_name: String,
        shuffle: bool,
    ) -> Self {
        // Prefer the track we know was playing: filtering may have shifted
        // every index, which would silently move the cursor.
        let start = active
            .and_then(|id| tracks.iter().position(|&t| t == id))
            .unwrap_or_else(|| persisted_position.min(tracks.len().saturating_sub(1)));
        Self::with_groups(tracks, groups, start, source_name, shuffle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: u64) -> Vec<TrackId> {
        (1..=n).map(TrackId).collect()
    }

    #[test]
    fn walks_a_list_with_duplicates_without_getting_stuck() {
        // A, B, C, A, B -- the shape that used to trap the cursor in a 3-cycle.
        let tracks = vec![TrackId(1), TrackId(2), TrackId(3), TrackId(1), TrackId(2)];
        let mut ctx = PlaybackContext::new(tracks, 0, "P".into(), false);

        let mut seen = vec![ctx.current().unwrap()];
        while let Some(t) = ctx.advance(&RepeatMode::Off) {
            seen.push(t);
        }

        assert_eq!(
            seen,
            vec![TrackId(1), TrackId(2), TrackId(3), TrackId(1), TrackId(2)],
            "every entry must be visited, including the repeated ones"
        );
        assert_eq!(ctx.natural_position(), 4, "cursor ends on the last entry");
    }

    #[test]
    fn stops_at_the_end_unless_repeat_wraps() {
        let mut ctx = PlaybackContext::new(ids(3), 2, "P".into(), false);
        assert_eq!(ctx.advance(&RepeatMode::Off), None);
        assert_eq!(ctx.current(), Some(TrackId(3)), "cursor must not move");

        assert_eq!(ctx.advance(&RepeatMode::Playlist), Some(TrackId(1)));
    }

    #[test]
    fn retreat_mirrors_advance() {
        let mut ctx = PlaybackContext::new(ids(3), 0, "P".into(), false);
        assert_eq!(ctx.retreat(&RepeatMode::Off), None);
        assert_eq!(ctx.retreat(&RepeatMode::Playlist), Some(TrackId(3)));
        assert_eq!(ctx.retreat(&RepeatMode::Off), Some(TrackId(2)));
    }

    #[test]
    fn peek_agrees_with_advance() {
        let mut ctx = PlaybackContext::new(ids(4), 0, "P".into(), false);
        for _ in 0..3 {
            let peeked = ctx.peek_next(&RepeatMode::Off);
            let taken = ctx.advance(&RepeatMode::Off);
            assert_eq!(peeked, taken, "'up next' must be what actually plays");
        }
    }

    #[test]
    fn shuffle_plays_everything_exactly_once_per_pass() {
        let mut ctx = PlaybackContext::new(ids(50), 0, "P".into(), true);
        let mut seen = vec![ctx.current().unwrap()];
        while let Some(t) = ctx.advance(&RepeatMode::Off) {
            seen.push(t);
        }
        let mut sorted = seen.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 50, "a shuffle pass must cover the whole list");
        assert_eq!(seen.len(), 50, "and must not repeat within the pass");
    }

    #[test]
    fn shuffle_starts_on_the_chosen_track() {
        for _ in 0..20 {
            let ctx = PlaybackContext::new(ids(20), 7, "P".into(), true);
            assert_eq!(
                ctx.current(),
                Some(TrackId(8)),
                "picking a track must play that track, shuffle or not"
            );
        }
    }

    #[test]
    fn shuffle_never_repeats_a_track_back_to_back_across_a_wrap() {
        for _ in 0..200 {
            let mut ctx = PlaybackContext::new(ids(8), 0, "P".into(), true);
            let mut prev = ctx.current().unwrap();
            for _ in 0..40 {
                let next = ctx.advance(&RepeatMode::Playlist).unwrap();
                assert_ne!(
                    next, prev,
                    "shuffle must never play the same track twice running"
                );
                prev = next;
            }
        }
    }

    #[test]
    fn toggling_shuffle_keeps_the_current_track_and_restores_position() {
        let mut ctx = PlaybackContext::new(ids(10), 0, "P".into(), false);
        ctx.advance(&RepeatMode::Off);
        ctx.advance(&RepeatMode::Off);
        let playing = ctx.current().unwrap();
        assert_eq!(ctx.natural_position(), 2);

        ctx.set_shuffle(true);
        assert_eq!(
            ctx.current(),
            Some(playing),
            "shuffling must not skip the track"
        );

        ctx.set_shuffle(false);
        assert_eq!(ctx.current(), Some(playing));
        assert_eq!(
            ctx.natural_position(),
            2,
            "unshuffling returns to the real position in the list"
        );
    }

    #[test]
    fn restore_follows_the_active_track_when_indices_shift() {
        // Two tracks were removed from the library since the session was saved,
        // so the persisted index no longer points at the right entry.
        let surviving = vec![TrackId(5), TrackId(6), TrackId(7)];
        let ctx = PlaybackContext::restore(surviving, 4, Some(TrackId(7)), "P".into(), false);
        assert_eq!(ctx.current(), Some(TrackId(7)));
        assert_eq!(ctx.natural_position(), 2);
    }

    #[test]
    fn empty_context_is_inert() {
        let mut ctx = PlaybackContext::default();
        assert!(ctx.is_empty());
        assert_eq!(ctx.current(), None);
        assert_eq!(ctx.advance(&RepeatMode::Playlist), None);
        assert_eq!(ctx.retreat(&RepeatMode::Playlist), None);
        assert_eq!(ctx.peek_next(&RepeatMode::Playlist), None);
    }

    /// 4 groups of 5 tracks each: group `i` holds track ids `i*5 + 1 ..= i*5 + 5`.
    fn grouped(groups_count: u32, per_group: u64) -> (Vec<TrackId>, Vec<u32>) {
        let mut tracks = Vec::new();
        let mut groups = Vec::new();
        for g in 0..groups_count {
            for n in 1..=per_group {
                tracks.push(TrackId(u64::from(g) * per_group + n));
                groups.push(g);
            }
        }
        (tracks, groups)
    }

    fn group_of(groups: &[u32], tracks: &[TrackId], id: TrackId) -> u32 {
        let pos = tracks.iter().position(|&t| t == id).unwrap();
        groups[pos]
    }

    #[test]
    fn declustered_shuffle_avoids_the_same_group_back_to_back() {
        // 6 artists x 8 tracks. A plain per-track shuffle would routinely play
        // two tracks by the same artist in a row purely by chance; declustering
        // must not, except when forced (only one artist has anything left).
        let (tracks, groups) = grouped(6, 8);
        for _ in 0..20 {
            let mut ctx =
                PlaybackContext::with_groups(tracks.clone(), groups.clone(), 0, "P".into(), true);
            let mut prev_group = group_of(&groups, &tracks, ctx.current().unwrap());
            let mut forced_repeats = 0;
            let mut consecutive_same_group = 0;

            for _ in 0..47 {
                let track = ctx.advance(&RepeatMode::Off).unwrap();
                let g = group_of(&groups, &tracks, track);
                if g == prev_group {
                    consecutive_same_group += 1;
                    forced_repeats += 1;
                }
                prev_group = g;
            }

            // With 6 groups of 8, a same-group repeat can only be forced once
            // every group but one is exhausted — a handful of times per full
            // pass at most, never routinely.
            assert!(
                forced_repeats <= 5,
                "{consecutive_same_group} same-group repeats in one pass is too many for 6 groups"
            );
        }
    }

    #[test]
    fn declustering_still_visits_every_track_exactly_once_per_pass() {
        let (tracks, groups) = grouped(5, 9);
        let mut ctx = PlaybackContext::with_groups(tracks, groups, 0, "P".into(), true);

        let mut seen = vec![ctx.current().unwrap()];
        for _ in 0..44 {
            seen.push(ctx.advance(&RepeatMode::Off).unwrap());
        }
        assert_eq!(ctx.advance(&RepeatMode::Off), None);

        let mut sorted = seen.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            45,
            "a pass must still cover the whole list exactly once"
        );
    }

    #[test]
    fn a_lopsided_group_is_spread_across_the_run_not_dumped_at_the_end() {
        // One artist with 40 tracks against three with 2 each: the big bucket
        // must not end up as one long unbroken tail once the small ones empty.
        let mut tracks = Vec::new();
        let mut groups = Vec::new();
        for n in 1..=40u64 {
            tracks.push(TrackId(n));
            groups.push(0u32);
        }
        for g in 1..=3u32 {
            for n in 1..=2u64 {
                tracks.push(TrackId(1000 * u64::from(g) + n));
                groups.push(g);
            }
        }

        let mut ctx =
            PlaybackContext::with_groups(tracks.clone(), groups.clone(), 0, "P".into(), true);
        let mut positions_of_group_0 = Vec::new();
        for i in 0..46 {
            let g = group_of(&groups, &tracks, ctx.current().unwrap());
            if g == 0 {
                positions_of_group_0.push(i);
            }
            if i < 45 {
                ctx.advance(&RepeatMode::Off);
            }
        }

        // If group 0 were dumped at the tail, its positions would cluster in
        // the back half. Assert its first appearance is early, proving it was
        // dealt in from the start rather than saved for last.
        assert!(
            positions_of_group_0[0] <= 2,
            "the 40-track group must appear near the start, not be held back; \
             first appeared at position {}",
            positions_of_group_0[0]
        );
    }

    #[test]
    fn no_grouping_information_degenerates_to_plain_shuffle() {
        // Every track in its own group (all distinct ids): declustering can
        // never be "forced" into a repeat except on the very last pick, so this
        // must behave identically to `new`'s ungrouped shuffle.
        let tracks: Vec<TrackId> = (1..=30).map(TrackId).collect();
        let groups: Vec<u32> = (0..30).collect();
        let mut ctx = PlaybackContext::with_groups(tracks, groups, 0, "P".into(), true);

        let mut seen = vec![ctx.current().unwrap()];
        while let Some(t) = ctx.advance(&RepeatMode::Off) {
            seen.push(t);
        }
        let mut sorted = seen.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 30);
    }

    #[test]
    fn mismatched_group_length_falls_back_safely_instead_of_panicking() {
        let tracks: Vec<TrackId> = (1..=10).map(TrackId).collect();
        let ctx = PlaybackContext::with_groups(tracks, vec![0, 1, 2], 0, "P".into(), true);
        assert_eq!(ctx.len(), 10);
        assert!(ctx.current().is_some());
    }
}
