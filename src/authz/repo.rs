use std::collections::{BTreeMap, HashMap};

use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{db_err, AppError},
    models::{
        access::{
            AccessItem, AccessQuery, AdminPageQuery, AuditLogItem, AuditLogResponse,
            CapabilitySource, CapabilitySummary, EffectiveCapabilitiesQuery,
            EffectiveCapabilitiesResponse, EffectiveCapability, EntityAccessResponse,
            EntitySummary, ExpiringCredentialItem, ExpiringCredentialsQuery,
            ExpiringCredentialsResponse, GrantSummary, GroupAccessItem, GroupAccessQuery,
            GroupAccessResponse, GroupInfo, OrphanPoliciesResponse, OrphanPolicyItem,
            ResourceAccessEntity, ResourceAccessItem, ResourceAccessQuery, ResourceAccessResponse,
            ResourceSummary, RoleHolderGroup, RoleHolderItem, RoleHoldersQuery,
            RoleHoldersResponse, RoleSummary, RoleWithCapabilities, SubjectRoleAssignment,
            SubjectRoleAssignmentList, SubjectRoleAssignmentsQuery, UnprotectedResourceItem,
            UnprotectedResourcesQuery, UnprotectedResourcesResponse,
        },
        capability::{Capability, CreateCapability, ListCapabilities},
        entity::Entity,
        enums::{CredentialKind, Effect, GrantKind, ScopeKind, SubjectKind},
        group::Group,
        policy::{CreatePolicyBinding, ListPolicies, PolicyBinding, PolicyList},
        resource::{CreateResource, ListResources, Resource, ResourceList, UpdateResource},
        role::{
            CreateRole, CreateRolePermissionBlock, ListRoles, Role, RoleDerivedKind, RoleList,
            RolePermissionBlock, UpdateRole,
        },
    },
};

// ─── Resources ────────────────────────────────────────────────────────────────

pub async fn create_resource(pool: &PgPool, req: CreateResource) -> Result<Resource, AppError> {
    let id = req.id.unwrap_or_else(Uuid::new_v4);
    let attrs = if req.attributes.is_null() {
        serde_json::json!({})
    } else {
        req.attributes
    };
    let parent_group_id = parent_group_id_from_attrs(&attrs)?;
    let mut tx = pool.begin().await.map_err(db_err)?;
    let resource = sqlx::query_as::<_, Resource>(
        r#"INSERT INTO resources (id, kind, name, tenant_id, owner_id, attributes)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, kind, name, tenant_id, owner_id, attributes, created_at, updated_at"#,
    )
    .bind(id)
    .bind(req.kind)
    .bind(req.name)
    .bind(req.tenant_id)
    .bind(req.owner_id)
    .bind(attrs)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;
    if let Some(parent_group_id) = parent_group_id {
        set_resource_parent_group_in_tx(&mut tx, resource.id, parent_group_id).await?;
    }
    tx.commit().await.map_err(db_err)?;
    Ok(resource)
}

pub async fn get_resource(pool: &PgPool, id: Uuid) -> Result<Resource, AppError> {
    sqlx::query_as::<_, Resource>(
        "SELECT id, kind, name, tenant_id, owner_id, attributes, created_at, updated_at FROM resources WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("resource {id} not found")),
        other => AppError::Database(other),
    })
}

pub async fn list_resources(
    pool: &PgPool,
    params: ListResources,
) -> Result<ResourceList, AppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let kind = params.kind;
    let tenant_id = params.tenant_id;
    let parent_group_id = params.parent_group_id;
    let include_descendants = params.include_descendants;
    let q = search_pattern(params.q);

    let items = sqlx::query_as::<_, Resource>(
        r#"WITH RECURSIVE target_groups(id) AS (
               SELECT $4::uuid WHERE $4::uuid IS NOT NULL
               UNION ALL
               SELECT gh.child_id
               FROM group_hierarchy gh
               JOIN target_groups tg ON tg.id = gh.parent_id
               WHERE $5::boolean
           )
           SELECT r.id, r.kind, r.name, r.tenant_id, r.owner_id, r.attributes, r.created_at, r.updated_at
           FROM resources r
           LEFT JOIN group_resource_parents grp ON grp.resource_id = r.id
           WHERE ($1::text IS NULL OR r.kind = $1)
             AND ($2::uuid IS NULL OR r.tenant_id = $2)
             AND ($3::text IS NULL OR r.name ILIKE $3 OR r.attributes::text ILIKE $3)
             AND ($4::uuid IS NULL OR grp.group_id IN (SELECT id FROM target_groups))
           ORDER BY r.created_at DESC
           LIMIT $6 OFFSET $7"#,
    )
    .bind(kind.clone())
    .bind(tenant_id)
    .bind(q.clone())
    .bind(parent_group_id)
    .bind(include_descendants)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total: i64 = sqlx::query_scalar(
        r#"WITH RECURSIVE target_groups(id) AS (
               SELECT $4::uuid WHERE $4::uuid IS NOT NULL
               UNION ALL
               SELECT gh.child_id
               FROM group_hierarchy gh
               JOIN target_groups tg ON tg.id = gh.parent_id
               WHERE $5::boolean
           )
           SELECT COUNT(*)
           FROM resources r
           LEFT JOIN group_resource_parents grp ON grp.resource_id = r.id
           WHERE ($1::text IS NULL OR r.kind = $1)
             AND ($2::uuid IS NULL OR r.tenant_id = $2)
             AND ($3::text IS NULL OR r.name ILIKE $3 OR r.attributes::text ILIKE $3)
             AND ($4::uuid IS NULL OR grp.group_id IN (SELECT id FROM target_groups))"#,
    )
    .bind(kind)
    .bind(tenant_id)
    .bind(q)
    .bind(parent_group_id)
    .bind(include_descendants)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    Ok(ResourceList { items, total })
}

pub async fn update_resource(
    pool: &PgPool,
    id: Uuid,
    req: UpdateResource,
) -> Result<Resource, AppError> {
    let parent_group_id = req
        .attributes
        .as_ref()
        .and_then(|attrs| attrs.get("parent_group_id"))
        .map(parent_group_id_from_value)
        .transpose()?;
    let mut tx = pool.begin().await.map_err(db_err)?;
    let resource = sqlx::query_as::<_, Resource>(
        r#"UPDATE resources
           SET name       = COALESCE($2, name),
               attributes = COALESCE($3, attributes),
               updated_at = now()
           WHERE id = $1
           RETURNING id, kind, name, tenant_id, owner_id, attributes, created_at, updated_at"#,
    )
    .bind(id)
    .bind(req.name)
    .bind(req.attributes)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("resource {id} not found")),
        other => AppError::Database(other),
    })?;
    if let Some(parent_group_id) = parent_group_id {
        match parent_group_id {
            Some(parent_group_id) => {
                set_resource_parent_group_in_tx(&mut tx, resource.id, parent_group_id).await?;
            }
            None => clear_resource_parent_group_in_tx(&mut tx, resource.id).await?,
        }
    }
    tx.commit().await.map_err(db_err)?;
    Ok(resource)
}

pub async fn delete_resource(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM resources WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(db_err)?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!("resource {id} not found")));
    }
    Ok(())
}

pub async fn get_resource_parent_group(
    pool: &PgPool,
    resource_id: Uuid,
) -> Result<Option<Uuid>, AppError> {
    sqlx::query_scalar("SELECT group_id FROM group_resource_parents WHERE resource_id = $1")
        .bind(resource_id)
        .fetch_optional(pool)
        .await
        .map_err(db_err)
}

pub async fn set_resource_parent_group(
    pool: &PgPool,
    resource_id: Uuid,
    group_id: Uuid,
) -> Result<Resource, AppError> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    set_resource_parent_group_in_tx(&mut tx, resource_id, group_id).await?;
    tx.commit().await.map_err(db_err)?;
    get_resource(pool, resource_id).await
}

pub async fn clear_resource_parent_group(
    pool: &PgPool,
    resource_id: Uuid,
) -> Result<Resource, AppError> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    clear_resource_parent_group_in_tx(&mut tx, resource_id).await?;
    tx.commit().await.map_err(db_err)?;
    get_resource(pool, resource_id).await
}

async fn set_resource_parent_group_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    resource_id: Uuid,
    group_id: Uuid,
) -> Result<(), AppError> {
    use sqlx::Row;
    let row = sqlx::query(
        r#"SELECT r.tenant_id AS resource_tenant_id, g.tenant_id AS group_tenant_id
           FROM resources r
           CROSS JOIN groups g
           WHERE r.id = $1 AND g.id = $2"#,
    )
    .bind(resource_id)
    .bind(group_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_err)?
    .ok_or_else(|| AppError::bad_request("resource parent group reference is invalid"))?;
    let resource_tenant_id: Option<Uuid> = row.try_get("resource_tenant_id").map_err(db_err)?;
    let group_tenant_id: Option<Uuid> = row.try_get("group_tenant_id").map_err(db_err)?;
    let Some(tenant_id) = resource_tenant_id else {
        return Err(AppError::bad_request(
            "platform resource cannot be placed in a group",
        ));
    };
    if group_tenant_id != Some(tenant_id) {
        return Err(AppError::bad_request(
            "resource and parent group must belong to the same tenant",
        ));
    }
    sqlx::query(
        r#"INSERT INTO group_resource_parents (group_id, resource_id, tenant_id)
           VALUES ($1, $2, $3)
           ON CONFLICT (resource_id) DO UPDATE
           SET group_id = EXCLUDED.group_id,
               tenant_id = EXCLUDED.tenant_id,
               updated_at = now()"#,
    )
    .bind(group_id)
    .bind(resource_id)
    .bind(tenant_id)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

async fn clear_resource_parent_group_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    resource_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM group_resource_parents WHERE resource_id = $1")
        .bind(resource_id)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    Ok(())
}

fn parent_group_id_from_attrs(attrs: &Value) -> Result<Option<Uuid>, AppError> {
    attrs
        .get("parent_group_id")
        .map(parent_group_id_from_value)
        .transpose()
        .map(Option::flatten)
}

fn parent_group_id_from_value(value: &Value) -> Result<Option<Uuid>, AppError> {
    match value.as_str().map(str::trim) {
        Some("") | None => Ok(None),
        Some(raw) => raw
            .parse::<Uuid>()
            .map(Some)
            .map_err(|_| AppError::bad_request("parent_group_id must be a UUID")),
    }
}

// ─── Roles ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExpandedRoleGrant {
    pub root_role_id: Uuid,
    pub role_id: Uuid,
    pub role_name: String,
    pub role_path: String,
    pub scope_kind: ScopeKind,
    pub scope_ref: Option<String>,
    pub capability_id: Uuid,
}

pub async fn create_role(pool: &PgPool, req: CreateRole) -> Result<Role, AppError> {
    let id = Uuid::new_v4();
    let scope_kind = req.scope_kind.unwrap_or_else(|| {
        if req.tenant_id.is_some() {
            "tenant".to_string()
        } else {
            "platform".to_string()
        }
    });
    let scope_ref = req
        .scope_ref
        .or_else(|| req.tenant_id.map(|tenant_id| tenant_id.to_string()));
    let parsed_scope_kind = parse_scope_kind_text(&scope_kind)?;
    validate_role_scope(
        pool,
        req.tenant_id,
        &parsed_scope_kind,
        scope_ref.as_deref(),
    )
    .await?;
    sqlx::query_as::<_, Role>(
        r#"INSERT INTO roles (id, name, tenant_id, description, scope_kind, scope_ref)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, name, tenant_id, description, scope_kind, scope_ref, created_at, updated_at"#,
    )
    .bind(id)
    .bind(req.name)
    .bind(req.tenant_id)
    .bind(req.description)
    .bind(scope_kind)
    .bind(scope_ref)
    .fetch_one(pool)
    .await
    .map_err(db_err)
}

