use async_graphql::{Context, Object, Result, ID};
use sqlx::PgPool;
use std::collections::HashSet;

use uuid::Uuid;

use crate::{
    auth::{has_capability_in_scope, require_capability, AuthContext, Scope},
    authz::repo as authz_repo,
    error::AppError,
    identity::{profile_repo, repo, service},
    models::{
        access::{
            AdminPageQuery, AuditQuery, ExpiringCredentialsQuery, RoleHoldersQuery,
            UnprotectedResourcesQuery,
        },
        capability::ListCapabilities,
        entity::ListEntities,
        group::ListGroups,
        policy::ListPolicies,
        profile::ListProfiles,
        resource::ListResources,
        role::ListRoles,
        tenant::ListTenants,
    },
    state::AppState,
    tenants::repo as tenant_repo,
};

use super::types::{
    parse_id, parse_optional_audit_outcome, parse_optional_credential_kind,
    parse_optional_entity_kind, parse_optional_entity_status, parse_optional_id,
    parse_optional_subject_kind, parse_optional_tenant_status, parse_optional_timestamp, AuditLog,
    AuditLogList, Capability, CapabilityList, Credential, CredentialList, Entity, EntityList,
    Group, GroupList, PolicyBinding, PolicyBindingList, Profile, ProfileList, ProfileVersion,
    Resource, ResourceList, Role, RoleList, Session, Tenant, TenantList,
};

