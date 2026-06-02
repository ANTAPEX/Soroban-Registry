//! Security event logging and monitoring
//!
//! This module provides structured logging for security-related events,
//! particularly validation failures. Logs are formatted for integration with
//! ELK, Splunk, and other observability platforms.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Security event types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SecurityEventType {
    /// Validation failed for a field
    #[serde(rename = "validation_failed")]
    ValidationFailed,
    /// Payload exceeded size limit
    #[serde(rename = "payload_too_large")]
    PayloadTooLarge,
    /// Rate limit exceeded for validation failures
    #[serde(rename = "rate_limit_exceeded")]
    RateLimitExceeded,
    /// Suspicious pattern detected
    #[serde(rename = "suspicious_pattern")]
    SuspiciousPattern,
    /// SQL injection attempt detected
    #[serde(rename = "injection_attempt")]
    InjectionAttempt,
}

/// Security event that gets logged
#[derive(Debug, Clone, Serialize)]
pub struct SecurityEvent {
    /// Event type
    pub event_type: String,
    /// Client IP address
    pub client_ip: String,
    /// Field that failed validation (if applicable)
    pub field: Option<String>,
    /// Error message
    pub message: String,
    /// Request path
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Number of validation failures from this IP in current window
    pub failure_count: u32,
    /// Request ID / correlation ID for tracing
    pub correlation_id: String,
    /// Timestamp in ISO 8601 format
    pub timestamp: String,
    /// Additional context as structured data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

impl SecurityEvent {
    pub fn new(
        event_type: SecurityEventType,
        client_ip: IpAddr,
        message: impl Into<String>,
        path: impl Into<String>,
        method: impl Into<String>,
        correlation_id: impl Into<String>,
    ) -> Self {
        Self {
            event_type: match event_type {
                SecurityEventType::ValidationFailed => "validation_failed".to_string(),
                SecurityEventType::PayloadTooLarge => "payload_too_large".to_string(),
                SecurityEventType::RateLimitExceeded => "rate_limit_exceeded".to_string(),
                SecurityEventType::SuspiciousPattern => "suspicious_pattern".to_string(),
                SecurityEventType::InjectionAttempt => "injection_attempt".to_string(),
            },
            client_ip: client_ip.to_string(),
            field: None,
            message: message.into(),
            path: path.into(),
            method: method.into(),
            failure_count: 1,
            correlation_id: correlation_id.into(),
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            context: None,
        }
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn with_failure_count(mut self, count: u32) -> Self {
        self.failure_count = count;
        self
    }

    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }

    /// Log the security event using the structured logging system
    pub fn log(self) {
        let _event_json = serde_json::to_string(&self).unwrap_or_else(|_| "{}".to_string());

        match self.event_type.as_str() {
            "validation_failed" => {
                tracing::warn!(
                    event = self.event_type,
                    client_ip = self.client_ip,
                    field = self.field,
                    message = self.message,
                    path = self.path,
                    method = self.method,
                    failure_count = self.failure_count,
                    correlation_id = self.correlation_id,
                    "Validation failed"
                );
            }
            "payload_too_large" => {
                tracing::warn!(
                    event = self.event_type,
                    client_ip = self.client_ip,
                    message = self.message,
                    path = self.path,
                    correlation_id = self.correlation_id,
                    "Oversized payload rejected"
                );
            }
            "rate_limit_exceeded" => {
                tracing::warn!(
                    event = self.event_type,
                    client_ip = self.client_ip,
                    message = self.message,
                    failure_count = self.failure_count,
                    correlation_id = self.correlation_id,
                    "Validation failure rate limit exceeded"
                );
            }
            "suspicious_pattern" | "injection_attempt" => {
                tracing::error!(
                    event = self.event_type,
                    client_ip = self.client_ip,
                    field = self.field,
                    message = self.message,
                    path = self.path,
                    correlation_id = self.correlation_id,
                    "Security threat detected"
                );
            }
            _ => {
                tracing::info!(
                    event = self.event_type,
                    client_ip = self.client_ip,
                    message = self.message,
                    "Security event"
                );
            }
        }
    }
}

/// Track validation failures per IP for rate limiting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IpFailureKey {
    pub ip: std::net::IpAddr,
    pub window_start: std::time::Instant,
}

