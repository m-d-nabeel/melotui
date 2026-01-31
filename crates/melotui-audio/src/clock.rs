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
        // Commit time spent at the old speed before changing
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
