use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a pagination cursor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    /// Timestamp for ordering
    pub timestamp: DateTime<Utc>,
    /// UUID for stable tie-breaking
    pub id: Uuid,
}

impl Cursor {
    pub fn new(timestamp: DateTime<Utc>, id: Uuid) -> Self {
        Self { timestamp, id }
    }

    /// Encodes the cursor into a base64 string
    pub fn encode(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        URL_SAFE_NO_PAD.encode(json)
    }

    /// Decodes a cursor from a base64 string
    pub fn decode(encoded: &str) -> Result<Self> {
        let decoded = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| anyhow!("Invalid cursor format (base64)"))?;

        let json =
            String::from_utf8(decoded).map_err(|_| anyhow!("Invalid cursor format (utf8)"))?;

        serde_json::from_str(&json).map_err(|_| anyhow!("Invalid cursor format (json)"))
    }
}

/// Helper to extract cursor from a list of items
pub trait CursorProvider {
    fn get_cursor(&self) -> Cursor;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_roundtrip() {
        let id = Uuid::new_v4();
        let ts = Utc::now();
        let cursor = Cursor::new(ts, id);

        let encoded = cursor.encode();
        let decoded = Cursor::decode(&encoded).unwrap();

        assert_eq!(cursor.id, decoded.id);
        // Compare timestamps with millisecond precision to avoid float noise if any
        assert_eq!(
            cursor.timestamp.timestamp_millis(),
            decoded.timestamp.timestamp_millis()
        );
    }

    #[test]
    fn test_invalid_cursor() {
        assert!(Cursor::decode("notbase64").is_err());
        assert!(Cursor::decode("YWJj").is_err()); // "abc" in base64, not JSON
    }
}
