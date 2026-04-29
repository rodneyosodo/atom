//! M2 integration tests.
//!
//! Verifies HTTP-edge legacy-form translation lands canonical values in
//! storage, and that `object_kind = entity` runs end-to-end through the PDP.
//!
//! Run with:
//! ```bash
//! DATABASE_URL=postgres://... cargo test --test m2_compat -- --ignored
//! ```

mod common;

use atom::models::enums::{Effect, GrantKind, ScopeKind, SubjectKind};
use atom::models::policy::{AuthzRequest, CreatePolicyBinding};
use common::{admin_id, pool};
use serde_json::json;
use uuid::Uuid;

async fn read_capability_id(pool: &sqlx::PgPool) -> Uuid {
    sqlx::query_scalar("SELECT id FROM capabilities WHERE name = 'read' LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("read cap")
}

async fn manage_capability_id(pool: &sqlx::PgPool) -> Uuid {
    sqlx::query_scalar("SELECT id FROM capabilities WHERE name = 'manage' LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("manage cap")
}

async fn make_active_entity(pool: &sqlx::PgPool, kind: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO entities (id, kind, name, status) VALUES ($1, $2, $3, 'active')")
        .bind(id)
        .bind(kind)
        .bind(format!("m2-ent-{id}"))
        .execute(pool)
        .await
        .expect("insert entity");
    id
}

#[tokio::test]
#[ignore]
async fn legacy_resource_kind_form_lands_namespaced_in_storage() {
    let p = pool().await;
    let cap_id = read_capability_id(&p).await;
    let subject_id = make_active_entity(&p, "service").await;

    // Simulate a legacy HTTP body. Deserialize through CreatePolicyBinding so
    // the edge translator runs.
    let body = json!({
        "subject_kind": "entity",
        "subject_id": subject_id,
        "grant_kind": "capability",
        "grant_id": cap_id,
        "scope_kind": "resource_kind",
        "scope_ref": "channel",
    });
    let req: CreatePolicyBinding = serde_json::from_value(body).expect("deserialize legacy form");
    req.validate().expect("post-translation form must validate");

    let stored = atom::authz::repo::create_policy(&p, req)
        .await
        .expect("create policy");

    assert_eq!(stored.scope_kind, ScopeKind::ObjectType);
    assert_eq!(stored.scope_ref.as_deref(), Some("resource:channel"));

    // Verify the value in the DB row directly — the legacy form must NOT have
    // landed in storage.
    let raw_kind: String =
        sqlx::query_scalar("SELECT scope_kind::text FROM policy_bindings WHERE id = $1")
            .bind(stored.id)
            .fetch_one(&p)
            .await
            .expect("read scope_kind");
    let raw_ref: String = sqlx::query_scalar("SELECT scope_ref FROM policy_bindings WHERE id = $1")
        .bind(stored.id)
        .fetch_one(&p)
        .await
        .expect("read scope_ref");
    assert_eq!(raw_kind, "object_type");
    assert_eq!(raw_ref, "resource:channel");

    let _ = sqlx::query("DELETE FROM policy_bindings WHERE id = $1")
        .bind(stored.id)
        .execute(&p)
        .await;
    let _ = sqlx::query("DELETE FROM entities WHERE id = $1")
        .bind(subject_id)
        .execute(&p)
        .await;
}

#[tokio::test]
#[ignore]
async fn legacy_all_form_lands_as_platform() {
    let p = pool().await;
    let cap_id = read_capability_id(&p).await;
    let subject_id = make_active_entity(&p, "service").await;

    let body = json!({
        "subject_kind": "entity",
        "subject_id": subject_id,
        "grant_kind": "capability",
        "grant_id": cap_id,
        "scope_kind": "all",
    });
    let req: CreatePolicyBinding = serde_json::from_value(body).expect("deserialize legacy 'all'");
    let stored = atom::authz::repo::create_policy(&p, req)
        .await
        .expect("create policy");

    assert_eq!(stored.scope_kind, ScopeKind::Platform);

    let raw: String =
        sqlx::query_scalar("SELECT scope_kind::text FROM policy_bindings WHERE id = $1")
            .bind(stored.id)
            .fetch_one(&p)
            .await
            .expect("read scope_kind");
    assert_eq!(raw, "platform");

    let _ = sqlx::query("DELETE FROM policy_bindings WHERE id = $1")
        .bind(stored.id)
        .execute(&p)
        .await;
    let _ = sqlx::query("DELETE FROM entities WHERE id = $1")
        .bind(subject_id)
        .execute(&p)
        .await;
}

#[tokio::test]
#[ignore]
async fn entity_as_object_can_be_authorised_via_object_kind_form() {
    let p = pool().await;
    let cap_id = manage_capability_id(&p).await;
    let alice = make_active_entity(&p, "human").await;
    let device = make_active_entity(&p, "device").await;

    // Alice can manage this specific device entity.
    let binding = atom::authz::repo::create_policy(
        &p,
        CreatePolicyBinding {
            tenant_id: None,
            subject_kind: SubjectKind::Entity,
            subject_id: alice,
            grant_kind: GrantKind::Capability,
            grant_id: cap_id,
            scope_kind: ScopeKind::Object,
            scope_ref: Some(device.to_string()),
            effect: Effect::Allow,
            conditions: json!({}),
        },
    )
    .await
    .expect("create policy");

    let req = AuthzRequest {
        subject_id: alice,
        action: "manage".into(),
        resource_id: None,
        object_kind: Some("entity".into()),
        object_id: Some(device),
        context: json!({}),
    };
    let resp = atom::authz::engine::evaluate(&p, &req)
        .await
        .expect("evaluate");
    assert!(
        resp.allowed,
        "alice should manage the device entity: {}",
        resp.reason
    );

    // A different entity should be denied.
    let other_device = make_active_entity(&p, "device").await;
    let deny = AuthzRequest {
        subject_id: alice,
        action: "manage".into(),
        resource_id: None,
        object_kind: Some("entity".into()),
        object_id: Some(other_device),
        context: json!({}),
    };
    let resp = atom::authz::engine::evaluate(&p, &deny)
        .await
        .expect("evaluate");
    assert!(!resp.allowed);

    let _ = sqlx::query("DELETE FROM policy_bindings WHERE id = $1")
        .bind(binding.id)
        .execute(&p)
        .await;
    let _ = sqlx::query("DELETE FROM entities WHERE id = ANY($1::uuid[])")
        .bind(&[alice, device, other_device][..])
        .execute(&p)
        .await;
}

#[tokio::test]
#[ignore]
async fn entity_subtype_scope_uses_namespaced_object_type() {
    let p = pool().await;
    let cap_id = manage_capability_id(&p).await;
    let alice = make_active_entity(&p, "human").await;
    let device1 = make_active_entity(&p, "device").await;
    let device2 = make_active_entity(&p, "device").await;
    let svc = make_active_entity(&p, "service").await;

    // Alice manages every device entity (object_type=entity:device).
    let binding = atom::authz::repo::create_policy(
        &p,
        CreatePolicyBinding {
            tenant_id: None,
            subject_kind: SubjectKind::Entity,
            subject_id: alice,
            grant_kind: GrantKind::Capability,
            grant_id: cap_id,
            scope_kind: ScopeKind::ObjectType,
            scope_ref: Some("entity:device".into()),
            effect: Effect::Allow,
            conditions: json!({}),
        },
    )
    .await
    .expect("create policy");

    for d in [device1, device2] {
        let req = AuthzRequest {
            subject_id: alice,
            action: "manage".into(),
            resource_id: None,
            object_kind: Some("entity".into()),
            object_id: Some(d),
            context: json!({}),
        };
        let resp = atom::authz::engine::evaluate(&p, &req)
            .await
            .expect("evaluate");
        assert!(resp.allowed, "device must match: {}", resp.reason);
    }

    // The service entity must NOT match entity:device.
    let req = AuthzRequest {
        subject_id: alice,
        action: "manage".into(),
        resource_id: None,
        object_kind: Some("entity".into()),
        object_id: Some(svc),
        context: json!({}),
    };
    let resp = atom::authz::engine::evaluate(&p, &req)
        .await
        .expect("evaluate");
    assert!(
        !resp.allowed,
        "service must NOT match entity:device, got {}",
        resp.reason
    );

    let _ = sqlx::query("DELETE FROM policy_bindings WHERE id = $1")
        .bind(binding.id)
        .execute(&p)
        .await;
    let _ = sqlx::query("DELETE FROM entities WHERE id = ANY($1::uuid[])")
        .bind(&[alice, device1, device2, svc][..])
        .execute(&p)
        .await;
}

#[tokio::test]
#[ignore]
async fn admin_platform_inherits_into_entity_objects() {
    // Regression: admin's seeded scope_kind=platform binding must authorise
    // checks against entity-as-object too, not just resources.
    let p = pool().await;
    let target = make_active_entity(&p, "device").await;

    let req = AuthzRequest {
        subject_id: admin_id(),
        action: "manage".into(),
        resource_id: None,
        object_kind: Some("entity".into()),
        object_id: Some(target),
        context: json!({}),
    };
    let resp = atom::authz::engine::evaluate(&p, &req)
        .await
        .expect("evaluate");
    assert!(
        resp.allowed,
        "admin should manage any entity via platform scope: {}",
        resp.reason
    );

    let _ = sqlx::query("DELETE FROM entities WHERE id = $1")
        .bind(target)
        .execute(&p)
        .await;
}
