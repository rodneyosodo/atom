use async_graphql::{Context, Object, Result, ID};
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::{
    auth::{require_capability, Scope},
    authz::repo as authz_repo,
    error::AppError,
    identity::repo as identity_repo,
    models::{
        access::RoleHoldersQuery,
        access::SubjectRoleAssignmentsQuery,
        capability::{CreateCapability, ListCapabilities, UpdateCapability},
        enums::{Effect, GrantKind, ScopeKind, SubjectKind},
        policy::{CreatePolicyBinding, ListPolicies},
        role::{CreateRole, CreateRolePermissionBlock, ListRoles, UpdateRole},
    },
    state::AppState,
};

use super::{
    auth::{gql_error, require_auth, require_policy_read, require_role_read, scope_for_tenant},
    types::{
        parse_effect_or_default, parse_grant_kind, parse_id, parse_optional_id,
        parse_optional_subject_kind, parse_scope_kind, parse_subject_kind, Capability,
        CapabilityList, CreateCapabilityInput, CreatePolicyInput, CreateRoleInput, Entity,
        GqlSubjectKind, PolicyBinding, PolicyBindingList, Role, RoleList,
        SubjectRoleAssignmentList, UpdateCapabilityInput, UpdateRoleInput,
    },
};

#[derive(Default)]
pub struct PolicyQuery;

