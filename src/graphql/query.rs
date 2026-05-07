use async_graphql::{Context, Object, Result, ID};

use crate::{
    auth::AuthContext,
    identity::{profile_repo, repo},
    models::{entity::ListEntities, profile::ListProfiles},
    state::AppState,
};

use super::types::{
    parse_id, parse_optional_entity_kind, parse_optional_entity_status, parse_optional_id, Entity,
    EntityList, Profile, ProfileList, ProfileVersion,
};

#[derive(Default)]
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn health(&self, ctx: &Context<'_>) -> Result<&'static str> {
        let _state = ctx.data::<AppState>()?;
        Ok("ok")
    }

    #[allow(clippy::too_many_arguments)]
    async fn profiles(
        &self,
        ctx: &Context<'_>,
        object_kind: Option<String>,
        kind: Option<String>,
        tenant_id: Option<ID>,
        status: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<ProfileList> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let list = profile_repo::list_profiles(
            &state.pool,
            ListProfiles {
                tenant_id: parse_optional_id(tenant_id, "tenantId")?,
                object_kind,
                kind,
                key: None,
                status,
                limit: limit.map(i64::from).unwrap_or(20),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(ProfileList {
            items: list.items.into_iter().map(Profile::from).collect(),
            total: list.total,
        })
    }

    async fn profile(&self, ctx: &Context<'_>, id: ID) -> Result<Profile> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let profile = profile_repo::get_profile(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(profile.into())
    }

    async fn profile_versions(
        &self,
        ctx: &Context<'_>,
        profile_id: ID,
    ) -> Result<Vec<ProfileVersion>> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let versions =
            profile_repo::list_profile_versions(&state.pool, parse_id(profile_id, "profileId")?)
                .await
                .map_err(gql_error)?;
        Ok(versions.into_iter().map(ProfileVersion::from).collect())
    }

    async fn entity(&self, ctx: &Context<'_>, id: ID) -> Result<Entity> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let entity = repo::get_entity(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(entity.into())
    }

    #[allow(clippy::too_many_arguments)]
    async fn entities(
        &self,
        ctx: &Context<'_>,
        kind: Option<String>,
        profile_id: Option<ID>,
        tenant_id: Option<ID>,
        status: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<EntityList> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let list = repo::list_entities(
            &state.pool,
            ListEntities {
                kind: parse_optional_entity_kind(kind)?,
                profile_id: parse_optional_id(profile_id, "profileId")?,
                tenant_id: parse_optional_id(tenant_id, "tenantId")?,
                status: parse_optional_entity_status(status)?,
                limit: limit.map(i64::from).unwrap_or(20),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(EntityList {
            items: list.items.into_iter().map(Entity::from).collect(),
            total: list.total,
        })
    }
}

fn gql_error(err: crate::error::AppError) -> async_graphql::Error {
    async_graphql::Error::new(err.to_string())
}

fn require_auth(ctx: &Context<'_>) -> Result<AuthContext> {
    ctx.data::<AuthContext>()
        .cloned()
        .map_err(|_| async_graphql::Error::new("missing authentication"))
}