pub async fn create_role_with_assignments(
    pool: &PgPool,
    req: CreateRole,
    capability_ids: &[Uuid],
    child_role_ids: &[Uuid],
    member_entity_ids: &[Uuid],
) -> Result<Role, AppError> {
    if !capability_ids.is_empty() && !child_role_ids.is_empty() {
        return Err(AppError::bad_request(
            "role cannot have both capabilities and child roles",
        ));
    }

    let id = Uuid::new_v4();
    let scope_kind = req.scope_kind.unwrap_or_else(|| {
        if req.tenant_id.is_some() {
            "tenant".to_string()
        } else {
            "platform".to_string()
        }
    });
    let scope_ref = req
        .scope_ref
        .or_else(|| req.tenant_id.map(|tenant_id| tenant_id.to_string()));
    let parsed_scope_kind = parse_scope_kind_text(&scope_kind)?;
    validate_role_scope(
        pool,
        req.tenant_id,
        &parsed_scope_kind,
        scope_ref.as_deref(),
    )
    .await?;

    ensure_entities_exist(pool, member_entity_ids).await?;
    if child_role_ids.is_empty() {
        crate::guardrails::validate_role_assignment_plan(
            pool,
            member_entity_ids,
            capability_ids,
            req.tenant_id,
            parsed_scope_kind.clone(),
            scope_ref.as_deref(),
        )
        .await?;
    } else {
        validate_composite_children(pool, id, req.tenant_id, child_role_ids).await?;
        crate::guardrails::validate_composite_role_assignment_plan(
            pool,
            member_entity_ids,
            child_role_ids,
            req.tenant_id,
        )
        .await?;
    }

    let mut tx = pool.begin().await.map_err(db_err)?;
    let role = sqlx::query_as::<_, Role>(
        r#"INSERT INTO roles (id, name, tenant_id, description, scope_kind, scope_ref)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, name, tenant_id, description, scope_kind, scope_ref, created_at, updated_at"#,
    )
    .bind(id)
    .bind(req.name)
    .bind(req.tenant_id)
    .bind(req.description)
    .bind(&scope_kind)
    .bind(&scope_ref)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;

    for capability_id in capability_ids {
        sqlx::query(
            "INSERT INTO role_capabilities (role_id, capability_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(role.id)
        .bind(capability_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }

    for child_role_id in child_role_ids {
        sqlx::query(
            "INSERT INTO role_composites (parent_role_id, child_role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(role.id)
        .bind(child_role_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }

    for member_id in member_entity_ids {
        sqlx::query(
            r#"INSERT INTO policy_bindings
                 (tenant_id, subject_kind, subject_id, grant_kind, grant_id, scope_kind, scope_ref, effect, conditions)
               VALUES ($1, 'entity', $2, 'role', $3, $4, $5, 'allow', '{}'::jsonb)"#,
        )
        .bind(req.tenant_id)
        .bind(member_id)
        .bind(role.id)
        .bind(&parsed_scope_kind)
        .bind(&scope_ref)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

        if let Some(tenant_id) = req.tenant_id {
            sqlx::query(
                r#"INSERT INTO tenant_memberships (tenant_id, entity_id, status)
                   SELECT $1, $2, 'active'
                   WHERE EXISTS (
                       SELECT 1 FROM entities
                       WHERE id = $2 AND kind = 'human'
                   )
                   ON CONFLICT (tenant_id, entity_id)
                   DO UPDATE SET status = 'active'"#,
            )
            .bind(tenant_id)
            .bind(member_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
    }

    tx.commit().await.map_err(db_err)?;
    Ok(role)
}

pub async fn create_role_with_permission_blocks(
    pool: &PgPool,
    req: CreateRole,
    permission_blocks: &[CreateRolePermissionBlock],
    member_entity_ids: &[Uuid],
) -> Result<Role, AppError> {
    let id = Uuid::new_v4();
    if permission_blocks.is_empty() {
        return Err(AppError::bad_request("role permission blocks are required"));
    }
    validate_role_permission_blocks(pool, permission_blocks).await?;
    let scope_kind = req.scope_kind.unwrap_or_else(|| {
        if req.tenant_id.is_some() {
            "tenant".to_string()
        } else {
            "platform".to_string()
        }
    });
    let scope_ref = req
        .scope_ref
        .or_else(|| req.tenant_id.map(|tenant_id| tenant_id.to_string()));
    let parsed_scope_kind = parse_scope_kind_text(&scope_kind)?;
    validate_role_scope(
        pool,
        req.tenant_id,
        &parsed_scope_kind,
        scope_ref.as_deref(),
    )
    .await?;

    ensure_entities_exist(pool, member_entity_ids).await?;

    let mut tx = pool.begin().await.map_err(db_err)?;
    let role = sqlx::query_as::<_, Role>(
        r#"INSERT INTO roles (id, name, tenant_id, description, scope_kind, scope_ref)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, name, tenant_id, description, scope_kind, scope_ref, created_at, updated_at"#,
    )
    .bind(id)
    .bind(req.name)
    .bind(req.tenant_id)
    .bind(req.description)
    .bind(&scope_kind)
    .bind(&scope_ref)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;

    for block in permission_blocks {
        insert_role_permission_block(&mut tx, role.id, block).await?;
    }

    for member_id in member_entity_ids {
        sqlx::query(
            r#"INSERT INTO policy_bindings
                 (tenant_id, subject_kind, subject_id, grant_kind, grant_id, scope_kind, scope_ref, effect, conditions)
               VALUES ($1, 'entity', $2, 'role', $3, $4, $5, 'allow', '{}'::jsonb)"#,
        )
        .bind(req.tenant_id)
        .bind(member_id)
        .bind(role.id)
        .bind(&parsed_scope_kind)
        .bind(&scope_ref)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

        if let Some(tenant_id) = req.tenant_id {
            sqlx::query(
                r#"INSERT INTO tenant_memberships (tenant_id, entity_id, status)
                   SELECT $1, $2, 'active'
                   WHERE EXISTS (
                       SELECT 1 FROM entities
                       WHERE id = $2 AND kind = 'human'
                   )
                   ON CONFLICT (tenant_id, entity_id)
                   DO UPDATE SET status = 'active'"#,
            )
            .bind(tenant_id)
            .bind(member_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
    }

    tx.commit().await.map_err(db_err)?;
    Ok(role)
}

pub async fn replace_role_permission_blocks(
    pool: &PgPool,
    role_id: Uuid,
    permission_blocks: &[CreateRolePermissionBlock],
) -> Result<(), AppError> {
    if permission_blocks.is_empty() {
        return Err(AppError::bad_request("role permission blocks are required"));
    }
    validate_role_permission_blocks(pool, permission_blocks).await?;

    let mut tx = pool.begin().await.map_err(db_err)?;
    sqlx::query(
        r#"DELETE FROM role_permission_actions
           WHERE block_id IN (
             SELECT id FROM role_permission_blocks WHERE role_id = $1
           )"#,
    )
    .bind(role_id)
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    sqlx::query("DELETE FROM role_permission_blocks WHERE role_id = $1")
        .bind(role_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    for block in permission_blocks {
        insert_role_permission_block(&mut tx, role_id, block).await?;
    }
    tx.commit().await.map_err(db_err)?;
    Ok(())
}

async fn insert_role_permission_block(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    role_id: Uuid,
    block: &CreateRolePermissionBlock,
) -> Result<Uuid, AppError> {
    let block_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO role_permission_blocks
             (role_id, applies_to, object_id, object_kind, object_type, tenant_id, group_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id"#,
    )
    .bind(role_id)
    .bind(&block.applies_to)
    .bind(block.object_id)
    .bind(&block.object_kind)
    .bind(&block.object_type)
    .bind(block.tenant_id)
    .bind(block.group_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)?;

    for capability_id in &block.capability_ids {
        sqlx::query(
            r#"INSERT INTO role_permission_actions (block_id, capability_id)
               VALUES ($1, $2)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(block_id)
        .bind(capability_id)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    }

    Ok(block_id)
}

async fn validate_role_permission_blocks(
    pool: &PgPool,
    blocks: &[CreateRolePermissionBlock],
) -> Result<(), AppError> {
    for block in blocks {
        validate_permission_block_shape(block)?;
        let target = permission_block_target(block);
        for capability_id in &block.capability_ids {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM capabilities WHERE id = $1)")
                    .bind(capability_id)
                    .fetch_one(pool)
                    .await
                    .map_err(db_err)?;
            if !exists {
                return Err(AppError::bad_request(format!(
                    "capability {capability_id} does not exist"
                )));
            }

            if let Some((object_kind, object_type)) = &target {
                let applicable: bool = sqlx::query_scalar(
                    r#"SELECT EXISTS (
                         SELECT 1
                         FROM capability_applicability
                         WHERE capability_id = $1
                           AND object_kind = $2
                           AND (
                             $3::text IS NULL
                             OR object_type IS NULL
                             OR object_type = $3
                           )
                       )"#,
                )
                .bind(capability_id)
                .bind(object_kind)
                .bind(object_type)
                .fetch_one(pool)
                .await
                .map_err(db_err)?;
                if !applicable {
                    return Err(AppError::bad_request(format!(
                        "capability {capability_id} is not applicable to {object_kind}{}",
                        object_type
                            .as_deref()
                            .map(|value| format!(":{value}"))
                            .unwrap_or_default()
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_permission_block_shape(block: &CreateRolePermissionBlock) -> Result<(), AppError> {
    if block.capability_ids.is_empty() {
        return Err(AppError::bad_request(
            "permission block requires at least one capability",
        ));
    }
    match block.applies_to.as_str() {
        "platform" => Ok(()),
        "tenant" => block
            .tenant_id
            .map(|_| ())
            .ok_or_else(|| AppError::bad_request("tenant permission block requires tenantId")),
        "object" => block
            .object_id
            .map(|_| ())
            .ok_or_else(|| AppError::bad_request("object permission block requires objectId")),
        "object_kind" => block.object_kind.as_ref().map(|_| ()).ok_or_else(|| {
            AppError::bad_request("object_kind permission block requires objectKind")
        }),
        "object_type" => match (&block.object_kind, &block.object_type) {
            (Some(_), Some(_)) => Ok(()),
            _ => Err(AppError::bad_request(
                "object_type permission block requires objectKind and objectType",
            )),
        },
        "object_group_type" | "object_group_tree_type" => {
            match (block.group_id, &block.object_kind, &block.object_type) {
                (Some(_), Some(_), Some(_)) => Ok(()),
                _ => Err(AppError::bad_request(
                    "object group permission block requires groupId, objectKind, and objectType",
                )),
            }
        }
        "object_group_child_kind" | "object_group_descendant_kind" => {
            match (block.group_id, block.object_kind.as_deref()) {
                (Some(_), Some("group")) => Ok(()),
                _ => Err(AppError::bad_request(
                    "object group child permission block requires groupId and objectKind=group",
                )),
            }
        }
        other => Err(AppError::bad_request(format!(
            "unsupported permission block appliesTo '{other}'"
        ))),
    }
}

fn permission_block_target(block: &CreateRolePermissionBlock) -> Option<(String, Option<String>)> {
    match block.applies_to.as_str() {
        "tenant" => None,
        "object" | "object_kind" => block
            .object_kind
            .as_ref()
            .map(|object_kind| (object_kind.clone(), block.object_type.clone())),
        "object_type" | "object_group_type" | "object_group_tree_type" => block
            .object_kind
            .as_ref()
            .map(|object_kind| (object_kind.clone(), block.object_type.clone())),
        "object_group_child_kind" | "object_group_descendant_kind" => {
            Some(("group".to_string(), None))
        }
        "platform" => None,
        _ => None,
    }
}

pub async fn list_role_permission_blocks(
    pool: &PgPool,
    role_id: Uuid,
) -> Result<Vec<RolePermissionBlock>, AppError> {
    sqlx::query_as::<_, RolePermissionBlock>(
        r#"SELECT id, role_id, applies_to, object_id, object_kind, object_type, tenant_id, group_id, created_at, updated_at
           FROM role_permission_blocks
           WHERE role_id = $1
           ORDER BY created_at, id"#,
    )
    .bind(role_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

pub async fn role_permission_block_capabilities(
    pool: &PgPool,
    block_id: Uuid,
) -> Result<Vec<Capability>, AppError> {
    sqlx::query_as::<_, Capability>(
        r#"SELECT c.id, c.name, c.resource_kind, c.description, c.created_at, c.updated_at
           FROM capabilities c
           JOIN role_permission_actions rpa ON rpa.capability_id = c.id
           WHERE rpa.block_id = $1
           ORDER BY c.name"#,
    )
    .bind(block_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

pub async fn get_role(pool: &PgPool, id: Uuid) -> Result<Role, AppError> {
    sqlx::query_as::<_, Role>(
        "SELECT id, name, tenant_id, description, scope_kind, scope_ref, created_at, updated_at FROM roles WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("role {id} not found")),
        other => AppError::Database(other),
    })
}

pub async fn list_roles(pool: &PgPool, params: ListRoles) -> Result<RoleList, AppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);
    let q = search_pattern(params.q);
    let derived_kind = params
        .derived_kind
        .as_deref()
        .map(str::trim)
        .filter(|kind| !kind.is_empty())
        .map(str::to_ascii_lowercase);

    if let Some(kind) = derived_kind.as_deref() {
        match kind {
            "simple" | "composite" | "empty" => {}
            _ => {
                return Err(AppError::bad_request(
                    "derivedKind must be simple, composite, or empty",
                ));
            }
        }
    }

    let items = sqlx::query_as::<_, Role>(
        r#"SELECT id, name, tenant_id, description, scope_kind, scope_ref, created_at, updated_at FROM roles
           WHERE ($1::uuid IS NULL OR tenant_id = $1)
             AND ($2::text IS NULL OR scope_kind = $2)
             AND ($3::text IS NULL OR scope_ref = $3)
             AND ($4::text IS NULL OR name ILIKE $4 OR description ILIKE $4)
             AND (
               $5::text IS NULL
               OR ($5 = 'simple' AND NOT EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = roles.id
                  ))
               OR ($5 = 'composite' AND EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = roles.id
                  ))
               OR ($5 = 'empty' AND NOT EXISTS (
                    SELECT 1 FROM role_capabilities WHERE role_id = roles.id
                  ) AND NOT EXISTS (
                    SELECT 1 FROM role_permission_blocks WHERE role_id = roles.id
                  ) AND NOT EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = roles.id
                  ))
             )
           ORDER BY name LIMIT $6 OFFSET $7"#,
    )
    .bind(params.tenant_id)
    .bind(params.scope_kind.clone())
    .bind(params.scope_ref.clone())
    .bind(q.clone())
    .bind(derived_kind.clone())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM roles
           WHERE ($1::uuid IS NULL OR tenant_id = $1)
             AND ($2::text IS NULL OR scope_kind = $2)
             AND ($3::text IS NULL OR scope_ref = $3)
             AND ($4::text IS NULL OR name ILIKE $4 OR description ILIKE $4)
             AND (
               $5::text IS NULL
               OR ($5 = 'simple' AND NOT EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = roles.id
                  ))
               OR ($5 = 'composite' AND EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = roles.id
                  ))
               OR ($5 = 'empty' AND NOT EXISTS (
                    SELECT 1 FROM role_capabilities WHERE role_id = roles.id
                  ) AND NOT EXISTS (
                    SELECT 1 FROM role_permission_blocks WHERE role_id = roles.id
                  ) AND NOT EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = roles.id
                  ))
             )"#,
    )
    .bind(params.tenant_id)
    .bind(params.scope_kind)
    .bind(params.scope_ref)
    .bind(q)
    .bind(derived_kind)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    Ok(RoleList { items, total })
}

pub async fn role_derived_kind(pool: &PgPool, role_id: Uuid) -> Result<RoleDerivedKind, AppError> {
    use sqlx::Row;
    let row = sqlx::query(
        r#"SELECT
              EXISTS (SELECT 1 FROM role_capabilities WHERE role_id = $1) AS has_capabilities,
              EXISTS (SELECT 1 FROM role_permission_blocks WHERE role_id = $1) AS has_permission_blocks,
              EXISTS (SELECT 1 FROM role_composites WHERE parent_role_id = $1) AS has_children"#,
    )
    .bind(role_id)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;
    let has_capabilities: bool = row.try_get("has_capabilities").map_err(db_err)?;
    let has_permission_blocks: bool = row.try_get("has_permission_blocks").map_err(db_err)?;
    let has_children: bool = row.try_get("has_children").map_err(db_err)?;
    let has_simple_permissions = has_capabilities || has_permission_blocks;
    Ok(match (has_simple_permissions, has_children) {
        (true, false) => RoleDerivedKind::Simple,
        (false, true) => RoleDerivedKind::Composite,
        (false, false) => RoleDerivedKind::Empty,
        (true, true) => {
            return Err(AppError::bad_request(
                "role cannot have both permissions and child roles",
            ))
        }
    })
}

pub async fn child_roles(pool: &PgPool, role_id: Uuid) -> Result<Vec<Role>, AppError> {
    sqlx::query_as::<_, Role>(
        r#"SELECT r.id, r.name, r.tenant_id, r.description, r.scope_kind, r.scope_ref, r.created_at, r.updated_at
           FROM roles r
           JOIN role_composites rc ON rc.child_role_id = r.id
           WHERE rc.parent_role_id = $1
           ORDER BY r.name"#,
    )
    .bind(role_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

pub async fn parent_roles(pool: &PgPool, role_id: Uuid) -> Result<Vec<Role>, AppError> {
    sqlx::query_as::<_, Role>(
        r#"SELECT r.id, r.name, r.tenant_id, r.description, r.scope_kind, r.scope_ref, r.created_at, r.updated_at
           FROM roles r
           JOIN role_composites rc ON rc.parent_role_id = r.id
           WHERE rc.child_role_id = $1
           ORDER BY r.name"#,
    )
    .bind(role_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

async fn ensure_entities_exist(pool: &PgPool, entity_ids: &[Uuid]) -> Result<(), AppError> {
    if entity_ids.is_empty() {
        return Ok(());
    }
    let mut unique_entity_ids = entity_ids.to_vec();
    unique_entity_ids.sort_unstable();
    unique_entity_ids.dedup();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entities WHERE id = ANY($1::uuid[])")
        .bind(&unique_entity_ids)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
    if count != unique_entity_ids.len() as i64 {
        return Err(AppError::bad_request("invalid member reference"));
    }
    Ok(())
}

async fn validate_composite_children(
    pool: &PgPool,
    parent_role_id: Uuid,
    parent_tenant_id: Option<Uuid>,
    child_role_ids: &[Uuid],
) -> Result<(), AppError> {
    if child_role_ids.is_empty() {
        return Ok(());
    }
    let mut unique_child_ids = child_role_ids.to_vec();
    unique_child_ids.sort_unstable();
    unique_child_ids.dedup();
    if unique_child_ids.contains(&parent_role_id) {
        return Err(AppError::bad_request("role cannot include itself"));
    }

    use sqlx::Row;
    let rows = sqlx::query(
        r#"SELECT r.id, r.tenant_id,
                  EXISTS (SELECT 1 FROM role_capabilities rc WHERE rc.role_id = r.id) AS has_capabilities,
                  EXISTS (SELECT 1 FROM role_composites rc WHERE rc.parent_role_id = r.id) AS has_children
           FROM roles r
           WHERE r.id = ANY($1::uuid[])"#,
    )
    .bind(&unique_child_ids)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    if rows.len() != unique_child_ids.len() {
        return Err(AppError::bad_request("invalid child role reference"));
    }

    for row in rows {
        let child_id: Uuid = row.try_get("id").map_err(db_err)?;
        let tenant_id: Option<Uuid> = row.try_get("tenant_id").map_err(db_err)?;
        let has_capabilities: bool = row.try_get("has_capabilities").map_err(db_err)?;
        let has_children: bool = row.try_get("has_children").map_err(db_err)?;
        if tenant_id != parent_tenant_id {
            return Err(AppError::bad_request(
                "parent and child roles must belong to the same tenant",
            ));
        }
        if has_children {
            return Err(AppError::bad_request(
                "nested composite roles are not supported",
            ));
        }
        if !has_capabilities {
            return Err(AppError::bad_request(
                "composite child role must have capabilities",
            ));
        }
        if child_id == parent_role_id {
            return Err(AppError::bad_request("role cannot include itself"));
        }
    }

    Ok(())
}

fn parse_scope_kind_text(value: &str) -> Result<ScopeKind, AppError> {
    serde_json::from_value(serde_json::Value::String(value.to_string())).map_err(|_| {
        AppError::bad_request(format!(
            "invalid scope_kind '{value}' (expected one of platform, tenant, object_kind, object_type, object, group_object_type, group_tree_object_type, group_child_kind, group_descendant_kind)"
        ))
    })
}

async fn validate_role_scope(
    pool: &PgPool,
    tenant_id: Option<Uuid>,
    scope_kind: &ScopeKind,
    scope_ref: Option<&str>,
) -> Result<(), AppError> {
    match scope_kind {
        ScopeKind::Platform => Ok(()),
        ScopeKind::Tenant => {
            let Some(scope_ref) = scope_ref else {
                return Err(AppError::bad_request("tenant scope requires scope_ref"));
            };
            scope_ref
                .parse::<Uuid>()
                .map(|_| ())
                .map_err(|_| AppError::bad_request("tenant scope_ref must be a UUID"))
        }
        ScopeKind::ObjectKind => {
            if scope_ref.is_some() {
                Ok(())
            } else {
                Err(AppError::bad_request(
                    "object_kind scope requires scope_ref",
                ))
            }
        }
        ScopeKind::ObjectType => match scope_ref {
            Some(scope_ref) if scope_ref.split_once(':').is_some() => Ok(()),
            Some(_) => Err(AppError::bad_request(
                "object_type scope_ref must be namespaced as '<kind>:<sub-kind>'",
            )),
            None => Err(AppError::bad_request(
                "object_type scope requires scope_ref",
            )),
        },
        ScopeKind::Object => {
            let Some(scope_ref) = scope_ref else {
                return Err(AppError::bad_request("object scope requires scope_ref"));
            };
            scope_ref
                .parse::<Uuid>()
                .map(|_| ())
                .map_err(|_| AppError::bad_request("object scope_ref must be a UUID"))
        }
        ScopeKind::GroupObjectType
        | ScopeKind::GroupTreeObjectType
        | ScopeKind::GroupChildKind
        | ScopeKind::GroupDescendantKind => {
            let (group_id, rest) = parse_group_scope_ref(scope_ref)?;
            match scope_kind {
                ScopeKind::GroupObjectType | ScopeKind::GroupTreeObjectType => {
                    if rest.split_once(':').is_none() {
                        return Err(AppError::bad_request(
                            "group object scope_ref must include namespaced object type",
                        ));
                    }
                }
                ScopeKind::GroupChildKind | ScopeKind::GroupDescendantKind => {
                    if rest != "group" {
                        return Err(AppError::bad_request(
                            "group kind scope_ref must end with ':group'",
                        ));
                    }
                }
                ScopeKind::Platform
                | ScopeKind::Tenant
                | ScopeKind::ObjectKind
                | ScopeKind::ObjectType
                | ScopeKind::Object => {}
            }
            let group_tenant_id: Option<Uuid> =
                sqlx::query_scalar("SELECT tenant_id FROM groups WHERE id = $1")
                    .bind(group_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(db_err)?
                    .ok_or_else(|| AppError::bad_request("group scope references unknown group"))?;
            if tenant_id.is_some() && group_tenant_id != tenant_id {
                return Err(AppError::bad_request(
                    "group scope must reference a group in the role tenant",
                ));
            }
            Ok(())
        }
    }
}

fn parse_group_scope_ref(scope_ref: Option<&str>) -> Result<(Uuid, &str), AppError> {
    let scope_ref =
        scope_ref.ok_or_else(|| AppError::bad_request("group scope requires scope_ref"))?;
    let (group_id, rest) = scope_ref
        .split_once(':')
        .ok_or_else(|| AppError::bad_request("group scope_ref must start with group UUID"))?;
    let group_id = group_id
        .parse::<Uuid>()
        .map_err(|_| AppError::bad_request("group scope_ref has invalid group UUID"))?;
    Ok((group_id, rest))
}

pub async fn update_role(pool: &PgPool, id: Uuid, req: UpdateRole) -> Result<Role, AppError> {
    sqlx::query_as::<_, Role>(
        r#"UPDATE roles
           SET name        = COALESCE($2, name),
               description = COALESCE($3, description),
               updated_at  = now()
           WHERE id = $1
           RETURNING id, name, tenant_id, description, scope_kind, scope_ref, created_at, updated_at"#,
    )
    .bind(id)
    .bind(req.name)
    .bind(req.description)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("role {id} not found")),
        other => AppError::Database(other),
    })
}

pub async fn delete_role(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM roles WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(db_err)?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!("role {id} not found")));
    }
    Ok(())
}

pub async fn add_role_capability(
    pool: &PgPool,
    role_id: Uuid,
    cap_id: Uuid,
) -> Result<(), AppError> {
    let has_children: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM role_composites WHERE parent_role_id = $1)",
    )
    .bind(role_id)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;
    if has_children {
        return Err(AppError::bad_request(
            "composite role cannot have direct capabilities",
        ));
    }
    crate::guardrails::validate_role_capability(pool, role_id, cap_id).await?;
    sqlx::query(
        "INSERT INTO role_capabilities (role_id, capability_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(role_id)
    .bind(cap_id)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

pub async fn add_composite_role_child(
    pool: &PgPool,
    parent_role_id: Uuid,
    child_role_id: Uuid,
) -> Result<(), AppError> {
    let parent = get_role(pool, parent_role_id).await?;
    let has_capabilities: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM role_capabilities WHERE role_id = $1)")
            .bind(parent_role_id)
            .fetch_one(pool)
            .await
            .map_err(db_err)?;
    if has_capabilities {
        return Err(AppError::bad_request("simple role cannot have child roles"));
    }
    validate_composite_children(pool, parent_role_id, parent.tenant_id, &[child_role_id]).await?;
    sqlx::query(
        "INSERT INTO role_composites (parent_role_id, child_role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(parent_role_id)
    .bind(child_role_id)
    .execute(pool)
    .await
    .map_err(db_err)?;
    Ok(())
}

pub async fn remove_composite_role_child(
    pool: &PgPool,
    parent_role_id: Uuid,
    child_role_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM role_composites WHERE parent_role_id = $1 AND child_role_id = $2")
        .bind(parent_role_id)
        .bind(child_role_id)
        .execute(pool)
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn replace_composite_role_children(
    pool: &PgPool,
    parent_role_id: Uuid,
    child_role_ids: &[Uuid],
) -> Result<(), AppError> {
    let parent = get_role(pool, parent_role_id).await?;
    let has_capabilities: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM role_capabilities WHERE role_id = $1)")
            .bind(parent_role_id)
            .fetch_one(pool)
            .await
            .map_err(db_err)?;
    if has_capabilities && !child_role_ids.is_empty() {
        return Err(AppError::bad_request("simple role cannot have child roles"));
    }
    validate_composite_children(pool, parent_role_id, parent.tenant_id, child_role_ids).await?;
    let mut tx = pool.begin().await.map_err(db_err)?;
    sqlx::query("DELETE FROM role_composites WHERE parent_role_id = $1")
        .bind(parent_role_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    for child_role_id in child_role_ids {
        sqlx::query(
            "INSERT INTO role_composites (parent_role_id, child_role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(parent_role_id)
        .bind(child_role_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }
    tx.commit().await.map_err(db_err)?;
    Ok(())
}

pub async fn remove_role_capability(
    pool: &PgPool,
    role_id: Uuid,
    cap_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM role_capabilities WHERE role_id = $1 AND capability_id = $2")
        .bind(role_id)
        .bind(cap_id)
        .execute(pool)
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn get_role_capabilities(
    pool: &PgPool,
    role_id: Uuid,
) -> Result<Vec<Capability>, AppError> {
    sqlx::query_as::<_, Capability>(
        r#"SELECT c.id, c.name, c.resource_kind, c.description, c.created_at, c.updated_at
           FROM capabilities c
           JOIN role_capabilities rc ON rc.capability_id = c.id
           WHERE rc.role_id = $1"#,
    )
    .bind(role_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

// ─── Capabilities ─────────────────────────────────────────────────────────────

pub async fn create_capability(
    pool: &PgPool,
    req: CreateCapability,
) -> Result<Capability, AppError> {
    let id = Uuid::new_v4();
    sqlx::query_as::<_, Capability>(
        r#"INSERT INTO capabilities (id, name, resource_kind, description)
           VALUES ($1, $2, $3, $4)
           RETURNING id, name, resource_kind, description, created_at, updated_at"#,
    )
    .bind(id)
    .bind(req.name)
    .bind(req.resource_kind)
    .bind(req.description)
    .fetch_one(pool)
    .await
    .map_err(db_err)
}

pub async fn get_capability(pool: &PgPool, id: Uuid) -> Result<Capability, AppError> {
    sqlx::query_as::<_, Capability>(
        "SELECT id, name, resource_kind, description, created_at, updated_at FROM capabilities WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("capability {id} not found")),
        other => AppError::Database(other),
    })
}

pub async fn list_capabilities(
    pool: &PgPool,
    params: ListCapabilities,
) -> Result<crate::models::capability::CapabilityList, AppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let items = sqlx::query_as::<_, Capability>(
        r#"SELECT id, name, resource_kind, description, created_at, updated_at FROM capabilities
           WHERE ($1::text IS NULL OR resource_kind = $1)
           ORDER BY name LIMIT $2 OFFSET $3"#,
    )
    .bind(&params.resource_kind)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM capabilities WHERE ($1::text IS NULL OR resource_kind = $1)",
    )
    .bind(&params.resource_kind)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    Ok(crate::models::capability::CapabilityList { items, total })
}

pub async fn update_capability(
    pool: &PgPool,
    id: Uuid,
    req: crate::models::capability::UpdateCapability,
) -> Result<Capability, AppError> {
    sqlx::query_as::<_, Capability>(
        r#"UPDATE capabilities
           SET name          = COALESCE($2, name),
               resource_kind = COALESCE($3, resource_kind),
               description   = COALESCE($4, description),
               updated_at    = now()
           WHERE id = $1
           RETURNING id, name, resource_kind, description, created_at, updated_at"#,
    )
    .bind(id)
    .bind(req.name)
    .bind(req.resource_kind)
    .bind(req.description)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("capability {id} not found")),
        other => AppError::Database(other),
    })
}

pub async fn delete_capability(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM capabilities WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(db_err)?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!("capability {id} not found")));
    }
    Ok(())
}

// ─── Policy Bindings ──────────────────────────────────────────────────────────

pub async fn create_policy(
    pool: &PgPool,
    req: CreatePolicyBinding,
) -> Result<PolicyBinding, AppError> {
    crate::guardrails::validate_policy(pool, &req).await?;
    let id = Uuid::new_v4();
    let membership_tenant_id = req.tenant_id;
    let membership_entity_id = req.subject_id;
    let should_sync_membership = req.tenant_id.is_some()
        && req.subject_kind == SubjectKind::Entity
        && req.effect == Effect::Allow;
    let conditions = if req.conditions.is_null() {
        serde_json::json!({})
    } else {
        req.conditions
    };
    let policy = sqlx::query_as::<_, PolicyBinding>(
        r#"INSERT INTO policy_bindings
             (id, tenant_id, subject_kind, subject_id, grant_kind, grant_id, scope_kind, scope_ref, effect, conditions)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, tenant_id, subject_kind, subject_id, grant_kind, grant_id, scope_kind, scope_ref, effect, conditions, created_at"#,
    )
    .bind(id)
    .bind(req.tenant_id)
    .bind(req.subject_kind)
    .bind(req.subject_id)
    .bind(req.grant_kind)
    .bind(req.grant_id)
    .bind(req.scope_kind)
    .bind(req.scope_ref)
    .bind(req.effect)
    .bind(conditions)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    if should_sync_membership {
        if let Some(tenant_id) = membership_tenant_id {
            sync_tenant_membership_for_policy(pool, tenant_id, membership_entity_id).await?;
        }
    }

    Ok(policy)
}

async fn sync_tenant_membership_for_policy(
    pool: &PgPool,
    tenant_id: Uuid,
    entity_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"INSERT INTO tenant_memberships (tenant_id, entity_id, status)
           SELECT $1, $2, 'active'
           WHERE EXISTS (
               SELECT 1 FROM entities
               WHERE id = $2 AND kind = 'human'
           )
           ON CONFLICT (tenant_id, entity_id)
           DO UPDATE SET status = 'active'"#,
    )
    .bind(tenant_id)
    .bind(entity_id)
    .execute(pool)
    .await
    .map_err(db_err)?;

    Ok(())
}

pub async fn get_policy(pool: &PgPool, id: Uuid) -> Result<PolicyBinding, AppError> {
    sqlx::query_as::<_, PolicyBinding>(
        r#"SELECT id, tenant_id, subject_kind, subject_id, grant_kind, grant_id, scope_kind, scope_ref, effect, conditions, created_at
           FROM policy_bindings WHERE id = $1"#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("policy {id} not found")),
        other => AppError::Database(other),
    })
}

pub async fn list_policies(pool: &PgPool, params: ListPolicies) -> Result<PolicyList, AppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);
    let tenant_id = params.tenant_id;
    let subject_id = params.subject_id;
    let subject_kind = params.subject_kind;

    let items = sqlx::query_as::<_, PolicyBinding>(
        r#"SELECT id, tenant_id, subject_kind, subject_id, grant_kind, grant_id, scope_kind, scope_ref, effect, conditions, created_at
           FROM policy_bindings
           WHERE ($1::uuid IS NULL OR tenant_id = $1)
             AND ($2::uuid IS NULL OR subject_id = $2)
             AND ($3::text IS NULL OR subject_kind = $3)
           ORDER BY created_at DESC
           LIMIT $4 OFFSET $5"#,
    )
    .bind(tenant_id)
    .bind(subject_id)
    .bind(subject_kind.clone())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM policy_bindings
           WHERE ($1::uuid IS NULL OR tenant_id = $1)
             AND ($2::uuid IS NULL OR subject_id = $2)
             AND ($3::text IS NULL OR subject_kind = $3)"#,
    )
    .bind(tenant_id)
    .bind(subject_id)
    .bind(subject_kind)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    Ok(PolicyList { items, total })
}

pub async fn role_policies(
    pool: &PgPool,
    role_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<PolicyList, AppError> {
    let limit = limit.clamp(1, 100);
    let offset = offset.max(0);

    let items = sqlx::query_as::<_, PolicyBinding>(
        r#"SELECT id, tenant_id, subject_kind, subject_id, grant_kind, grant_id,
                  scope_kind, scope_ref, effect, conditions, created_at
           FROM policy_bindings
           WHERE grant_kind = 'role' AND grant_id = $1
           ORDER BY created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(role_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM policy_bindings
           WHERE grant_kind = 'role' AND grant_id = $1"#,
    )
    .bind(role_id)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    Ok(PolicyList { items, total })
}

pub async fn subject_role_assignments(
    pool: &PgPool,
    params: SubjectRoleAssignmentsQuery,
) -> Result<SubjectRoleAssignmentList, AppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);
    let q = search_pattern(params.q);
    let derived_kind = params
        .derived_kind
        .as_deref()
        .map(str::trim)
        .filter(|kind| !kind.is_empty())
        .map(str::to_ascii_lowercase);

    if let Some(kind) = derived_kind.as_deref() {
        match kind {
            "simple" | "composite" | "empty" => {}
            _ => {
                return Err(AppError::bad_request(
                    "derivedKind must be simple, composite, or empty",
                ));
            }
        }
    }

    use sqlx::Row;
    let rows = sqlx::query(
        r#"SELECT
             pb.id AS policy_id,
             pb.tenant_id AS policy_tenant_id,
             pb.subject_kind,
             pb.subject_id,
             pb.grant_kind,
             pb.grant_id,
             pb.scope_kind AS policy_scope_kind,
             pb.scope_ref AS policy_scope_ref,
             pb.effect,
             pb.conditions,
             pb.created_at AS policy_created_at,
             r.id AS role_id,
             r.name AS role_name,
             r.tenant_id AS role_tenant_id,
             r.description AS role_description,
             r.scope_kind AS role_scope_kind,
             r.scope_ref AS role_scope_ref,
             r.created_at AS role_created_at,
             r.updated_at AS role_updated_at
           FROM policy_bindings pb
           JOIN roles r ON pb.grant_kind = 'role' AND pb.grant_id = r.id
           WHERE ($1::uuid IS NULL OR pb.tenant_id = $1)
             AND pb.subject_kind = $2
             AND pb.subject_id = $3
             AND ($4::text IS NULL OR r.name ILIKE $4 OR r.description ILIKE $4)
             AND (
               $5::text IS NULL
               OR ($5 = 'simple' AND NOT EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = r.id
                  ))
               OR ($5 = 'composite' AND EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = r.id
                  ))
               OR ($5 = 'empty' AND NOT EXISTS (
                    SELECT 1 FROM role_capabilities WHERE role_id = r.id
                  ) AND NOT EXISTS (
                    SELECT 1 FROM role_permission_blocks WHERE role_id = r.id
                  ) AND NOT EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = r.id
                  ))
             )
           ORDER BY pb.created_at DESC
           LIMIT $6 OFFSET $7"#,
    )
    .bind(params.tenant_id)
    .bind(params.subject_kind.clone())
    .bind(params.subject_id)
    .bind(q.clone())
    .bind(derived_kind.clone())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let items = rows
        .into_iter()
        .map(|row| {
            Ok(SubjectRoleAssignment {
                policy: PolicyBinding {
                    id: row.try_get("policy_id").map_err(db_err)?,
                    tenant_id: row.try_get("policy_tenant_id").map_err(db_err)?,
                    subject_kind: row.try_get("subject_kind").map_err(db_err)?,
                    subject_id: row.try_get("subject_id").map_err(db_err)?,
                    grant_kind: row.try_get("grant_kind").map_err(db_err)?,
                    grant_id: row.try_get("grant_id").map_err(db_err)?,
                    scope_kind: row.try_get("policy_scope_kind").map_err(db_err)?,
                    scope_ref: row.try_get("policy_scope_ref").map_err(db_err)?,
                    effect: row.try_get("effect").map_err(db_err)?,
                    conditions: row.try_get("conditions").map_err(db_err)?,
                    created_at: row.try_get("policy_created_at").map_err(db_err)?,
                },
                role: Role {
                    id: row.try_get("role_id").map_err(db_err)?,
                    name: row.try_get("role_name").map_err(db_err)?,
                    tenant_id: row.try_get("role_tenant_id").map_err(db_err)?,
                    description: row.try_get("role_description").map_err(db_err)?,
                    scope_kind: row.try_get("role_scope_kind").map_err(db_err)?,
                    scope_ref: row.try_get("role_scope_ref").map_err(db_err)?,
                    created_at: row.try_get("role_created_at").map_err(db_err)?,
                    updated_at: row.try_get("role_updated_at").map_err(db_err)?,
                },
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM policy_bindings pb
           JOIN roles r ON pb.grant_kind = 'role' AND pb.grant_id = r.id
           WHERE ($1::uuid IS NULL OR pb.tenant_id = $1)
             AND pb.subject_kind = $2
             AND pb.subject_id = $3
             AND ($4::text IS NULL OR r.name ILIKE $4 OR r.description ILIKE $4)
             AND (
               $5::text IS NULL
               OR ($5 = 'simple' AND NOT EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = r.id
                  ))
               OR ($5 = 'composite' AND EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = r.id
                  ))
               OR ($5 = 'empty' AND NOT EXISTS (
                    SELECT 1 FROM role_capabilities WHERE role_id = r.id
                  ) AND NOT EXISTS (
                    SELECT 1 FROM role_permission_blocks WHERE role_id = r.id
                  ) AND NOT EXISTS (
                    SELECT 1 FROM role_composites WHERE parent_role_id = r.id
                  ))
             )"#,
    )
    .bind(params.tenant_id)
    .bind(params.subject_kind)
    .bind(params.subject_id)
    .bind(q)
    .bind(derived_kind)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    Ok(SubjectRoleAssignmentList { items, total })
}

