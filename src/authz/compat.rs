//! HTTP-edge backward compatibility for the M1 scope_kind rename.
//!
//! Per the PRD's Compatibility NFR, legacy callers may still send the old
//! `scope_kind` vocabulary (`all`, `resource_kind`, `resource`). This module
//! converts those into the canonical post-M1 form
//! (`platform`, `object_type`, `object`) at the API edge so the legacy form
//! never reaches storage, audit, or the PDP.
//!
//! - `all` → `platform`
//! - `resource` → `object` (scope_ref preserved)
//! - `resource_kind` → `object_type`, with `scope_ref` prefixed `resource:` if
//!   the value is bare. Already-namespaced values pass through unchanged.

/// Translate a legacy `(scope_kind, scope_ref)` pair to the canonical form.
/// Unknown `scope_kind` values pass through so that the downstream enum
/// deserializer can produce the proper error.
pub fn translate_legacy_scope(
    scope_kind: &str,
    scope_ref: Option<String>,
) -> (String, Option<String>) {
    match scope_kind {
        "all" => ("platform".to_string(), scope_ref),
        "resource" => ("object".to_string(), scope_ref),
        "resource_kind" => {
            let new_ref = scope_ref.map(|r| {
                if r.contains(':') {
                    r
                } else {
                    format!("resource:{r}")
                }
            });
            ("object_type".to_string(), new_ref)
        }
        other => (other.to_string(), scope_ref),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_all_becomes_platform() {
        let (k, r) = translate_legacy_scope("all", None);
        assert_eq!(k, "platform");
        assert!(r.is_none());
    }

    #[test]
    fn legacy_resource_becomes_object_preserving_ref() {
        let uuid = "11111111-1111-1111-1111-111111111111".to_string();
        let (k, r) = translate_legacy_scope("resource", Some(uuid.clone()));
        assert_eq!(k, "object");
        assert_eq!(r, Some(uuid));
    }

    #[test]
    fn legacy_resource_kind_namespaces_bare_scope_ref() {
        let (k, r) = translate_legacy_scope("resource_kind", Some("channel".into()));
        assert_eq!(k, "object_type");
        assert_eq!(r, Some("resource:channel".into()));
    }

    #[test]
    fn legacy_resource_kind_preserves_already_namespaced_ref() {
        let (k, r) = translate_legacy_scope("resource_kind", Some("resource:channel".into()));
        assert_eq!(k, "object_type");
        assert_eq!(r, Some("resource:channel".into()));
    }

    #[test]
    fn legacy_resource_kind_with_no_ref_passes_through() {
        // Caller error, but we don't lose data. Validation will reject later.
        let (k, r) = translate_legacy_scope("resource_kind", None);
        assert_eq!(k, "object_type");
        assert!(r.is_none());
    }

    #[test]
    fn canonical_values_pass_through_unchanged() {
        for canonical in [
            "platform",
            "tenant",
            "object_kind",
            "object_type",
            "object",
            "group_object_type",
            "group_tree_object_type",
            "group_child_kind",
            "group_descendant_kind",
        ] {
            let (k, r) = translate_legacy_scope(canonical, Some("x".into()));
            assert_eq!(k, canonical);
            assert_eq!(r, Some("x".to_string()));
        }
    }

    #[test]
    fn unknown_value_passes_through_for_downstream_error() {
        let (k, _) = translate_legacy_scope("nonsense", None);
        assert_eq!(k, "nonsense");
    }
}
