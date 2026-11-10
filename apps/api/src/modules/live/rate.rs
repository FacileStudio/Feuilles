use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// At most `per_second` events in any one-second window.
#[derive(Debug, Clone)]
pub struct Rate {
    per_second: usize,
    recent: VecDeque<Instant>,
}

impl Rate {
    /// A window allowing `per_second` events.
    pub fn new(per_second: usize) -> Rate {
        Rate { per_second, recent: VecDeque::with_capacity(per_second) }
    }

    /// Counts one event at `now`, or refuses it when the window is full.
    pub fn allow(&mut self, now: Instant) -> bool {
        while self.recent.front().is_some_and(|at| now.duration_since(*at) >= Duration::from_secs(1)) {
            self.recent.pop_front();
        }
        if self.recent.len() >= self.per_second {
            return false;
        }
        self.recent.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixty_per_second_then_refused_until_the_window_moves() {
        let mut rate = Rate::new(60);
        let start = Instant::now();
        assert!((0..60).all(|i| rate.allow(start + Duration::from_millis(i))));
        assert!(!rate.allow(start + Duration::from_millis(500)));
        assert!(rate.allow(start + Duration::from_millis(1000)));
        assert!(!rate.allow(start + Duration::from_millis(1000)));
    }
}
