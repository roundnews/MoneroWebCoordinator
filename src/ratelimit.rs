use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    window: Duration,
    max_count: u32,
    timestamps: VecDeque<Instant>,
}

impl RateLimiter {
    pub fn new(max_count: u32, window_secs: u64) -> Self {
        Self {
            window: Duration::from_secs(window_secs),
            max_count,
            timestamps: VecDeque::with_capacity(max_count as usize + 1),
        }
    }

    pub fn check(&mut self) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window;

        // Remove old timestamps
        while self.timestamps.front().map_or(false, |&t| t < cutoff) {
            self.timestamps.pop_front();
        }

        if self.timestamps.len() >= self.max_count as usize {
            return false;
        }

        self.timestamps.push_back(now);
        true
    }

    pub fn remaining(&self) -> u32 {
        self.max_count.saturating_sub(self.timestamps.len() as u32)
    }
}

pub struct SessionLimits {
    pub messages: RateLimiter,
    pub submits: RateLimiter,
}

impl SessionLimits {
    pub fn new(messages_per_second: u32, submits_per_minute: u32) -> Self {
        Self {
            messages: RateLimiter::new(messages_per_second, 1),
            submits: RateLimiter::new(submits_per_minute, 60),
        }
    }
}
