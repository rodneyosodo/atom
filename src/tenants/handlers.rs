use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    auth::{require_capability, require_list_access, require_read_access, AuthContext, Scope},
    error::AppError,
    models::{
        enums::TenantStatus,
        tenant::{CreateTenant, ListTenants, UpdateTenant},
    },
    state::AppState,
};

use super::repo;

pub async fn create_tenant(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateTenant>,
) -> Result<impl IntoResponse, AppError> {
    require_capability(
        &state.pool,
        auth.entity_id,
        "tenant.manage",
        Scope::Platform,
    )
    .await?;
    let tenant = repo::create_tenant(&state.pool, req, Some(auth.entity_id)).await?;
    Ok((StatusCode::CREATED, Json(tenant)))
}

pub async fn list_tenants(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ListTenants>,
) -> Result<impl IntoResponse, AppError> {
    require_list_access(&state.pool, auth.entity_id, None).await?;
    let list = repo::list_tenants(&state.pool, params).await?;
    Ok(Json(list))
}

pub async fn get_tenant(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_read_access(&state.pool, auth.entity_id, Some(id), id).await?;
    let tenant = repo::get_tenant(&state.pool, id).await?;
    Ok(Json(tenant))
}

pub async fn update_tenant(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTenant>,
) -> Result<impl IntoResponse, AppError> {
    require_capability(
        &state.pool,
        auth.entity_id,
        "tenant.manage",
        Scope::Platform,
    )
    .await?;
    let tenant = repo::update_tenant(&state.pool, id, req, Some(auth.entity_id)).await?;
    Ok(Json(tenant))
}

pub async fn enable_tenant(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_capability(
        &state.pool,
        auth.entity_id,
        "tenant.manage",
        Scope::Platform,
    )
    .await?;
    let tenant =
        repo::change_tenant_status(&state.pool, id, TenantStatus::Active, Some(auth.entity_id))
            .await?;
    Ok(Json(tenant))
}

pub async fn disable_tenant(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_capability(
        &state.pool,
        auth.entity_id,
        "tenant.manage",
        Scope::Platform,
    )
    .await?;
    let tenant = repo::change_tenant_status(
        &state.pool,
        id,
        TenantStatus::Inactive,
        Some(auth.entity_id),
    )
    .await?;
    Ok(Json(tenant))
}

pub async fn freeze_tenant(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_capability(
        &state.pool,
        auth.entity_id,
        "tenant.manage",
        Scope::Platform,
    )
    .await?;
    let tenant =
        repo::change_tenant_status(&state.pool, id, TenantStatus::Frozen, Some(auth.entity_id))
            .await?;
    Ok(Json(tenant))
}

pub async fn delete_tenant(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_capability(
        &state.pool,
        auth.entity_id,
        "tenant.manage",
        Scope::Platform,
    )
    .await?;
    repo::change_tenant_status(&state.pool, id, TenantStatus::Deleted, Some(auth.entity_id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