pub async fn delete_policy(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM policy_bindings WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(db_err)?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!("policy {id} not found")));
    }
    Ok(())
}

/// Best-effort ownership lookup for exact-object policy scopes. `None` means
/// no object with that UUID exists in the known Atom object tables; `Some(None)`
/// means the object is platform/global.
pub async fn object_tenant_id_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<Option<Uuid>>, AppError> {
    let row = sqlx::query(
        r#"SELECT tenant_id FROM (
             SELECT tenant_id FROM entities WHERE id = $1
             UNION ALL
             SELECT tenant_id FROM groups WHERE id = $1
             UNION ALL
             SELECT tenant_id FROM resources WHERE id = $1
             UNION ALL
             SELECT tenant_id FROM roles WHERE id = $1
             UNION ALL
             SELECT tenant_id FROM policy_bindings WHERE id = $1
             UNION ALL
             SELECT t.id AS tenant_id FROM tenants t WHERE t.id = $1
             UNION ALL
             SELECT e.tenant_id FROM credentials c JOIN entities e ON e.id = c.entity_id WHERE c.id = $1
             UNION ALL
             SELECT tenant_id FROM audit_logs WHERE id = $1
           ) matches
           LIMIT 1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(db_err)?;

    row.map(|r| {
        use sqlx::Row;
        r.try_get::<Option<Uuid>, _>("tenant_id").map_err(db_err)
    })
    .transpose()
}

