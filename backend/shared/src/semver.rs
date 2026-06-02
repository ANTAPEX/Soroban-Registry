use serde::{Deserialize, Serialize};

/// Semantic Versioning (SemVer) implementation
/// Supports parsing MAJOR.MINOR.PATCH and constraints like ^1.0.0, ~2.3.0

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_release: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_metadata: Option<String>,
}

// Custom PartialEq and Eq implementations that ignore build_metadata per semver spec
impl PartialEq for SemVer {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.patch == other.patch
            && self.pre_release == other.pre_release
        // build_metadata is intentionally ignored
    }
}

impl Eq for SemVer {}

impl SemVer {
    pub fn parse(s: &str) -> Option<Self> {
        // Strip build metadata first (everything after +)
        let (version_part, build_metadata) = if let Some(idx) = s.find('+') {
            let (v, b) = s.split_at(idx);
            (v, Some(b[1..].to_string()))
        } else {
            (s, None)
        };

        // Strip pre-release (everything after -)
        let (core_version, pre_release) = if let Some(idx) = version_part.find('-') {
            let (v, p) = version_part.split_at(idx);
            (v, Some(p[1..].to_string()))
        } else {
            (version_part, None)
        };

        // Parse MAJOR.MINOR.PATCH
        let parts: Vec<&str> = core_version.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        Some(SemVer {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
            pre_release,
            build_metadata,
        })
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(ref build) = self.build_metadata {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare major.minor.patch first
        let base_cmp = self
            .major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch));

        if base_cmp != std::cmp::Ordering::Equal {
            return base_cmp;
        }

        // Per semver spec: pre-release versions have lower precedence than normal versions
        // 1.0.0-alpha < 1.0.0
        match (&self.pre_release, &other.pre_release) {
            (None, None) => std::cmp::Ordering::Equal,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => compare_pre_release(a, b),
        }
        // Build metadata is ignored for precedence per semver spec
    }
}

/// Compare pre-release versions per semver 2.0 spec
fn compare_pre_release(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();

    for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
        // Try to parse as numbers
        let a_num = a_part.parse::<u64>();
        let b_num = b_part.parse::<u64>();

        let cmp = match (a_num, b_num) {
            (Ok(a_n), Ok(b_n)) => a_n.cmp(&b_n),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less, // numeric < alphanumeric
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => a_part.cmp(b_part), // lexical comparison
        };

        if cmp != std::cmp::Ordering::Equal {
            return cmp;
        }
    }

    // Shorter pre-release has lower precedence
    a_parts.len().cmp(&b_parts.len())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionConstraint {
    Exact(SemVer),
    Caret(SemVer), // ^1.2.3 := >=1.2.3 <2.0.0
    Tilde(SemVer), // ~1.2.3 := >=1.2.3 <1.3.0
}

