use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DisasterRecoveryPlan {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub rto_minutes: i32,      // Recovery Time Objective in minutes
    pub rpo_minutes: i32,      // Recovery Point Objective in minutes
    pub recovery_strategy: String,
    pub backup_frequency_minutes: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDisasterRecoveryPlanRequest {
    pub rto_minutes: i32,
    pub rpo_minutes: i32,
    pub recovery_strategy: String,
    pub backup_frequency_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryMetrics {
    pub rto_achieved_seconds: i32,
    pub rpo_ached_seconds: i32,
    pub recovery_success: bool,
    pub recovery_duration_seconds: i32,
    pub data_loss_seconds: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRecoveryRequest {
    pub force_recovery: bool,
    pub recovery_target: Option<String>, // Specific backup date or 'latest'
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryNotification {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub incident_id: Option<Uuid>,
    pub notification_type: String, // 'recovery_started', 'recovery_completed', 'recovery_failed'
    pub message: String,
    pub recipients: Vec<String>, // User addresses to notify
    pub sent_at: DateTime<Utc>,
    pub status: String, // 'pending', 'sent', 'failed'
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostIncidentReport {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub contract_id: Uuid,
    pub title: String,
    pub description: String,
    pub root_cause: String,
    pub impact_assessment: String,
    pub recovery_steps: Vec<String>,
    pub lessons_learned: Vec<String>,
    pub action_items: Vec<ActionItem>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

// Database row type for PostIncidentReport (without nested action_items)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PostIncidentReportRow {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub contract_id: Uuid,
    pub title: String,
    pub description: String,
    pub root_cause: String,
    pub impact_assessment: String,
    pub recovery_steps: Vec<String>,
    pub lessons_learned: Vec<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

impl PostIncidentReportRow {
    pub fn into_report(self, action_items: Vec<ActionItem>) -> PostIncidentReport {
        PostIncidentReport {
            id: self.id,
            incident_id: self.incident_id,
            contract_id: self.contract_id,
            title: self.title,
            description: self.description,
            root_cause: self.root_cause,
            impact_assessment: self.impact_assessment,
            recovery_steps: self.recovery_steps,
            lessons_learned: self.lessons_learned,
            action_items,
            created_by: self.created_by,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActionItem {
    pub id: Uuid,
    pub description: String,
    pub owner: String,
    pub due_date: DateTime<Utc>,
    pub status: String, // 'todo', 'in_progress', 'completed'
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostIncidentReportRequest {
    pub incident_id: Uuid,
    pub title: String,
    pub description: String,
    pub root_cause: String,
    pub impact_assessment: String,
    pub recovery_steps: Vec<String>,
    pub lessons_learned: Vec<String>,
    pub action_items: Vec<CreateActionItemRequest>,
    pub created_by: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateActionItemRequest {
    pub description: String,
    pub owner: String,
    pub due_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationTemplate {
    pub id: Uuid,
    pub name: String,           // e.g., 'recovery_started', 'recovery_completed'
    pub subject: String,
    pub message_template: String, // Template with placeholders
    pub channel: String,        // 'email', 'sms', 'push', 'webhook'
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNotificationTemplateRequest {
    pub name: String,
    pub subject: String,
    pub message_template: String,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserNotificationPreference {
    pub id: Uuid,
    pub user_id: Uuid,
    pub contract_id: Option<Uuid>,  // If null, applies to all contracts
    pub notification_types: Vec<String>, // ['recovery_started', 'recovery_completed', 'incident_detected']
    pub channels: Vec<String>,     // ['email', 'sms', 'push']
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserNotificationPreferenceRequest {
    pub user_id: Uuid,
    pub contract_id: Option<Uuid>,
    pub notification_types: Vec<String>,
    pub channels: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendNotificationRequest {
    pub contract_id: Uuid,
    pub notification_type: String,
    pub template_variables: std::collections::HashMap<String, String>,
    pub recipients: Vec<String>,  // User IDs or addresses
    pub priority: Option<String>, // 'low', 'normal', 'high', 'critical'
}