// ─── Query Views ──────────────────────────────────────────────────────────────

pub async fn entity_access(
    pool: &PgPool,
    entity_id: Uuid,
    params: AccessQuery,
) -> Result<EntityAccessResponse, AppError> {
    let entity = sqlx::query_as::<_, Entity>(
        r#"SELECT id, kind, name, tenant_id, profile_id, profile_version_id,
                  status, attributes, created_at, updated_at
           FROM entities
           WHERE id = $1"#,
    )
    .bind(entity_id)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("entity {entity_id} not found")),
        other => AppError::Database(other),
    })?;

    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);
    let action_cap = optional_capability_id(pool, params.action.as_deref()).await?;
    if params.action.is_some() && action_cap.is_none() {
        return Ok(EntityAccessResponse {
            entity_id: entity.id,
            entity_name: entity.name,
            entity_kind: entity.kind,
            items: Vec::new(),
            total: 0,
        });
    }

    let items = sqlx::query(
        r#"WITH bindings AS (
             SELECT pb.*, 'direct'::text AS via
             FROM policy_bindings pb
             WHERE pb.subject_kind = 'entity' AND pb.subject_id = $1
             UNION ALL
             SELECT pb.*, ('group:' || g.name)::text AS via
             FROM policy_bindings pb
             JOIN group_members gm ON gm.group_id = pb.subject_id
             JOIN groups g ON g.id = gm.group_id AND g.status = 'active'
             WHERE pb.subject_kind = 'group' AND gm.entity_id = $1
           ), expanded AS (
             SELECT b.id AS policy_id, b.grant_kind, b.grant_id, b.scope_kind, b.scope_ref,
                    b.effect, b.conditions, b.via, r.id AS resource_id, r.kind AS resource_kind,
                    r.name AS resource_name, r.tenant_id AS resource_tenant_id,
                    role.id AS role_id, role.name AS role_name
             FROM bindings b
             JOIN resources r ON
               b.scope_kind = 'platform'
               OR (b.scope_kind = 'object_kind' AND b.scope_ref = 'resource')
               OR (b.scope_kind = 'object_type' AND b.scope_ref = 'resource:' || r.kind)
               OR (b.scope_kind = 'object' AND b.scope_ref = r.id::text)
             LEFT JOIN roles role ON b.grant_kind = 'role' AND role.id = b.grant_id
             WHERE ($2::uuid IS NULL OR r.tenant_id = $2)
               AND ($3::text IS NULL OR r.kind = $3)
               AND ($4::text IS NULL OR b.effect = $4)
               AND (
                 $5::uuid IS NULL
                 OR (b.grant_kind = 'capability' AND b.grant_id = $5)
                 OR (b.grant_kind = 'role' AND EXISTS (
                   SELECT 1 FROM role_capabilities rc
                   WHERE rc.role_id = b.grant_id AND rc.capability_id = $5
                 ))
               )
           )
           SELECT e.*,
                  COALESCE(jsonb_agg(jsonb_build_object(
                    'id', c.id,
                    'name', c.name,
                    'resource_kind', c.resource_kind
                  ) ORDER BY c.name) FILTER (WHERE c.id IS NOT NULL), '[]'::jsonb) AS capabilities
           FROM expanded e
           LEFT JOIN role_capabilities rc ON e.grant_kind = 'role' AND rc.role_id = e.grant_id
           LEFT JOIN capabilities c ON
             (e.grant_kind = 'capability' AND c.id = e.grant_id)
             OR (e.grant_kind = 'role' AND c.id = rc.capability_id)
           GROUP BY e.policy_id, e.grant_kind, e.grant_id, e.scope_kind, e.scope_ref,
                    e.effect, e.conditions, e.via, e.resource_id, e.resource_kind,
                    e.resource_name, e.resource_tenant_id, e.role_id, e.role_name
           ORDER BY e.resource_kind, e.resource_name NULLS LAST, e.policy_id
           LIMIT $6 OFFSET $7"#,
    )
    .bind(entity_id)
    .bind(params.tenant_id)
    .bind(params.resource_kind.clone())
    .bind(params.effect.clone())
    .bind(action_cap)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total: i64 = sqlx::query_scalar(
        r#"WITH bindings AS (
             SELECT pb.* FROM policy_bindings pb
             WHERE pb.subject_kind = 'entity' AND pb.subject_id = $1
             UNION ALL
             SELECT pb.* FROM policy_bindings pb
             JOIN group_members gm ON gm.group_id = pb.subject_id
             WHERE pb.subject_kind = 'group' AND gm.entity_id = $1
           )
           SELECT COUNT(*)
           FROM bindings b
           JOIN resources r ON
             b.scope_kind = 'platform'
             OR (b.scope_kind = 'object_kind' AND b.scope_ref = 'resource')
             OR (b.scope_kind = 'object_type' AND b.scope_ref = 'resource:' || r.kind)
             OR (b.scope_kind = 'object' AND b.scope_ref = r.id::text)
           WHERE ($2::uuid IS NULL OR r.tenant_id = $2)
             AND ($3::text IS NULL OR r.kind = $3)
             AND ($4::text IS NULL OR b.effect = $4)
             AND (
               $5::uuid IS NULL
               OR (b.grant_kind = 'capability' AND b.grant_id = $5)
               OR (b.grant_kind = 'role' AND EXISTS (
                 SELECT 1 FROM role_capabilities rc
                 WHERE rc.role_id = b.grant_id AND rc.capability_id = $5
               ))
             )"#,
    )
    .bind(entity_id)
    .bind(params.tenant_id)
    .bind(params.resource_kind)
    .bind(params.effect)
    .bind(action_cap)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    Ok(EntityAccessResponse {
        entity_id: entity.id,
        entity_name: entity.name,
        entity_kind: entity.kind,
        items: rows_to_access_items(items)?,
        total,
    })
}

