/// Exponential backoff handler for RPC failures
/// Manages retry logic with exponential backoff and configurable maximum intervals
use std::time::Duration;
use tracing::{error, info, warn};

/// Exponential backoff state tracker
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    base_interval_secs: u64,
    max_interval_secs: u64,
    current_attempt: u32,
    current_interval_secs: u64,
}

impl ExponentialBackoff {
    /// Create new backoff handler
    pub fn new(base_interval_secs: u64, max_interval_secs: u64) -> Self {
        ExponentialBackoff {
            base_interval_secs,
            max_interval_secs,
            current_attempt: 0,
            current_interval_secs: base_interval_secs,
        }
    }

    /// Record a failure and return the backoff duration
    pub fn on_failure(&mut self, error_message: &str) -> Duration {
        self.current_attempt += 1;

        // Calculate next interval using exponential backoff: base * 2^(attempts - 1)
        let next_interval = self
            .base_interval_secs
            .saturating_mul(2_u64.saturating_pow(self.current_attempt.saturating_sub(1)));

        // Cap at maximum interval
        self.current_interval_secs = next_interval.min(self.max_interval_secs);

        error!(
            attempt = self.current_attempt,
            interval_secs = self.current_interval_secs,
            error = error_message,
            "RPC failure: backing off before retry"
        );

        Duration::from_secs(self.current_interval_secs)
    }

    /// Reset backoff on successful operation
    pub fn on_success(&mut self) {
        if self.current_attempt > 0 {
            info!(
                attempts = self.current_attempt,
                "RPC recovered after {} attempts, resetting backoff", self.current_attempt
            );
        }
        self.current_attempt = 0;
        self.current_interval_secs = self.base_interval_secs;
    }

    /// Get current number of attempts
    pub fn attempts(&self) -> u32 {
        self.current_attempt
    }

    /// Get current interval seconds
    pub fn interval_secs(&self) -> u64 {
        self.current_interval_secs
    }

    /// Check if we should give up (optional - depends on business logic)
    pub fn should_give_up(&self, max_total_attempts: u32) -> bool {
        self.current_attempt >= max_total_attempts
    }
}

/// Helper function to execute with exponential backoff
pub async fn execute_with_backoff<F, T, Fut>(
    mut backoff: ExponentialBackoff,
    max_attempts: u32,
    mut operation: F,
) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    loop {
        match operation().await {
            Ok(result) => {
                backoff.on_success();
                return Ok(result);
            }
            Err(err) => {
                // Record the failure and compute next interval first
                let wait_duration = backoff.on_failure(&err);

                // If we've reached the maximum allowed attempts, give up
                if backoff.should_give_up(max_attempts) {
                    error!(
                        attempts = backoff.attempts(),
                        error = err,
                        "Giving up after {} attempts",
                        backoff.attempts()
                    );
                    return Err(format!(
                        "Failed after {} attempts: {}",
                        backoff.attempts(),
                        err
                    ));
                }

                warn!(
                    attempt = backoff.attempts(),
                    wait_secs = wait_duration.as_secs(),
                    "Retrying after backoff"
                );

                tokio::time::sleep(wait_duration).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_initialization() {
        let backoff = ExponentialBackoff::new(1, 60);
        assert_eq!(backoff.base_interval_secs, 1);
        assert_eq!(backoff.max_interval_secs, 60);
        assert_eq!(backoff.attempts(), 0);
        assert_eq!(backoff.interval_secs(), 1);
    }

    #[test]
    fn test_exponential_backoff_sequence() {
        let mut backoff = ExponentialBackoff::new(1, 120);

        let d1 = backoff.on_failure("error1");
        assert_eq!(d1.as_secs(), 1);

        let d2 = backoff.on_failure("error2");
        assert_eq!(d2.as_secs(), 2);

        let d3 = backoff.on_failure("error3");
        assert_eq!(d3.as_secs(), 4);

        let d4 = backoff.on_failure("error4");
        assert_eq!(d4.as_secs(), 8);
    }

    #[test]
    fn test_backoff_max_interval() {
        let mut backoff = ExponentialBackoff::new(1, 10);

        for _ in 0..10 {
            backoff.on_failure("test");
        }

        assert_eq!(backoff.interval_secs(), 10);
    }

    #[test]
    fn test_backoff_reset_on_success() {
        let mut backoff = ExponentialBackoff::new(1, 60);

        backoff.on_failure("error1");
        backoff.on_failure("error2");
        assert_eq!(backoff.attempts(), 2);

        backoff.on_success();
        assert_eq!(backoff.attempts(), 0);
        assert_eq!(backoff.interval_secs(), 1);
    }

    #[test]
    fn test_should_give_up() {
        let mut backoff = ExponentialBackoff::new(1, 60);

        assert!(!backoff.should_give_up(3));

        backoff.on_failure("error1");
        assert!(!backoff.should_give_up(3));

        backoff.on_failure("error2");
        assert!(!backoff.should_give_up(3));

        backoff.on_failure("error3");
        assert!(backoff.should_give_up(3));
    }

    #[tokio::test]
    async fn test_execute_with_backoff_success() {
        let backoff = ExponentialBackoff::new(1, 60);

        let result = execute_with_backoff(backoff, 5, || async { Ok::<i32, String>(42) }).await;

        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_execute_with_backoff_retry_then_success() {
        let backoff = ExponentialBackoff::new(1, 60);
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = execute_with_backoff(backoff, 5, || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if count.load(std::sync::atomic::Ordering::SeqCst) < 3 {
                    Err::<i32, _>("temporary failure".to_string())
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(result, Ok(42));
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_execute_with_backoff_max_attempts() {
        let backoff = ExponentialBackoff::new(1, 60);
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = execute_with_backoff(backoff, 3, || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err::<i32, _>("persistent failure".to_string())
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }
}
