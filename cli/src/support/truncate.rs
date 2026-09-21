//! Truncating a string to a column width for table output.

// Helper for string truncation
pub(crate) trait Truncate {
    fn truncate_str(&self, max: usize) -> String;
}

impl Truncate for str {
    fn truncate_str(&self, max: usize) -> String {
        if self.len() > max {
            format!("{}...", &self[..max - 3])
        } else {
            self.to_string()
        }
    }
}
