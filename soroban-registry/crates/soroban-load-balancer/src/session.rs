use dashmap::DashMap;
use std::time::{Duration, Instant};

/// Entry in the session store
struct SessionEntry {
    instance_id: String,
    created_at: Instant,
    ttl: Duration,
}

impl SessionEntry {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Thread-safe session affinity manager
/// Maps session keys to pinned instance IDs
pub struct SessionManager {
    sessions: DashMap<String, SessionEntry>,
    ttl: Duration,
}

impl SessionManager {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            sessions: DashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Get the pinned instance ID for a session key, if valid
    pub fn get(&self, session_key: &str) -> Option<String> {
        if let Some(entry) = self.sessions.get(session_key) {
            if !entry.is_expired() {
                return Some(entry.instance_id.clone());
            }
        }
        // Clean up expired entry
        self.sessions.remove(session_key);
        None
    }

    /// Pin a session key to a specific instance
    pub fn set(&self, session_key: impl Into<String>, instance_id: impl Into<String>) {
        self.sessions.insert(
            session_key.into(),
            SessionEntry {
                instance_id: instance_id.into(),
                created_at: Instant::now(),
                ttl: self.ttl,
            },
        );
    }

    /// Remove a session (e.g. on logout or instance removal)
    pub fn remove(&self, session_key: &str) {
        self.sessions.remove(session_key);
    }

    /// Evict all sessions pinned to a specific instance (called when instance goes unhealthy)
    pub fn evict_instance(&self, instance_id: &str) {
        self.sessions.retain(|_, v| v.instance_id != instance_id);
    }

    /// Remove all expired sessions
    pub fn purge_expired(&self) {
        self.sessions.retain(|_, v| !v.is_expired());
    }

    /// Count active (non-expired) sessions
    pub fn active_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|e| !e.value().is_expired())
            .count()
    }
}