#[derive(Default)]
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn health(&self, ctx: &Context<'_>) -> Result<&'static str> {
        let _state = ctx.data::<AppState>()?;
        Ok("ok")
    }

    async fn session(&self, ctx: &Context<'_>, id: ID) -> Result<Session> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let session = repo::get_session(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(session.into())
    }

    #[allow(clippy::too_many_arguments)]
    async fn tenants(
        &self,
        ctx: &Context<'_>,
        name: Option<String>,
        route: Option<String>,
        status: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<TenantList> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let list = tenant_repo::list_tenants(
            &state.pool,
            ListTenants {
                name,
                route,
                status: parse_optional_tenant_status(status)?,
                limit: limit.map(i64::from).unwrap_or(20),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(TenantList {
            items: list.items.into_iter().map(Tenant::from).collect(),
            total: list.total,
        })
    }

    async fn tenant(&self, ctx: &Context<'_>, id: ID) -> Result<Tenant> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant = tenant_repo::get_tenant(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(tenant.into())
    }

    async fn resources(
        &self,
        ctx: &Context<'_>,
        kind: Option<String>,
        tenant_id: Option<ID>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<ResourceList> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let list = authz_repo::list_resources(
            &state.pool,
            ListResources {
                kind,
                tenant_id: parse_optional_id(tenant_id, "tenantId")?,
                limit: limit.map(i64::from).unwrap_or(20),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(ResourceList {
            items: list.items.into_iter().map(Resource::from).collect(),
            total: list.total,
        })
    }

    async fn resource(&self, ctx: &Context<'_>, id: ID) -> Result<Resource> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let resource = authz_repo::get_resource(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(resource.into())
    }

    async fn groups(
        &self,
        ctx: &Context<'_>,
        tenant_id: Option<ID>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<GroupList> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let list = repo::list_groups(
            &state.pool,
            ListGroups {
                tenant_id: parse_optional_id(tenant_id, "tenantId")?,
                limit: limit.map(i64::from).unwrap_or(20),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(GroupList {
            items: list.items.into_iter().map(Group::from).collect(),
            total: list.total,
        })
    }

    async fn group(&self, ctx: &Context<'_>, id: ID) -> Result<Group> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let group = repo::get_group(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(group.into())
    }

    async fn group_members(&self, ctx: &Context<'_>, group_id: ID) -> Result<Vec<Entity>> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let members = repo::list_group_members(&state.pool, parse_id(group_id, "groupId")?)
            .await
            .map_err(gql_error)?;
        Ok(members.into_iter().map(Entity::from).collect())
    }

    async fn entity_groups(&self, ctx: &Context<'_>, entity_id: ID) -> Result<Vec<ID>> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let group_ids = repo::get_entity_groups(&state.pool, parse_id(entity_id, "entityId")?)
            .await
            .map_err(gql_error)?;
        Ok(group_ids
            .into_iter()
            .map(|group_id| ID(group_id.to_string()))
            .collect())
    }

    async fn credentials(&self, ctx: &Context<'_>, entity_id: ID) -> Result<CredentialList> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let entity_id = parse_id(entity_id, "entityId")?;
        require_credential_management(state, auth.entity_id, entity_id).await?;
        let credentials = service::list_credentials(&state.pool, entity_id)
            .await
            .map_err(gql_error)?;
        let total = credentials.len() as i64;
        Ok(CredentialList {
            items: credentials.into_iter().map(Credential::from).collect(),
            total,
        })
    }

    async fn owned_entities(&self, ctx: &Context<'_>, owner_id: ID) -> Result<Vec<Entity>> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let entities = repo::list_owned(&state.pool, parse_id(owner_id, "ownerId")?)
            .await
            .map_err(gql_error)?;
        Ok(entities.into_iter().map(Entity::from).collect())
    }

    async fn roles(
        &self,
        ctx: &Context<'_>,
        tenant_id: Option<ID>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<RoleList> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let list = authz_repo::list_roles(
            &state.pool,
            ListRoles {
                tenant_id: parse_optional_id(tenant_id, "tenantId")?,
                limit: limit.map(i64::from).unwrap_or(20),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(RoleList {
            items: list.items.into_iter().map(Role::from).collect(),
            total: list.total,
        })
    }

    async fn role(&self, ctx: &Context<'_>, id: ID) -> Result<Role> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let role = authz_repo::get_role(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(role.into())
    }

    async fn role_capabilities(&self, ctx: &Context<'_>, role_id: ID) -> Result<Vec<Capability>> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let capabilities =
            authz_repo::get_role_capabilities(&state.pool, parse_id(role_id, "roleId")?)
                .await
                .map_err(gql_error)?;
        Ok(capabilities.into_iter().map(Capability::from).collect())
    }

    async fn role_holders(&self, ctx: &Context<'_>, role_id: ID) -> Result<Vec<Entity>> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let holders = authz_repo::role_holders(
            &state.pool,
            parse_id(role_id, "roleId")?,
            RoleHoldersQuery {
                tenant_id: None,
                subject_kind: None,
                limit: 200,
                offset: 0,
            },
        )
        .await
        .map_err(gql_error)?;

        let mut seen = HashSet::new();
        let mut entities = Vec::new();
        for holder in holders.items {
            if let Some(entity) = holder.entity {
                if seen.insert(entity.id) {
                    let full = repo::get_entity(&state.pool, entity.id)
                        .await
                        .map_err(gql_error)?;
                    entities.push(Entity::from(full));
                }
            }
            if let Some(group) = holder.group {
                let members = repo::list_group_members(&state.pool, group.id)
                    .await
                    .map_err(gql_error)?;
                for member in members {
                    if seen.insert(member.id) {
                        entities.push(Entity::from(member));
                    }
                }
            }
        }

        Ok(entities)
    }

    async fn capabilities(
        &self,
        ctx: &Context<'_>,
        resource_kind: Option<String>,
    ) -> Result<CapabilityList> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let capabilities =
            authz_repo::list_capabilities(&state.pool, ListCapabilities { resource_kind })
                .await
                .map_err(gql_error)?;
        let total = capabilities.len() as i64;
        Ok(CapabilityList {
            items: capabilities.into_iter().map(Capability::from).collect(),
            total,
        })
    }

    async fn capability(&self, ctx: &Context<'_>, id: ID) -> Result<Capability> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let capability = authz_repo::get_capability(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(capability.into())
    }

    async fn policies(
        &self,
        ctx: &Context<'_>,
        subject_id: Option<ID>,
        subject_kind: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<PolicyBindingList> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let list = authz_repo::list_policies(
            &state.pool,
            ListPolicies {
                subject_id: parse_optional_id(subject_id, "subjectId")?,
                subject_kind: parse_optional_subject_kind(subject_kind)?,
                limit: limit.map(i64::from).unwrap_or(20),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(PolicyBindingList {
            items: list.items.into_iter().map(PolicyBinding::from).collect(),
            total: list.total,
        })
    }

    async fn policy(&self, ctx: &Context<'_>, id: ID) -> Result<PolicyBinding> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let policy = authz_repo::get_policy(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(policy.into())
    }

    #[allow(clippy::too_many_arguments)]
    async fn audit_logs(
        &self,
        ctx: &Context<'_>,
        entity_id: Option<ID>,
        tenant_id: Option<ID>,
        event: Option<String>,
        outcome: Option<String>,
        from: Option<String>,
        to: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<AuditLogList> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(tenant_id, "tenantId")?;
        let params = AuditQuery {
            entity_id: parse_optional_id(entity_id, "entityId")?,
            tenant_id,
            event,
            outcome: parse_optional_audit_outcome(outcome)?,
            from: parse_optional_timestamp(from, "from")?,
            to: parse_optional_timestamp(to, "to")?,
            limit: limit.map(i64::from).unwrap_or(50),
            offset: offset.map(i64::from).unwrap_or(0),
        };
        let allowed_tenant_ids = audit_tenant_filter(&state.pool, &auth, tenant_id)
            .await
            .map_err(gql_error)?;
        let logs = authz_repo::audit_logs(&state.pool, params, allowed_tenant_ids)
            .await
            .map_err(gql_error)?;
        Ok(AuditLogList {
            items: logs.items.into_iter().map(AuditLog::from).collect(),
            total: logs.total,
        })
    }

    async fn entity_audit_logs(&self, ctx: &Context<'_>, entity_id: ID) -> Result<AuditLogList> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let params = AuditQuery {
            entity_id: Some(parse_id(entity_id, "entityId")?),
            tenant_id: None,
            event: None,
            outcome: None,
            from: None,
            to: None,
            limit: 50,
            offset: 0,
        };
        let logs = authz_repo::audit_logs(
            &state.pool,
            params,
            audit_tenant_filter(&state.pool, &auth, None)
                .await
                .map_err(gql_error)?,
        )
        .await
        .map_err(gql_error)?;
        Ok(AuditLogList {
            items: logs.items.into_iter().map(AuditLog::from).collect(),
            total: logs.total,
        })
    }

    async fn orphan_policies(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<PolicyBinding>> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        require_capability(&state.pool, auth.entity_id, "manage", Scope::Platform)
            .await
            .map_err(gql_error)?;
        let policies = authz_repo::orphan_policies(
            &state.pool,
            AdminPageQuery {
                limit: limit.map(i64::from).unwrap_or(50),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;
        Ok(policies
            .items
            .into_iter()
            .map(PolicyBinding::from)
            .collect())
    }

    async fn unprotected_resources(
        &self,
        ctx: &Context<'_>,
        tenant_id: Option<ID>,
        kind: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<Resource>> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        require_capability(&state.pool, auth.entity_id, "manage", Scope::Platform)
            .await
            .map_err(gql_error)?;
        let resources = authz_repo::unprotected_resources(
            &state.pool,
            UnprotectedResourcesQuery {
                tenant_id: parse_optional_id(tenant_id, "tenantId")?,
                kind,
                limit: limit.map(i64::from).unwrap_or(50),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;

        let mut items = Vec::new();
        for resource in resources.items {
            items.push(
                authz_repo::get_resource(&state.pool, resource.id)
                    .await
                    .map(Resource::from)
                    .map_err(gql_error)?,
            );
        }
        Ok(items)
    }

    async fn expiring_credentials(
        &self,
        ctx: &Context<'_>,
        days: Option<i32>,
        entity_id: Option<ID>,
        kind: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<Credential>> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        require_capability(&state.pool, auth.entity_id, "manage", Scope::Platform)
            .await
            .map_err(gql_error)?;
        let credentials = authz_repo::expiring_credentials(
            &state.pool,
            ExpiringCredentialsQuery {
                days: days.map(i64::from).unwrap_or(30),
                entity_id: parse_optional_id(entity_id, "entityId")?,
                kind: parse_optional_credential_kind(kind)?,
                limit: limit.map(i64::from).unwrap_or(50),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;
        Ok(credentials
            .items
            .into_iter()
            .map(Credential::from)
            .collect())
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

async fn require_credential_management(
    state: &AppState,
    actor_id: Uuid,
    target_entity_id: Uuid,
) -> Result<Option<Uuid>> {
    let target = repo::get_entity(&state.pool, target_entity_id)
        .await
        .map_err(gql_error)?;
    if has_capability_in_scope(
        &state.pool,
        actor_id,
        "credential.manage",
        Scope::Object(target_entity_id),
    )
    .await
    .map_err(gql_error)?
    {
        return Ok(target.tenant_id);
    }
    require_capability(
        &state.pool,
        actor_id,
        "credential.manage",
        scope_for_tenant(target.tenant_id),
    )
    .await
    .map_err(gql_error)?;
    Ok(target.tenant_id)
}

async fn audit_tenant_filter(
    pool: &PgPool,
    auth: &AuthContext,
    requested_tenant_id: Option<Uuid>,
) -> std::result::Result<Option<Vec<Uuid>>, AppError> {
    if has_capability_in_scope(pool, auth.entity_id, "audit.read", Scope::Platform).await?
        || has_capability_in_scope(pool, auth.entity_id, "manage", Scope::Platform).await?
    {
        return Ok(None);
    }

    let mut tenant_ids =
        authz_repo::tenant_ids_for_capability(pool, auth.entity_id, "audit.read").await?;
    tenant_ids.sort_unstable();
    tenant_ids.dedup();

    if let Some(requested_tenant_id) = requested_tenant_id {
        if tenant_ids.contains(&requested_tenant_id) {
            return Ok(Some(vec![requested_tenant_id]));
        }
        return Err(AppError::Forbidden);
    }

    if tenant_ids.is_empty() {
        Err(AppError::Forbidden)
    } else {
        Ok(Some(tenant_ids))
    }
}

fn scope_for_tenant(tenant_id: Option<Uuid>) -> Scope {
    match tenant_id {
        Some(tenant_id) => Scope::Tenant(tenant_id),
        None => Scope::Platform,
    }
}
