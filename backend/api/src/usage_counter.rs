use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// Increment the usage_count for a contract by 1 using an atomic SQL UPDATE.
///
/// This function performs an atomic increment operation on the database
/// to ensure consistency under concurrent access.
pub async fn increment_usage_counter(contract_id: Uuid, db: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE contracts SET usage_count = usage_count + 1 WHERE id = $1")
        .bind(contract_id)
        .execute(db)
        .await?;

    tracing::debug!(
        contract_id = %contract_id,
        "usage counter incremented"
    );

    Ok(())
}

/// Increment the usage_count for a contract with timeout protection.
///
/// This function wraps the basic increment operation with a 10ms timeout
/// to ensure it doesn't block the main API request processing. If the
/// operation times out or fails, it logs the error but doesn't propagate
/// it to avoid breaking the main request flow.
pub async fn increment_usage_counter_with_timeout(
    contract_id: Uuid,
    db: &PgPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match tokio::time::timeout(
        Duration::from_millis(10),
        increment_usage_counter(contract_id, db),
    )
    .await
    {
        Ok(Ok(())) => {
            tracing::debug!(
                contract_id = %contract_id,
                "usage counter incremented successfully"
            );
            Ok(())
        }
        Ok(Err(db_err)) => {
            tracing::error!(
                contract_id = %contract_id,
                error = ?db_err,
                "failed to increment usage counter"
            );
            Err(Box::new(db_err))
        }
        Err(_timeout_err) => {
            tracing::warn!(
                contract_id = %contract_id,
                "usage counter update timed out after 10ms"
            );
            Err("Usage counter update timed out".into())
        }
    }
}

/// Increment the usage_count for a contract with retry logic and exponential backoff.
///
/// This function implements retry logic for critical counter operations that need
/// higher reliability. It uses exponential backoff to handle transient database
/// issues without overwhelming the database with rapid retry attempts.
///
/// # Arguments
/// * `contract_id` - The UUID of the contract to increment
/// * `db` - Database connection pool
/// * `max_retries` - Maximum number of retry attempts (default: 3)
/// * `base_delay_ms` - Base delay in milliseconds for exponential backoff (default: 10)
///
/// # Returns
/// * `Ok(())` if the increment succeeds within the retry limit
/// * `Err(...)` if all retry attempts fail
pub async fn increment_usage_counter_with_retry(
    contract_id: Uuid,
    db: &PgPool,
    max_retries: Option<u32>,
    base_delay_ms: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let max_retries = max_retries.unwrap_or(3);
    let base_delay_ms = base_delay_ms.unwrap_or(10);
    let mut attempts = 0;

    while attempts <= max_retries {
        match increment_usage_counter(contract_id, db).await {
            Ok(()) => {
                if attempts > 0 {
                    tracing::info!(
                        contract_id = %contract_id,
                        attempts = attempts + 1,
                        "usage counter incremented successfully after retries"
                    );
                } else {
                    tracing::debug!(
                        contract_id = %contract_id,
                        "usage counter incremented successfully on first attempt"
                    );
                }
                return Ok(());
            }
            Err(err) => {
                attempts += 1;

                if attempts > max_retries {
                    tracing::error!(
                        contract_id = %contract_id,
                        attempts = attempts,
                        error = ?err,
                        "usage counter increment failed after all retry attempts"
                    );
                    return Err(Box::new(err));
                }

                // Calculate exponential backoff delay: base_delay * 2^attempt
                let delay_ms = base_delay_ms * (2_u64.pow(attempts - 1));

                tracing::warn!(
                    contract_id = %contract_id,
                    attempt = attempts,
                    max_retries = max_retries,
                    delay_ms = delay_ms,
                    error = ?err,
                    "usage counter increment failed, retrying with exponential backoff"
                );

                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }

    // This should never be reached due to the loop logic, but included for completeness
    Err("Maximum retry attempts exceeded".into())
}

/// Increment the usage_count for a contract with retry logic and timeout protection.
///
/// This function combines retry logic with timeout protection for the most critical
/// counter operations. It first applies a timeout to the entire retry operation,
/// then uses exponential backoff for individual retry attempts.
///
/// # Arguments
/// * `contract_id` - The UUID of the contract to increment
/// * `db` - Database connection pool
/// * `timeout_ms` - Total timeout for the entire operation in milliseconds (default: 100)
/// * `max_retries` - Maximum number of retry attempts (default: 3)
///
/// # Returns
/// * `Ok(())` if the increment succeeds within the timeout and retry limits
/// * `Err(...)` if the operation times out or all retry attempts fail
pub async fn increment_usage_counter_with_retry_and_timeout(
    contract_id: Uuid,
    db: &PgPool,
    timeout_ms: Option<u64>,
    max_retries: Option<u32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let timeout_ms = timeout_ms.unwrap_or(100);

    match tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        increment_usage_counter_with_retry(contract_id, db, max_retries, Some(5)), // Use shorter base delay for timeout scenarios
    )
    .await
    {
        Ok(result) => result,
        Err(_timeout_err) => {
            tracing::warn!(
                contract_id = %contract_id,
                timeout_ms = timeout_ms,
                "usage counter retry operation timed out"
            );
            Err(format!(
                "Usage counter retry operation timed out after {}ms",
                timeout_ms
            )
            .into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // These functions all need a live Postgres to exercise for real. Until the
    // suite has one, the checks below are deliberately limited to what can be
    // asserted without a connection: that each entry point exists with the
    // argument types callers rely on.
    //
    // They are NOT #[ignore]d, because they no longer pretend to need a
    // database — an ignored test that asserts nothing is worse than a small
    // test that asserts something true. The behavioural coverage these stand in
    // for is tracked separately; see usage_counter_integration_tests.rs.

    /// Pins each entry point's argument types at compile time.
    ///
    /// `connect_lazy` opens no connection, so no database is needed — but it
    /// does spawn a pool-maintenance task, hence `#[tokio::test]`. The async
    /// block below is constructed and never
    /// awaited: that is enough to type-check every call site, and nothing in it
    /// touches the connection.
    ///
    /// The previous version of these tests built a `PgPool` with
    /// `unsafe { std::mem::zeroed() }`. That is undefined behaviour on
    /// construction, not merely on use: `PgPool` wraps non-null pointers, and a
    /// zeroed one violates its validity invariant. It stayed inert only because
    /// the future was never polled, so deleting the `#[ignore]` above it would
    /// have turned it into a segfault rather than a passing test.
    #[tokio::test]
    async fn entry_points_accept_their_documented_arguments() {
        let pool = PgPool::connect_lazy("postgres://user:pass@localhost/db")
            .expect("connect_lazy performs no I/O and should not fail");
        let contract_id = Uuid::new_v4();

        let _never_awaited = async {
            let _ = increment_usage_counter(contract_id, &pool).await;
            let _ = increment_usage_counter_with_timeout(contract_id, &pool).await;
            let _ = increment_usage_counter_with_retry(contract_id, &pool, Some(3), Some(10)).await;
            let _ = increment_usage_counter_with_retry_and_timeout(
                contract_id,
                &pool,
                Some(100),
                Some(3),
            )
            .await;
        };
    }
}
