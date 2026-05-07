use async_graphql::{Context, Object, Result, ID};
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{require_capability, AuthContext, Scope},
    identity::{profile_repo, repo},
    models::{
        entity as entity_model,
        profile::{CreateProfile, CreateProfileVersion},
    },
    state::AppState,
};

use super::types::{
    parse_id, parse_optional_entity_kind, parse_optional_id, CreateEntityInput, CreateProfileInput,
    CreateProfileVersionInput, Entity, Profile, ProfileVersion,
};

#[derive(Default)]
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_profile(
        &self,
        ctx: &Context<'_>,
        input: CreateProfileInput,
    ) -> Result<Profile> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(input.tenant_id, "tenantId")?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "manage",
            scope_for_tenant(tenant_id),
        )
        .await
        .map_err(gql_error)?;

        let profile = profile_repo::create_profile(
            &state.pool,
            CreateProfile {
                tenant_id,
                object_kind: input.object_kind,
                kind: input.kind,
                key: input.key,
                display_name: input.display_name,
                description: input.description,
                status: input.status,
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(profile.into())
    }

    async fn create_profile_version(
        &self,
        ctx: &Context<'_>,
        profile_id: ID,
        input: CreateProfileVersionInput,
    ) -> Result<ProfileVersion> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let profile_id = parse_id(profile_id, "profileId")?;
        let profile = profile_repo::get_profile(&state.pool, profile_id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "manage",
            scope_for_tenant(profile.tenant_id),
        )
        .await
        .map_err(gql_error)?;

        let version = profile_repo::create_profile_version(
            &state.pool,
            profile_id,
            CreateProfileVersion {
                version: input.version,
                json_schema: input.json_schema.unwrap_or_else(|| json!({})),
                ui_schema: input.ui_schema.unwrap_or_else(|| json!({})),
                status: input.status,
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(version.into())
    }

    async fn create_entity(&self, ctx: &Context<'_>, input: CreateEntityInput) -> Result<Entity> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(input.tenant_id, "tenantId")?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "manage",
            scope_for_tenant(tenant_id),
        )
        .await
        .map_err(gql_error)?;

        let entity = repo::create_entity(
            &state.pool,
            entity_model::CreateEntity {
                kind: parse_optional_entity_kind(input.kind)?,
                profile_id: parse_optional_id(input.profile_id, "profileId")?,
                profile_version_id: parse_optional_id(
                    input.profile_version_id,
                    "profileVersionId",
                )?,
                name: input.name,
                tenant_id,
                attributes: input.attributes,
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(entity.into())
    }
}

pub fn mutation_root() -> MutationRoot {
    MutationRoot
}

fn gql_error(err: crate::error::AppError) -> async_graphql::Error {
    async_graphql::Error::new(err.to_string())
}

fn require_auth(ctx: &Context<'_>) -> Result<AuthContext> {
    ctx.data::<AuthContext>()
        .cloned()
        .map_err(|_| async_graphql::Error::new("missing authentication"))
}

fn scope_for_tenant(tenant_id: Option<Uuid>) -> Scope {
    match tenant_id {
        Some(tenant_id) => Scope::Tenant(tenant_id),
        None => Scope::Platform,
    }
}
