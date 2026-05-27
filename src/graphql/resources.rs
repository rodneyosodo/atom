use async_graphql::{Context, Object, Result, ID};

use crate::{
    authz::{engine, repo as authz_repo},
    models::{
        policy::AuthzRequest,
        resource::{CreateResource, ListResources, UpdateResource},
    },
    state::AppState,
};

use super::{
    auth::{
        gql_error, require_any_capability, require_auth, require_list_access, require_read_access,
        scope_for_tenant,
    },
    types::{
        parse_id, parse_optional_id, CreateResourceInput, Resource, ResourceList,
        UpdateResourceInput,
    },
};

#[derive(Default)]
pub struct ResourceQuery;

#[Object]
impl ResourceQuery {
    #[allow(clippy::too_many_arguments)]
    async fn resources(
        &self,
        ctx: &Context<'_>,
        q: Option<String>,
        kind: Option<String>,
        tenant_id: Option<ID>,
        parent_group_id: Option<ID>,
        include_descendants: Option<bool>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<ResourceList> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(tenant_id, "tenantId")?;
        let parent_group_id = parse_optional_id(parent_group_id, "parentGroupId")?;
        if parent_group_id.is_none() {
            require_list_access(&state.pool, auth.entity_id, tenant_id).await?;
        }
        let list = authz_repo::list_resources(
            &state.pool,
            ListResources {
                q,
                kind,
                tenant_id,
                parent_group_id,
                include_descendants: include_descendants.unwrap_or(false),
                limit: limit.map(i64::from).unwrap_or(20),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;
        if parent_group_id.is_some() {
            let mut authorized = Vec::new();
            for item in list.items {
                let allowed = engine::evaluate(
                    &state.pool,
                    &AuthzRequest {
                        subject_id: auth.entity_id,
                        action: "read".to_string(),
                        resource_id: None,
                        object_kind: Some("resource".to_string()),
                        object_id: Some(item.id),
                        context: serde_json::Value::Null,
                    },
                )
                .await
                .map_err(gql_error)?
                .allowed;
                if allowed {
                    authorized.push(Resource::from(item));
                }
            }
            let total = authorized.len() as i64;
            return Ok(ResourceList {
                items: authorized,
                total,
            });
        }

        Ok(ResourceList {
            items: list.items.into_iter().map(Resource::from).collect(),
            total: list.total,
        })
    }

    async fn resource(&self, ctx: &Context<'_>, id: ID) -> Result<Resource> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let id = parse_id(id, "id")?;
        let resource = authz_repo::get_resource(&state.pool, id)
            .await
            .map_err(gql_error)?;
        if !engine::evaluate(
            &state.pool,
            &AuthzRequest {
                subject_id: auth.entity_id,
                action: "read".to_string(),
                resource_id: None,
                object_kind: Some("resource".to_string()),
                object_id: Some(id),
                context: serde_json::Value::Null,
            },
        )
        .await
        .map_err(gql_error)?
        .allowed
        {
            require_read_access(&state.pool, auth.entity_id, resource.tenant_id, id).await?;
        }
        Ok(resource.into())
    }
}

#[derive(Default)]
pub struct ResourceMutation;

#[Object]
impl ResourceMutation {
    async fn create_resource(
        &self,
        ctx: &Context<'_>,
        input: CreateResourceInput,
    ) -> Result<Resource> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(input.tenant_id, "tenantId")?;
        require_any_capability(
            &state.pool,
            auth.entity_id,
            &[
                ("manage", scope_for_tenant(tenant_id)),
                ("write", scope_for_tenant(tenant_id)),
            ],
        )
        .await?;

        let resource = authz_repo::create_resource(
            &state.pool,
            CreateResource {
                id: parse_optional_id(input.id, "id")?,
                kind: input.kind,
                name: input.name,
                tenant_id,
                owner_id: parse_optional_id(input.owner_id, "ownerId")?,
                attributes: input.attributes.unwrap_or(serde_json::Value::Null),
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(resource.into())
    }

    async fn update_resource(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateResourceInput,
    ) -> Result<Resource> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let id = parse_id(id, "id")?;
        let existing = authz_repo::get_resource(&state.pool, id)
            .await
            .map_err(gql_error)?;
        require_any_capability(
            &state.pool,
            auth.entity_id,
            &[
                ("manage", crate::auth::Scope::Object(id)),
                ("manage", scope_for_tenant(existing.tenant_id)),
            ],
        )
        .await?;

        let resource = authz_repo::update_resource(
            &state.pool,
            id,
            UpdateResource {
                name: input.name,
                attributes: input.attributes,
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(resource.into())
    }

    async fn delete_resource(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let id = parse_id(id, "id")?;
        let existing = authz_repo::get_resource(&state.pool, id)
            .await
            .map_err(gql_error)?;
        require_any_capability(
            &state.pool,
            auth.entity_id,
            &[
                ("manage", crate::auth::Scope::Object(id)),
                ("manage", scope_for_tenant(existing.tenant_id)),
            ],
        )
        .await?;

        authz_repo::delete_resource(&state.pool, id)
            .await
            .map_err(gql_error)?;

        Ok(true)
    }

    async fn set_resource_parent_group(
        &self,
        ctx: &Context<'_>,
        resource_id: ID,
        group_id: ID,
    ) -> Result<Resource> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let resource_id = parse_id(resource_id, "resourceId")?;
        let group_id = parse_id(group_id, "groupId")?;
        let resource = authz_repo::get_resource(&state.pool, resource_id)
            .await
            .map_err(gql_error)?;
        require_any_capability(
            &state.pool,
            auth.entity_id,
            &[
                ("manage", crate::auth::Scope::Object(resource_id)),
                ("write", crate::auth::Scope::Object(resource_id)),
                ("write", crate::auth::Scope::Object(group_id)),
                ("manage", scope_for_tenant(resource.tenant_id)),
                ("write", scope_for_tenant(resource.tenant_id)),
            ],
        )
        .await?;
        authz_repo::set_resource_parent_group(&state.pool, resource_id, group_id)
            .await
            .map(Resource::from)
            .map_err(gql_error)
    }

    async fn add_resource_to_object_group(
        &self,
        ctx: &Context<'_>,
        resource_id: ID,
        object_group_id: ID,
    ) -> Result<Resource> {
        self.set_resource_parent_group(ctx, resource_id, object_group_id)
            .await
    }

    async fn clear_resource_parent_group(
        &self,
        ctx: &Context<'_>,
        resource_id: ID,
    ) -> Result<Resource> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let resource_id = parse_id(resource_id, "resourceId")?;
        let resource = authz_repo::get_resource(&state.pool, resource_id)
            .await
            .map_err(gql_error)?;
        require_any_capability(
            &state.pool,
            auth.entity_id,
            &[
                ("manage", crate::auth::Scope::Object(resource_id)),
                ("write", crate::auth::Scope::Object(resource_id)),
                ("manage", scope_for_tenant(resource.tenant_id)),
                ("write", scope_for_tenant(resource.tenant_id)),
            ],
        )
        .await?;
        authz_repo::clear_resource_parent_group(&state.pool, resource_id)
            .await
            .map(Resource::from)
            .map_err(gql_error)
    }

    async fn remove_resource_from_object_group(
        &self,
        ctx: &Context<'_>,
        resource_id: ID,
    ) -> Result<Resource> {
        self.clear_resource_parent_group(ctx, resource_id).await
    }
}
