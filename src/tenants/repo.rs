use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    error::{db_err, AppError},
    models::{
        enums::TenantStatus,
        tenant::{CreateTenant, ListTenants, Tenant, TenantList, UpdateTenant},
    },
};

const TENANT_COLS: &str =
    "id, name, route, status, tags, attributes, created_by, updated_by, created_at, updated_at";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantAdminBootstrap {
    pub tenant_id: Uuid,
    pub creator_id: Uuid,
    pub role_name: &'static str,
    pub capabilities: [&'static str; 5],
    pub scope_ref: String,
}

pub fn tenant_admin_bootstrap(tenant_id: Uuid, creator_id: Uuid) -> TenantAdminBootstrap {
    TenantAdminBootstrap {
        tenant_id,
        creator_id,
        role_name: "tenant-admin",
        capabilities: [
            "manage",
            "audit.read",
            "credential.manage",
            "policy.manage",
            "role.manage",
        ],
        scope_ref: tenant_id.to_string(),
    }
}

pub async fn create_tenant(
    pool: &PgPool,
    req: CreateTenant,
    created_by: Option<Uuid>,
) -> Result<Tenant, AppError> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    let tenant = create_tenant_in_tx(&mut tx, req, created_by).await?;
    if let Some(creator_id) = created_by {
        bootstrap_tenant_admin(&mut tx, tenant_admin_bootstrap(tenant.id, creator_id)).await?;
    }
    tx.commit().await.map_err(db_err)?;
    Ok(tenant)
}

async fn create_tenant_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    req: CreateTenant,
    created_by: Option<Uuid>,
) -> Result<Tenant, AppError> {
    let id = Uuid::new_v4();
    let attrs = if req.attributes.is_null() {
        serde_json::json!({})
    } else {
        req.attributes
    };
    sqlx::query_as::<_, Tenant>(&format!(
        r#"INSERT INTO tenants (id, name, route, tags, attributes, created_by, updated_by)
           VALUES ($1, $2, $3, $4, $5, $6, $6)
           RETURNING {TENANT_COLS}"#,
    ))
    .bind(id)
    .bind(req.name)
    .bind(req.route)
    .bind(&req.tags)
    .bind(attrs)
    .bind(created_by)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)
}

async fn bootstrap_tenant_admin(
    tx: &mut Transaction<'_, Postgres>,
    plan: TenantAdminBootstrap,
) -> Result<(), AppError> {
    use sqlx::Row;

    let role_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO roles (id, name, tenant_id, description)
           VALUES ($1, $2, $3, 'Default tenant administration role')"#,
    )
    .bind(role_id)
    .bind(plan.role_name)
    .bind(plan.tenant_id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;

    sqlx::query(
        r#"INSERT INTO role_capabilities (role_id, capability_id)
           SELECT $1, c.id
           FROM capabilities c
           WHERE c.name = ANY($2::text[])
           ON CONFLICT DO NOTHING"#,
    )
    .bind(role_id)
    .bind(plan.capabilities.as_slice())
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;

    let linked_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM role_capabilities WHERE role_id = $1")
            .bind(role_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(db_err)?;
    if linked_count != plan.capabilities.len() as i64 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "tenant-admin bootstrap missing seeded capabilities"
        )));
    }

    sqlx::query(
        r#"INSERT INTO policy_bindings
             (tenant_id, subject_kind, subject_id, grant_kind, grant_id, scope_kind, scope_ref, effect, conditions)
           VALUES ($1, 'entity', $2, 'role', $3, 'tenant', $4, 'allow', '{}')"#,
    )
    .bind(plan.tenant_id)
    .bind(plan.creator_id)
    .bind(role_id)
    .bind(plan.scope_ref)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;

    let creator = sqlx::query("SELECT kind FROM entities WHERE id = $1")
        .bind(plan.creator_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_err)?;

    if creator
        .and_then(|row| row.try_get::<String, _>("kind").ok())
        .as_deref()
        == Some("human")
    {
        sqlx::query(
            r#"INSERT INTO tenant_memberships (tenant_id, entity_id, status)
               VALUES ($1, $2, 'active')
               ON CONFLICT (tenant_id, entity_id) DO NOTHING"#,
        )
        .bind(plan.tenant_id)
        .bind(plan.creator_id)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    }

    Ok(())
}