#[Object]
impl PolicyQuery {
    #[allow(clippy::too_many_arguments)]
    async fn roles(
        &self,
        ctx: &Context<'_>,
        tenant_id: Option<ID>,
        scope_kind: Option<String>,
        scope_ref: Option<String>,
        derived_kind: Option<String>,
        q: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<RoleList> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(tenant_id, "tenantId")?;
        require_role_read(&state.pool, auth.entity_id, tenant_id).await?;
        let list = authz_repo::list_roles(
            &state.pool,
            ListRoles {
                tenant_id,
                scope_kind,
                scope_ref,
                derived_kind,
                q,
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
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let id = parse_id(id, "id")?;
        let role = authz_repo::get_role(&state.pool, id)
            .await
            .map_err(gql_error)?;
        require_role_read(&state.pool, auth.entity_id, role.tenant_id).await?;
        Ok(role.into())
    }

    async fn role_capabilities(&self, ctx: &Context<'_>, role_id: ID) -> Result<Vec<Capability>> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let role_id = parse_id(role_id, "roleId")?;
        let role = authz_repo::get_role(&state.pool, role_id)
            .await
            .map_err(gql_error)?;
        require_role_read(&state.pool, auth.entity_id, role.tenant_id).await?;
        let capabilities = authz_repo::get_role_capabilities(&state.pool, role_id)
            .await
            .map_err(gql_error)?;
        Ok(capabilities.into_iter().map(Capability::from).collect())
    }

    async fn role_holders(&self, ctx: &Context<'_>, role_id: ID) -> Result<Vec<Entity>> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let role_id = parse_id(role_id, "roleId")?;
        let role = authz_repo::get_role(&state.pool, role_id)
            .await
            .map_err(gql_error)?;
        require_role_read(&state.pool, auth.entity_id, role.tenant_id).await?;
        let holders = authz_repo::role_holders(
            &state.pool,
            role_id,
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
                    let full = identity_repo::get_entity(&state.pool, entity.id)
                        .await
                        .map_err(gql_error)?;
                    entities.push(Entity::from(full));
                }
            }
            if let Some(group) = holder.group {
                let members = identity_repo::list_group_members(&state.pool, group.id)
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

    async fn role_policies(
        &self,
        ctx: &Context<'_>,
        role_id: ID,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<PolicyBindingList> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let role_id = parse_id(role_id, "roleId")?;
        let role = authz_repo::get_role(&state.pool, role_id)
            .await
            .map_err(gql_error)?;
        require_role_read(&state.pool, auth.entity_id, role.tenant_id).await?;
        let list = authz_repo::role_policies(
            &state.pool,
            role_id,
            limit.map(i64::from).unwrap_or(20),
            offset.map(i64::from).unwrap_or(0),
        )
        .await
        .map_err(gql_error)?;

        Ok(PolicyBindingList {
            items: list.items.into_iter().map(PolicyBinding::from).collect(),
            total: list.total,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn subject_role_assignments(
        &self,
        ctx: &Context<'_>,
        subject_kind: GqlSubjectKind,
        subject_id: ID,
        tenant_id: Option<ID>,
        derived_kind: Option<String>,
        q: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<SubjectRoleAssignmentList> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(tenant_id, "tenantId")?;
        require_role_read(&state.pool, auth.entity_id, tenant_id).await?;
        let list = authz_repo::subject_role_assignments(
            &state.pool,
            SubjectRoleAssignmentsQuery {
                tenant_id,
                subject_kind: parse_subject_kind(subject_kind),
                subject_id: parse_id(subject_id, "subjectId")?,
                derived_kind,
                q,
                limit: limit.map(i64::from).unwrap_or(20),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(SubjectRoleAssignmentList {
            items: list.items.into_iter().map(Into::into).collect(),
            total: list.total,
        })
    }

    async fn capabilities(
        &self,
        ctx: &Context<'_>,
        resource_kind: Option<String>,
        tenant_id: Option<ID>,
        #[graphql(default = 50)] limit: i64,
        #[graphql(default = 0)] offset: i64,
    ) -> Result<CapabilityList> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(tenant_id, "tenantId")?;
        require_policy_read(&state.pool, auth.entity_id, tenant_id).await?;
        let list = authz_repo::list_capabilities(
            &state.pool,
            ListCapabilities {
                resource_kind,
                limit,
                offset,
            },
        )
        .await
        .map_err(gql_error)?;
        Ok(CapabilityList {
            items: list.items.into_iter().map(Capability::from).collect(),
            total: list.total,
        })
    }

    async fn capability(&self, ctx: &Context<'_>, id: ID) -> Result<Capability> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        require_policy_read(&state.pool, auth.entity_id, None).await?;
        let capability = authz_repo::get_capability(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        Ok(capability.into())
    }

    async fn policies(
        &self,
        ctx: &Context<'_>,
        tenant_id: Option<ID>,
        subject_id: Option<ID>,
        subject_kind: Option<GqlSubjectKind>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<PolicyBindingList> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(tenant_id, "tenantId")?;
        require_policy_read(&state.pool, auth.entity_id, tenant_id).await?;
        let list = authz_repo::list_policies(
            &state.pool,
            ListPolicies {
                tenant_id,
                subject_id: parse_optional_id(subject_id, "subjectId")?,
                subject_kind: parse_optional_subject_kind(subject_kind),
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
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let policy = authz_repo::get_policy(&state.pool, parse_id(id, "id")?)
            .await
            .map_err(gql_error)?;
        require_policy_read(&state.pool, auth.entity_id, policy.tenant_id).await?;
        Ok(policy.into())
    }
}

#[derive(Default)]
pub struct PolicyMutation;

#[Object]
impl PolicyMutation {
    async fn create_role(&self, ctx: &Context<'_>, input: CreateRoleInput) -> Result<Role> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(input.tenant_id.clone(), "tenantId")?;
        let capability_ids = input
            .capability_ids
            .unwrap_or_default()
            .into_iter()
            .map(|id| parse_id(id, "capabilityId"))
            .collect::<Result<Vec<_>>>()?;
        let child_role_ids = input
            .child_role_ids
            .unwrap_or_default()
            .into_iter()
            .map(|id| parse_id(id, "childRoleId"))
            .collect::<Result<Vec<_>>>()?;
        let member_entity_ids = input
            .member_entity_ids
            .unwrap_or_default()
            .into_iter()
            .map(|id| parse_id(id, "memberEntityId"))
            .collect::<Result<Vec<_>>>()?;
        let permission_blocks = parse_permission_blocks(input.permission_blocks)?;
        if !permission_blocks.is_empty()
            && (!capability_ids.is_empty() || !child_role_ids.is_empty())
        {
            return Err(gql_error(AppError::bad_request(
                "permissionBlocks cannot be combined with capabilityIds or childRoleIds",
            )));
        }
        let policy_scope_kind = role_policy_scope_kind(tenant_id, input.scope_kind.as_deref())
            .map_err(|err| gql_error(AppError::bad_request(err)))?;
        let policy_scope_ref = input
            .scope_ref
            .clone()
            .or_else(|| tenant_id.map(|tenant_id| tenant_id.to_string()));
        require_capability(
            &state.pool,
            auth.entity_id,
            "role.manage",
            scope_for_tenant(tenant_id),
        )
        .await
        .map_err(gql_error)?;
        for member_id in &member_entity_ids {
            let req = CreatePolicyBinding {
                tenant_id,
                subject_kind: SubjectKind::Entity,
                subject_id: *member_id,
                grant_kind: GrantKind::Role,
                grant_id: Uuid::nil(),
                scope_kind: policy_scope_kind.clone(),
                scope_ref: policy_scope_ref.clone(),
                effect: Effect::Allow,
                conditions: serde_json::json!({}),
            };
            req.validate()
                .map_err(|err| gql_error(AppError::bad_request(err)))?;
            if let Some(policy_tenant_id) = req.tenant_id {
                validate_tenant_policy_subject(
                    &state.pool,
                    req.subject_kind.clone(),
                    req.subject_id,
                    policy_tenant_id,
                )
                .await
                .map_err(gql_error)?;
            }
            validate_tenant_policy_scope(&state.pool, &req)
                .await
                .map_err(gql_error)?;
        }
        let create_req = CreateRole {
            name: input.name,
            tenant_id,
            description: input.description,
            scope_kind: input.scope_kind,
            scope_ref: input.scope_ref,
        };
        let role = if permission_blocks.is_empty() {
            authz_repo::create_role_with_assignments(
                &state.pool,
                create_req,
                &capability_ids,
                &child_role_ids,
                &member_entity_ids,
            )
            .await
            .map_err(gql_error)?
        } else {
            authz_repo::create_role_with_permission_blocks(
                &state.pool,
                create_req,
                &permission_blocks,
                &member_entity_ids,
            )
            .await
            .map_err(gql_error)?
        };
        Ok(role.into())
    }

    async fn update_role(&self, ctx: &Context<'_>, id: ID, input: UpdateRoleInput) -> Result<Role> {
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
        let updated = authz_repo::update_role(
            &state.pool,
            id,
            UpdateRole {
                name: input.name,
                description: input.description,
            },
        )
        .await
        .map_err(gql_error)?;
        Ok(updated.into())
    }

    async fn add_composite_role_child(
        &self,
        ctx: &Context<'_>,
        parent_role_id: ID,
        child_role_id: ID,
    ) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let parent_role_id = parse_id(parent_role_id, "parentRoleId")?;
        let child_role_id = parse_id(child_role_id, "childRoleId")?;
        let parent = authz_repo::get_role(&state.pool, parent_role_id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "role.manage",
            scope_for_tenant(parent.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        authz_repo::add_composite_role_child(&state.pool, parent_role_id, child_role_id)
            .await
            .map_err(gql_error)?;
        Ok(true)
    }

    async fn remove_composite_role_child(
        &self,
        ctx: &Context<'_>,
        parent_role_id: ID,
        child_role_id: ID,
    ) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let parent_role_id = parse_id(parent_role_id, "parentRoleId")?;
        let child_role_id = parse_id(child_role_id, "childRoleId")?;
        let parent = authz_repo::get_role(&state.pool, parent_role_id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "role.manage",
            scope_for_tenant(parent.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        authz_repo::remove_composite_role_child(&state.pool, parent_role_id, child_role_id)
            .await
            .map_err(gql_error)?;
        Ok(true)
    }

    async fn replace_composite_role_children(
        &self,
        ctx: &Context<'_>,
        parent_role_id: ID,
        child_role_ids: Vec<ID>,
    ) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let parent_role_id = parse_id(parent_role_id, "parentRoleId")?;
        let child_role_ids = child_role_ids
            .into_iter()
            .map(|id| parse_id(id, "childRoleId"))
            .collect::<Result<Vec<_>>>()?;
        let parent = authz_repo::get_role(&state.pool, parent_role_id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "role.manage",
            scope_for_tenant(parent.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        authz_repo::replace_composite_role_children(&state.pool, parent_role_id, &child_role_ids)
            .await
            .map_err(gql_error)?;
        Ok(true)
    }

    async fn replace_role_permission_blocks(
        &self,
        ctx: &Context<'_>,
        role_id: ID,
        permission_blocks: Vec<super::types::CreateRolePermissionBlockInput>,
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
        let blocks = parse_permission_blocks(Some(permission_blocks))?;
        authz_repo::replace_role_permission_blocks(&state.pool, role_id, &blocks)
            .await
            .map_err(gql_error)?;
        Ok(true)
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
            CreateCapability {
                name: input.name,
                resource_kind: input.resource_kind,
                description: input.description,
            },
        )
        .await
        .map_err(gql_error)?;
        Ok(capability.into())
    }

    async fn update_capability(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateCapabilityInput,
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
        let updated = authz_repo::update_capability(
            &state.pool,
            parse_id(id, "id")?,
            UpdateCapability {
                name: input.name,
                resource_kind: input.resource_kind,
                description: input.description,
            },
        )
        .await
        .map_err(gql_error)?;
        Ok(updated.into())
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

    async fn assign_role_to_entity(
        &self,
        ctx: &Context<'_>,
        role_id: ID,
        entity_id: ID,
    ) -> Result<PolicyBinding> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let role_id = parse_id(role_id, "roleId")?;
        let entity_id = parse_id(entity_id, "entityId")?;
        let role = authz_repo::get_role(&state.pool, role_id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "policy.manage",
            scope_for_tenant(role.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        ensure_tenant_membership_for_assignment(&state.pool, role.tenant_id, entity_id)
            .await
            .map_err(gql_error)?;
        let policy = authz_repo::create_policy(
            &state.pool,
            role_assignment_policy(role.tenant_id, SubjectKind::Entity, entity_id, role_id),
        )
        .await
        .map_err(gql_error)?;
        Ok(policy.into())
    }

    async fn assign_role_to_principal_group(
        &self,
        ctx: &Context<'_>,
        role_id: ID,
        principal_group_id: ID,
    ) -> Result<PolicyBinding> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let role_id = parse_id(role_id, "roleId")?;
        let group_id = parse_id(principal_group_id, "principalGroupId")?;
        let role = authz_repo::get_role(&state.pool, role_id)
            .await
            .map_err(gql_error)?;
        require_capability(
            &state.pool,
            auth.entity_id,
            "policy.manage",
            scope_for_tenant(role.tenant_id),
        )
        .await
        .map_err(gql_error)?;
        validate_principal_group_for_assignment(&state.pool, role.tenant_id, group_id)
            .await
            .map_err(gql_error)?;
        let policy = authz_repo::create_policy(
            &state.pool,
            role_assignment_policy(role.tenant_id, SubjectKind::Group, group_id, role_id),
        )
        .await
        .map_err(gql_error)?;
        Ok(policy.into())
    }

    async fn remove_assignment(&self, ctx: &Context<'_>, assignment_id: ID) -> Result<bool> {
        self.delete_policy(ctx, assignment_id).await
    }

    async fn remove_role_assignment(&self, ctx: &Context<'_>, assignment_id: ID) -> Result<bool> {
        self.remove_assignment(ctx, assignment_id).await
    }
}

fn create_policy_binding(input: CreatePolicyInput) -> Result<CreatePolicyBinding> {
    Ok(CreatePolicyBinding {
        tenant_id: parse_optional_id(input.tenant_id, "tenantId")?,
        subject_kind: parse_subject_kind(input.subject_kind),
        subject_id: parse_id(input.subject_id, "subjectId")?,
        grant_kind: parse_grant_kind(input.grant_kind),
        grant_id: parse_id(input.grant_id, "grantId")?,
        scope_kind: parse_scope_kind(input.scope_kind),
        scope_ref: input.scope_ref,
        effect: parse_effect_or_default(input.effect),
        conditions: input.conditions.unwrap_or_else(|| serde_json::json!({})),
    })
}

fn parse_permission_blocks(
    blocks: Option<Vec<super::types::CreateRolePermissionBlockInput>>,
) -> Result<Vec<CreateRolePermissionBlock>> {
    blocks
        .unwrap_or_default()
        .into_iter()
        .map(|block| {
            Ok(CreateRolePermissionBlock {
                applies_to: block.applies_to,
                object_id: parse_optional_id(block.object_id, "objectId")?,
                object_kind: block.object_kind,
                object_type: block.object_type,
                tenant_id: parse_optional_id(block.tenant_id, "tenantId")?,
                group_id: parse_optional_id(block.group_id, "groupId")?,
                capability_ids: block
                    .capability_ids
                    .into_iter()
                    .map(|id| parse_id(id, "capabilityId"))
                    .collect::<Result<Vec<_>>>()?,
            })
        })
        .collect()
}

fn role_assignment_policy(
    tenant_id: Option<Uuid>,
    subject_kind: SubjectKind,
    subject_id: Uuid,
    role_id: Uuid,
) -> CreatePolicyBinding {
    CreatePolicyBinding {
        tenant_id,
        subject_kind,
        subject_id,
        grant_kind: GrantKind::Role,
        grant_id: role_id,
        scope_kind: if tenant_id.is_some() {
            ScopeKind::Tenant
        } else {
            ScopeKind::Platform
        },
        scope_ref: tenant_id.map(|tenant_id| tenant_id.to_string()),
        effect: Effect::Allow,
        conditions: serde_json::json!({}),
    }
}

async fn ensure_tenant_membership_for_assignment(
    pool: &PgPool,
    tenant_id: Option<Uuid>,
    entity_id: Uuid,
) -> std::result::Result<(), AppError> {
    let Some(tenant_id) = tenant_id else {
        return Ok(());
    };
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM entities WHERE id = $1)")
        .bind(entity_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
    if !exists {
        return Err(AppError::not_found(format!("entity {entity_id} not found")));
    }
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
    .map_err(AppError::Database)?;
    Ok(())
}

async fn validate_principal_group_for_assignment(
    pool: &PgPool,
    tenant_id: Option<Uuid>,
    group_id: Uuid,
) -> std::result::Result<(), AppError> {
    let row = sqlx::query_as::<_, (Option<Uuid>, String)>(
        "SELECT tenant_id, group_type FROM groups WHERE id = $1",
    )
    .bind(group_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;
    let Some((group_tenant_id, group_type)) = row else {
        return Err(AppError::not_found(format!(
            "principal group {group_id} not found"
        )));
    };
    if group_type != "principal" {
        return Err(AppError::bad_request(
            "role assignment subject group must be a Principal Group",
        ));
    }
    if tenant_id.is_some() && group_tenant_id != tenant_id {
        return Err(AppError::bad_request(
            "tenant role can only be assigned to a Principal Group in the same tenant",
        ));
    }
    Ok(())
}

fn role_policy_scope_kind(
    tenant_id: Option<Uuid>,
    scope_kind: Option<&str>,
) -> std::result::Result<ScopeKind, String> {
    let value = scope_kind.unwrap_or(if tenant_id.is_some() {
        "tenant"
    } else {
        "platform"
    });
    serde_json::from_value(serde_json::Value::String(value.to_string())).map_err(|_| {
        format!(
            "invalid scope_kind '{value}' (expected one of platform, tenant, object_kind, object_type, object, group_object_type, group_tree_object_type, group_child_kind, group_descendant_kind)"
        )
    })
}

async fn validate_tenant_owned_policy(
    pool: &PgPool,
    req: &CreatePolicyBinding,
) -> std::result::Result<(), AppError> {
    let Some(policy_tenant_id) = req.tenant_id else {
        return Ok(());
    };

    validate_tenant_policy_subject(
        pool,
        req.subject_kind.clone(),
        req.subject_id,
        policy_tenant_id,
    )
    .await?;
    validate_tenant_policy_grant(pool, req.grant_kind.clone(), req.grant_id, policy_tenant_id)
        .await?;

    validate_tenant_policy_scope(pool, req).await
}

async fn validate_tenant_policy_scope(
    pool: &PgPool,
    req: &CreatePolicyBinding,
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
        ScopeKind::GroupObjectType
        | ScopeKind::GroupTreeObjectType
        | ScopeKind::GroupChildKind
        | ScopeKind::GroupDescendantKind => {
            let group_id = parse_group_scope_ref_group_id(req.scope_ref.as_deref())?;
            let tenant_id =
                sqlx::query_scalar::<_, Option<Uuid>>("SELECT tenant_id FROM groups WHERE id = $1")
                    .bind(group_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(AppError::Database)?;
            match tenant_id {
                Some(Some(group_tenant_id)) if group_tenant_id == policy_tenant_id => Ok(()),
                Some(Some(_)) => Err(AppError::bad_request(
                    "tenant-owned policy cannot reference a group in another tenant",
                )),
                Some(None) => Err(AppError::bad_request(
                    "tenant-owned policy cannot reference a platform group",
                )),
                None => Err(AppError::bad_request(
                    "tenant-owned policy cannot reference an unknown group",
                )),
            }
        }
    }
}

fn parse_group_scope_ref_group_id(scope_ref: Option<&str>) -> std::result::Result<Uuid, AppError> {
    let scope_ref =
        scope_ref.ok_or_else(|| AppError::bad_request("group scope requires scope_ref"))?;
    let (group_id, _) = scope_ref
        .split_once(':')
        .ok_or_else(|| AppError::bad_request("group scope_ref must start with group UUID"))?;
    group_id
        .parse::<Uuid>()
        .map_err(|_| AppError::bad_request("group scope_ref has invalid group UUID"))
}

async fn validate_tenant_policy_subject(
    pool: &PgPool,
    subject_kind: SubjectKind,
    subject_id: Uuid,
    policy_tenant_id: Uuid,
) -> std::result::Result<(), AppError> {
    let tenant_id = match subject_kind {
        SubjectKind::Entity => {
            sqlx::query_scalar::<_, Option<Uuid>>("SELECT tenant_id FROM entities WHERE id = $1")
                .bind(subject_id)
                .fetch_optional(pool)
                .await
                .map_err(AppError::Database)?
        }
        SubjectKind::Group => {
            sqlx::query_scalar::<_, Option<Uuid>>("SELECT tenant_id FROM groups WHERE id = $1")
                .bind(subject_id)
                .fetch_optional(pool)
                .await
                .map_err(AppError::Database)?
        }
    };

    match tenant_id {
        Some(Some(subject_tenant_id)) if subject_tenant_id == policy_tenant_id => Ok(()),
        Some(Some(_)) => Err(AppError::bad_request(
            "tenant-owned policy cannot target a subject in another tenant",
        )),
        Some(None) => Err(AppError::bad_request(
            "tenant-owned policy cannot target a platform subject",
        )),
        None => Err(AppError::bad_request(
            "tenant-owned policy cannot target an unknown subject",
        )),
    }
}

async fn validate_tenant_policy_grant(
    pool: &PgPool,
    grant_kind: GrantKind,
    grant_id: Uuid,
    policy_tenant_id: Uuid,
) -> std::result::Result<(), AppError> {
    match grant_kind {
        GrantKind::Capability => Ok(()),
        GrantKind::Role => {
            let tenant_id =
                sqlx::query_scalar::<_, Option<Uuid>>("SELECT tenant_id FROM roles WHERE id = $1")
                    .bind(grant_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(AppError::Database)?;

            match tenant_id {
                Some(Some(role_tenant_id)) if role_tenant_id == policy_tenant_id => Ok(()),
                Some(Some(_)) => Err(AppError::bad_request(
                    "tenant-owned policy cannot assign a role from another tenant",
                )),
                Some(None) => Err(AppError::bad_request(
                    "tenant-owned policy cannot assign a platform role",
                )),
                None => Err(AppError::bad_request(
                    "tenant-owned policy cannot assign an unknown role",
                )),
            }
        }
    }
}
