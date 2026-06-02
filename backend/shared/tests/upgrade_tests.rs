use shared::upgrade::{compare_schemas, Field, Schema, Severity};

#[test]
fn detects_field_removal_as_error() {
    let old = Schema {
        fields: vec![Field {
            name: "count".into(),
            type_name: "u64".into(),
        }],
    };
    let new = Schema { fields: vec![] };
    let findings = compare_schemas(&old, &new);
    assert!(findings.iter().any(|f| f.severity == Severity::Error));
}

#[test]
fn detects_type_change_as_warning() {
    let old = Schema {
        fields: vec![Field {
            name: "owner".into(),
            type_name: "bytes".into(),
        }],
    };
    let new = Schema {
        fields: vec![Field {
            name: "owner".into(),
            type_name: "string".into(),
        }],
    };
    let findings = compare_schemas(&old, &new);
    assert!(findings.iter().any(|f| f.severity == Severity::Warning));
}

#[test]
fn additions_are_info() {
    let old = Schema { fields: vec![] };
    let new = Schema {
        fields: vec![Field {
            name: "balance".into(),
            type_name: "u128".into(),
        }],
    };
    let findings = compare_schemas(&old, &new);
    assert!(findings.iter().any(|f| f.severity == Severity::Info));
}
