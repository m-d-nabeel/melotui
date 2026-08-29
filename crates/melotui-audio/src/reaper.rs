//! Off-thread deallocation for playing sources.
//!
//! Rodio pulls samples on the cpal callback thread, which means that thread also
//! owns the source and *drops* it when the sink advances. Our source is not cheap
//! to drop: it holds the whole fundsp graph (reverb tank, four ping-pong delay
//! lines up to 1 s, the echo delays) plus symphonia's decoder — on the order of a
//! megabyte spread across dozens of separate allocations. Running that many
//! `free()` calls inside the audio callback can block on the allocator's arena
//! lock, which is exactly the kind of stall that shows up as a click on track
//! change.
//!
//! [`ReapOnDrop`] hands the corpse to a background thread instead. The channel is
//! bounded, so its slots are allocated up front and the handoff is a single
//! pointer move — nothing is allocated, freed, or locked on the audio thread.

use rodio::Source;
use std::{
    sync::mpsc::{self, SyncSender, TrySendError},
    thread,
    time::Duration,
};

/// A boxed source. Boxing happens on the worker thread before playback starts,
/// so reaping only ever moves a pointer.
pub type BoxedSource = Box<dyn Source + Send>;

/// Sources that may be awaiting collection at once. Overflowing this needs more
/// track changes than the collector can drain, which a human cannot produce; if
/// it ever happens we drop inline, which is just the old behaviour.
const GRAVE_CAPACITY: usize = 8;

/// Handle used to retire sources. Cloned onto each new source on the audio
/// *worker* thread, never on the callback thread.
#[derive(Clone)]
pub struct Reaper {
    grave: SyncSender<BoxedSource>,
}

impl Reaper {
    /// Spawns the collector thread.
    pub fn spawn() -> Self {
        // Bounded: the slot storage is allocated here, once, so `try_send` on
        // the audio thread never has to.
        let (grave, corpses) = mpsc::sync_channel::<BoxedSource>(GRAVE_CAPACITY);

        thread::Builder::new()
            .name("melotui-reaper".into())
            .spawn(move || {
                // Ends when the last sender is dropped, i.e. at shutdown.
                for corpse in corpses {
                    drop(corpse);
                }
                tracing::debug!("Reaper thread exited");
            })
            .expect("spawn reaper thread");

        Self { grave }
    }

    /// Wraps a source so that dropping it frees off-thread.
    pub fn guard(&self, inner: BoxedSource) -> ReapOnDrop {
        ReapOnDrop {
            inner: Some(inner),
            grave: self.grave.clone(),
        }
    }
}

/// Transparent source wrapper whose `Drop` moves the payload off the audio thread.
pub struct ReapOnDrop {
    inner: Option<BoxedSource>,
    grave: SyncSender<BoxedSource>,
}

impl ReapOnDrop {
    #[inline]
    fn inner(&self) -> &BoxedSource {
        self.inner.as_ref().expect("source taken before drop")
    }

    #[inline]
    fn inner_mut(&mut self) -> &mut BoxedSource {
        self.inner.as_mut().expect("source taken before drop")
    }
}

impl Drop for ReapOnDrop {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            // On `Full` or `Disconnected` the box is handed back and drops here,
            // which is the pre-existing behaviour — never a leak.
            if let Err(TrySendError::Full(corpse) | TrySendError::Disconnected(corpse)) =
                self.grave.try_send(inner)
            {
                drop(corpse);
            }
        }
    }
}

impl Iterator for ReapOnDrop {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        self.inner_mut().next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner().size_hint()
    }
}

impl Source for ReapOnDrop {
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        self.inner().current_span_len()
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.inner().channels()
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.inner().sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.inner().total_duration()
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.inner_mut().try_seek(pos)
    }
}
