use async_graphql::{Context, Object, Result, ID};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    audit,
    auth::{has_capability_in_scope, require_capability, AuthContext, Scope},
    authz::{compat::translate_legacy_scope, engine, repo as authz_repo},
    error::AppError,
    identity::{profile_repo, repo, service},
    models::{
        capability as capability_model, entity as entity_model,
        enums::{AuditOutcome, ScopeKind, TenantStatus},
        group as group_model, policy as policy_model,
        profile::{CreateProfile, CreateProfileVersion},
        resource as resource_model, role as role_model, tenant as tenant_model,
        token as token_model,
    },
    state::AppState,
    tenants::repo as tenant_repo,
};

use super::types::{
    parse_effect_or_default, parse_grant_kind, parse_id, parse_optional_entity_kind,
    parse_optional_id, parse_optional_timestamp, parse_scope_kind, parse_subject_kind,
    ApiKeyResponse, AuthzCheckInput, AuthzExplainResponse, AuthzResponse, Capability,
    CreateApiKeyInput, CreateCapabilityInput, CreateEntityInput, CreateGroupInput,
    CreatePolicyInput, CreateProfileInput, CreateProfileVersionInput, CreateResourceInput,
    CreateRoleInput, CreateTenantInput, Entity, Group, LoginInput, LoginResponse, Ownership,
    PolicyBinding, Profile, ProfileVersion, Resource, Role, Tenant, UpdateResourceInput,
    UpdateTenantInput,
};

