use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    disaster_recovery_models::{
        CreateNotificationTemplateRequest, CreateUserNotificationPreferenceRequest, 
        NotificationTemplate, SendNotificationRequest, UserNotificationPreference
    },
    error::{ApiError, ApiResult},
    state::AppState,
};

pub async fn create_notification_template(
    State(state): State<AppState>,
    Json(req): Json<CreateNotificationTemplateRequest>,
) -> ApiResult<Json<NotificationTemplate>> {
    let template = sqlx::query_as::<_, NotificationTemplate>(
        r#"
        INSERT INTO notification_templates 
        (name, subject, message_template, channel)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(&req.subject)
    .bind(&req.message_template)
    .bind(&req.channel)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create notification template: {}", e)))?;

    Ok(Json(template))
}

pub async fn get_notification_template(
    State(state): State<AppState>,
    Path(template_name): Path<String>,
) -> ApiResult<Json<NotificationTemplate>> {
    let template = sqlx::query_as::<_, NotificationTemplate>(
        "SELECT * FROM notification_templates WHERE name = $1",
    )
    .bind(&template_name)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("notification_template", "Notification template not found"))?;

    Ok(Json(template))
}

pub async fn create_user_notification_preference(
    State(state): State<AppState>,
    Json(req): Json<CreateUserNotificationPreferenceRequest>,
) -> ApiResult<Json<UserNotificationPreference>> {
    let preference = sqlx::query_as::<_, UserNotificationPreference>(
        r#"
        INSERT INTO user_notification_preferences 
        (user_id, contract_id, notification_types, channels, enabled)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(req.user_id)
    .bind(req.contract_id)
    .bind(&req.notification_types)
    .bind(&req.channels)
    .bind(req.enabled)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create notification preference: {}", e)))?;

    Ok(Json(preference))
}

pub async fn get_user_notification_preferences(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<Vec<UserNotificationPreference>>> {
    let preferences = sqlx::query_as::<_, UserNotificationPreference>(
        "SELECT * FROM user_notification_preferences WHERE user_id = $1 AND enabled = true",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(preferences))
}

pub async fn send_notification(
    State(state): State<AppState>,
    Json(req): Json<SendNotificationRequest>,
) -> ApiResult<StatusCode> {
    // First, get the notification template
    let template = sqlx::query_as::<_, NotificationTemplate>(
        "SELECT * FROM notification_templates WHERE name = $1",
    )
    .bind(&req.notification_type)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("notification_template", "Notification template not found"))?;
    
    // Process template with variables
    let mut message = template.message_template.clone();
    for (key, value) in &req.template_variables {
        let placeholder = format!("{{{{{}}}}}", key); // {{variable}}
        message = message.replace(&placeholder, value);
    }
    
    // For now, just log the notification - in a real implementation this would send via email/SMS/etc.
    println!("Notification sent to {:?}: {} - {}", req.recipients, template.subject, message);
    
    // In a real system, we'd store the notification in a queue table for processing
    // and track delivery status
    
    // Log the notification for audit purposes
    sqlx::query(
        r#"
        INSERT INTO notification_logs 
        (contract_id, notification_type, recipients, message, sent_at, status)
        VALUES ($1, $2, $3, $4, $5, 'sent')
        "#,
    )
    .bind(req.contract_id)
    .bind(&req.notification_type)
    .bind(&req.recipients)
    .bind(&message)
    .bind(Utc::now())
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to log notification: {}", e)))?;

    Ok(StatusCode::OK)
}

pub async fn get_user_notifications(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // In a real system, this would return user-specific notifications
    // For now, return a placeholder response
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "notifications": [],
        "unread_count": 0,
        "last_checked": Utc::now()
    })))
}