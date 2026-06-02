// Verification tests for the bug fixes described in the issue

use shared::{SemVer, VersionConstraint};

#[test]
fn bug_1_pre_release_versions_parse_correctly() {
    // Bug 1: Valid semver rejected
    assert!(
        SemVer::parse("1.0.0-beta.1").is_some(),
        "1.0.0-beta.1 should parse"
    );
    assert!(
        SemVer::parse("2.0.0-rc.1").is_some(),
        "2.0.0-rc.1 should parse"
    );
    assert!(
        SemVer::parse("1.0.0+build.123").is_some(),
        "1.0.0+build.123 should parse"
    );
}

#[test]
fn bug_2_caret_constraint_0_0_x_matches_correctly() {
    // Bug 2: Wrong version matches
    let constraint = VersionConstraint::parse("^0.0.3").unwrap();

    // Should NOT match 0.5.3 (this was the bug - it incorrectly matched)
    let version = SemVer {
        major: 0,
        minor: 5,
        patch: 3,
        pre_release: None,
        build_metadata: None,
    };
    assert!(
        !constraint.matches(&version),
        "^0.0.3 should NOT match 0.5.3 (bug fix verification)"
    );
}
