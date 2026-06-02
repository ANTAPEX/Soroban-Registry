//! Input sanitization functions
//!
//! This module provides functions to clean and normalize input data
//! before validation and storage.

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Pattern to match HTML tags
    static ref HTML_TAG_PATTERN: Regex = Regex::new(r"<[^>]*>").unwrap();

    /// Pattern to match multiple whitespace characters
    static ref MULTI_WHITESPACE: Regex = Regex::new(r"\s+").unwrap();

    /// Pattern to match control characters (except newline and tab)
    static ref CONTROL_CHARS: Regex = Regex::new(r"[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]").unwrap();
}

/// Trim leading and trailing whitespace from a string
pub fn trim(value: &str) -> String {
    value.trim().to_string()
}

/// Trim a string in-place (modifies Option<String>)
pub fn trim_optional(value: &mut Option<String>) {
    if let Some(ref mut s) = value {
        *s = s.trim().to_string();
        if s.is_empty() {
            *value = None;
        }
    }
}

/// Normalize whitespace: collapse multiple spaces/newlines into single space
pub fn normalize_whitespace(value: &str) -> String {
    MULTI_WHITESPACE.replace_all(value.trim(), " ").to_string()
}

/// Strip all HTML tags from a string
pub fn strip_html(value: &str) -> String {
    HTML_TAG_PATTERN.replace_all(value, "").to_string()
}

/// Strip HTML tags from an optional string
pub fn strip_html_optional(value: &mut Option<String>) {
    if let Some(ref mut s) = value {
        *s = strip_html(s);
        if s.trim().is_empty() {
            *value = None;
        }
    }
}

/// Remove control characters from a string
pub fn remove_control_chars(value: &str) -> String {
    CONTROL_CHARS.replace_all(value, "").to_string()
}

/// Normalize a Stellar address: uppercase and trim
pub fn normalize_stellar_address(address: &str) -> String {
    address.trim().to_uppercase()
}

/// Normalize a contract ID: uppercase and trim
pub fn normalize_contract_id(contract_id: &str) -> String {
    contract_id.trim().to_uppercase()
}

/// Sanitize a name field: trim, remove control chars, strip HTML
pub fn sanitize_name(name: &str) -> String {
    let trimmed = trim(name);
    let no_control = remove_control_chars(&trimmed);
    let no_html = strip_html(&no_control);
    normalize_whitespace(&no_html)
}

/// Sanitize a description field: trim, remove control chars, strip HTML
pub fn sanitize_description(desc: &str) -> String {
    let trimmed = trim(desc);
    let no_control = remove_control_chars(&trimmed);
    strip_html(&no_control)
}

/// Sanitize an optional description field
pub fn sanitize_description_optional(desc: &mut Option<String>) {
    if let Some(ref mut s) = desc {
        *s = sanitize_description(s);
        if s.trim().is_empty() {
            *desc = None;
        }
    }
}

/// Sanitize a URL: trim whitespace only (preserve URL encoding)
pub fn sanitize_url(url: &str) -> String {
    url.trim().to_string()
}

/// Sanitize an optional URL field
pub fn sanitize_url_optional(url: &mut Option<String>) {
    if let Some(ref mut s) = url {
        *s = sanitize_url(s);
        if s.is_empty() {
            *url = None;
        }
    }
}

/// Sanitize a vector of tags: trim each, remove empty, strip HTML
pub fn sanitize_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(|t| sanitize_name(t))
        .filter(|t| !t.is_empty())
        .collect()
}

/// Sanitize source code: remove control chars but preserve structure
pub fn sanitize_source_code(source: &str) -> String {
    // Only remove truly problematic control chars, preserve newlines/tabs
    source
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .collect()
}

/// Escape special characters for safe display (not for HTML context)
pub fn escape_for_display(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Sanitize JSON value recursively (trim strings, remove empty)
pub fn sanitize_json_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => {
            *s = trim(s);
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                sanitize_json_value(item);
            }
        }
        serde_json::Value::Object(obj) => {
            for (_, v) in obj {
                sanitize_json_value(v);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim() {
        assert_eq!(trim("  hello  "), "hello");
        assert_eq!(trim("\n\tspaces\t\n"), "spaces");
    }

    #[test]
    fn test_strip_html() {
        assert_eq!(strip_html("<b>bold</b>"), "bold");
        assert_eq!(strip_html("<script>alert('xss')</script>"), "alert('xss')");
        assert_eq!(strip_html("no tags here"), "no tags here");
        assert_eq!(strip_html("<p>paragraph</p><br/>more"), "paragraphmore");
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("hello   world"), "hello world");
        assert_eq!(
            normalize_whitespace("  multiple   spaces  "),
            "multiple spaces"
        );
        assert_eq!(normalize_whitespace("line\n\nbreaks"), "line breaks");
    }

    #[test]
    fn test_normalize_stellar_address() {
        assert_eq!(
            normalize_stellar_address(
                "  gdlzfc3syjydzt7k67vz75hpjvieuvnixf47zg2fb2rmqqvu2hhgcysc  "
            ),
            "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"
        );
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("  My <b>Contract</b>  "), "My Contract");
        assert_eq!(sanitize_name("Normal Name"), "Normal Name");
    }

    #[test]
    fn test_sanitize_tags() {
        let tags = vec![
            "  defi  ".to_string(),
            "<script>bad</script>".to_string(),
            "  ".to_string(),
            "token".to_string(),
        ];
        let sanitized = sanitize_tags(&tags);
        assert_eq!(sanitized, vec!["defi", "bad", "token"]);
    }

    #[test]
    fn test_trim_optional() {
        let mut some_value = Some("  hello  ".to_string());
        trim_optional(&mut some_value);
        assert_eq!(some_value, Some("hello".to_string()));

        let mut empty_value = Some("   ".to_string());
        trim_optional(&mut empty_value);
        assert_eq!(empty_value, None);

        let mut none_value: Option<String> = None;
        trim_optional(&mut none_value);
        assert_eq!(none_value, None);
    }

    #[test]
    fn test_remove_control_chars() {
        let with_null = "hello\x00world";
        assert_eq!(remove_control_chars(with_null), "helloworld");

        // Newlines and tabs should be preserved by the main sanitize functions
        let with_newline = "hello\nworld";
        assert_eq!(remove_control_chars(with_newline), "hello\nworld");
    }
}