#[derive(Default)]
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn login(&self, ctx: &Context<'_>, input: LoginInput) -> Result<LoginResponse> {
        if input.kind != "password" {
            return Err(async_graphql::Error::new(format!(
                "unsupported credential kind: {}",
                input.kind
            )));
        }

        let state = ctx.data::<AppState>()?;
        let keys = state.keys.read().await;
        let response = service::login_password(
            &state.pool,
            &state.config,
            &keys.primary,
            &input.identifier,
            &input.secret,
        )
        .await
        .map_err(gql_error)?;

        Ok(response.into())
    }

    async fn logout(&self, ctx: &Context<'_>) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;

        if let Some(session_id) = auth.session_id {
            repo::revoke_session(&state.pool, session_id)
                .await
                .map_err(gql_error)?;
        }
        audit::write(
            &state.pool,
            Some(auth.entity_id),
            auth.tenant_id,
            "auth.logout",
            AuditOutcome::Allow,
            serde_json::json!({}),
        )
        .await;

        Ok(true)
    }

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

    async fn create_tenant(&self, ctx: &Context<'_>, input: CreateTenantInput) -> Result<Tenant> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "tenant.manage",
            Scope::Platform,
        )
        .await
        .map_err(gql_error)?;

        let tenant = tenant_repo::create_tenant(
            &state.pool,
            tenant_model::CreateTenant {
                name: input.name,
                route: input.route,
                tags: input.tags.unwrap_or_default(),
                attributes: input.attributes.unwrap_or(serde_json::Value::Null),
            },
            Some(auth.entity_id),
        )
        .await
        .map_err(gql_error)?;

        Ok(tenant.into())
    }

    async fn update_tenant(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateTenantInput,
    ) -> Result<Tenant> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "tenant.manage",
            Scope::Platform,
        )
        .await
        .map_err(gql_error)?;

        let tenant = tenant_repo::update_tenant(
            &state.pool,
            parse_id(id, "id")?,
            tenant_model::UpdateTenant {
                name: input.name,
                route: input.route,
                tags: input.tags,
                attributes: input.attributes,
            },
            Some(auth.entity_id),
        )
        .await
        .map_err(gql_error)?;

        Ok(tenant.into())
    }

    async fn delete_tenant(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "tenant.manage",
            Scope::Platform,
        )
        .await
        .map_err(gql_error)?;

        tenant_repo::change_tenant_status(
            &state.pool,
            parse_id(id, "id")?,
            TenantStatus::Deleted,
            Some(auth.entity_id),
        )
        .await
        .map_err(gql_error)?;

        Ok(true)
    }

    async fn enable_tenant(&self, ctx: &Context<'_>, id: ID) -> Result<Tenant> {
        change_tenant_status(ctx, id, TenantStatus::Active).await
    }

    async fn disable_tenant(&self, ctx: &Context<'_>, id: ID) -> Result<Tenant> {
        change_tenant_status(ctx, id, TenantStatus::Inactive).await
    }

    async fn freeze_tenant(&self, ctx: &Context<'_>, id: ID) -> Result<Tenant> {
        change_tenant_status(ctx, id, TenantStatus::Frozen).await
    }

    async fn create_resource(
        &self,
        ctx: &Context<'_>,
        input: CreateResourceInput,
    ) -> Result<Resource> {
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

        let resource = authz_repo::create_resource(
            &state.pool,
            resource_model::CreateResource {
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
        require_capability(
            &state.pool,
            auth.entity_id,
            "manage",
            scope_for_tenant(existing.tenant_id),
        )
        .await
        .map_err(gql_error)?;

        let resource = authz_repo::update_resource(
            &state.pool,
            id,
            resource_model::UpdateResource {
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
        require_capability(
            &state.pool,
            auth.entity_id,
            "manage",
            scope_for_tenant(existing.tenant_id),
        )
        .await
        .map_err(gql_error)?;

        authz_repo::delete_resource(&state.pool, id)
            .await
            .map_err(gql_error)?;

        Ok(true)
    }

    async fn create_group(&self, ctx: &Context<'_>, input: CreateGroupInput) -> Result<Group> {
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

        let group = repo::create_group(
            &state.pool,
            group_model::CreateGroup {
                name: input.name,
                tenant_id,
                description: input.description,
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(group.into())
    }

    async fn delete_group(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let id = parse_id(id, "id")?;
        let group = repo::get_group(&state.pool, id).await.map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "manage",
            scope_for_tenant(group.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        repo::delete_group(&state.pool, id)
            .await
            .map_err(gql_error)?;
        Ok(true)
    }

    async fn add_group_member(
        &self,
        ctx: &Context<'_>,
        group_id: ID,
        entity_id: ID,
    ) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let group_id = parse_id(group_id, "groupId")?;
        let entity_id = parse_id(entity_id, "entityId")?;
        let group = repo::get_group(&state.pool, group_id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "manage",
            scope_for_tenant(group.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        repo::add_group_member(&state.pool, group_id, entity_id)
            .await
            .map_err(gql_error)?;
        Ok(true)
    }

    async fn remove_group_member(
        &self,
        ctx: &Context<'_>,
        group_id: ID,
        entity_id: ID,
    ) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let group_id = parse_id(group_id, "groupId")?;
        let entity_id = parse_id(entity_id, "entityId")?;
        let group = repo::get_group(&state.pool, group_id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "manage",
            scope_for_tenant(group.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        repo::remove_group_member(&state.pool, group_id, entity_id)
            .await
            .map_err(gql_error)?;
        Ok(true)
    }

    async fn create_password(
        &self,
        ctx: &Context<'_>,
        entity_id: ID,
        password: String,
    ) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let entity_id = parse_id(entity_id, "entityId")?;
        let tenant_id = require_credential_management(state, auth.entity_id, entity_id).await?;
        service::create_password(&state.pool, entity_id, &password)
            .await
            .map_err(gql_error)?;
        audit::write(
            &state.pool,
            Some(entity_id),
            tenant_id,
            "credential.create",
            AuditOutcome::Allow,
            serde_json::json!({"kind": "password"}),
        )
        .await;
        Ok(true)
    }

    async fn create_api_key(
        &self,
        ctx: &Context<'_>,
        entity_id: ID,
        input: CreateApiKeyInput,
    ) -> Result<ApiKeyResponse> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let entity_id = parse_id(entity_id, "entityId")?;
        let tenant_id = require_credential_management(state, auth.entity_id, entity_id).await?;
        let response = service::create_api_key(
            &state.pool,
            entity_id,
            token_model::CreateApiKey {
                expires_at: parse_optional_timestamp(input.expires_at, "expiresAt")?,
                description: input.description,
            },
        )
        .await
        .map_err(gql_error)?;
        audit::write(
            &state.pool,
            Some(entity_id),
            tenant_id,
            "credential.create",
            AuditOutcome::Allow,
            serde_json::json!({"kind": "api_key", "credential_id": response.credential_id}),
        )
        .await;
        Ok(response.into())
    }

    async fn revoke_credential(
        &self,
        ctx: &Context<'_>,
        entity_id: ID,
        credential_id: ID,
    ) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let entity_id = parse_id(entity_id, "entityId")?;
        let credential_id = parse_id(credential_id, "credentialId")?;
        let tenant_id = require_credential_management(state, auth.entity_id, entity_id).await?;
        service::revoke_credential(&state.pool, entity_id, credential_id)
            .await
            .map_err(gql_error)?;
        audit::write(
            &state.pool,
            Some(auth.entity_id),
            tenant_id,
            "credential.revoke",
            AuditOutcome::Allow,
            serde_json::json!({"entity_id": entity_id, "credential_id": credential_id}),
        )
        .await;
        Ok(true)
    }

    async fn add_ownership(
        &self,
        ctx: &Context<'_>,
        owner_id: ID,
        owned_id: ID,
        relation: Option<String>,
    ) -> Result<Ownership> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let ownership = repo::create_ownership(
            &state.pool,
            parse_id(owner_id, "ownerId")?,
            parse_id(owned_id, "ownedId")?,
            relation.unwrap_or_else(|| "owner".to_string()),
        )
        .await
        .map_err(gql_error)?;
        Ok(ownership.into())
    }

    async fn remove_ownership(
        &self,
        ctx: &Context<'_>,
        owner_id: ID,
        owned_id: ID,
    ) -> Result<bool> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        repo::delete_ownership(
            &state.pool,
            parse_id(owner_id, "ownerId")?,
            parse_id(owned_id, "ownedId")?,
        )
        .await
        .map_err(gql_error)?;
        Ok(true)
    }

    async fn create_role(&self, ctx: &Context<'_>, input: CreateRoleInput) -> Result<Role> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(input.tenant_id, "tenantId")?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "role.manage",
            scope_for_tenant(tenant_id),
        )
        .await
        .map_err(gql_error)?;
        let role = authz_repo::create_role(
            &state.pool,
            role_model::CreateRole {
                name: input.name,
                tenant_id,
                description: input.description,
            },
        )
        .await
        .map_err(gql_error)?;
        Ok(role.into())
    }

    async fn delete_role(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let id = parse_id(id, "id")?;
        let role = authz_repo::get_role(&state.pool, id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "role.manage",
            scope_for_tenant(role.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        authz_repo::delete_role(&state.pool, id)
            .await
            .map_err(gql_error)?;
        Ok(true)
    }

    async fn add_role_capability(
        &self,
        ctx: &Context<'_>,
        role_id: ID,
        capability_id: ID,
    ) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let role_id = parse_id(role_id, "roleId")?;
        let role = authz_repo::get_role(&state.pool, role_id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "role.manage",
            scope_for_tenant(role.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        authz_repo::add_role_capability(
            &state.pool,
            role_id,
            parse_id(capability_id, "capabilityId")?,
        )
        .await
        .map_err(gql_error)?;
        Ok(true)
    }

    async fn remove_role_capability(
        &self,
        ctx: &Context<'_>,
        role_id: ID,
        capability_id: ID,
    ) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let role_id = parse_id(role_id, "roleId")?;
        let role = authz_repo::get_role(&state.pool, role_id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "role.manage",
            scope_for_tenant(role.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        authz_repo::remove_role_capability(
            &state.pool,
            role_id,
            parse_id(capability_id, "capabilityId")?,
        )
        .await
        .map_err(gql_error)?;
        Ok(true)
    }

    async fn create_capability(
        &self,
        ctx: &Context<'_>,
        input: CreateCapabilityInput,
    ) -> Result<Capability> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "policy.manage",
            Scope::Platform,
        )
        .await
        .map_err(gql_error)?;
        let capability = authz_repo::create_capability(
            &state.pool,
            capability_model::CreateCapability {
                name: input.name,
                resource_kind: input.resource_kind,
                description: input.description,
            },
        )
        .await
        .map_err(gql_error)?;
        Ok(capability.into())
    }

    async fn delete_capability(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "policy.manage",
            Scope::Platform,
        )
        .await
        .map_err(gql_error)?;
        authz_repo::delete_capability(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(true)
    }

    async fn create_policy(
        &self,
        ctx: &Context<'_>,
        input: CreatePolicyInput,
    ) -> Result<PolicyBinding> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let req = create_policy_binding(input)?;
        req.validate()
            .map_err(|err| gql_error(AppError::bad_request(err)))?;
        validate_tenant_owned_policy(&state.pool, &req)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "policy.manage",
            scope_for_tenant(req.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        let policy = authz_repo::create_policy(&state.pool, req)
            .await
            .map_err(gql_error)?;
        Ok(policy.into())
    }

    async fn delete_policy(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let id = parse_id(id, "id")?;
        let policy = authz_repo::get_policy(&state.pool, id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "policy.manage",
            scope_for_tenant(policy.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        authz_repo::delete_policy(&state.pool, id)
            .await
            .map_err(gql_error)?;
        Ok(true)
    }

    async fn authz_check(
        &self,
        ctx: &Context<'_>,
        input: AuthzCheckInput,
    ) -> Result<AuthzResponse> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let req = authz_request(input)?;
        let tenant_id = authz_request_tenant_id(&state.pool, &req)
            .await
            .map_err(gql_error)?;
        let response = engine::evaluate(&state.pool, &req)
            .await
            .map_err(gql_error)?;
        audit_authz_check(&state.pool, &req, &response, tenant_id).await;
        Ok(response.into())
    }

    async fn authz_explain(
        &self,
        ctx: &Context<'_>,
        input: AuthzCheckInput,
    ) -> Result<AuthzExplainResponse> {
        require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let req = authz_request(input)?;
        let tenant_id = authz_request_tenant_id(&state.pool, &req)
            .await
            .map_err(gql_error)?;
        let response = engine::explain(&state.pool, &req)
            .await
            .map_err(gql_error)?;
        audit_authz_explain(&state.pool, &req, &response, tenant_id).await;
        Ok(response.into())
    }

    async fn authz_bulk_check(
        &self,
        ctx: &Context<'_>,
        input: Vec<AuthzCheckInput>,
    ) -> Result<Vec<AuthzResponse>> {
        require_auth(ctx)?;
        if input.len() > 20 {
            return Err(gql_error(AppError::bad_request(
                "input must contain at most 20 items",
            )));
        }
        let state = ctx.data::<AppState>()?;
        let mut responses = Vec::with_capacity(input.len());
        for item in input {
            let req = authz_request(item)?;
            let tenant_id = authz_request_tenant_id(&state.pool, &req)
                .await
                .map_err(gql_error)?;
            let response = engine::evaluate(&state.pool, &req)
                .await
                .map_err(gql_error)?;
            audit_authz_check(&state.pool, &req, &response, tenant_id).await;
            responses.push(response.into());
        }
        Ok(responses)
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

fn create_policy_binding(input: CreatePolicyInput) -> Result<policy_model::CreatePolicyBinding> {
    let (scope_kind, scope_ref) = translate_legacy_scope(&input.scope_kind, input.scope_ref);
    Ok(policy_model::CreatePolicyBinding {
        tenant_id: parse_optional_id(input.tenant_id, "tenantId")?,
        subject_kind: parse_subject_kind(&input.subject_kind)?,
        subject_id: parse_id(input.subject_id, "subjectId")?,
        grant_kind: parse_grant_kind(&input.grant_kind)?,
        grant_id: parse_id(input.grant_id, "grantId")?,
        scope_kind: parse_scope_kind(&scope_kind)?,
        scope_ref,
        effect: parse_effect_or_default(input.effect)?,
        conditions: input.conditions.unwrap_or_else(|| json!({})),
    })
}

fn authz_request(input: AuthzCheckInput) -> Result<policy_model::AuthzRequest> {
    Ok(policy_model::AuthzRequest {
        subject_id: parse_id(input.subject_id, "subjectId")?,
        action: input.action,
        resource_id: parse_optional_id(input.resource_id, "resourceId")?,
        object_kind: input.object_kind,
        object_id: parse_optional_id(input.object_id, "objectId")?,
        context: input.context.unwrap_or_else(|| json!({})),
    })
}

async fn validate_tenant_owned_policy(
    pool: &PgPool,
    req: &policy_model::CreatePolicyBinding,
) -> std::result::Result<(), AppError> {
    let Some(policy_tenant_id) = req.tenant_id else {
        return Ok(());
    };

    match req.scope_kind {
        ScopeKind::Platform => Err(AppError::bad_request(
            "tenant-owned policy cannot use platform scope",
        )),
        ScopeKind::Tenant => {
            let Some(scope_ref) = req.scope_ref.as_deref() else {
                return Err(AppError::bad_request(
                    "tenant policy scope_ref must match tenant_id",
                ));
            };
            let scope_tenant_id = scope_ref
                .parse::<Uuid>()
                .map_err(|_| AppError::bad_request("tenant scope_ref must be a UUID"))?;
            if scope_tenant_id == policy_tenant_id {
                Ok(())
            } else {
                Err(AppError::bad_request(
                    "tenant-owned policy cannot reference another tenant",
                ))
            }
        }
        ScopeKind::ObjectKind | ScopeKind::ObjectType => Ok(()),
        ScopeKind::Object => {
            let scope_ref = req
                .scope_ref
                .as_deref()
                .ok_or_else(|| AppError::bad_request("object scope requires scope_ref"))?;
            let object_id = scope_ref
                .parse::<Uuid>()
                .map_err(|_| AppError::bad_request("object scope_ref must be a UUID"))?;
            match authz_repo::object_tenant_id_by_id(pool, object_id).await? {
                Some(Some(object_tenant_id)) if object_tenant_id == policy_tenant_id => Ok(()),
                Some(Some(_)) => Err(AppError::bad_request(
                    "tenant-owned policy cannot reference an object in another tenant",
                )),
                Some(None) => Err(AppError::bad_request(
                    "tenant-owned policy cannot reference a platform object",
                )),
                None => Err(AppError::bad_request(
                    "tenant-owned policy cannot reference an unknown object",
                )),
            }
        }
    }
}

async fn authz_request_tenant_id(
    pool: &PgPool,
    req: &policy_model::AuthzRequest,
) -> std::result::Result<Option<Uuid>, AppError> {
    if req.object_kind.as_deref() == Some("tenant") {
        return Ok(req.object_id);
    }

    if let Some(resource_id) = req.resource_id {
        return sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT tenant_id FROM resources WHERE id = $1",
        )
        .bind(resource_id)
        .fetch_optional(pool)
        .await
        .map(|value| value.flatten())
        .map_err(crate::error::db_err);
    }

    match (req.object_kind.as_deref(), req.object_id) {
        (Some("resource"), Some(id)) => {
            sqlx::query_scalar::<_, Option<Uuid>>("SELECT tenant_id FROM resources WHERE id = $1")
                .bind(id)
                .fetch_optional(pool)
                .await
                .map(|value| value.flatten())
                .map_err(crate::error::db_err)
        }
        (Some("entity"), Some(id)) => {
            sqlx::query_scalar::<_, Option<Uuid>>("SELECT tenant_id FROM entities WHERE id = $1")
                .bind(id)
                .fetch_optional(pool)
                .await
                .map(|value| value.flatten())
                .map_err(crate::error::db_err)
        }
        _ => Ok(None),
    }
}

async fn audit_authz_check(
    pool: &PgPool,
    req: &policy_model::AuthzRequest,
    response: &policy_model::AuthzResponse,
    tenant_id: Option<Uuid>,
) {
    let mut details = json!({
        "action": req.action,
        "resource_id": req.resource_id,
        "object_kind": req.object_kind,
        "object_id": req.object_id,
        "reason": response.reason,
    });
    if let Some(extra) = response
        .details
        .as_ref()
        .and_then(|value| value.as_object())
    {
        let map = details.as_object_mut().expect("json object");
        for (key, value) in extra {
            map.insert(key.clone(), value.clone());
        }
    }

    audit::write(
        pool,
        Some(req.subject_id),
        tenant_id,
        "authz.check",
        if response.allowed {
            AuditOutcome::Allow
        } else {
            AuditOutcome::Deny
        },
        details,
    )
    .await;
}

async fn audit_authz_explain(
    pool: &PgPool,
    req: &policy_model::AuthzRequest,
    response: &crate::models::access::AuthzExplainResponse,
    tenant_id: Option<Uuid>,
) {
    let mut details = json!({
        "action": req.action,
        "resource_id": req.resource_id,
        "object_kind": req.object_kind,
        "object_id": req.object_id,
        "reason": response.reason,
    });
    if response.reason.starts_with("tenant is ") {
        if let Some(state_word) = response.reason.strip_prefix("tenant is ") {
            details
                .as_object_mut()
                .expect("json object")
                .insert("tenant_status".into(), json!(state_word));
        }
    }

    audit::write(
        pool,
        Some(req.subject_id),
        tenant_id,
        "authz.explain",
        if response.allowed {
            AuditOutcome::Allow
        } else {
            AuditOutcome::Deny
        },
        details,
    )
    .await;
}

async fn change_tenant_status(ctx: &Context<'_>, id: ID, status: TenantStatus) -> Result<Tenant> {
    let auth = require_auth(ctx)?;
    let state = ctx.data::<AppState>()?;
    require_capability(
        &state.pool,
        auth.entity_id,
        "tenant.manage",
        Scope::Platform,
    )
    .await
    .map_err(gql_error)?;

    let tenant = tenant_repo::change_tenant_status(
        &state.pool,
        parse_id(id, "id")?,
        status,
        Some(auth.entity_id),
    )
    .await
    .map_err(gql_error)?;

    Ok(tenant.into())
}