pub async fn get_tenant(pool: &PgPool, id: Uuid) -> Result<Tenant, AppError> {
    sqlx::query_as::<_, Tenant>(&format!("SELECT {TENANT_COLS} FROM tenants WHERE id = $1"))
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::not_found(format!("tenant {id} not found")),
            other => AppError::Database(other),
        })
}

pub async fn list_tenants(pool: &PgPool, params: ListTenants) -> Result<TenantList, AppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);
    let name = params.name;
    let route = params.route;
    let status = params.status;

    let items = sqlx::query_as::<_, Tenant>(&format!(
        r#"SELECT {TENANT_COLS} FROM tenants
           WHERE ($1::text IS NULL OR name = $1)
             AND ($2::text IS NULL OR route = $2)
             AND ($3::text IS NULL OR status = $3)
           ORDER BY created_at DESC
           LIMIT $4 OFFSET $5"#,
    ))
    .bind(name.clone())
    .bind(route.clone())
    .bind(status.clone())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM tenants
           WHERE ($1::text IS NULL OR name = $1)
             AND ($2::text IS NULL OR route = $2)
             AND ($3::text IS NULL OR status = $3)"#,
    )
    .bind(name)
    .bind(route)
    .bind(status)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    Ok(TenantList { items, total })
}

pub async fn update_tenant(
    pool: &PgPool,
    id: Uuid,
    req: UpdateTenant,
    updated_by: Option<Uuid>,
) -> Result<Tenant, AppError> {
    sqlx::query_as::<_, Tenant>(&format!(
        r#"UPDATE tenants
           SET name       = COALESCE($2, name),
               route      = COALESCE($3, route),
               tags       = COALESCE($4, tags),
               attributes = COALESCE($5, attributes),
               updated_by = $6,
               updated_at = now()
           WHERE id = $1
           RETURNING {TENANT_COLS}"#,
    ))
    .bind(id)
    .bind(req.name)
    .bind(req.route)
    .bind(req.tags)
    .bind(req.attributes)
    .bind(updated_by)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("tenant {id} not found")),
        other => AppError::Database(other),
    })
}

/// Sets `status` to a new value. `Deleted` is the soft-delete state.
/// The row is retained so historical references (audit logs, attributes,
/// etc.) remain resolvable.
pub async fn change_tenant_status(
    pool: &PgPool,
    id: Uuid,
    status: TenantStatus,
    updated_by: Option<Uuid>,
) -> Result<Tenant, AppError> {
    sqlx::query_as::<_, Tenant>(&format!(
        r#"UPDATE tenants
           SET status = $2, updated_by = $3, updated_at = now()
           WHERE id = $1
           RETURNING {TENANT_COLS}"#,
    ))
    .bind(id)
    .bind(status)
    .bind(updated_by)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("tenant {id} not found")),
        other => AppError::Database(other),
    })
}

#[cfg(test)]
mod tests {
    //! DB-gated tests. Each is `#[ignore]` because it needs a live
    //! Postgres reachable via `DATABASE_URL`. Run with:
    //!
    //!     DATABASE_URL=postgres://... cargo test tenants:: -- --ignored
    use super::*;
    use crate::models::tenant::{CreateTenant, ListTenants, UpdateTenant};
    use serde_json::{json, Value};
    use sqlx::PgPool;