pub async fn resource_access(
    pool: &PgPool,
    resource_id: Uuid,
    params: ResourceAccessQuery,
) -> Result<ResourceAccessResponse, AppError> {
    use sqlx::Row;

    let resource = get_resource(pool, resource_id).await?;
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);
    let action_cap = optional_capability_id(pool, params.action.as_deref()).await?;
    let resource_id_str = resource_id.to_string();
    if params.action.is_some() && action_cap.is_none() {
        return Ok(ResourceAccessResponse {
            resource_id,
            resource: ResourceSummary {
                id: resource.id,
                kind: resource.kind,
                name: resource.name,
                tenant_id: resource.tenant_id,
            },
            items: Vec::new(),
            total: 0,
        });
    }

    let base_sql = r#"WITH covered AS (
             SELECT pb.*, e.id AS entity_id, e.name AS entity_name, e.kind AS entity_kind,
                    e.tenant_id AS entity_tenant_id, 'direct'::text AS via
             FROM policy_bindings pb
             JOIN entities e ON pb.subject_kind = 'entity' AND e.id = pb.subject_id
             WHERE pb.scope_kind = 'platform'
                OR (pb.scope_kind = 'object_kind' AND pb.scope_ref = 'resource')
                OR (pb.scope_kind = 'object_type' AND pb.scope_ref = 'resource:' || $1)
                OR (pb.scope_kind = 'object' AND pb.scope_ref = $2)
             UNION ALL
             SELECT pb.*, e.id AS entity_id, e.name AS entity_name, e.kind AS entity_kind,
                    e.tenant_id AS entity_tenant_id, ('group:' || g.name)::text AS via
             FROM policy_bindings pb
             JOIN groups g ON pb.subject_kind = 'group' AND g.id = pb.subject_id
             JOIN group_members gm ON gm.group_id = g.id
             JOIN entities e ON e.id = gm.entity_id
             WHERE pb.scope_kind = 'platform'
                OR (pb.scope_kind = 'object_kind' AND pb.scope_ref = 'resource')
                OR (pb.scope_kind = 'object_type' AND pb.scope_ref = 'resource:' || $1)
                OR (pb.scope_kind = 'object' AND pb.scope_ref = $2)
           ), filtered AS (
             SELECT c.*, role.id AS role_id, role.name AS role_name
             FROM covered c
             LEFT JOIN roles role ON c.grant_kind = 'role' AND role.id = c.grant_id
             WHERE ($3::text IS NULL OR c.entity_kind = $3)
               AND ($4::text IS NULL OR c.effect = $4)
               AND (
                 $5::uuid IS NULL
                 OR (c.grant_kind = 'capability' AND c.grant_id = $5)
                 OR (c.grant_kind = 'role' AND EXISTS (
                   SELECT 1 FROM role_capabilities rc
                   WHERE rc.role_id = c.grant_id AND rc.capability_id = $5
                 ))
               )
           )"#;

    let rows = sqlx::query(&format!(
        r#"{base_sql}
           SELECT f.id AS policy_id, f.grant_kind, f.grant_id, f.scope_kind, f.scope_ref,
                  f.effect, f.conditions, f.via, f.entity_id, f.entity_name, f.entity_kind,
                  f.entity_tenant_id, f.role_id, f.role_name,
                  COALESCE(jsonb_agg(jsonb_build_object(
                    'id', cap.id,
                    'name', cap.name,
                    'resource_kind', cap.resource_kind
                  ) ORDER BY cap.name) FILTER (WHERE cap.id IS NOT NULL), '[]'::jsonb) AS capabilities
           FROM filtered f
           LEFT JOIN role_capabilities rc ON f.grant_kind = 'role' AND rc.role_id = f.grant_id
           LEFT JOIN capabilities cap ON
             (f.grant_kind = 'capability' AND cap.id = f.grant_id)
             OR (f.grant_kind = 'role' AND cap.id = rc.capability_id)
           GROUP BY f.id, f.grant_kind, f.grant_id, f.scope_kind, f.scope_ref, f.effect,
                    f.conditions, f.via, f.entity_id, f.entity_name, f.entity_kind,
                    f.entity_tenant_id, f.role_id, f.role_name
           ORDER BY f.entity_name, f.id
           LIMIT $6 OFFSET $7"#
    ))
    .bind(&resource.kind)
    .bind(&resource_id_str)
    .bind(params.entity_kind.clone())
    .bind(params.effect.clone())
    .bind(action_cap)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total: i64 = sqlx::query_scalar(&format!("{base_sql} SELECT COUNT(*) FROM filtered"))
        .bind(&resource.kind)
        .bind(&resource_id_str)
        .bind(params.entity_kind)
        .bind(params.effect)
        .bind(action_cap)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;

    let items = rows
        .into_iter()
        .map(|row| {
            let caps: Value = row.try_get("capabilities").map_err(db_err)?;
            Ok(ResourceAccessItem {
                entity: ResourceAccessEntity {
                    id: row.try_get("entity_id").map_err(db_err)?,
                    name: row.try_get("entity_name").map_err(db_err)?,
                    kind: row.try_get("entity_kind").map_err(db_err)?,
                    tenant_id: row.try_get("entity_tenant_id").map_err(db_err)?,
                },
                effect: row.try_get("effect").map_err(db_err)?,
                scope_kind: row.try_get("scope_kind").map_err(db_err)?,
                scope_ref: row.try_get("scope_ref").map_err(db_err)?,
                policy_id: row.try_get("policy_id").map_err(db_err)?,
                grant: grant_from_row(&row, caps)?,
                conditions: row.try_get("conditions").map_err(db_err)?,
                via: row.try_get("via").map_err(db_err)?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(ResourceAccessResponse {
        resource_id,
        resource: ResourceSummary {
            id: resource.id,
            kind: resource.kind,
            name: resource.name,
            tenant_id: resource.tenant_id,
        },
        items,
        total,
    })
}

pub async fn group_access(
    pool: &PgPool,
    group_id: Uuid,
    params: GroupAccessQuery,
) -> Result<GroupAccessResponse, AppError> {
    let group = get_group_with_count(pool, group_id).await?;
    let query = AccessQuery {
        tenant_id: None,
        resource_kind: params.resource_kind,
        action: params.action,
        effect: params.effect,
        limit: params.limit,
        offset: params.offset,
    };
    let response = subject_access(pool, SubjectKind::Group, group_id, query, false).await?;
    Ok(GroupAccessResponse {
        group_id,
        group: GroupInfo {
            name: group.1.name,
            tenant_id: group.1.tenant_id,
            member_count: group.0,
        },
        items: response
            .items
            .into_iter()
            .map(|item| GroupAccessItem {
                resource: item.resource,
                effect: item.effect,
                scope_kind: item.scope_kind,
                scope_ref: item.scope_ref,
                policy_id: item.policy_id,
                grant: item.grant,
                conditions: item.conditions,
            })
            .collect(),
        total: response.total,
    })
}

async fn subject_access(
    pool: &PgPool,
    subject_kind: SubjectKind,
    subject_id: Uuid,
    params: AccessQuery,
    include_groups: bool,
) -> Result<EntityAccessResponse, AppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);
    let action_cap = optional_capability_id(pool, params.action.as_deref()).await?;
    let tenant_id = params.tenant_id;
    let resource_kind = params.resource_kind.clone();
    let effect = params.effect.clone();
    if params.action.is_some() && action_cap.is_none() {
        return Ok(EntityAccessResponse {
            entity_id: subject_id,
            entity_name: String::new(),
            entity_kind: crate::models::enums::EntityKind::Service,
            items: Vec::new(),
            total: 0,
        });
    }

    let group_clause = if include_groups {
        r#"UNION ALL
           SELECT pb.*, ('group:' || g.name)::text AS via
           FROM policy_bindings pb
           JOIN group_members gm ON gm.group_id = pb.subject_id
           JOIN groups g ON g.id = gm.group_id
           WHERE pb.subject_kind = 'group' AND gm.entity_id = $2"#
    } else {
        ""
    };
    let direct_via = if subject_kind == SubjectKind::Group {
        "'group'::text"
    } else {
        "'direct'::text"
    };
    let base_sql = format!(
        r#"WITH bindings AS (
             SELECT pb.*, {direct_via} AS via
             FROM policy_bindings pb
             WHERE pb.subject_kind = $1 AND pb.subject_id = $2
             {group_clause}
           ), expanded AS (
             SELECT b.id AS policy_id, b.grant_kind, b.grant_id, b.scope_kind, b.scope_ref,
                    b.effect, b.conditions, b.via, r.id AS resource_id, r.kind AS resource_kind,
                    r.name AS resource_name, r.tenant_id AS resource_tenant_id,
                    role.id AS role_id, role.name AS role_name
             FROM bindings b
             JOIN resources r ON
               b.scope_kind = 'platform'
               OR (b.scope_kind = 'object_kind' AND b.scope_ref = 'resource')
               OR (b.scope_kind = 'object_type' AND b.scope_ref = 'resource:' || r.kind)
               OR (b.scope_kind = 'object' AND b.scope_ref = r.id::text)
             LEFT JOIN roles role ON b.grant_kind = 'role' AND role.id = b.grant_id
             WHERE ($3::uuid IS NULL OR r.tenant_id = $3)
               AND ($4::text IS NULL OR r.kind = $4)
               AND ($5::text IS NULL OR b.effect = $5)
               AND (
                 $6::uuid IS NULL
                 OR (b.grant_kind = 'capability' AND b.grant_id = $6)
                 OR (b.grant_kind = 'role' AND EXISTS (
                   SELECT 1 FROM role_capabilities rc
                   WHERE rc.role_id = b.grant_id AND rc.capability_id = $6
                 ))
               )
           )"#
    );
    let sql = format!(
        r#"{base_sql}
           SELECT e.*,
                  COALESCE(jsonb_agg(jsonb_build_object(
                    'id', c.id,
                    'name', c.name,
                    'resource_kind', c.resource_kind
                  ) ORDER BY c.name) FILTER (WHERE c.id IS NOT NULL), '[]'::jsonb) AS capabilities
           FROM expanded e
           LEFT JOIN role_capabilities rc ON e.grant_kind = 'role' AND rc.role_id = e.grant_id
           LEFT JOIN capabilities c ON
             (e.grant_kind = 'capability' AND c.id = e.grant_id)
             OR (e.grant_kind = 'role' AND c.id = rc.capability_id)
           GROUP BY e.policy_id, e.grant_kind, e.grant_id, e.scope_kind, e.scope_ref,
                    e.effect, e.conditions, e.via, e.resource_id, e.resource_kind,
                    e.resource_name, e.resource_tenant_id, e.role_id, e.role_name
           ORDER BY e.resource_kind, e.resource_name NULLS LAST, e.policy_id
           LIMIT $7 OFFSET $8"#
    );
    let rows = sqlx::query(&sql)
        .bind(subject_kind.clone())
        .bind(subject_id)
        .bind(tenant_id)
        .bind(resource_kind.clone())
        .bind(effect.clone())
        .bind(action_cap)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(db_err)?;
    let total: i64 = sqlx::query_scalar(&format!("{base_sql} SELECT COUNT(*) FROM expanded"))
        .bind(subject_kind)
        .bind(subject_id)
        .bind(tenant_id)
        .bind(resource_kind)
        .bind(effect)
        .bind(action_cap)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;

    Ok(EntityAccessResponse {
        entity_id: subject_id,
        entity_name: String::new(),
        entity_kind: crate::models::enums::EntityKind::Service,
        total,
        items: rows_to_access_items(rows)?,
    })
}

