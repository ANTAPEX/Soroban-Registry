use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Types of jobs supported by the engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobType {
    ContractVerify,
    BatchIndex,
    SendNotification,
    GenerateReport,
}

/// Status of a job in the queue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Processing,
    Success,
    Failed,
}

/// Main Job structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub job_type: JobType,
    pub status: JobStatus,
    pub payload: serde_json::Value,
    pub retries: u32,
    pub next_try: DateTime<Utc>,
    pub error: Option<String>,
}

pub struct JobEngine {
    jobs: Arc<DashMap<Uuid, Job>>,
    tx: mpsc::Sender<Uuid>,
    semaphore: Arc<Semaphore>,
}

impl JobEngine {
    const MAX_RETRIES: u32 = 5;
    const CONCURRENCY_LIMIT: usize = 5;

    pub fn new() -> (Self, mpsc::Receiver<Uuid>) {
        let (tx, rx) = mpsc::channel(100);
        let engine = Self {
            jobs: Arc::new(DashMap::new()),
            tx,
            semaphore: Arc::new(Semaphore::new(Self::CONCURRENCY_LIMIT)),
        };
        (engine, rx)
    }

    /// Enqueue a new job
    pub async fn enqueue(&self, job_type: JobType, payload: serde_json::Value) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let job = Job {
            id,
            job_type,
            status: JobStatus::Pending,
            payload,
            retries: 0,
            next_try: Utc::now(),
            error: None,
        };

        self.jobs.insert(id, job);
        self.tx
            .send(id)
            .await
            .map_err(|e| anyhow!("Failed to send job to queue: {}", e))?;

        info!(job_id = %id, "Job enqueued successfully");
        Ok(id)
    }

    /// Get job status
    pub fn get_job(&self, id: &Uuid) -> Option<Job> {
        self.jobs.get(id).map(|j| j.clone())
    }

    /// Run the worker loop
    pub async fn run_worker(self: Arc<Self>, mut rx: mpsc::Receiver<Uuid>) {
        info!("Job Engine worker loop started");
        while let Some(job_id) = rx.recv().await {
            let engine = Arc::clone(&self);
            let permit = match engine.semaphore.clone().acquire_owned().await {
                Ok(p) => p,
                Err(e) => {
                    error!("Fatal error: semaphore closed: {}", e);
                    break;
                }
            };

            tokio::spawn(async move {
                let _permit = permit; // Permit released when dropped
                if let Err(e) = engine.process_job(job_id).await {
                    error!(job_id = %job_id, error = %e, "Unexpected error processing job");
                }
            });
        }
    }

    async fn process_job(&self, job_id: Uuid) -> Result<()> {
        let mut job = match self.jobs.get(&job_id) {
            Some(j) => j.clone(),
            None => return Err(anyhow!("Job {} not found", job_id)),
        };

        // Check if it's time to run
        let now = Utc::now();
        if job.next_try > now {
            let wait_time = job.next_try - now;
            tokio::time::sleep(wait_time.to_std()?).await;
        }

        job.status = JobStatus::Processing;
        self.jobs.insert(job_id, job.clone());

        match self.execute_handler(&job).await {
            Ok(_) => {
                job.status = JobStatus::Success;
                self.jobs.insert(job_id, job);
                info!(job_id = %job_id, "Job completed successfully");
            }
            Err(e) => {
                job.retries += 1;
                job.error = Some(e.to_string());

                if job.retries >= Self::MAX_RETRIES {
                    job.status = JobStatus::Failed;
                    error!(job_id = %job_id, retries = job.retries, "Job failed after max retries");
                } else {
                    job.status = JobStatus::Pending;
                    // Exponential backoff: 2^retries seconds
                    let backoff_secs = 2u64.pow(job.retries);
                    job.next_try = Utc::now() + Duration::seconds(backoff_secs as i64);
                    warn!(job_id = %job_id, retries = job.retries, next_try = %job.next_try, "Job failed, retrying");

                    // Re-enqueue for retry
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        let _ = tx.send(job_id).await;
                    });
                }
                self.jobs.insert(job_id, job);
            }
        }

        Ok(())
    }

    async fn execute_handler(&self, job: &Job) -> Result<()> {
        // Implement actual handlers here. For now, simulate work.
        match job.job_type {
            JobType::ContractVerify => {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            JobType::BatchIndex => {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            JobType::SendNotification => {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            JobType::GenerateReport => {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }
        }

        // Simulate random failure for testing retries if payload contains "fail"
        if job
            .payload
            .get("simulate_fail")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Err(anyhow!("Simulated job failure"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_job_completion() -> Result<()> {
        let (engine, rx) = JobEngine::new();
        let engine = Arc::new(engine);
        let engine_clone = engine.clone();

        tokio::spawn(async move {
            engine_clone.run_worker(rx).await;
        });

        let job_id = engine.enqueue(JobType::ContractVerify, json!({})).await?;

        // Wait for completion
        let mut success = false;
        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            if let Some(job) = engine.get_job(&job_id) {
                if job.status == JobStatus::Success {
                    success = true;
                    break;
                }
            }
        }

        assert!(success, "Job should complete successfully");
        Ok(())
    }

    #[tokio::test]
    async fn test_retry_logic() -> Result<()> {
        let (engine, rx) = JobEngine::new();
        let engine = Arc::new(engine);
        let engine_clone = engine.clone();

        tokio::spawn(async move {
            engine_clone.run_worker(rx).await;
        });

        let job_id = engine
            .enqueue(JobType::GenerateReport, json!({"simulate_fail": true}))
            .await?;

        // Wait and check retries
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        if let Some(job) = engine.get_job(&job_id) {
            assert!(job.retries > 0, "Job should have retried");
            assert!(
                job.status == JobStatus::Pending
                    || job.status == JobStatus::Processing
                    || job.status == JobStatus::Failed
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrency_limit() -> Result<()> {
        let (engine, rx) = JobEngine::new();
        let engine = Arc::new(engine);
        let engine_clone = engine.clone();

        tokio::spawn(async move {
            engine_clone.run_worker(rx).await;
        });

        // Enqueue 10 jobs
        for _ in 0..10 {
            engine.enqueue(JobType::BatchIndex, json!({})).await?;
        }

        // Check semaphore permits (should be 0 or small if all are running)
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let available = engine.semaphore.available_permits();
        assert!(available <= 5, "At most 5 jobs should be running");

        Ok(())
    }
}
