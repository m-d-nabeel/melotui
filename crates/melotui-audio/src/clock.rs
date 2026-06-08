use std::time::{Duration, Instant};

pub struct TimeKeeper {
    last_resume_time: Option<Instant>,
    accumulated_time: Duration,
    pub current_speed: f32,
    pub is_playing: bool,
}

impl TimeKeeper {
    pub fn new() -> Self {
        Self {
            last_resume_time: None,
            accumulated_time: Duration::ZERO,
            current_speed: 1.0,
            is_playing: false,
        }
    }

    pub fn start(&mut self) {
        if !self.is_playing {
            self.last_resume_time = Some(Instant::now());
            self.is_playing = true;
        }
    }

    pub fn pause(&mut self) {
        if self.is_playing {
            self.commit_time();
            self.last_resume_time = None;
            self.is_playing = false;
        }
    }

    pub fn stop(&mut self) {
        self.is_playing = false;
        self.accumulated_time = Duration::ZERO;
        self.last_resume_time = None;
    }

    pub fn seek(&mut self, position: Duration) {
        self.accumulated_time = position;
        if self.is_playing {
            self.last_resume_time = Some(Instant::now());
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        // commit time at old speed
        if self.is_playing {
            self.commit_time();
            self.last_resume_time = Some(Instant::now());
        }
        self.current_speed = speed;
    }

    pub fn get_position(&self) -> Duration {
        if let Some(start) = self.last_resume_time {
            let elapsed = start.elapsed().as_secs_f32();
            self.accumulated_time + Duration::from_secs_f32(elapsed * self.current_speed)
        } else {
            self.accumulated_time
        }
    }

    fn commit_time(&mut self) {
        if let Some(start) = self.last_resume_time {
            let elapsed = start.elapsed().as_secs_f32();
            self.accumulated_time += Duration::from_secs_f32(elapsed * self.current_speed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_timekeeper_basic_flow() {
        let mut clock = TimeKeeper::new();
        assert_eq!(clock.get_position(), Duration::ZERO);
        assert!(!clock.is_playing);

        clock.start();
        assert!(clock.is_playing);
        sleep(Duration::from_millis(50));
        let pos1 = clock.get_position();
        assert!(pos1 > Duration::ZERO);

        clock.pause();
        assert!(!clock.is_playing);
        let pos2 = clock.get_position();
        sleep(Duration::from_millis(50));
        let pos3 = clock.get_position();
        assert_eq!(pos2, pos3, "Position should not change while paused");

        clock.stop();
        assert_eq!(clock.get_position(), Duration::ZERO);
    }

    #[test]
    fn test_timekeeper_speed() {
        let mut clock = TimeKeeper::new();
        clock.start();
        clock.set_speed(2.0);
        sleep(Duration::from_millis(50));
        clock.pause();
        
        // At 2x speed, 50ms real time is ~100ms accumulated
        let pos = clock.get_position().as_millis();
        assert!(pos >= 80 && pos <= 150, "Expected around 100ms, got {}ms", pos);
    }

    #[test]
    fn test_timekeeper_seek() {
        let mut clock = TimeKeeper::new();
        clock.seek(Duration::from_secs(10));
        assert_eq!(clock.get_position(), Duration::from_secs(10));

        clock.start();
        sleep(Duration::from_millis(50));
        assert!(clock.get_position() > Duration::from_secs(10));
    }
}