pub async fn role_holders(
    pool: &PgPool,
    role_id: Uuid,
    params: RoleHoldersQuery,
) -> Result<RoleHoldersResponse, AppError> {
    use sqlx::Row;
    let role = get_role(pool, role_id).await?;
    let capabilities = get_role_capabilities(pool, role_id)
        .await?
        .into_iter()
        .map(|cap| CapabilitySummary {
            id: cap.id,
            name: cap.name,
            resource_kind: cap.resource_kind,
        })
        .collect();
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let rows = sqlx::query(
        r#"SELECT pb.id AS policy_id, pb.subject_kind, pb.effect, pb.scope_kind, pb.scope_ref,
                  pb.conditions,
                  e.id AS entity_id, e.name AS entity_name, e.kind AS entity_kind,
                  e.tenant_id AS entity_tenant_id,
                  g.id AS group_id, g.name AS group_name, g.tenant_id AS group_tenant_id,
                  COALESCE(gmc.member_count, 0)::bigint AS member_count
           FROM policy_bindings pb
           LEFT JOIN entities e ON pb.subject_kind = 'entity' AND e.id = pb.subject_id
           LEFT JOIN groups g ON pb.subject_kind = 'group' AND g.id = pb.subject_id
           LEFT JOIN (
             SELECT group_id, COUNT(*) AS member_count
             FROM group_members GROUP BY group_id
           ) gmc ON gmc.group_id = g.id
           WHERE pb.grant_kind = 'role'
             AND pb.grant_id = $1
             AND ($2::uuid IS NULL OR e.tenant_id = $2 OR g.tenant_id = $2)
             AND ($3::text IS NULL OR pb.subject_kind = $3)
           ORDER BY pb.created_at DESC
           LIMIT $4 OFFSET $5"#,
    )
    .bind(role_id)
    .bind(params.tenant_id)
    .bind(params.subject_kind.clone())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM policy_bindings pb
           LEFT JOIN entities e ON pb.subject_kind = 'entity' AND e.id = pb.subject_id
           LEFT JOIN groups g ON pb.subject_kind = 'group' AND g.id = pb.subject_id
           WHERE pb.grant_kind = 'role'
             AND pb.grant_id = $1
             AND ($2::uuid IS NULL OR e.tenant_id = $2 OR g.tenant_id = $2)
             AND ($3::text IS NULL OR pb.subject_kind = $3)"#,
    )
    .bind(role_id)
    .bind(params.tenant_id)
    .bind(params.subject_kind)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    let items = rows
        .into_iter()
        .map(|row| {
            let subject_kind: SubjectKind = row.try_get("subject_kind").map_err(db_err)?;
            let entity = match subject_kind {
                SubjectKind::Entity => Some(EntitySummary {
                    id: row.try_get("entity_id").map_err(db_err)?,
                    name: row.try_get("entity_name").map_err(db_err)?,
                    kind: row.try_get("entity_kind").map_err(db_err)?,
                    tenant_id: row.try_get("entity_tenant_id").map_err(db_err)?,
                }),
                SubjectKind::Group => None,
            };
            let group = match subject_kind {
                SubjectKind::Entity => None,
                SubjectKind::Group => Some(RoleHolderGroup {
                    id: row.try_get("group_id").map_err(db_err)?,
                    name: row.try_get("group_name").map_err(db_err)?,
                    tenant_id: row.try_get("group_tenant_id").map_err(db_err)?,
                    member_count: row.try_get("member_count").map_err(db_err)?,
                }),
            };
            Ok(RoleHolderItem {
                subject_kind,
                entity,
                group,
                policy_id: row.try_get("policy_id").map_err(db_err)?,
                effect: row.try_get("effect").map_err(db_err)?,
                scope_kind: row.try_get("scope_kind").map_err(db_err)?,
                scope_ref: row.try_get("scope_ref").map_err(db_err)?,
                conditions: row.try_get("conditions").map_err(db_err)?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(RoleHoldersResponse {
        role: RoleWithCapabilities {
            id: role.id,
            name: role.name,
            tenant_id: role.tenant_id,
            description: role.description,
            capabilities,
        },
        items,
        total,
    })
}

pub async fn effective_capabilities(
    pool: &PgPool,
    entity_id: Uuid,
    params: EffectiveCapabilitiesQuery,
) -> Result<EffectiveCapabilitiesResponse, AppError> {
    use sqlx::Row;
    let entity = sqlx::query_as::<_, Entity>(
        r#"SELECT id, kind, name, tenant_id, profile_id, profile_version_id,
                  status, attributes, created_at, updated_at
           FROM entities
           WHERE id = $1"#,
    )
    .bind(entity_id)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("entity {entity_id} not found")),
        other => AppError::Database(other),
    })?;

    let rows = sqlx::query(
        r#"WITH bindings AS (
             SELECT pb.*, 'direct'::text AS via
             FROM policy_bindings pb
             LEFT JOIN roles r ON pb.grant_kind = 'role' AND r.id = pb.grant_id
             WHERE pb.subject_kind = 'entity' AND pb.subject_id = $1
               AND ($2::uuid IS NULL OR r.tenant_id = $2)
             UNION ALL
             SELECT pb.*, ('group:' || g.name)::text AS via
             FROM policy_bindings pb
             JOIN group_members gm ON gm.group_id = pb.subject_id
             JOIN groups g ON g.id = gm.group_id AND g.status = 'active'
             LEFT JOIN roles r ON pb.grant_kind = 'role' AND r.id = pb.grant_id
             WHERE pb.subject_kind = 'group' AND gm.entity_id = $1
               AND ($2::uuid IS NULL OR g.tenant_id = $2 OR r.tenant_id = $2)
           )
           SELECT c.id AS capability_id, c.name AS capability_name, c.resource_kind,
                  b.grant_kind, b.grant_id, role.name AS role_name, b.id AS policy_id,
                  b.scope_kind, b.scope_ref, b.effect, b.via
           FROM bindings b
           LEFT JOIN roles role ON b.grant_kind = 'role' AND role.id = b.grant_id
           LEFT JOIN role_capabilities rc ON b.grant_kind = 'role' AND rc.role_id = b.grant_id
           JOIN capabilities c ON
             (b.grant_kind = 'capability' AND c.id = b.grant_id)
             OR (b.grant_kind = 'role' AND c.id = rc.capability_id)
           WHERE ($3::text IS NULL OR c.resource_kind = $3 OR c.resource_kind IS NULL)
           ORDER BY c.name, b.created_at DESC"#,
    )
    .bind(entity_id)
    .bind(params.tenant_id)
    .bind(params.resource_kind)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let mut caps: BTreeMap<(String, Uuid), EffectiveCapability> = BTreeMap::new();
    for row in rows {
        let cap_id: Uuid = row.try_get("capability_id").map_err(db_err)?;
        let cap_name: String = row.try_get("capability_name").map_err(db_err)?;
        let entry = caps
            .entry((cap_name.clone(), cap_id))
            .or_insert_with(|| EffectiveCapability {
                id: cap_id,
                name: cap_name,
                resource_kind: row.try_get("resource_kind").unwrap_or(None),
                sources: Vec::new(),
            });
        let grant_kind: GrantKind = row.try_get("grant_kind").map_err(db_err)?;
        entry.sources.push(CapabilitySource {
            kind: grant_kind.clone(),
            role_id: match grant_kind {
                GrantKind::Capability => None,
                GrantKind::Role => Some(row.try_get("grant_id").map_err(db_err)?),
            },
            role_name: row.try_get("role_name").map_err(db_err)?,
            policy_id: row.try_get("policy_id").map_err(db_err)?,
            scope_kind: row.try_get("scope_kind").map_err(db_err)?,
            scope_ref: row.try_get("scope_ref").map_err(db_err)?,
            effect: row.try_get("effect").map_err(db_err)?,
            via: row.try_get("via").map_err(db_err)?,
        });
    }

    Ok(EffectiveCapabilitiesResponse {
        entity_id: entity.id,
        entity_name: entity.name,
        entity_kind: entity.kind,
        capabilities: caps.into_values().collect(),
    })
}

