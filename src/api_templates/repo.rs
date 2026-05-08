use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{db_err, AppError},
    models::api_template::{
        ApiTemplate, ApiTemplateList, ApiTemplateStatus, CreateApiTemplate, ListApiTemplates,
        UpdateApiTemplate,
    },
};

const API_TEMPLATE_COLS: &str = "id, tenant_id, key, name, description, operation_kind, graphql, variables_schema, default_variables, result_selector, tags, status, created_by, updated_by, created_at, updated_at";

pub async fn create_api_template(
    pool: &PgPool,
    req: CreateApiTemplate,
    created_by: Option<Uuid>,
) -> Result<ApiTemplate, AppError> {
    validate_graphql_template(&req.graphql)?;
    let id = Uuid::new_v4();
    sqlx::query_as::<_, ApiTemplate>(&format!(
        r#"INSERT INTO api_templates
           (id, tenant_id, key, name, description, operation_kind, graphql,
            variables_schema, default_variables, result_selector, tags, status,
            created_by, updated_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                   COALESCE($12, 'active'), $13, $13)
           RETURNING {API_TEMPLATE_COLS}"#,
    ))
    .bind(id)
    .bind(req.tenant_id)
    .bind(req.key)
    .bind(req.name)
    .bind(req.description)
    .bind(req.operation_kind)
    .bind(req.graphql)
    .bind(json_object_or_default(req.variables_schema))
    .bind(json_object_or_default(req.default_variables))
    .bind(json_object_or_default(req.result_selector))
    .bind(req.tags)
    .bind(req.status)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .map_err(template_db_err)
}

pub async fn get_api_template(pool: &PgPool, id: Uuid) -> Result<ApiTemplate, AppError> {
    sqlx::query_as::<_, ApiTemplate>(&format!(
        "SELECT {API_TEMPLATE_COLS} FROM api_templates WHERE id = $1",
    ))
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("api template {id} not found")),
        other => template_db_err(other),
    })
}

pub async fn list_api_templates(
    pool: &PgPool,
    params: ListApiTemplates,
) -> Result<ApiTemplateList, AppError> {
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);
    let tenant_id = params.tenant_id;
    let status = params.status;
    let tag = params.tag;

    let items = sqlx::query_as::<_, ApiTemplate>(&format!(
        r#"SELECT {API_TEMPLATE_COLS} FROM api_templates
           WHERE ($1::uuid IS NULL OR tenant_id = $1)
             AND ($2::text IS NULL OR status = $2)
             AND ($3::text IS NULL OR $3 = ANY(tags))
           ORDER BY tenant_id NULLS FIRST, key
           LIMIT $4 OFFSET $5"#,
    ))
    .bind(tenant_id)
    .bind(status)
    .bind(tag.clone())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(template_db_err)?;

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM api_templates
           WHERE ($1::uuid IS NULL OR tenant_id = $1)
             AND ($2::text IS NULL OR status = $2)
             AND ($3::text IS NULL OR $3 = ANY(tags))"#,
    )
    .bind(tenant_id)
    .bind(status)
    .bind(tag)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    Ok(ApiTemplateList { items, total })
}

pub async fn update_api_template(
    pool: &PgPool,
    id: Uuid,
    req: UpdateApiTemplate,
    updated_by: Option<Uuid>,
) -> Result<ApiTemplate, AppError> {
    if let Some(graphql) = req.graphql.as_deref() {
        validate_graphql_template(graphql)?;
    }

    sqlx::query_as::<_, ApiTemplate>(&format!(
        r#"UPDATE api_templates
           SET key               = COALESCE($2, key),
               name              = COALESCE($3, name),
               description       = COALESCE($4, description),
               operation_kind    = COALESCE($5, operation_kind),
               graphql           = COALESCE($6, graphql),
               variables_schema  = COALESCE($7, variables_schema),
               default_variables = COALESCE($8, default_variables),
               result_selector   = COALESCE($9, result_selector),
               tags              = COALESCE($10, tags),
               status            = COALESCE($11, status),
               updated_by        = $12,
               updated_at        = now()
           WHERE id = $1
           RETURNING {API_TEMPLATE_COLS}"#,
    ))
    .bind(id)
    .bind(req.key)
    .bind(req.name)
    .bind(req.description)
    .bind(req.operation_kind)
    .bind(req.graphql)
    .bind(req.variables_schema.map(json_object_or_default))
    .bind(req.default_variables.map(json_object_or_default))
    .bind(req.result_selector.map(json_object_or_default))
    .bind(req.tags)
    .bind(req.status)
    .bind(updated_by)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::not_found(format!("api template {id} not found")),
        other => template_db_err(other),
    })
}

pub async fn disable_api_template(
    pool: &PgPool,
    id: Uuid,
    updated_by: Option<Uuid>,
) -> Result<(), AppError> {
    let result = sqlx::query(
        r#"UPDATE api_templates
           SET status = $2, updated_by = $3, updated_at = now()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(ApiTemplateStatus::Disabled)
    .bind(updated_by)
    .execute(pool)
    .await
    .map_err(template_db_err)?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!("api template {id} not found")));
    }
    Ok(())
}

fn json_object_or_default(value: Value) -> Value {
    if value.is_null() {
        serde_json::json!({})
    } else {
        value
    }
}

fn validate_graphql_template(graphql: &str) -> Result<(), AppError> {
    for operation in ["createDomain", "createClient", "createChannel"] {
        if graphql.contains(operation) {
            return Err(AppError::bad_request(format!(
                "api templates must use generic Atom GraphQL operations; found {operation}"
            )));
        }
    }
    Ok(())
}

fn template_db_err(err: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db) = &err {
        if db.code().as_deref() == Some("23505") {
            return AppError::conflict("api template key already exists");
        }
    }
    db_err(err)
}
