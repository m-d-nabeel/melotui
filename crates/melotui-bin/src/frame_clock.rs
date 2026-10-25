//! When the spectrum's frames are due.
//!
//! Frames come on a schedule of their own, one every `1 / fps`, rather than
//! a frame's time after the last one was drawn and again at every key: that
//! ran slower than asked, by however long drawing took, and unevenly.

use std::time::{Duration, Instant};

pub struct FrameClock {
    next: Instant,
    last: Instant,
}

impl FrameClock {
    pub fn new(now: Instant) -> Self {
        Self {
            next: now,
            last: now,
        }
    }

    /// How long until the next frame is due.
    pub fn wait(&self, now: Instant) -> Duration {
        self.next.saturating_duration_since(now)
    }

    /// If a frame is due at `now`, the seconds since the last one; the next
    /// is then due `every` after this one was. A frame that comes late is
    /// not made up for with a burst of quick ones.
    pub fn tick(&mut self, now: Instant, every: Duration) -> Option<f32> {
        if now < self.next {
            return None;
        }
        let dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        self.next += every;
        if self.next <= now {
            self.next = now + every;
        }
        Some(dt)
    }

    /// Nothing is moving: the first frame once something does is due at
    /// once, and measured from then.
    pub fn rest(&mut self, now: Instant) {
        self.next = now;
        self.last = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERY: Duration = Duration::from_micros(16_667);

    #[test]
    fn frames_come_evenly_however_long_drawing_takes() {
        let start = Instant::now();
        let mut clock = FrameClock::new(start);
        let mut now = start;
        let mut drawn = Vec::new();
        for _ in 0..600 {
            now += clock.wait(now);
            if clock.tick(now, EVERY).is_some() {
                drawn.push(now);
            }
            // Drawing and everything else take 5 ms.
            now += Duration::from_millis(5);
        }
        let gaps: Vec<Duration> = drawn.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(
            gaps.iter().all(|&g| g == EVERY),
            "uneven frames: {:?}",
            &gaps[..5]
        );
    }

    #[test]
    fn a_late_frame_is_not_made_up_for_with_a_burst() {
        let start = Instant::now();
        let mut clock = FrameClock::new(start);
        clock.tick(start, EVERY);
        // Held up for ten frames' time.
        let late = start + EVERY * 10;
        let dt = clock.tick(late, EVERY).unwrap();
        assert!((dt - 10.0 * EVERY.as_secs_f32()).abs() < 1e-4);
        assert!(clock.tick(late + Duration::from_millis(1), EVERY).is_none());
        assert_eq!(clock.wait(late), EVERY);
    }

    #[test]
    fn after_resting_the_first_frame_is_due_at_once() {
        let start = Instant::now();
        let mut clock = FrameClock::new(start);
        clock.tick(start, EVERY);
        let later = start + Duration::from_secs(30);
        clock.rest(later);
        assert_eq!(clock.wait(later), Duration::ZERO);
        let dt = clock.tick(later, EVERY).unwrap();
        assert_eq!(dt, 0.0, "not thirty seconds of motion at once");
    }
}
