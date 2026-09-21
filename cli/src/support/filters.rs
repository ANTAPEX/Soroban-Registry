//! Normalising the repeated comma-separated filter arguments that `list` and
//! `search` accept.

use std::collections::HashSet;

/// Split a repeatable filter flag into individual values, so
/// `--category a,b` and `--category a --category b` produce the same list.
/// Trims whitespace, drops blanks, and de-duplicates while preserving order.
pub(crate) fn normalize_filter_values(values: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for value in values {
        for part in value.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            if seen.insert(part.to_string()) {
                normalized.push(part.to_string());
            }
        }
    }

    normalized
}

/// Split a comma-separated filter into trimmed, de-duplicated values.
pub(crate) fn normalize_list_values(value: Option<&str>) -> Vec<String> {
    normalize_filter_values(&value.into_iter().map(str::to_string).collect::<Vec<_>>())
}
