#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IncidentSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl FromStr for IncidentSeverity {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Self::Critical),
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => bail!(
                "invalid severity: {} (expected critical|high|medium|low)",
                s
            ),
        }
    }
}

impl fmt::Display for IncidentSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Critical => write!(f, "CRITICAL"),
            Self::High => write!(f, "HIGH"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::Low => write!(f, "LOW"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentState {
    Detected,
    Responding,
    Contained,
    Recovered,
    PostReview,
}

impl FromStr for IncidentState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "detected" => Ok(Self::Detected),
            "responding" => Ok(Self::Responding),
            "contained" => Ok(Self::Contained),
            "recovered" => Ok(Self::Recovered),
            "post_review" | "postreview" => Ok(Self::PostReview),
            _ => bail!(
                "invalid state: {} (expected detected|responding|contained|recovered|post_review)",
                s
            ),
        }
    }
}

impl fmt::Display for IncidentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Detected => write!(f, "Detected"),
            Self::Responding => write!(f, "Responding"),
            Self::Contained => write!(f, "Contained"),
            Self::Recovered => write!(f, "Recovered"),
            Self::PostReview => write!(f, "PostReview"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: Uuid,
    pub contract_id: String,
    pub severity: IncidentSeverity,
    pub state: IncidentState,
    pub triggered_at: DateTime<Utc>,
}

#[derive(Default)]
pub struct IncidentManager {
    incidents: HashMap<Uuid, Incident>,
    halted: HashSet<String>,
}

impl IncidentManager {
    pub fn trigger(&mut self, contract_id: String, severity: IncidentSeverity) -> Uuid {
        let id = Uuid::new_v4();
        let incident = Incident {
            id,
            contract_id: contract_id.clone(),
            severity,
            state: IncidentState::Detected,
            triggered_at: Utc::now(),
        };
        self.incidents.insert(id, incident);
        if severity == IncidentSeverity::Critical {
            self.halted.insert(contract_id);
        }
        id
    }

    pub fn update_state(&mut self, id: Uuid, new_state: IncidentState) -> Result<()> {
        let incident = self
            .incidents
            .get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("incident not found: {}", id))?;

        if incident.state == new_state {
            bail!("incident {} is already in state {}", id, new_state);
        }

        incident.state = new_state;

        if matches!(
            new_state,
            IncidentState::Recovered | IncidentState::PostReview
        ) {
            self.halted.remove(&incident.contract_id);
        }

        Ok(())
    }

    pub fn is_halted(&self, contract_id: &str) -> bool {
        self.halted.contains(contract_id)
    }

    pub fn get(&self, id: Uuid) -> Option<&Incident> {
        self.incidents.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_non_critical_no_halt() {
        let mut mgr = IncidentManager::default();
        let id = mgr.trigger("C1".into(), IncidentSeverity::High);
        assert!(!mgr.is_halted("C1"));
        assert_eq!(mgr.get(id).unwrap().state, IncidentState::Detected);
    }

    #[test]
    fn trigger_critical_auto_halts() {
        let mut mgr = IncidentManager::default();
        mgr.trigger("C2".into(), IncidentSeverity::Critical);
        assert!(mgr.is_halted("C2"));
    }

    #[test]
    fn update_state_transitions() {
        let mut mgr = IncidentManager::default();
        let id = mgr.trigger("C3".into(), IncidentSeverity::High);
        mgr.update_state(id, IncidentState::Responding).unwrap();
        mgr.update_state(id, IncidentState::Contained).unwrap();
        assert_eq!(mgr.get(id).unwrap().state, IncidentState::Contained);
    }

    #[test]
    fn recovery_clears_breaker() {
        let mut mgr = IncidentManager::default();
        let id = mgr.trigger("C4".into(), IncidentSeverity::Critical);
        assert!(mgr.is_halted("C4"));
        mgr.update_state(id, IncidentState::Responding).unwrap();
        mgr.update_state(id, IncidentState::Contained).unwrap();
        mgr.update_state(id, IncidentState::Recovered).unwrap();
        assert!(!mgr.is_halted("C4"));
    }

    #[test]
    fn post_review_clears_breaker() {
        let mut mgr = IncidentManager::default();
        let id = mgr.trigger("C5".into(), IncidentSeverity::Critical);
        mgr.update_state(id, IncidentState::Responding).unwrap();
        mgr.update_state(id, IncidentState::Contained).unwrap();
        mgr.update_state(id, IncidentState::PostReview).unwrap();
        assert!(!mgr.is_halted("C5"));
    }

    #[test]
    fn invalid_state_transition_same_state() {
        let mut mgr = IncidentManager::default();
        let id = mgr.trigger("C6".into(), IncidentSeverity::Medium);
        let err = mgr.update_state(id, IncidentState::Detected).unwrap_err();
        assert!(err.to_string().contains("already in state"));
    }
}
