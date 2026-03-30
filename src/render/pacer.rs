use std::time::Instant;

#[derive(Clone, Debug)]
pub struct TickPacer {
    tick_period_seconds: f64,
    max_catch_up_ticks: usize,
    last_frame: Option<Instant>,
    accumulator_seconds: f64,
}

impl TickPacer {
    pub fn new(tick_rate_hz: f64, max_catch_up_ticks: usize) -> Self {
        assert!(tick_rate_hz.is_finite() && tick_rate_hz > 0.0);
        Self {
            tick_period_seconds: 1.0 / tick_rate_hz,
            max_catch_up_ticks: max_catch_up_ticks.max(1),
            last_frame: None,
            accumulator_seconds: 0.0,
        }
    }

    pub fn consume_due_ticks(&mut self, now: Instant) -> usize {
        let Some(last_frame) = self.last_frame.replace(now) else {
            return 0;
        };
        let delta = now.duration_since(last_frame).as_secs_f64();
        self.accumulator_seconds = (self.accumulator_seconds + delta)
            .min(self.tick_period_seconds * self.max_catch_up_ticks as f64);

        let mut due_ticks = 0usize;
        while due_ticks < self.max_catch_up_ticks
            && self.accumulator_seconds >= self.tick_period_seconds
        {
            self.accumulator_seconds -= self.tick_period_seconds;
            due_ticks += 1;
        }
        due_ticks
    }

    pub fn interpolation_alpha(&self) -> f32 {
        (self.accumulator_seconds / self.tick_period_seconds).clamp(0.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::TickPacer;

    #[test]
    fn pacer_converts_wall_clock_time_to_due_ticks() {
        let start = Instant::now();
        let mut pacer = TickPacer::new(20.0, 4);
        assert_eq!(pacer.consume_due_ticks(start), 0);
        assert_eq!(
            pacer.consume_due_ticks(start + Duration::from_millis(120)),
            2
        );
        assert!(pacer.interpolation_alpha() > 0.0);
    }

    #[test]
    fn pacer_caps_runaway_catch_up() {
        let start = Instant::now();
        let mut pacer = TickPacer::new(60.0, 3);
        assert_eq!(pacer.consume_due_ticks(start), 0);
        assert_eq!(pacer.consume_due_ticks(start + Duration::from_secs(2)), 3);
    }
}