    async fn pool() -> PgPool {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&url).await.expect("connect");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrate");
        pool
    }

    async fn cleanup(pool: &PgPool, ids: &[Uuid]) {
        for id in ids {
            let _ = sqlx::query("DELETE FROM tenants WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await;
        }
    }

    fn unique_name(prefix: &str) -> String {
        format!("{prefix}-{}", Uuid::new_v4())
    }

    #[test]
    fn tenant_admin_bootstrap_plan_matches_m5_contract() {
        let tenant_id = Uuid::new_v4();
        let creator_id = Uuid::new_v4();
        let plan = tenant_admin_bootstrap(tenant_id, creator_id);

        assert_eq!(plan.tenant_id, tenant_id);
        assert_eq!(plan.creator_id, creator_id);
        assert_eq!(plan.role_name, "tenant-admin");
        assert_eq!(plan.scope_ref, tenant_id.to_string());
        assert_eq!(
            plan.capabilities,
            [
                "manage",
                "audit.read",
                "credential.manage",
                "policy.manage",
                "role.manage"
            ]
        );
        assert!(!plan.capabilities.contains(&"tenant.manage"));
    }

    #[tokio::test]
    #[ignore]
    async fn create_and_get_roundtrips() {
        let pool = pool().await;
        let req = CreateTenant {
            name: unique_name("acme"),
            route: Some(unique_name("acme-route")),
            tags: vec!["pilot".into()],
            attributes: json!({"region": "eu"}),
        };
        let created = create_tenant(&pool, req, None).await.expect("create");
        assert_eq!(created.status, TenantStatus::Active);
        assert_eq!(created.tags, vec!["pilot".to_string()]);
        let fetched = get_tenant(&pool, created.id).await.expect("get");
        assert_eq!(fetched.id, created.id);
        cleanup(&pool, &[created.id]).await;
    }

    #[tokio::test]
    #[ignore]
    async fn list_filters_by_status() {
        let pool = pool().await;
        let a = create_tenant(
            &pool,
            CreateTenant {
                name: unique_name("list-a"),
                route: None,
                tags: vec![],
                attributes: Value::Null,
            },
            None,
        )
        .await
        .expect("create a");
        let b = create_tenant(
            &pool,
            CreateTenant {
                name: unique_name("list-b"),
                route: None,
                tags: vec![],
                attributes: Value::Null,
            },
            None,
        )
        .await
        .expect("create b");
        change_tenant_status(&pool, b.id, TenantStatus::Inactive, None)
            .await
            .expect("disable b");

        let active = list_tenants(
            &pool,
            ListTenants {
                name: None,
                route: None,
                status: Some(TenantStatus::Active),
                limit: 100,
                offset: 0,
            },
        )
        .await
        .expect("list active");
        assert!(active.items.iter().any(|t| t.id == a.id));
        assert!(!active.items.iter().any(|t| t.id == b.id));
        cleanup(&pool, &[a.id, b.id]).await;
    }

    #[tokio::test]
    #[ignore]
    async fn update_replaces_only_provided_fields() {
        let pool = pool().await;
        let t = create_tenant(
            &pool,
            CreateTenant {
                name: unique_name("upd"),
                route: Some("orig-route".into()),
                tags: vec!["x".into()],
                attributes: json!({"k": "v"}),
            },
            None,
        )
        .await
        .expect("create");
        let upd = update_tenant(
            &pool,
            t.id,
            UpdateTenant {
                name: Some("renamed".into()),
                route: None,
                tags: None,
                attributes: None,
            },
            None,
        )
        .await
        .expect("update");
        assert_eq!(upd.name, "renamed");
        assert_eq!(upd.route.as_deref(), Some("orig-route"));
        assert_eq!(upd.tags, vec!["x".to_string()]);
        cleanup(&pool, &[t.id]).await;
    }

    #[tokio::test]
    #[ignore]
    async fn status_transitions_cover_all_variants() {
        let pool = pool().await;
        let t = create_tenant(
            &pool,
            CreateTenant {
                name: unique_name("status"),
                route: None,
                tags: vec![],
                attributes: Value::Null,
            },
            None,
        )
        .await
        .expect("create");
        for next in [
            TenantStatus::Inactive,
            TenantStatus::Frozen,
            TenantStatus::Active,
            TenantStatus::Deleted,
        ] {
            let updated = change_tenant_status(&pool, t.id, next.clone(), None)
                .await
                .expect("change status");
            assert_eq!(updated.status, next);
        }
        cleanup(&pool, &[t.id]).await;
    }

    #[tokio::test]
    #[ignore]
    async fn entity_with_unknown_tenant_id_is_rejected_by_fk() {
        let pool = pool().await;
        let bogus = Uuid::new_v4();
        let res = sqlx::query(
            "INSERT INTO entities (id, kind, name, tenant_id)
             VALUES (gen_random_uuid(), 'service', 'fk-test', $1)",
        )
        .bind(bogus)
        .execute(&pool)
        .await;
        let err = res.expect_err("FK should reject unknown tenant_id");
        let msg = format!("{err}");
        assert!(
            msg.contains("foreign key") || msg.contains("entities_tenant_id_fkey"),
            "unexpected error: {msg}"
        );
    }
}