impl VersionConstraint {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(rest) = s.strip_prefix('^') {
            SemVer::parse(rest).map(VersionConstraint::Caret)
        } else if let Some(rest) = s.strip_prefix('~') {
            SemVer::parse(rest).map(VersionConstraint::Tilde)
        } else {
            SemVer::parse(s).map(VersionConstraint::Exact)
        }
    }

    pub fn matches(&self, version: &SemVer) -> bool {
        match self {
            VersionConstraint::Exact(req) => version == req,
            VersionConstraint::Caret(req) => {
                if version < req {
                    return false;
                }
                if req.major == 0 {
                    if req.minor == 0 {
                        // ^0.0.x := >=0.0.x <0.0.(x+1) - exact patch match with major=0 and minor=0
                        return version.major == 0
                            && version.minor == 0
                            && version.patch == req.patch;
                    }
                    // ^0.x.y := >=0.x.y <0.(x+1).0
                    return version.major == 0 && version.minor == req.minor;
                }
                // ^1.x.y := >=1.x.y <2.0.0
                version.major == req.major
            }
            VersionConstraint::Tilde(req) => {
                if version < req {
                    return false;
                }
                // ~1.2.3 := >=1.2.3 <1.3.0
                version.major == req.major && version.minor == req.minor
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Bug 1 Tests: Pre-release and build metadata parsing
    #[test]
    fn test_parse_pre_release_versions() {
        // Simple pre-release
        let v = SemVer::parse("1.0.0-alpha").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
        assert_eq!(v.pre_release, Some("alpha".to_string()));
        assert_eq!(v.build_metadata, None);

        // Pre-release with numeric suffix
        let v = SemVer::parse("1.0.0-alpha.1").unwrap();
        assert_eq!(v.pre_release, Some("alpha.1".to_string()));

        // Pre-release with multiple parts
        let v = SemVer::parse("1.0.0-0.3.7").unwrap();
        assert_eq!(v.pre_release, Some("0.3.7".to_string()));

        // RC version
        let v = SemVer::parse("2.0.0-rc.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.pre_release, Some("rc.1".to_string()));

        // Beta version
        let v = SemVer::parse("1.0.0-beta.1").unwrap();
        assert_eq!(v.pre_release, Some("beta.1".to_string()));
    }

    #[test]
    fn test_parse_build_metadata() {
        // Build metadata only
        let v = SemVer::parse("1.0.0+build.1").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
        assert_eq!(v.pre_release, None);
        assert_eq!(v.build_metadata, Some("build.1".to_string()));

        // Build metadata with SHA
        let v = SemVer::parse("1.0.0+20130313144700").unwrap();
        assert_eq!(v.build_metadata, Some("20130313144700".to_string()));
    }

    #[test]
    fn test_parse_pre_release_and_build_metadata() {
        // Both pre-release and build metadata
        let v = SemVer::parse("1.0.0-beta+exp.sha.5114f85").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
        assert_eq!(v.pre_release, Some("beta".to_string()));
        assert_eq!(v.build_metadata, Some("exp.sha.5114f85".to_string()));

        // Complex example
        let v = SemVer::parse("1.0.0-alpha.1+build.123").unwrap();
        assert_eq!(v.pre_release, Some("alpha.1".to_string()));
        assert_eq!(v.build_metadata, Some("build.123".to_string()));
    }

    #[test]
    fn test_display_with_pre_release_and_build() {
        let v = SemVer {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: Some("beta.1".to_string()),
            build_metadata: Some("build.123".to_string()),
        };
        assert_eq!(v.to_string(), "1.0.0-beta.1+build.123");

        let v = SemVer {
            major: 2,
            minor: 1,
            patch: 3,
            pre_release: Some("alpha".to_string()),
            build_metadata: None,
        };
        assert_eq!(v.to_string(), "2.1.3-alpha");

        let v = SemVer {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: None,
            build_metadata: Some("build.1".to_string()),
        };
        assert_eq!(v.to_string(), "1.0.0+build.1");
    }

    // Bug 2 Tests: Caret constraint for 0.0.x versions
    #[test]
    fn test_caret_constraint_0_0_x() {
        let constraint = VersionConstraint::parse("^0.0.3").unwrap();

        // Should match 0.0.3
        let v = SemVer {
            major: 0,
            minor: 0,
            patch: 3,
            pre_release: None,
            build_metadata: None,
        };
        assert!(constraint.matches(&v), "^0.0.3 should match 0.0.3");

        // Should NOT match 0.0.4
        let v = SemVer {
            major: 0,
            minor: 0,
            patch: 4,
            pre_release: None,
            build_metadata: None,
        };
        assert!(!constraint.matches(&v), "^0.0.3 should not match 0.0.4");

        // Should NOT match 0.5.3 (bug fix - was incorrectly matching)
        let v = SemVer {
            major: 0,
            minor: 5,
            patch: 3,
            pre_release: None,
            build_metadata: None,
        };
        assert!(!constraint.matches(&v), "^0.0.3 should not match 0.5.3");

        // Should NOT match 1.0.3
        let v = SemVer {
            major: 1,
            minor: 0,
            patch: 3,
            pre_release: None,
            build_metadata: None,
        };
        assert!(!constraint.matches(&v), "^0.0.3 should not match 1.0.3");

        // Should NOT match 0.0.2
        let v = SemVer {
            major: 0,
            minor: 0,
            patch: 2,
            pre_release: None,
            build_metadata: None,
        };
        assert!(!constraint.matches(&v), "^0.0.3 should not match 0.0.2");
    }

    #[test]
    fn test_caret_constraint_0_x_y() {
        let constraint = VersionConstraint::parse("^0.2.3").unwrap();

        // Should match 0.2.3
        let v = SemVer {
            major: 0,
            minor: 2,
            patch: 3,
            pre_release: None,
            build_metadata: None,
        };
        assert!(constraint.matches(&v));

        // Should match 0.2.5
        let v = SemVer {
            major: 0,
            minor: 2,
            patch: 5,
            pre_release: None,
            build_metadata: None,
        };
        assert!(constraint.matches(&v));

        // Should NOT match 0.3.0
        let v = SemVer {
            major: 0,
            minor: 3,
            patch: 0,
            pre_release: None,
            build_metadata: None,
        };
        assert!(!constraint.matches(&v));

        // Should NOT match 1.2.3
        let v = SemVer {
            major: 1,
            minor: 2,
            patch: 3,
            pre_release: None,
            build_metadata: None,
        };
        assert!(!constraint.matches(&v));
    }

    #[test]
    fn test_caret_constraint_x_y_z() {
        let constraint = VersionConstraint::parse("^1.2.3").unwrap();

        // Should match 1.2.3
        let v = SemVer {
            major: 1,
            minor: 2,
            patch: 3,
            pre_release: None,
            build_metadata: None,
        };
        assert!(constraint.matches(&v));

        // Should match 1.5.0
        let v = SemVer {
            major: 1,
            minor: 5,
            patch: 0,
            pre_release: None,
            build_metadata: None,
        };
        assert!(constraint.matches(&v));

        // Should NOT match 2.0.0
        let v = SemVer {
            major: 2,
            minor: 0,
            patch: 0,
            pre_release: None,
            build_metadata: None,
        };
        assert!(!constraint.matches(&v));
    }

    #[test]
    fn test_pre_release_ordering() {
        // Per semver spec: 1.0.0-alpha < 1.0.0
        let v1 = SemVer::parse("1.0.0-alpha").unwrap();
        let v2 = SemVer::parse("1.0.0").unwrap();
        assert!(v1 < v2);

        // 1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta < 1.0.0-beta < 1.0.0-beta.2 < 1.0.0-beta.11 < 1.0.0-rc.1 < 1.0.0
        let versions = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];

        for i in 0..(versions.len() - 1) {
            let v1 = SemVer::parse(versions[i]).unwrap();
            let v2 = SemVer::parse(versions[i + 1]).unwrap();
            assert!(v1 < v2, "{} should be < {}", versions[i], versions[i + 1]);
        }
    }

    #[test]
    fn test_build_metadata_ignored_in_comparison() {
        // Build metadata should be ignored for precedence
        let v1 = SemVer::parse("1.0.0+build.1").unwrap();
        let v2 = SemVer::parse("1.0.0+build.2").unwrap();
        assert_eq!(v1, v2);

        let v1 = SemVer::parse("1.0.0-alpha+build.1").unwrap();
        let v2 = SemVer::parse("1.0.0-alpha+build.2").unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_tilde_constraint() {
        let constraint = VersionConstraint::parse("~1.2.3").unwrap();

        // Should match 1.2.3
        let v = SemVer {
            major: 1,
            minor: 2,
            patch: 3,
            pre_release: None,
            build_metadata: None,
        };
        assert!(constraint.matches(&v));

        // Should match 1.2.5
        let v = SemVer {
            major: 1,
            minor: 2,
            patch: 5,
            pre_release: None,
            build_metadata: None,
        };
        assert!(constraint.matches(&v));

        // Should NOT match 1.3.0
        let v = SemVer {
            major: 1,
            minor: 3,
            patch: 0,
            pre_release: None,
            build_metadata: None,
        };
        assert!(!constraint.matches(&v));
    }

    #[test]
    fn test_exact_constraint() {
        let constraint = VersionConstraint::parse("1.2.3").unwrap();

        // Should match 1.2.3
        let v = SemVer {
            major: 1,
            minor: 2,
            patch: 3,
            pre_release: None,
            build_metadata: None,
        };
        assert!(constraint.matches(&v));

        // Should NOT match 1.2.4
        let v = SemVer {
            major: 1,
            minor: 2,
            patch: 4,
            pre_release: None,
            build_metadata: None,
        };
        assert!(!constraint.matches(&v));
    }

    #[test]
    fn test_invalid_versions() {
        assert!(SemVer::parse("1.0").is_none());
        assert!(SemVer::parse("1").is_none());
        assert!(SemVer::parse("1.0.0.0").is_none());
        assert!(SemVer::parse("a.b.c").is_none());
        assert!(SemVer::parse("").is_none());
    }
}
