//! GraphQL authorization and admin operation tests.
//!
//! Run with:
//! ```bash
//! DATABASE_URL=postgres://... cargo test --test m13_graphql_authz_admin -- --ignored
//! ```

mod common;

use async_graphql::Request;
use atom::{
    auth::AuthContext,
    config::{Config, ADMIN_ENTITY_ID},
    graphql::build_schema,
    keys::{ActiveKeys, LoadedKey},
    state::AppState,
};
use sqlx::PgPool;
use uuid::Uuid;

fn state(pool: PgPool) -> AppState {
    let config = Config {
        database_url: "postgres://atom:atom@localhost/atom_test".into(),
        listen_addr: "127.0.0.1:0".into(),
        grpc_addr: "127.0.0.1:0".into(),
        jwt_expiry_secs: 3600,
        jwt_issuer: "http://localhost:8080".to_string(),
        jwt_audience: "magistrala".to_string(),
        admin_entity_id: ADMIN_ENTITY_ID,
        admin_secret: None,
        service_secret: None,
        service_entity_id: atom::config::SERVICE_ENTITY_ID,
        signup_enabled: false,
        dev_allow_unverified_email_login: false,
        public_base_url: "http://localhost:8080".into(),
        cors_allowed_origins: vec!["http://localhost:8080".into()],
        email_verification_redirect: "http://localhost:8080/auth/email/verify".into(),
        password_reset_redirect: "http://localhost:8080/reset-password".into(),
        invitation_redirect: "http://localhost:8080/invitations/accept".into(),
        oauth_success_redirect: "http://localhost:8080".into(),
        oauth_error_redirect: "http://localhost:8080".into(),
        oidc_providers: vec![],
        smtp: None,
        email_verification_expiry_secs: 86_400,
        invitation_expiry_secs: 604_800,
        oauth_state_expiry_secs: 600,
        auth_exchange_code_expiry_secs: 300,
    };
    let primary = LoadedKey {
        kid: "test".into(),
        public_key_pem: String::new(),
        private_key_pem: String::new(),
        x_b64: String::new(),
        y_b64: String::new(),
    };
    AppState::new(
        pool,
        config,
        ActiveKeys {
            primary,
            standby: None,
        },
    )
}

fn authed(query: impl Into<String>) -> Request {
    Request::new(query).data(AuthContext {
        entity_id: common::admin_id(),
        tenant_id: None,
        session_id: None,
    })
}

fn authed_as(entity_id: Uuid, query: impl Into<String>) -> Request {
    Request::new(query).data(AuthContext {
        entity_id,
        tenant_id: None,
        session_id: None,
    })
}

async fn entity(pool: &PgPool, kind: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO entities (id, kind, name, status) VALUES ($1, $2, $3, 'active')")
        .bind(id)
        .bind(kind)
        .bind(format!("graphql-authz-{kind}-{id}"))
        .execute(pool)
        .await
        .expect("insert entity");
    id
}

async fn channel(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO resources (id, kind, name, attributes) VALUES ($1, 'channel', $2, '{}')",
    )
    .bind(id)
    .bind(format!("graphql-channel-{id}"))
    .execute(pool)
    .await
    .expect("insert channel");
    id
}

async fn seeded_capability(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar("SELECT id FROM capabilities WHERE name = $1 LIMIT 1")
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("seeded capability")
}

async fn tenant(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (id, name, status) VALUES ($1, $2, 'active')")
        .bind(id)
        .bind(format!("graphql-tenant-{id}"))
        .execute(pool)
        .await
        .expect("insert tenant");
    id
}

async fn tenant_entity(pool: &PgPool, tenant_id: Uuid, kind: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO entities (id, kind, name, tenant_id, status) VALUES ($1, $2, $3, $4, 'active')",
    )
    .bind(id)
    .bind(kind)
    .bind(format!("graphql-tenant-{kind}-{id}"))
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("insert tenant entity");
    id
}

async fn tenant_group(pool: &PgPool, tenant_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO groups (id, name, tenant_id, attributes) VALUES ($1, $2, $3, '{}')")
        .bind(id)
        .bind(format!("graphql-group-{id}"))
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("insert group");
    id
}

