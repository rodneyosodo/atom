//! API template metadata tests.
//!
//! Run with:
//! ```bash
//! DATABASE_URL=postgres://... cargo test --test m14_api_templates -- --ignored
//! ```

mod common;

use async_graphql::Request;
use atom::{
    api_templates::repo as api_template_repo,
    auth::AuthContext,
    config::{Config, ADMIN_ENTITY_ID},
    graphql::build_schema,
    keys::{ActiveKeys, LoadedKey},
    models::{
        api_template::{
            ApiTemplateOperationKind, ApiTemplateStatus, CreateApiTemplate, ListApiTemplates,
            UpdateApiTemplate,
        },
        tenant::CreateTenant,
    },
    state::AppState,
    tenants::repo as tenant_repo,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn state(pool: PgPool) -> AppState {
    let config = Config {
        database_url: "postgres://atom:atom@localhost/atom_test".into(),
        listen_addr: "127.0.0.1:0".into(),
        grpc_addr: "127.0.0.1:0".into(),
        jwt_expiry_secs: 3600,
        admin_entity_id: ADMIN_ENTITY_ID,
        admin_secret: None,
        graphql_console_enabled: false,
        graphql_console_dist_dir: "console/dist".into(),
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

async fn tenant(pool: &PgPool) -> Uuid {
    let suffix = Uuid::new_v4();
    tenant_repo::create_tenant(
        pool,
        CreateTenant {
            name: format!("api-template-tenant-{suffix}"),
            route: Some(format!("api-template-tenant-{suffix}")),
            tags: vec!["api-template-test".into()],
            attributes: json!({}),
        },
        Some(common::admin_id()),
    )
    .await
    .expect("create tenant")
    .id
}

fn create_req(key: String, tenant_id: Option<Uuid>) -> CreateApiTemplate {
    CreateApiTemplate {
        tenant_id,
        key,
        name: "Repo template".into(),
        description: Some("created by repo test".into()),
        operation_kind: ApiTemplateOperationKind::Query,
        graphql: "query RepoTemplate { health }".into(),
        variables_schema: json!({"type": "object"}),
        default_variables: json!({}),
        result_selector: json!({"path": ["health"]}),
        tags: vec!["repo".into(), "api-template-test".into()],
        status: Some(ApiTemplateStatus::Active),
    }
}

#[tokio::test]
#[ignore]
async fn repo_create_list_get_update_and_disable_template() {
    let pool = common::pool().await;
    let key = format!("repo_template_{}", Uuid::new_v4());

    let created = api_template_repo::create_api_template(
        &pool,
        create_req(key.clone(), None),
        Some(common::admin_id()),
    )
    .await
    .expect("create api template");
    assert_eq!(created.key, key);
    assert_eq!(created.operation_kind, ApiTemplateOperationKind::Query);
    assert_eq!(created.status, ApiTemplateStatus::Active);

    let list = api_template_repo::list_api_templates(
        &pool,
        ListApiTemplates {
            tenant_id: None,
            status: Some(ApiTemplateStatus::Active),
            tag: Some("repo".into()),
            limit: 50,
            offset: 0,
        },
    )
    .await
    .expect("list api templates");
    assert!(list.items.iter().any(|template| template.id == created.id));

    let fetched = api_template_repo::get_api_template(&pool, created.id)
        .await
        .expect("get api template");
    assert_eq!(fetched.key, key);

    let updated = api_template_repo::update_api_template(
        &pool,
        created.id,
        UpdateApiTemplate {
            key: None,
            name: Some("Updated repo template".into()),
            description: Some("updated".into()),
            operation_kind: Some(ApiTemplateOperationKind::Mutation),
            graphql: Some("mutation UpdatedRepoTemplate { logout }".into()),
            variables_schema: None,
            default_variables: Some(json!({"input": {}})),
            result_selector: None,
            tags: Some(vec!["updated".into()]),
            status: Some(ApiTemplateStatus::Deprecated),
        },
        Some(common::admin_id()),
    )
    .await
    .expect("update api template");
    assert_eq!(updated.name, "Updated repo template");
    assert_eq!(updated.operation_kind, ApiTemplateOperationKind::Mutation);
    assert_eq!(updated.status, ApiTemplateStatus::Deprecated);
    assert_eq!(updated.tags, vec!["updated"]);

    api_template_repo::disable_api_template(&pool, created.id, Some(common::admin_id()))
        .await
        .expect("disable api template");
    let disabled = api_template_repo::get_api_template(&pool, created.id)
        .await
        .expect("get disabled api template");
    assert_eq!(disabled.status, ApiTemplateStatus::Disabled);
}

#[tokio::test]
#[ignore]
async fn repo_enforces_global_and_tenant_unique_keys() {
    let pool = common::pool().await;
    let key = format!("unique_template_{}", Uuid::new_v4());
    let tenant_one = tenant(&pool).await;
    let tenant_two = tenant(&pool).await;

    api_template_repo::create_api_template(
        &pool,
        create_req(key.clone(), None),
        Some(common::admin_id()),
    )
    .await
    .expect("create global template");
    let duplicate_global = api_template_repo::create_api_template(
        &pool,
        create_req(key.clone(), None),
        Some(common::admin_id()),
    )
    .await;
    assert!(duplicate_global.is_err());

    api_template_repo::create_api_template(
        &pool,
        create_req(key.clone(), Some(tenant_one)),
        Some(common::admin_id()),
    )
    .await
    .expect("create tenant template with global key");
    let duplicate_tenant = api_template_repo::create_api_template(
        &pool,
        create_req(key.clone(), Some(tenant_one)),
        Some(common::admin_id()),
    )
    .await;
    assert!(duplicate_tenant.is_err());

    api_template_repo::create_api_template(
        &pool,
        create_req(key, Some(tenant_two)),
        Some(common::admin_id()),
    )
    .await
    .expect("same key is allowed in a different tenant");
}

#[tokio::test]
#[ignore]
async fn graphql_api_templates_query_returns_seeded_templates() {
    let pool = common::pool().await;
    let schema = build_schema(state(pool));

    let response = schema
        .execute(authed(
            r#"
            {
              apiTemplates(status: active, tag: "setup", limit: 20) {
                items {
                  key
                  name
                  operationKind
                  status
                  tags
                }
                total
              }
            }
            "#,
        ))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().expect("json data");
    assert!(data["apiTemplates"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .any(|item| item["key"] == "create_tenant"));
}

#[tokio::test]
#[ignore]
async fn graphql_create_api_template_mutation_creates_template() {
    let pool = common::pool().await;
    let schema = build_schema(state(pool));
    let key = format!("graphql_template_{}", Uuid::new_v4());

    let response = schema
        .execute(authed(format!(
            r#"
            mutation {{
              createApiTemplate(input: {{
                key: "{key}",
                name: "GraphQL-created template",
                operationKind: mutation,
                graphql: "mutation GraphqlTemplate {{ logout }}",
                defaultVariables: {{}},
                resultSelector: {{ path: ["logout"] }},
                tags: ["graphql", "api-template-test"]
              }}) {{
                id
                key
                name
                operationKind
                status
                tags
              }}
            }}
            "#
        )))
        .await;

    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let template = &response.data.into_json().expect("json data")["createApiTemplate"];
    assert_eq!(template["key"], key);
    assert_eq!(template["operationKind"], "mutation");
    assert_eq!(template["status"], "active");
}

#[tokio::test]
#[ignore]
async fn seeded_templates_use_generic_atom_operations_only() {
    let pool = common::pool().await;
    let rows: Vec<(String, String)> = sqlx::query_as(
        r#"SELECT key, graphql FROM api_templates
           WHERE tenant_id IS NULL
             AND key = ANY($1::text[])
           ORDER BY key"#,
    )
    .bind([
        "create_tenant",
        "create_entity_from_profile",
        "create_resource",
        "create_policy",
        "authz_check",
        "create_api_key",
    ])
    .fetch_all(&pool)
    .await
    .expect("seeded templates");

    assert_eq!(rows.len(), 6);
    for (key, graphql) in rows {
        for operation in ["createDomain", "createClient", "createChannel"] {
            assert!(
                !graphql.contains(operation),
                "seeded template {key} contained {operation}"
            );
        }
    }
}
