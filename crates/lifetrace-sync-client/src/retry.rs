use std::time::Duration;

/// Deterministic exponential backoff. The platform may add random jitter; the
/// core accepts a signed basis-point jitter to keep tests free of real sleeps.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub initial: Duration,
    pub maximum: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial: Duration::from_secs(2),
            maximum: Duration::from_secs(300),
        }
    }
}

impl RetryPolicy {
    pub fn delay(&self, attempt: u32, jitter_basis_points: i16) -> Duration {
        let shift = attempt.saturating_sub(1).min(31);
        let multiplier = 1u64 << shift;
        let base_ms = self
            .initial
            .as_millis()
            .saturating_mul(multiplier as u128)
            .min(self.maximum.as_millis()) as i128;
        let bounded = jitter_basis_points.clamp(-2_000, 2_000) as i128;
        let jittered = base_ms + (base_ms * bounded / 10_000);
        Duration::from_millis(jittered.max(0) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_delay_is_capped_and_jittered() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay(1, 0), Duration::from_secs(2));
        assert_eq!(policy.delay(2, 0), Duration::from_secs(4));
        assert_eq!(policy.delay(20, 0), Duration::from_secs(300));
        assert_eq!(policy.delay(1, 2_000), Duration::from_millis(2400));
        assert_eq!(policy.delay(1, -2_000), Duration::from_millis(1600));
    }
}
