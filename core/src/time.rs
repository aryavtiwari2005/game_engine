use std::time::{Duration, Instant};

pub struct EngineClock {
    last_frame_time: Instant,
    accumulated_time: Duration,
    fixed_timestep: Duration,
    delta_time: Duration,
}

impl EngineClock {
    pub fn new(fixed_fps: u32) -> Self {
        Self {
            last_frame_time: Instant::now(),
            accumulated_time: Duration::ZERO,
            fixed_timestep: Duration::from_secs_f64(1.0 / fixed_fps as f64),
            delta_time: Duration::ZERO,
        }
    }

    /// Updates the clock and returns the elapsed delta time in seconds.
    pub fn tick(&mut self) -> f64 {
        let now = Instant::now();
        self.delta_time = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        // Prevent "spiral of death" during frame rate spikes
        let clamped_delta = self.delta_time.min(Duration::from_millis(250));
        self.accumulated_time += clamped_delta;

        self.delta_time.as_secs_f64()
    }

    /// Checks if a physics/simulation update is due, consuming one fixed timestep.
    pub fn should_fixed_update(&mut self) -> bool {
        if self.accumulated_time >= self.fixed_timestep {
            self.accumulated_time -= self.fixed_timestep;
            true
        } else {
            false
        }
    }

    pub fn fixed_timestep_seconds(&self) -> f64 {
        self.fixed_timestep.as_secs_f64()
    }

    pub fn delta_time_seconds(&self) -> f64 {
        self.delta_time.as_secs_f64()
    }

    /// The fractional percentage of time remaining between fixed ticks, useful for rendering interpolation.
    pub fn interpolation_factor(&self) -> f64 {
        self.accumulated_time.as_secs_f64() / self.fixed_timestep.as_secs_f64()
    }
}