async fn scoped_role(
    pool: &PgPool,
    tenant_id: Uuid,
    name: &str,
    scope_kind: &str,
    scope_ref: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO roles (id, name, tenant_id, scope_kind, scope_ref) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(format!("{name}-{id}"))
    .bind(tenant_id)
    .bind(scope_kind)
    .bind(scope_ref)
    .execute(pool)
    .await
    .expect("insert role");
    id
}

#[tokio::test]
#[ignore]
async fn create_capability_and_list_it() {
    let pool = common::pool().await;
    let schema = build_schema(state(pool));
    let name = format!("graphql-cap-{}", Uuid::new_v4());

    let created = schema
        .execute(authed(format!(
            r#"
            mutation {{
              createCapability(input: {{
                name: "{name}",
                resourceKind: "channel",
                description: "GraphQL capability"
              }}) {{
                id
                name
                resourceKind
                description
              }}
            }}
            "#
        )))
        .await;
    assert!(created.errors.is_empty(), "{:?}", created.errors);
    let capability = &created.data.into_json().expect("json data")["createCapability"];
    let capability_id = capability["id"].as_str().expect("capability id").to_owned();
    assert_eq!(capability["name"], name);
    assert_eq!(capability["resourceKind"], "channel");

    let listed = schema
        .execute(authed(format!(
            r#"
            {{
              capabilities(resourceKind: "channel") {{
                items {{ id name resourceKind }}
                total
              }}
              capability(id: "{capability_id}") {{
                id
                name
              }}
            }}
            "#
        )))
        .await;
    assert!(listed.errors.is_empty(), "{:?}", listed.errors);
    let data = listed.data.into_json().expect("json data");
    assert_eq!(data["capability"]["id"], capability_id);
    assert!(data["capabilities"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .any(|item| item["id"] == capability_id));
}

#[tokio::test]
#[ignore]
async fn unauthenticated_protected_query_fails() {
    let pool = common::pool().await;
    let schema = build_schema(state(pool));

    let response = schema
        .execute(Request::new(
            r#"
            {
              tenants {
                total
              }
            }
            "#,
        ))
        .await;

    assert!(!response.errors.is_empty());
    assert!(response.errors[0]
        .message
        .contains("missing authentication"));
}

#[tokio::test]
#[ignore]
async fn unauthorized_read_query_fails_and_admin_read_succeeds() {
    let pool = common::pool().await;
    let reader_id = entity(&pool, "human").await;
    let schema = build_schema(state(pool));

    let unauthorized = schema
        .execute(authed_as(
            reader_id,
            r#"
            {
              resources {
                total
              }
            }
            "#,
        ))
        .await;
    assert!(!unauthorized.errors.is_empty());
    assert!(unauthorized.errors[0].message.contains("forbidden"));

    let authorized = schema
        .execute(authed(
            r#"
            {
              resources(limit: 1) {
                total
              }
            }
            "#,
        ))
        .await;
    assert!(authorized.errors.is_empty(), "{:?}", authorized.errors);
}

#[tokio::test]
#[ignore]
async fn entity_query_allows_self_read_without_policy() {
    let pool = common::pool().await;
    let entity_id = entity(&pool, "human").await;
    let schema = build_schema(state(pool));

    let response = schema
        .execute(authed_as(
            entity_id,
            format!(
                r#"
                {{
                  entity(id: "{entity_id}") {{
                    id
                    name
                  }}
                }}
                "#
            ),
        ))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let entity = &response.data.into_json().expect("json data")["entity"];
    assert_eq!(entity["id"], entity_id.to_string());
}

#[tokio::test]
#[ignore]
async fn create_role_and_attach_capability() {
    let pool = common::pool().await;
    let publish_id = seeded_capability(&pool, "publish").await;
    let schema = build_schema(state(pool));
    let name = format!("graphql-role-{}", Uuid::new_v4());

    let created = schema
        .execute(authed(format!(
            r#"
            mutation {{
              createRole(input: {{
                name: "{name}",
                description: "GraphQL role"
              }}) {{
                id
                name
                description
              }}
            }}
            "#
        )))
        .await;
    assert!(created.errors.is_empty(), "{:?}", created.errors);
    let role = &created.data.into_json().expect("json data")["createRole"];
    let role_id = role["id"].as_str().expect("role id").to_owned();
    assert_eq!(role["name"], name);

    let added = schema
        .execute(authed(format!(
            r#"
            mutation {{
              addRoleCapability(roleId: "{role_id}", capabilityId: "{publish_id}")
            }}
            "#
        )))
        .await;
    assert!(added.errors.is_empty(), "{:?}", added.errors);
    assert_eq!(
        added.data.into_json().expect("json data")["addRoleCapability"],
        true
    );

    let listed = schema
        .execute(authed(format!(
            r#"
            {{
              roles {{
                items {{ id name }}
              }}
              role(id: "{role_id}") {{ id name }}
              roleCapabilities(roleId: "{role_id}") {{ id name }}
            }}
            "#
        )))
        .await;
    assert!(listed.errors.is_empty(), "{:?}", listed.errors);
    let data = listed.data.into_json().expect("json data");
    assert_eq!(data["role"]["id"], role_id);
    assert!(data["roles"]["items"]
        .as_array()
        .expect("roles")
        .iter()
        .any(|item| item["id"] == role_id));
    assert!(data["roleCapabilities"]
        .as_array()
        .expect("capabilities")
        .iter()
        .any(|item| item["id"] == publish_id.to_string()));
}

#[tokio::test]
#[ignore]
async fn create_policy_and_authz_check_allow_and_deny() {
    let pool = common::pool().await;
    let device_id = entity(&pool, "device").await;
    let channel_id = channel(&pool).await;
    let publish_id = seeded_capability(&pool, "publish").await;
    let schema = build_schema(state(pool));

    let created = schema
        .execute(authed(format!(
            r#"
            mutation {{
              createPolicy(input: {{
                subjectKind: entity,
                subjectId: "{device_id}",
                grantKind: capability,
                grantId: "{publish_id}",
                scopeKind: object,
                scopeRef: "{channel_id}",
                effect: allow
              }}) {{
                id
                subjectKind
                grantKind
                scopeKind
                scopeRef
                effect
              }}
            }}
            "#
        )))
        .await;
    assert!(created.errors.is_empty(), "{:?}", created.errors);
    let policy = &created.data.into_json().expect("json data")["createPolicy"];
    let policy_id = policy["id"].as_str().expect("policy id").to_owned();
    assert_eq!(policy["effect"], "allow");

    let checked = schema
        .execute(authed(format!(
            r#"
            mutation {{
              allow: authzCheck(input: {{
                subjectId: "{device_id}",
                action: "publish",
                resourceId: "{channel_id}"
              }}) {{
                allowed
                reason
              }}
              deny: authzCheck(input: {{
                subjectId: "{device_id}",
                action: "subscribe",
                resourceId: "{channel_id}"
              }}) {{
                allowed
                reason
              }}
            }}
            "#
        )))
        .await;
    assert!(checked.errors.is_empty(), "{:?}", checked.errors);
    let data = checked.data.into_json().expect("json data");
    assert_eq!(data["allow"]["allowed"], true);
    assert_eq!(data["deny"]["allowed"], false);

    let listed = schema
        .execute(authed(format!(
            r#"
            {{
              policies(subjectId: "{device_id}", subjectKind: entity) {{
                items {{ id subjectId }}
                total
              }}
              policy(id: "{policy_id}") {{ id subjectId }}
            }}
            "#
        )))
        .await;
    assert!(listed.errors.is_empty(), "{:?}", listed.errors);
    let data = listed.data.into_json().expect("json data");
    assert_eq!(data["policy"]["id"], policy_id);
    assert!(data["policies"]["items"]
        .as_array()
        .expect("policies")
        .iter()
        .any(|item| item["id"] == policy_id));
}

#[tokio::test]
#[ignore]
async fn subject_role_assignments_list_group_composites_and_authorize_members() {
    let pool = common::pool().await;
    let tenant_id = tenant(&pool).await;
    let user_id = tenant_entity(&pool, tenant_id, "human").await;
    let group_id = tenant_group(&pool, tenant_id).await;
    sqlx::query("INSERT INTO group_members (group_id, entity_id) VALUES ($1, $2)")
        .bind(group_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("insert group member");
    let read_id = seeded_capability(&pool, "read").await;
    let simple_role_id = scoped_role(
        &pool,
        tenant_id,
        "group-reader",
        "object",
        &group_id.to_string(),
    )
    .await;
    sqlx::query("INSERT INTO role_capabilities (role_id, capability_id) VALUES ($1, $2)")
        .bind(simple_role_id)
        .bind(read_id)
        .execute(&pool)
        .await
        .expect("insert role capability");
    let composite_role_id = scoped_role(
        &pool,
        tenant_id,
        "group-operator",
        "object",
        &group_id.to_string(),
    )
    .await;
    sqlx::query("INSERT INTO role_composites (parent_role_id, child_role_id) VALUES ($1, $2)")
        .bind(composite_role_id)
        .bind(simple_role_id)
        .execute(&pool)
        .await
        .expect("insert role composite");
    let schema = build_schema(state(pool));

    let assigned = schema
        .execute(authed(format!(
            r#"
            mutation {{
              createPolicy(input: {{
                tenantId: "{tenant_id}",
                subjectKind: group,
                subjectId: "{group_id}",
                grantKind: role,
                grantId: "{composite_role_id}",
                scopeKind: tenant,
                scopeRef: "{tenant_id}",
                effect: allow
              }}) {{
                id
              }}
            }}
            "#
        )))
        .await;
    assert!(assigned.errors.is_empty(), "{:?}", assigned.errors);

    let listed = schema
        .execute(authed(format!(
            r#"
            {{
              subjectRoleAssignments(
                tenantId: "{tenant_id}",
                subjectKind: group,
                subjectId: "{group_id}",
                derivedKind: "composite"
              ) {{
                total
                items {{
                  policy {{ subjectKind subjectId grantId }}
                  role {{ id name derivedKind childRoles {{ id }} }}
                }}
              }}
            }}
            "#
        )))
        .await;
    assert!(listed.errors.is_empty(), "{:?}", listed.errors);
    let data = listed.data.into_json().expect("json data");
    let assignments = &data["subjectRoleAssignments"];
    assert_eq!(assignments["total"], 1);
    assert_eq!(
        assignments["items"][0]["role"]["id"],
        composite_role_id.to_string()
    );
    assert_eq!(assignments["items"][0]["role"]["derivedKind"], "composite");
    assert_eq!(
        assignments["items"][0]["role"]["childRoles"][0]["id"],
        simple_role_id.to_string()
    );

    let checked = schema
        .execute(authed_as(
            user_id,
            format!(
                r#"
                mutation {{
                  authzCheck(input: {{
                    subjectId: "{user_id}",
                    action: "read",
                    objectKind: "group",
                    objectId: "{group_id}"
                  }}) {{
                    allowed
                    reason
                  }}
                }}
                "#
            ),
        ))
        .await;
    assert!(checked.errors.is_empty(), "{:?}", checked.errors);
    let data = checked.data.into_json().expect("json data");
    assert_eq!(data["authzCheck"]["allowed"], true);
}

#[tokio::test]
#[ignore]
async fn create_policy_rejects_cross_tenant_role_assignment() {
    let pool = common::pool().await;
    let tenant_a = tenant(&pool).await;
    let tenant_b = tenant(&pool).await;
    let group_id = tenant_group(&pool, tenant_a).await;
    let foreign_role_id = scoped_role(
        &pool,
        tenant_b,
        "foreign-role",
        "tenant",
        &tenant_b.to_string(),
    )
    .await;
    let schema = build_schema(state(pool));

    let response = schema
        .execute(authed(format!(
            r#"
            mutation {{
              createPolicy(input: {{
                tenantId: "{tenant_a}",
                subjectKind: group,
                subjectId: "{group_id}",
                grantKind: role,
                grantId: "{foreign_role_id}",
                scopeKind: tenant,
                scopeRef: "{tenant_a}",
                effect: allow
              }}) {{
                id
              }}
            }}
            "#
        )))
        .await;

    assert!(!response.errors.is_empty());
    assert!(response.errors[0]
        .message
        .contains("role from another tenant"));
}

#[tokio::test]
#[ignore]
async fn authz_explain_returns_decision_details() {
    let pool = common::pool().await;
    let device_id = entity(&pool, "device").await;
    let channel_id = channel(&pool).await;
    let publish_id = seeded_capability(&pool, "publish").await;
    let schema = build_schema(state(pool));

    let created = schema
        .execute(authed(format!(
            r#"
            mutation {{
              createPolicy(input: {{
                subjectKind: entity,
                subjectId: "{device_id}",
                grantKind: capability,
                grantId: "{publish_id}",
                scopeKind: object,
                scopeRef: "{channel_id}"
              }}) {{
                id
              }}
            }}
            "#
        )))
        .await;
    assert!(created.errors.is_empty(), "{:?}", created.errors);

    let explained = schema
        .execute(authed(format!(
            r#"
            mutation {{
              authzExplain(input: {{
                subjectId: "{device_id}",
                action: "publish",
                resourceId: "{channel_id}"
              }}) {{
                allowed
                reason
                subject
                resource
                capability
                matchedBinding
                evaluatedBindings
              }}
            }}
            "#
        )))
        .await;
    assert!(explained.errors.is_empty(), "{:?}", explained.errors);
    let explain = &explained.data.into_json().expect("json data")["authzExplain"];
    assert_eq!(explain["allowed"], true);
    assert_eq!(explain["reason"], "allowed");
    assert!(explain["matchedBinding"].is_object());
    assert!(explain["evaluatedBindings"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
}

#[tokio::test]
#[ignore]
async fn authz_explain_requires_stronger_permission_than_check() {
    let pool = common::pool().await;
    let device_id = entity(&pool, "device").await;
    let channel_id = channel(&pool).await;
    let schema = build_schema(state(pool));

    let check = schema
        .execute(authed_as(
            device_id,
            format!(
                r#"
                mutation {{
                  authzCheck(input: {{
                    subjectId: "{device_id}",
                    action: "publish",
                    resourceId: "{channel_id}"
                  }}) {{
                    allowed
                  }}
                }}
                "#
            ),
        ))
        .await;
    assert!(check.errors.is_empty(), "{:?}", check.errors);

    let explain = schema
        .execute(authed_as(
            device_id,
            format!(
                r#"
                mutation {{
                  authzExplain(input: {{
                    subjectId: "{device_id}",
                    action: "publish",
                    resourceId: "{channel_id}"
                  }}) {{
                    allowed
                  }}
                }}
                "#
            ),
        ))
        .await;
    assert!(!explain.errors.is_empty());
    assert!(explain.errors[0].message.contains("forbidden"));
}

#[tokio::test]
#[ignore]
async fn ownership_mutation_requires_manage_permission() {
    let pool = common::pool().await;
    let owner_id = entity(&pool, "human").await;
    let owned_id = entity(&pool, "device").await;
    let schema = build_schema(state(pool));

    let response = schema
        .execute(authed_as(
            owner_id,
            format!(
                r#"
                mutation {{
                  addOwnership(ownerId: "{owner_id}", ownedId: "{owned_id}") {{
                    ownerId
                    ownedId
                  }}
                }}
                "#
            ),
        ))
        .await;

    assert!(!response.errors.is_empty());
    assert!(response.errors[0].message.contains("forbidden"));
}

#[tokio::test]
#[ignore]
async fn audit_and_admin_queries_smoke() {
    let pool = common::pool().await;
    let device_id = entity(&pool, "device").await;
    let channel_id = channel(&pool).await;
    let schema = build_schema(state(pool));

    let checked = schema
        .execute(authed(format!(
            r#"
            mutation {{
              authzCheck(input: {{
                subjectId: "{device_id}",
                action: "subscribe",
                resourceId: "{channel_id}"
              }}) {{
                allowed
              }}
            }}
            "#
        )))
        .await;
    assert!(checked.errors.is_empty(), "{:?}", checked.errors);

    let queried = schema
        .execute(authed(format!(
            r#"
            {{
              auditLogs(limit: 5) {{
                items {{ id event outcome }}
                total
              }}
              entityAuditLogs(entityId: "{device_id}") {{
                items {{ id event outcome }}
                total
              }}
              orphanPolicies(limit: 1) {{ id }}
              unprotectedResources(limit: 1) {{ id kind }}
              expiringCredentials(limit: 1) {{ id entityId kind status }}
            }}
            "#
        )))
        .await;
    assert!(queried.errors.is_empty(), "{:?}", queried.errors);
    let data = queried.data.into_json().expect("json data");
    assert!(data["auditLogs"]["items"].is_array());
    assert!(data["entityAuditLogs"]["items"].is_array());
    assert!(data["orphanPolicies"].is_array());
    assert!(data["unprotectedResources"].is_array());
    assert!(data["expiringCredentials"].is_array());
}