/// Helper to log validation failures with IP tracking
pub fn log_validation_failure(
    client_ip: IpAddr,
    field: &str,
    message: &str,
    path: &str,
    method: &str,
    correlation_id: &str,
    failure_count: u32,
) {
    SecurityEvent::new(
        SecurityEventType::ValidationFailed,
        client_ip,
        message,
        path,
        method,
        correlation_id,
    )
    .with_field(field)
    .with_failure_count(failure_count)
    .log();
}

/// Helper to log payload size violations
pub fn log_payload_too_large(
    client_ip: IpAddr,
    size: usize,
    max_size: usize,
    path: &str,
    correlation_id: &str,
) {
    let context = serde_json::json!({
        "actual_size": size,
        "max_size": max_size,
        "excess": size - max_size,
    });

    SecurityEvent::new(
        SecurityEventType::PayloadTooLarge,
        client_ip,
        format!("Payload {} bytes exceeds limit of {} bytes", size, max_size),
        path,
        "POST",
        correlation_id,
    )
    .with_context(context)
    .log();
}

/// Helper to log rate limit violations
pub fn log_validation_rate_limit_exceeded(
    client_ip: IpAddr,
    failure_count: u32,
    window_seconds: u64,
    correlation_id: &str,
) {
    SecurityEvent::new(
        SecurityEventType::RateLimitExceeded,
        client_ip,
        format!(
            "Validation failure rate limit exceeded: {} failures in {} seconds",
            failure_count, window_seconds
        ),
        "api",
        "ANY",
        correlation_id,
    )
    .with_failure_count(failure_count)
    .log();
}

/// Helper to log suspicious patterns (potential injection attempts)
pub fn log_suspicious_pattern(
    client_ip: IpAddr,
    field: &str,
    pattern_type: &str,
    sample: &str,
    path: &str,
    correlation_id: &str,
) {
    let context = serde_json::json!({
        "pattern_type": pattern_type,
        "sample_value": sample,
    });

    SecurityEvent::new(
        match pattern_type {
            "sql_injection" | "xss" | "html_injection" => SecurityEventType::InjectionAttempt,
            _ => SecurityEventType::SuspiciousPattern,
        },
        client_ip,
        format!("Suspicious {} pattern detected in {}", pattern_type, field),
        path,
        "POST",
        correlation_id,
    )
    .with_field(field)
    .with_context(context)
    .log();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_security_event_creation() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let event = SecurityEvent::new(
            SecurityEventType::ValidationFailed,
            ip,
            "contract_id must start with 'C'",
            "/api/contracts",
            "POST",
            "uuid-123",
        )
        .with_field("contract_id");

        assert_eq!(event.event_type, "validation_failed");
        assert_eq!(event.client_ip, "192.168.1.1");
        assert_eq!(event.field, Some("contract_id".to_string()));
        assert_eq!(event.path, "/api/contracts");
    }

    #[test]
    fn test_security_event_with_context() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let context = serde_json::json!({
            "attempted_pattern": "DROP TABLE users"
        });

        let event = SecurityEvent::new(
            SecurityEventType::InjectionAttempt,
            ip,
            "Potential SQL injection detected",
            "/api/contracts",
            "POST",
            "uuid-456",
        )
        .with_context(context);

        assert_eq!(event.event_type, "injection_attempt");
        assert!(event.context.is_some());
    }

    #[test]
    fn test_security_event_serialization() {
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let event = SecurityEvent::new(
            SecurityEventType::PayloadTooLarge,
            ip,
            "Payload too large",
            "/api/test",
            "POST",
            "uuid-789",
        )
        .with_failure_count(5);

        let json = serde_json::to_string(&event).expect("should serialize");
        assert!(json.contains("payload_too_large"));
        assert!(json.contains("127.0.0.1"));
    }
}
