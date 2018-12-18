use std::time::Instant;

pub trait ElapsedSeconds {
    fn elapsed_seconds(&self) -> f64;
}

impl ElapsedSeconds for Instant {
    fn elapsed_seconds(&self) -> f64 {
        let duration = self.elapsed();
        duration.as_secs() as f64 + f64::from(duration.subsec_nanos()) / 1_000_000_000f64
    }
}