pub async fn audit_logs(
    pool: &PgPool,
    params: crate::models::access::AuditQuery,
    allowed_tenant_ids: Option<Vec<Uuid>>,
) -> Result<AuditLogResponse, AppError> {
    let limit = params.limit.clamp(1, 200);
    let offset = params.offset.max(0);
    let items = sqlx::query_as::<_, AuditLogItem>(
        r#"SELECT id, entity_id, tenant_id, event, outcome, details, created_at
           FROM audit_logs
           WHERE ($1::uuid IS NULL OR entity_id = $1)
             AND ($2::text IS NULL OR event = $2)
             AND ($3::text IS NULL OR outcome = $3)
             AND ($4::timestamptz IS NULL OR created_at >= $4)
             AND ($5::timestamptz IS NULL OR created_at < $5)
             AND ($6::uuid IS NULL OR tenant_id = $6)
             AND ($7::uuid[] IS NULL OR tenant_id = ANY($7))
           ORDER BY created_at DESC
           LIMIT $8 OFFSET $9"#,
    )
    .bind(params.entity_id)
    .bind(params.event.clone())
    .bind(params.outcome.clone())
    .bind(params.from)
    .bind(params.to)
    .bind(params.tenant_id)
    .bind(allowed_tenant_ids.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM audit_logs
           WHERE ($1::uuid IS NULL OR entity_id = $1)
             AND ($2::text IS NULL OR event = $2)
             AND ($3::text IS NULL OR outcome = $3)
             AND ($4::timestamptz IS NULL OR created_at >= $4)
             AND ($5::timestamptz IS NULL OR created_at < $5)
             AND ($6::uuid IS NULL OR tenant_id = $6)
             AND ($7::uuid[] IS NULL OR tenant_id = ANY($7))"#,
    )
    .bind(params.entity_id)
    .bind(params.event)
    .bind(params.outcome)
    .bind(params.from)
    .bind(params.to)
    .bind(params.tenant_id)
    .bind(allowed_tenant_ids.as_deref())
    .fetch_one(pool)
    .await
    .map_err(db_err)?;
    Ok(AuditLogResponse { items, total })
}

pub async fn tenant_ids_for_capability(
    pool: &PgPool,
    entity_id: Uuid,
    capability_name: &str,
) -> Result<Vec<Uuid>, AppError> {
    sqlx::query_scalar(
        r#"WITH RECURSIVE group_paths(group_id) AS (
               SELECT gm.group_id
               FROM group_members gm
               JOIN groups g ON g.id = gm.group_id AND g.status = 'active'
               WHERE gm.entity_id = $1
               UNION ALL
               SELECT gh.parent_id
               FROM group_hierarchy gh
               JOIN group_paths gp ON gp.group_id = gh.child_id
               JOIN groups parent ON parent.id = gh.parent_id AND parent.status = 'active'
           )
           SELECT DISTINCT pb.scope_ref::uuid
           FROM policy_bindings pb
           WHERE (
               (pb.subject_kind = 'entity' AND pb.subject_id = $1)
               OR (pb.subject_kind = 'group' AND pb.subject_id IN (SELECT group_id FROM group_paths))
           )
             AND pb.effect = 'allow'
             AND pb.scope_kind = 'tenant'
             AND pb.scope_ref IS NOT NULL
             AND (
               (pb.grant_kind = 'capability' AND pb.grant_id IN (
                   SELECT id FROM capabilities WHERE name = $2 AND resource_kind IS NULL
               ))
               OR (pb.grant_kind = 'role' AND pb.grant_id IN (
                   SELECT role_id
                   FROM role_capabilities rc
                   JOIN capabilities c ON c.id = rc.capability_id
                   WHERE c.name = $2 AND c.resource_kind IS NULL
                   UNION
                   SELECT rcomp.parent_role_id
                   FROM role_composites rcomp
                   JOIN role_capabilities rc ON rc.role_id = rcomp.child_role_id
                   JOIN capabilities c ON c.id = rc.capability_id
                   WHERE c.name = $2 AND c.resource_kind IS NULL
               ))
             )"#,
    )
    .bind(entity_id)
    .bind(capability_name)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

