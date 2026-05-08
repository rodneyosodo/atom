use async_graphql::{Context, Object, Result, ID};
use serde_json::json;

use crate::{
    api_templates::repo as api_template_repo,
    models::api_template::{CreateApiTemplate, ListApiTemplates, UpdateApiTemplate},
    state::AppState,
};

use super::{
    auth::{
        gql_error, require_any_capability, require_auth, require_list_access, require_read_access,
        scope_for_tenant,
    },
    types::{
        parse_api_template_operation_kind, parse_id, parse_optional_api_template_operation_kind,
        parse_optional_api_template_status, parse_optional_id, ApiTemplate, ApiTemplateList,
        CreateApiTemplateInput, GqlApiTemplateStatus, UpdateApiTemplateInput,
    },
};

#[derive(Default)]
pub struct ApiTemplateQuery;

#[Object]
impl ApiTemplateQuery {
    async fn api_templates(
        &self,
        ctx: &Context<'_>,
        tenant_id: Option<ID>,
        status: Option<GqlApiTemplateStatus>,
        tag: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<ApiTemplateList> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(tenant_id, "tenantId")?;
        require_list_access(&state.pool, auth.entity_id, tenant_id).await?;
        let list = api_template_repo::list_api_templates(
            &state.pool,
            ListApiTemplates {
                tenant_id,
                status: parse_optional_api_template_status(status),
                tag,
                limit: limit.map(i64::from).unwrap_or(20),
                offset: offset.map(i64::from).unwrap_or(0),
            },
        )
        .await
        .map_err(gql_error)?;

        Ok(ApiTemplateList {
            items: list.items.into_iter().map(ApiTemplate::from).collect(),
            total: list.total,
        })
    }

    async fn api_template(&self, ctx: &Context<'_>, id: ID) -> Result<ApiTemplate> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let id = parse_id(id, "id")?;
        let template = api_template_repo::get_api_template(&state.pool, id)
            .await
            .map_err(gql_error)?;
        require_read_access(&state.pool, auth.entity_id, template.tenant_id, id).await?;
        Ok(template.into())
    }
}

#[derive(Default)]
pub struct ApiTemplateMutation;

#[Object]
impl ApiTemplateMutation {
    async fn create_api_template(
        &self,
        ctx: &Context<'_>,
        input: CreateApiTemplateInput,
    ) -> Result<ApiTemplate> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let tenant_id = parse_optional_id(input.tenant_id, "tenantId")?;
        require_any_capability(
            &state.pool,
            auth.entity_id,
            &[("manage", scope_for_tenant(tenant_id))],
        )
        .await?;

        let template = api_template_repo::create_api_template(
            &state.pool,
            CreateApiTemplate {
                tenant_id,
                key: input.key,
                name: input.name,
                description: input.description,
                operation_kind: parse_api_template_operation_kind(input.operation_kind),
                graphql: input.graphql,
                variables_schema: input.variables_schema.unwrap_or_else(|| json!({})),
                default_variables: input.default_variables.unwrap_or_else(|| json!({})),
                result_selector: input.result_selector.unwrap_or_else(|| json!({})),
                tags: input.tags.unwrap_or_default(),
                status: parse_optional_api_template_status(input.status),
            },
            Some(auth.entity_id),
        )
        .await
        .map_err(gql_error)?;

        Ok(template.into())
    }

    async fn update_api_template(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateApiTemplateInput,
    ) -> Result<ApiTemplate> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let id = parse_id(id, "id")?;
        let existing = api_template_repo::get_api_template(&state.pool, id)
            .await
            .map_err(gql_error)?;
        require_any_capability(
            &state.pool,
            auth.entity_id,
            &[("manage", scope_for_tenant(existing.tenant_id))],
        )
        .await?;

        let template = api_template_repo::update_api_template(
            &state.pool,
            id,
            UpdateApiTemplate {
                key: input.key,
                name: input.name,
                description: input.description,
                operation_kind: parse_optional_api_template_operation_kind(input.operation_kind),
                graphql: input.graphql,
                variables_schema: input.variables_schema,
                default_variables: input.default_variables,
                result_selector: input.result_selector,
                tags: input.tags,
                status: parse_optional_api_template_status(input.status),
            },
            Some(auth.entity_id),
        )
        .await
        .map_err(gql_error)?;

        Ok(template.into())
    }

    async fn disable_api_template(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let auth = require_auth(ctx)?;
        let state = ctx.data::<AppState>()?;
        let id = parse_id(id, "id")?;
        let existing = api_template_repo::get_api_template(&state.pool, id)
            .await
            .map_err(gql_error)?;
        require_any_capability(
            &state.pool,
            auth.entity_id,
            &[("manage", scope_for_tenant(existing.tenant_id))],
        )
        .await?;

        api_template_repo::disable_api_template(&state.pool, id, Some(auth.entity_id))
            .await
            .map_err(gql_error)?;
        Ok(true)
    }
}
