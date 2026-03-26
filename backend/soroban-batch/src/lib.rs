/// Minimal stub for the soroban-batch crate.
///
/// The full implementation lives in a separate repository. This stub satisfies
/// the compiler so the registry API can be built without that dependency.
pub mod engine {
    use tokio::sync::mpsc;

    /// A no-op job engine stub. The real implementation queues and executes
    /// batched Soroban contract jobs.
    pub struct JobEngine;

    impl JobEngine {
        /// Create a new engine and its corresponding work channel.
        pub fn new() -> (Self, mpsc::Receiver<()>) {
            let (_tx, rx) = mpsc::channel(1);
            (JobEngine, rx)
        }

        /// Drain the work channel until it is closed. No-op in this stub.
        pub async fn run_worker(&self, mut rx: mpsc::Receiver<()>) {
            while rx.recv().await.is_some() {}
        }
    }

    impl Default for JobEngine {
        fn default() -> Self {
            Self::new().0
        }
    }
}