pub async fn orphan_policies(
    pool: &PgPool,
    params: AdminPageQuery,
) -> Result<OrphanPoliciesResponse, AppError> {
    use sqlx::Row;
    let limit = params.limit.clamp(1, 200);
    let offset = params.offset.max(0);
    let rows = sqlx::query(
        r#"WITH orphaned AS (
             SELECT pb.*,
                    CASE
                      WHEN (pb.subject_kind = 'entity' AND e.id IS NULL)
                        OR (pb.subject_kind = 'group' AND g.id IS NULL)
                      THEN 'subject_not_found'
                      WHEN (pb.grant_kind = 'capability' AND c.id IS NULL)
                        OR (pb.grant_kind = 'role' AND r.id IS NULL)
                      THEN 'grant_not_found'
                    END AS orphan_reason
             FROM policy_bindings pb
             LEFT JOIN entities e ON pb.subject_kind = 'entity' AND pb.subject_id = e.id
             LEFT JOIN groups g ON pb.subject_kind = 'group' AND pb.subject_id = g.id
             LEFT JOIN capabilities c ON pb.grant_kind = 'capability' AND pb.grant_id = c.id
             LEFT JOIN roles r ON pb.grant_kind = 'role' AND pb.grant_id = r.id
           )
           SELECT * FROM orphaned
           WHERE orphan_reason IS NOT NULL
           ORDER BY created_at DESC
           LIMIT $1 OFFSET $2"#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let total: i64 = sqlx::query_scalar(
        r#"WITH orphaned AS (
             SELECT CASE
                      WHEN (pb.subject_kind = 'entity' AND e.id IS NULL)
                        OR (pb.subject_kind = 'group' AND g.id IS NULL)
                      THEN 'subject_not_found'
                      WHEN (pb.grant_kind = 'capability' AND c.id IS NULL)
                        OR (pb.grant_kind = 'role' AND r.id IS NULL)
                      THEN 'grant_not_found'
                    END AS orphan_reason
             FROM policy_bindings pb
             LEFT JOIN entities e ON pb.subject_kind = 'entity' AND pb.subject_id = e.id
             LEFT JOIN groups g ON pb.subject_kind = 'group' AND pb.subject_id = g.id
             LEFT JOIN capabilities c ON pb.grant_kind = 'capability' AND pb.grant_id = c.id
             LEFT JOIN roles r ON pb.grant_kind = 'role' AND pb.grant_id = r.id
           )
           SELECT COUNT(*) FROM orphaned WHERE orphan_reason IS NOT NULL"#,
    )
    .fetch_one(pool)
    .await
    .map_err(db_err)?;
    let items = rows
        .into_iter()
        .map(|row| {
            Ok(OrphanPolicyItem {
                id: row.try_get("id").map_err(db_err)?,
                subject_kind: row.try_get("subject_kind").map_err(db_err)?,
                subject_id: row.try_get("subject_id").map_err(db_err)?,
                grant_kind: row.try_get("grant_kind").map_err(db_err)?,
                grant_id: row.try_get("grant_id").map_err(db_err)?,
                scope_kind: row.try_get("scope_kind").map_err(db_err)?,
                scope_ref: row.try_get("scope_ref").map_err(db_err)?,
                effect: row.try_get("effect").map_err(db_err)?,
                conditions: row.try_get("conditions").map_err(db_err)?,
                created_at: row.try_get("created_at").map_err(db_err)?,
                orphan_reason: row.try_get("orphan_reason").map_err(db_err)?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(OrphanPoliciesResponse { items, total })
}

pub async fn unprotected_resources(
    pool: &PgPool,
    params: UnprotectedResourcesQuery,
) -> Result<UnprotectedResourcesResponse, AppError> {
    let limit = params.limit.clamp(1, 200);
    let offset = params.offset.max(0);
    let items = sqlx::query_as::<_, Resource>(
        r#"SELECT id, kind, name, tenant_id, owner_id, attributes, created_at, updated_at
           FROM resources r
           WHERE ($1::uuid IS NULL OR tenant_id = $1)
             AND ($2::text IS NULL OR kind = $2)
             AND NOT EXISTS (
               SELECT 1 FROM policy_bindings pb
               WHERE pb.scope_kind = 'object' AND pb.scope_ref = r.id::text
             )
             AND NOT EXISTS (
               SELECT 1 FROM policy_bindings pb
               WHERE pb.scope_kind = 'object_type' AND pb.scope_ref = 'resource:' || r.kind
             )
             AND NOT EXISTS (
               SELECT 1 FROM policy_bindings pb
               WHERE pb.scope_kind = 'object_kind' AND pb.scope_ref = 'resource'
             )
             AND NOT EXISTS (
               SELECT 1 FROM policy_bindings pb WHERE pb.scope_kind = 'platform'
             )
           ORDER BY created_at DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(params.tenant_id)
    .bind(params.kind.clone())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM resources r
           WHERE ($1::uuid IS NULL OR tenant_id = $1)
             AND ($2::text IS NULL OR kind = $2)
             AND NOT EXISTS (
               SELECT 1 FROM policy_bindings pb
               WHERE pb.scope_kind = 'object' AND pb.scope_ref = r.id::text
             )
             AND NOT EXISTS (
               SELECT 1 FROM policy_bindings pb
               WHERE pb.scope_kind = 'object_type' AND pb.scope_ref = 'resource:' || r.kind
             )
             AND NOT EXISTS (
               SELECT 1 FROM policy_bindings pb
               WHERE pb.scope_kind = 'object_kind' AND pb.scope_ref = 'resource'
             )
             AND NOT EXISTS (
               SELECT 1 FROM policy_bindings pb WHERE pb.scope_kind = 'platform'
             )"#,
    )
    .bind(params.tenant_id)
    .bind(params.kind)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;
    Ok(UnprotectedResourcesResponse {
        items: items
            .into_iter()
            .map(|r| UnprotectedResourceItem {
                id: r.id,
                kind: r.kind,
                name: r.name,
                tenant_id: r.tenant_id,
                owner_id: r.owner_id,
                created_at: r.created_at,
            })
            .collect(),
        total,
    })
}

pub async fn expiring_credentials(
    pool: &PgPool,
    params: ExpiringCredentialsQuery,
) -> Result<ExpiringCredentialsResponse, AppError> {
    use sqlx::Row;
    let limit = params.limit.clamp(1, 200);
    let offset = params.offset.max(0);
    let days = params.days.max(0);
    let rows = sqlx::query(
        r#"SELECT c.id, c.entity_id, e.name AS entity_name, e.kind AS entity_kind,
                  c.kind, c.status, c.expires_at, c.created_at
           FROM credentials c
           JOIN entities e ON e.id = c.entity_id
           WHERE c.status = 'active'
             AND c.expires_at IS NOT NULL
             AND c.expires_at <= now() + ($1::text || ' days')::interval
             AND ($2::uuid IS NULL OR c.entity_id = $2)
             AND ($3::text IS NULL OR c.kind = $3)
           ORDER BY c.expires_at ASC
           LIMIT $4 OFFSET $5"#,
    )
    .bind(days.to_string())
    .bind(params.entity_id)
    .bind(params.kind.clone())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;
    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM credentials c
           WHERE c.status = 'active'
             AND c.expires_at IS NOT NULL
             AND c.expires_at <= now() + ($1::text || ' days')::interval
             AND ($2::uuid IS NULL OR c.entity_id = $2)
             AND ($3::text IS NULL OR c.kind = $3)"#,
    )
    .bind(days.to_string())
    .bind(params.entity_id)
    .bind(params.kind)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;
    let now = Utc::now();
    let items = rows
        .into_iter()
        .map(|row| {
            let expires_at = row.try_get("expires_at").map_err(db_err)?;
            Ok(ExpiringCredentialItem {
                id: row.try_get("id").map_err(db_err)?,
                entity_id: row.try_get("entity_id").map_err(db_err)?,
                entity_name: row.try_get("entity_name").map_err(db_err)?,
                entity_kind: row.try_get("entity_kind").map_err(db_err)?,
                kind: row.try_get::<CredentialKind, _>("kind").map_err(db_err)?,
                status: row.try_get("status").map_err(db_err)?,
                expires_at,
                days_remaining: (expires_at - now).num_days(),
                created_at: row.try_get("created_at").map_err(db_err)?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(ExpiringCredentialsResponse { items, total })
}

async fn optional_capability_id(
    pool: &PgPool,
    action: Option<&str>,
) -> Result<Option<Uuid>, AppError> {
    match action {
        Some(name) => sqlx::query_scalar("SELECT id FROM capabilities WHERE name = $1 LIMIT 1")
            .bind(name)
            .fetch_optional(pool)
            .await
            .map_err(db_err),
        None => Ok(None),
    }
}

fn rows_to_access_items(rows: Vec<sqlx::postgres::PgRow>) -> Result<Vec<AccessItem>, AppError> {
    use sqlx::Row;
    rows.into_iter()
        .map(|row| {
            let caps: Value = row.try_get("capabilities").map_err(db_err)?;
            Ok(AccessItem {
                resource: ResourceSummary {
                    id: row.try_get("resource_id").map_err(db_err)?,
                    kind: row.try_get("resource_kind").map_err(db_err)?,
                    name: row.try_get("resource_name").map_err(db_err)?,
                    tenant_id: row.try_get("resource_tenant_id").map_err(db_err)?,
                },
                effect: row.try_get("effect").map_err(db_err)?,
                scope_kind: row.try_get("scope_kind").map_err(db_err)?,
                scope_ref: row.try_get("scope_ref").map_err(db_err)?,
                policy_id: row.try_get("policy_id").map_err(db_err)?,
                grant: grant_from_row(&row, caps)?,
                conditions: row.try_get("conditions").map_err(db_err)?,
                via: row.try_get("via").map_err(db_err)?,
            })
        })
        .collect()
}

fn grant_from_row(row: &sqlx::postgres::PgRow, caps: Value) -> Result<GrantSummary, AppError> {
    use sqlx::Row;
    let kind: GrantKind = row.try_get("grant_kind").map_err(db_err)?;
    let capabilities = serde_json::from_value::<Vec<CapabilitySummary>>(caps)
        .map_err(|e| AppError::bad_request(format!("invalid capability aggregate: {e}")))?;
    let role = match kind {
        GrantKind::Capability => None,
        GrantKind::Role => Some(RoleSummary {
            id: row.try_get("role_id").map_err(db_err)?,
            name: row.try_get("role_name").map_err(db_err)?,
        }),
    };
    Ok(GrantSummary {
        kind,
        role,
        capabilities,
    })
}

async fn get_group_with_count(pool: &PgPool, group_id: Uuid) -> Result<(i64, Group), AppError> {
    let group = sqlx::query_as::<_, Group>(
        r#"SELECT g.id, g.name, g.tenant_id, g.group_type, g.description, gh.parent_id,
                  g.status, g.attributes, g.created_at, g.updated_at
           FROM groups g
           LEFT JOIN group_hierarchy gh ON gh.child_id = g.id
           WHERE g.id = $1"#,
    )
    .bind(group_id)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("group {group_id} not found")),
        other => AppError::Database(other),
    })?;
    let count = sqlx::query_scalar("SELECT COUNT(*) FROM group_members WHERE group_id = $1")
        .bind(group_id)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
    Ok((count, group))
}

// ─── Engine helpers ───────────────────────────────────────────────────────────

pub async fn load_bindings_for_entity(
    pool: &PgPool,
    entity_id: Uuid,
) -> Result<Vec<PolicyBinding>, AppError> {
    sqlx::query_as::<_, PolicyBinding>(
        r#"WITH RECURSIVE group_paths(group_id, path) AS (
               SELECT gm.group_id, g.name
               FROM group_members gm
               JOIN groups g ON g.id = gm.group_id AND g.status = 'active'
               WHERE gm.entity_id = $1
               UNION ALL
               SELECT gh.parent_id, parent.name || ' -> ' || gp.path
               FROM group_hierarchy gh
               JOIN group_paths gp ON gp.group_id = gh.child_id
               JOIN groups parent ON parent.id = gh.parent_id AND parent.status = 'active'
           )
           SELECT pb.id, pb.tenant_id, pb.subject_kind, pb.subject_id, pb.grant_kind, pb.grant_id,
                  pb.scope_kind, pb.scope_ref, pb.effect, pb.conditions, pb.created_at
           FROM policy_bindings pb
           WHERE
             (pb.subject_kind = 'entity' AND pb.subject_id = $1)
             OR
             (pb.subject_kind = 'group' AND pb.subject_id IN (SELECT group_id FROM group_paths))"#,
    )
    .bind(entity_id)
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

/// Batch load capability IDs for multiple roles in a single query.
/// Returns a map of role_id → Vec<capability_id>.
pub async fn capability_ids_for_roles(
    pool: &PgPool,
    role_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<Uuid>>, AppError> {
    if role_ids.is_empty() {
        return Ok(HashMap::new());
    }

    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT role_id, capability_id FROM role_capabilities WHERE role_id = ANY($1::uuid[])",
    )
    .bind(role_ids)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let mut map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for row in rows {
        let role_id: Uuid = row.try_get("role_id").map_err(db_err)?;
        let cap_id: Uuid = row.try_get("capability_id").map_err(db_err)?;
        map.entry(role_id).or_default().push(cap_id);
    }

    Ok(map)
}

/// Batch-load primitive role grants for direct roles and one-level composite roles.
/// Returns a map of root role_id → expanded grants. For primitive roles, the root
/// role is also the grant role. For composite roles, each child primitive role
/// contributes its own scope and capabilities.
pub async fn expanded_role_grants_for_roles(
    pool: &PgPool,
    role_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<ExpandedRoleGrant>>, AppError> {
    if role_ids.is_empty() {
        return Ok(HashMap::new());
    }

    use sqlx::Row;
    let rows = sqlx::query(
        r#"SELECT r.id AS root_role_id,
                  r.id AS role_id,
                  r.name AS role_name,
                  r.name AS role_path,
                  r.scope_kind,
                  r.scope_ref,
                  rc.capability_id
           FROM roles r
           JOIN role_capabilities rc ON rc.role_id = r.id
           WHERE r.id = ANY($1::uuid[])
           UNION ALL
           SELECT r.id AS root_role_id,
                  r.id AS role_id,
                  r.name AS role_name,
                  r.name AS role_path,
                  CASE rpb.applies_to
                    WHEN 'object_group_type' THEN 'group_object_type'
                    WHEN 'object_group_tree_type' THEN 'group_tree_object_type'
                    WHEN 'object_group_child_kind' THEN 'group_child_kind'
                    WHEN 'object_group_descendant_kind' THEN 'group_descendant_kind'
                    ELSE rpb.applies_to
                  END AS scope_kind,
                  CASE rpb.applies_to
                    WHEN 'platform' THEN NULL
                    WHEN 'tenant' THEN rpb.tenant_id::text
                    WHEN 'object_kind' THEN rpb.object_kind
                    WHEN 'object_type' THEN
                      CASE
                        WHEN rpb.object_type LIKE '%:%' THEN rpb.object_type
                        ELSE rpb.object_kind || ':' || rpb.object_type
                      END
                    WHEN 'object' THEN rpb.object_id::text
                    WHEN 'object_group_type' THEN
                      rpb.group_id::text || ':' ||
                      CASE
                        WHEN rpb.object_type LIKE '%:%' THEN rpb.object_type
                        ELSE rpb.object_kind || ':' || rpb.object_type
                      END
                    WHEN 'object_group_tree_type' THEN
                      rpb.group_id::text || ':' ||
                      CASE
                        WHEN rpb.object_type LIKE '%:%' THEN rpb.object_type
                        ELSE rpb.object_kind || ':' || rpb.object_type
                      END
                    WHEN 'object_group_child_kind' THEN rpb.group_id::text || ':' || rpb.object_kind
                    WHEN 'object_group_descendant_kind' THEN rpb.group_id::text || ':' || rpb.object_kind
                    ELSE NULL
                  END AS scope_ref,
                  rpa.capability_id
           FROM roles r
           JOIN role_permission_blocks rpb ON rpb.role_id = r.id
           JOIN role_permission_actions rpa ON rpa.block_id = rpb.id
           WHERE r.id = ANY($1::uuid[])
           UNION ALL
           SELECT parent.id AS root_role_id,
                  child.id AS role_id,
                  child.name AS role_name,
                  parent.name || ' -> ' || child.name AS role_path,
                  child.scope_kind,
                  child.scope_ref,
                  rc.capability_id
           FROM roles parent
           JOIN role_composites composite ON composite.parent_role_id = parent.id
           JOIN roles child ON child.id = composite.child_role_id
           JOIN role_capabilities rc ON rc.role_id = child.id
           WHERE parent.id = ANY($1::uuid[])
           UNION ALL
           SELECT parent.id AS root_role_id,
                  child.id AS role_id,
                  child.name AS role_name,
                  parent.name || ' -> ' || child.name AS role_path,
                  CASE rpb.applies_to
                    WHEN 'object_group_type' THEN 'group_object_type'
                    WHEN 'object_group_tree_type' THEN 'group_tree_object_type'
                    WHEN 'object_group_child_kind' THEN 'group_child_kind'
                    WHEN 'object_group_descendant_kind' THEN 'group_descendant_kind'
                    ELSE rpb.applies_to
                  END AS scope_kind,
                  CASE rpb.applies_to
                    WHEN 'platform' THEN NULL
                    WHEN 'tenant' THEN rpb.tenant_id::text
                    WHEN 'object_kind' THEN rpb.object_kind
                    WHEN 'object_type' THEN
                      CASE
                        WHEN rpb.object_type LIKE '%:%' THEN rpb.object_type
                        ELSE rpb.object_kind || ':' || rpb.object_type
                      END
                    WHEN 'object' THEN rpb.object_id::text
                    WHEN 'object_group_type' THEN
                      rpb.group_id::text || ':' ||
                      CASE
                        WHEN rpb.object_type LIKE '%:%' THEN rpb.object_type
                        ELSE rpb.object_kind || ':' || rpb.object_type
                      END
                    WHEN 'object_group_tree_type' THEN
                      rpb.group_id::text || ':' ||
                      CASE
                        WHEN rpb.object_type LIKE '%:%' THEN rpb.object_type
                        ELSE rpb.object_kind || ':' || rpb.object_type
                      END
                    WHEN 'object_group_child_kind' THEN rpb.group_id::text || ':' || rpb.object_kind
                    WHEN 'object_group_descendant_kind' THEN rpb.group_id::text || ':' || rpb.object_kind
                    ELSE NULL
                  END AS scope_ref,
                  rpa.capability_id
           FROM roles parent
           JOIN role_composites composite ON composite.parent_role_id = parent.id
           JOIN roles child ON child.id = composite.child_role_id
           JOIN role_permission_blocks rpb ON rpb.role_id = child.id
           JOIN role_permission_actions rpa ON rpa.block_id = rpb.id
           WHERE parent.id = ANY($1::uuid[])"#,
    )
    .bind(role_ids)
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    let mut map: HashMap<Uuid, Vec<ExpandedRoleGrant>> = HashMap::new();
    for row in rows {
        let scope_kind_text: String = row.try_get("scope_kind").map_err(db_err)?;
        let scope_kind = parse_scope_kind_text(&scope_kind_text)?;
        let root_role_id: Uuid = row.try_get("root_role_id").map_err(db_err)?;
        map.entry(root_role_id)
            .or_default()
            .push(ExpandedRoleGrant {
                root_role_id,
                role_id: row.try_get("role_id").map_err(db_err)?,
                role_name: row.try_get("role_name").map_err(db_err)?,
                role_path: row.try_get("role_path").map_err(db_err)?,
                scope_kind,
                scope_ref: row.try_get("scope_ref").map_err(db_err)?,
                capability_id: row.try_get("capability_id").map_err(db_err)?,
            });
    }

    Ok(map)
}

pub async fn find_capability_ids_by_name(
    pool: &PgPool,
    name: &str,
    object_kind: &str,
    object_type: &str,
) -> Result<Vec<Uuid>, AppError> {
    sqlx::query_scalar(
        r#"SELECT c.id
           FROM capabilities c
           JOIN capability_applicability ca ON ca.capability_id = c.id
           WHERE c.name = $1
             AND ca.object_kind = $2
             AND (ca.object_type IS NULL OR ca.object_type = $3)
           ORDER BY c.resource_kind NULLS LAST, c.id"#,
    )
    .bind(name)
    .bind(object_kind)
    .bind(format!("{object_kind}:{object_type}"))
    .fetch_all(pool)
    .await
    .map_err(db_err)
}

fn search_pattern(q: Option<String>) -> Option<String> {
    q.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{value}%"))
}
