use async_graphql::{InputObject, Object, ID};
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    identity::service as identity_service,
    models::{
        access as access_model, capability as capability_model, entity as entity_model,
        enums::{
            AuditOutcome, CredentialKind, CredentialStatus, Effect, EntityKind, EntityStatus,
            GrantKind, ScopeKind, SubjectKind, TenantStatus,
        },
        group as group_model, profile as profile_model, resource as resource_model,
        role as role_model, session as session_model, tenant as tenant_model, token as token_model,
    },
};

pub struct Profile(pub profile_model::Profile);

#[Object]
impl Profile {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn tenant_id(&self) -> Option<ID> {
        self.0.tenant_id.map(id)
    }

    async fn object_kind(&self) -> &str {
        &self.0.object_kind
    }

    async fn kind(&self) -> &str {
        &self.0.kind
    }

    async fn key(&self) -> &str {
        &self.0.key
    }

    async fn display_name(&self) -> &str {
        &self.0.display_name
    }

    async fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }

    async fn status(&self) -> &str {
        &self.0.status
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }

    async fn updated_at(&self) -> String {
        timestamp(self.0.updated_at)
    }
}

pub struct ProfileVersion(pub profile_model::ProfileVersion);

#[Object]
impl ProfileVersion {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn profile_id(&self) -> ID {
        id(self.0.profile_id)
    }

    async fn version(&self) -> i32 {
        self.0.version
    }

    async fn json_schema(&self) -> &Value {
        &self.0.json_schema
    }

    async fn ui_schema(&self) -> &Value {
        &self.0.ui_schema
    }

    async fn status(&self) -> &str {
        &self.0.status
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }
}

pub struct Entity(pub entity_model::Entity);

#[Object]
impl Entity {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn kind(&self) -> &'static str {
        entity_kind_as_str(&self.0.kind)
    }

    async fn profile_id(&self) -> Option<ID> {
        self.0.profile_id.map(id)
    }

    async fn profile_version_id(&self) -> Option<ID> {
        self.0.profile_version_id.map(id)
    }

    async fn name(&self) -> &str {
        &self.0.name
    }

    async fn tenant_id(&self) -> Option<ID> {
        self.0.tenant_id.map(id)
    }

    async fn status(&self) -> &'static str {
        entity_status_as_str(&self.0.status)
    }

    async fn attributes(&self) -> &Value {
        &self.0.attributes
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }

    async fn updated_at(&self) -> String {
        timestamp(self.0.updated_at)
    }
}

pub struct Session(pub session_model::Session);

#[Object]
impl Session {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn entity_id(&self) -> ID {
        id(self.0.entity_id)
    }

    async fn expires_at(&self) -> String {
        timestamp(self.0.expires_at)
    }

    async fn revoked_at(&self) -> Option<String> {
        self.0.revoked_at.map(timestamp)
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }
}

pub struct LoginResponse(pub session_model::LoginResponse);

#[Object]
impl LoginResponse {
    async fn token(&self) -> &str {
        &self.0.token
    }

    async fn entity_id(&self) -> ID {
        id(self.0.entity_id)
    }

    async fn session_id(&self) -> ID {
        id(self.0.session_id)
    }

    async fn expires_at(&self) -> String {
        timestamp(self.0.expires_at)
    }
}

pub struct Tenant(pub tenant_model::Tenant);

#[Object]
impl Tenant {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn name(&self) -> &str {
        &self.0.name
    }

    async fn route(&self) -> Option<&str> {
        self.0.route.as_deref()
    }

    async fn status(&self) -> &'static str {
        tenant_status_as_str(&self.0.status)
    }

    async fn tags(&self) -> &[String] {
        &self.0.tags
    }

    async fn attributes(&self) -> &Value {
        &self.0.attributes
    }

    async fn created_by(&self) -> Option<ID> {
        self.0.created_by.map(id)
    }

    async fn updated_by(&self) -> Option<ID> {
        self.0.updated_by.map(id)
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }

    async fn updated_at(&self) -> String {
        timestamp(self.0.updated_at)
    }
}

pub struct Resource(pub resource_model::Resource);

#[Object]
impl Resource {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn kind(&self) -> &str {
        &self.0.kind
    }

    async fn name(&self) -> Option<&str> {
        self.0.name.as_deref()
    }

    async fn tenant_id(&self) -> Option<ID> {
        self.0.tenant_id.map(id)
    }

    async fn owner_id(&self) -> Option<ID> {
        self.0.owner_id.map(id)
    }

    async fn attributes(&self) -> &Value {
        &self.0.attributes
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }

    async fn updated_at(&self) -> String {
        timestamp(self.0.updated_at)
    }
}

pub struct Group(pub group_model::Group);

#[Object]
impl Group {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn name(&self) -> &str {
        &self.0.name
    }

    async fn tenant_id(&self) -> Option<ID> {
        self.0.tenant_id.map(id)
    }

    async fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }

    async fn updated_at(&self) -> String {
        timestamp(self.0.updated_at)
    }
}

pub struct Credential(pub CredentialData);

pub struct CredentialData {
    pub id: Uuid,
    pub entity_id: Option<Uuid>,
    pub kind: CredentialKind,
    pub identifier: Option<String>,
    pub status: CredentialStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[Object]
impl Credential {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn entity_id(&self) -> Option<ID> {
        self.0.entity_id.map(id)
    }

    async fn kind(&self) -> &'static str {
        credential_kind_as_str(&self.0.kind)
    }

    async fn identifier(&self) -> Option<&str> {
        self.0.identifier.as_deref()
    }

    async fn status(&self) -> &'static str {
        credential_status_as_str(&self.0.status)
    }

    async fn expires_at(&self) -> Option<String> {
        self.0.expires_at.map(timestamp)
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }
}

pub struct ApiKeyResponse(pub token_model::ApiKeyResponse);

#[Object]
impl ApiKeyResponse {
    async fn credential_id(&self) -> ID {
        id(self.0.credential_id)
    }

    async fn key(&self) -> &str {
        &self.0.key
    }

    async fn expires_at(&self) -> Option<String> {
        self.0.expires_at.map(timestamp)
    }
}

pub struct Ownership(pub entity_model::Ownership);

#[Object]
impl Ownership {
    async fn owner_id(&self) -> ID {
        id(self.0.owner_id)
    }

    async fn owned_id(&self) -> ID {
        id(self.0.owned_id)
    }

    async fn relation(&self) -> &str {
        &self.0.relation
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }
}

pub struct Role(pub role_model::Role);

#[Object]
impl Role {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn name(&self) -> &str {
        &self.0.name
    }

    async fn tenant_id(&self) -> Option<ID> {
        self.0.tenant_id.map(id)
    }

    async fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }
}

pub struct Capability(pub capability_model::Capability);

#[Object]
impl Capability {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn name(&self) -> &str {
        &self.0.name
    }

    async fn resource_kind(&self) -> Option<&str> {
        self.0.resource_kind.as_deref()
    }

    async fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }
}

pub struct PolicyBinding(pub PolicyBindingData);

pub struct PolicyBindingData {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub subject_kind: SubjectKind,
    pub subject_id: Uuid,
    pub grant_kind: GrantKind,
    pub grant_id: Uuid,
    pub scope_kind: ScopeKind,
    pub scope_ref: Option<String>,
    pub effect: Effect,
    pub conditions: Value,
    pub created_at: DateTime<Utc>,
}

#[Object]
impl PolicyBinding {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn tenant_id(&self) -> Option<ID> {
        self.0.tenant_id.map(id)
    }

    async fn subject_kind(&self) -> &'static str {
        subject_kind_as_str(&self.0.subject_kind)
    }

    async fn subject_id(&self) -> ID {
        id(self.0.subject_id)
    }

    async fn grant_kind(&self) -> &'static str {
        grant_kind_as_str(&self.0.grant_kind)
    }

    async fn grant_id(&self) -> ID {
        id(self.0.grant_id)
    }

    async fn scope_kind(&self) -> &'static str {
        scope_kind_as_str(&self.0.scope_kind)
    }

    async fn scope_ref(&self) -> Option<&str> {
        self.0.scope_ref.as_deref()
    }

    async fn effect(&self) -> &'static str {
        effect_as_str(&self.0.effect)
    }

    async fn conditions(&self) -> &Value {
        &self.0.conditions
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }
}

pub struct AuthzResponse(pub crate::models::policy::AuthzResponse);

#[Object]
impl AuthzResponse {
    async fn allowed(&self) -> bool {
        self.0.allowed
    }

    async fn reason(&self) -> &str {
        &self.0.reason
    }

    async fn details(&self) -> Option<&Value> {
        self.0.details.as_ref()
    }
}

pub struct AuthzExplainResponse {
    pub allowed: bool,
    pub reason: String,
    pub subject: Option<Value>,
    pub resource: Option<Value>,
    pub capability: Option<Value>,
    pub matched_binding: Option<Value>,
    pub evaluated_bindings: Value,
}

#[Object]
impl AuthzExplainResponse {
    async fn allowed(&self) -> bool {
        self.allowed
    }

    async fn reason(&self) -> &str {
        &self.reason
    }

    async fn subject(&self) -> Option<&Value> {
        self.subject.as_ref()
    }

    async fn resource(&self) -> Option<&Value> {
        self.resource.as_ref()
    }

    async fn capability(&self) -> Option<&Value> {
        self.capability.as_ref()
    }

    async fn matched_binding(&self) -> Option<&Value> {
        self.matched_binding.as_ref()
    }

    async fn evaluated_bindings(&self) -> &Value {
        &self.evaluated_bindings
    }
}

pub struct AuditLog(pub access_model::AuditLogItem);

#[Object]
impl AuditLog {
    async fn id(&self) -> ID {
        id(self.0.id)
    }

    async fn entity_id(&self) -> Option<ID> {
        self.0.entity_id.map(id)
    }

    async fn tenant_id(&self) -> Option<ID> {
        self.0.tenant_id.map(id)
    }

    async fn event(&self) -> &str {
        &self.0.event
    }

    async fn outcome(&self) -> &'static str {
        audit_outcome_as_str(&self.0.outcome)
    }

    async fn details(&self) -> &Value {
        &self.0.details
    }

    async fn created_at(&self) -> String {
        timestamp(self.0.created_at)
    }
}

#[derive(InputObject)]
pub struct LoginInput {
    pub identifier: String,
    pub secret: String,
    #[graphql(default = "password")]
    pub kind: String,
}

#[derive(InputObject)]
pub struct CreateProfileInput {
    pub tenant_id: Option<ID>,
    pub object_kind: String,
    pub kind: String,
    pub key: String,
    pub display_name: String,
    pub description: Option<String>,
    pub status: Option<String>,
}

#[derive(InputObject)]
pub struct CreateProfileVersionInput {
    pub version: i32,
    pub json_schema: Option<Value>,
    pub ui_schema: Option<Value>,
    pub status: Option<String>,
}

#[derive(InputObject)]
pub struct CreateEntityInput {
    pub profile_id: Option<ID>,
    pub profile_version_id: Option<ID>,
    pub kind: Option<String>,
    pub name: String,
    pub tenant_id: Option<ID>,
    pub attributes: Value,
}

#[derive(InputObject)]
pub struct CreateTenantInput {
    pub name: String,
    pub route: Option<String>,
    pub tags: Option<Vec<String>>,
    pub attributes: Option<Value>,
}

#[derive(InputObject)]
pub struct UpdateTenantInput {
    pub name: Option<String>,
    pub route: Option<String>,
    pub tags: Option<Vec<String>>,
    pub attributes: Option<Value>,
}

#[derive(InputObject)]
pub struct CreateResourceInput {
    pub kind: String,
    pub name: Option<String>,
    pub tenant_id: Option<ID>,
    pub owner_id: Option<ID>,
    pub attributes: Option<Value>,
}

#[derive(InputObject)]
pub struct UpdateResourceInput {
    pub name: Option<String>,
    pub attributes: Option<Value>,
}

#[derive(InputObject)]
pub struct CreateGroupInput {
    pub name: String,
    pub tenant_id: Option<ID>,
    pub description: Option<String>,
}

#[derive(InputObject)]
pub struct CreateApiKeyInput {
    pub expires_at: Option<String>,
    pub description: Option<String>,
}

#[derive(InputObject)]
pub struct CreateRoleInput {
    pub name: String,
    pub tenant_id: Option<ID>,
    pub description: Option<String>,
}

#[derive(InputObject)]
pub struct CreateCapabilityInput {
    pub name: String,
    pub resource_kind: Option<String>,
    pub description: Option<String>,
}

#[derive(InputObject)]
pub struct CreatePolicyInput {
    pub tenant_id: Option<ID>,
    pub subject_kind: String,
    pub subject_id: ID,
    pub grant_kind: String,
    pub grant_id: ID,
    pub scope_kind: String,
    pub scope_ref: Option<String>,
    pub effect: Option<String>,
    pub conditions: Option<Value>,
}

#[derive(InputObject)]
pub struct AuthzCheckInput {
    pub subject_id: ID,
    pub action: String,
    pub resource_id: Option<ID>,
    pub object_kind: Option<String>,
    pub object_id: Option<ID>,
    pub context: Option<Value>,
}

#[derive(Default)]
pub struct ProfileList {
    pub items: Vec<Profile>,
    pub total: i64,
}

#[Object]
impl ProfileList {
    async fn items(&self) -> &[Profile] {
        &self.items
    }

    async fn total(&self) -> i64 {
        self.total
    }
}

#[derive(Default)]
pub struct EntityList {
    pub items: Vec<Entity>,
    pub total: i64,
}

#[Object]
impl EntityList {
    async fn items(&self) -> &[Entity] {
        &self.items
    }

    async fn total(&self) -> i64 {
        self.total
    }
}

#[derive(Default)]
pub struct TenantList {
    pub items: Vec<Tenant>,
    pub total: i64,
}

#[Object]
impl TenantList {
    async fn items(&self) -> &[Tenant] {
        &self.items
    }

    async fn total(&self) -> i64 {
        self.total
    }
}

#[derive(Default)]
pub struct ResourceList {
    pub items: Vec<Resource>,
    pub total: i64,
}

#[Object]
impl ResourceList {
    async fn items(&self) -> &[Resource] {
        &self.items
    }

    async fn total(&self) -> i64 {
        self.total
    }
}

#[derive(Default)]
pub struct GroupList {
    pub items: Vec<Group>,
    pub total: i64,
}

#[Object]
impl GroupList {
    async fn items(&self) -> &[Group] {
        &self.items
    }

    async fn total(&self) -> i64 {
        self.total
    }
}

#[derive(Default)]
pub struct CredentialList {
    pub items: Vec<Credential>,
    pub total: i64,
}

#[derive(Default)]
pub struct RoleList {
    pub items: Vec<Role>,
    pub total: i64,
}

#[Object]
impl RoleList {
    async fn items(&self) -> &[Role] {
        &self.items
    }

    async fn total(&self) -> i64 {
        self.total
    }
}

#[derive(Default)]
pub struct CapabilityList {
    pub items: Vec<Capability>,
    pub total: i64,
}

#[Object]
impl CapabilityList {
    async fn items(&self) -> &[Capability] {
        &self.items
    }

    async fn total(&self) -> i64 {
        self.total
    }
}

#[derive(Default)]
pub struct PolicyBindingList {
    pub items: Vec<PolicyBinding>,
    pub total: i64,
}

#[Object]
impl PolicyBindingList {
    async fn items(&self) -> &[PolicyBinding] {
        &self.items
    }

    async fn total(&self) -> i64 {
        self.total
    }
}

#[derive(Default)]
pub struct AuditLogList {
    pub items: Vec<AuditLog>,
    pub total: i64,
}

#[Object]
impl AuditLogList {
    async fn items(&self) -> &[AuditLog] {
        &self.items
    }

    async fn total(&self) -> i64 {
        self.total
    }
}

#[Object]
impl CredentialList {
    async fn items(&self) -> &[Credential] {
        &self.items
    }

    async fn total(&self) -> i64 {
        self.total
    }
}

impl From<profile_model::Profile> for Profile {
    fn from(profile: profile_model::Profile) -> Self {
        Profile(profile)
    }
}

impl From<profile_model::ProfileVersion> for ProfileVersion {
    fn from(version: profile_model::ProfileVersion) -> Self {
        ProfileVersion(version)
    }
}

impl From<entity_model::Entity> for Entity {
    fn from(entity: entity_model::Entity) -> Self {
        Entity(entity)
    }
}

impl From<session_model::Session> for Session {
    fn from(session: session_model::Session) -> Self {
        Session(session)
    }
}

impl From<session_model::LoginResponse> for LoginResponse {
    fn from(response: session_model::LoginResponse) -> Self {
        LoginResponse(response)
    }
}

impl From<tenant_model::Tenant> for Tenant {
    fn from(tenant: tenant_model::Tenant) -> Self {
        Tenant(tenant)
    }
}

impl From<resource_model::Resource> for Resource {
    fn from(resource: resource_model::Resource) -> Self {
        Resource(resource)
    }
}

impl From<group_model::Group> for Group {
    fn from(group: group_model::Group) -> Self {
        Group(group)
    }
}

impl From<identity_service::CredentialSummary> for Credential {
    fn from(credential: identity_service::CredentialSummary) -> Self {
        Credential(CredentialData {
            id: credential.id,
            entity_id: None,
            kind: credential.kind,
            identifier: credential.identifier,
            status: credential.status,
            expires_at: credential.expires_at,
            created_at: credential.created_at,
        })
    }
}

impl From<token_model::ApiKeyResponse> for ApiKeyResponse {
    fn from(response: token_model::ApiKeyResponse) -> Self {
        ApiKeyResponse(response)
    }
}

impl From<entity_model::Ownership> for Ownership {
    fn from(ownership: entity_model::Ownership) -> Self {
        Ownership(ownership)
    }
}

impl From<role_model::Role> for Role {
    fn from(role: role_model::Role) -> Self {
        Role(role)
    }
}

impl From<capability_model::Capability> for Capability {
    fn from(capability: capability_model::Capability) -> Self {
        Capability(capability)
    }
}

impl From<crate::models::policy::PolicyBinding> for PolicyBinding {
    fn from(policy: crate::models::policy::PolicyBinding) -> Self {
        PolicyBinding(PolicyBindingData {
            id: policy.id,
            tenant_id: policy.tenant_id,
            subject_kind: policy.subject_kind,
            subject_id: policy.subject_id,
            grant_kind: policy.grant_kind,
            grant_id: policy.grant_id,
            scope_kind: policy.scope_kind,
            scope_ref: policy.scope_ref,
            effect: policy.effect,
            conditions: policy.conditions,
            created_at: policy.created_at,
        })
    }
}

impl From<access_model::OrphanPolicyItem> for PolicyBinding {
    fn from(policy: access_model::OrphanPolicyItem) -> Self {
        PolicyBinding(PolicyBindingData {
            id: policy.id,
            tenant_id: None,
            subject_kind: policy.subject_kind,
            subject_id: policy.subject_id,
            grant_kind: policy.grant_kind,
            grant_id: policy.grant_id,
            scope_kind: policy.scope_kind,
            scope_ref: policy.scope_ref,
            effect: policy.effect,
            conditions: policy.conditions,
            created_at: policy.created_at,
        })
    }
}

impl From<crate::models::policy::AuthzResponse> for AuthzResponse {
    fn from(response: crate::models::policy::AuthzResponse) -> Self {
        AuthzResponse(response)
    }
}

impl From<access_model::AuthzExplainResponse> for AuthzExplainResponse {
    fn from(response: access_model::AuthzExplainResponse) -> Self {
        AuthzExplainResponse {
            allowed: response.allowed,
            reason: response.reason,
            subject: response
                .subject
                .map(|value| serde_json::to_value(value).expect("subject serializes")),
            resource: response
                .resource
                .map(|value| serde_json::to_value(value).expect("resource serializes")),
            capability: response
                .capability
                .map(|value| serde_json::to_value(value).expect("capability serializes")),
            matched_binding: response
                .matched_binding
                .map(|value| serde_json::to_value(value).expect("binding serializes")),
            evaluated_bindings: serde_json::to_value(response.evaluated_bindings)
                .expect("bindings serialize"),
        }
    }
}

impl From<access_model::AuditLogItem> for AuditLog {
    fn from(log: access_model::AuditLogItem) -> Self {
        AuditLog(log)
    }
}

impl From<access_model::ExpiringCredentialItem> for Credential {
    fn from(credential: access_model::ExpiringCredentialItem) -> Self {
        Credential(CredentialData {
            id: credential.id,
            entity_id: Some(credential.entity_id),
            kind: credential.kind,
            identifier: None,
            status: credential.status,
            expires_at: Some(credential.expires_at),
            created_at: credential.created_at,
        })
    }
}

pub fn parse_id(value: ID, name: &str) -> async_graphql::Result<Uuid> {
    value
        .as_str()
        .parse()
        .map_err(|_| async_graphql::Error::new(format!("{name} must be a UUID")))
}

pub fn parse_optional_id(value: Option<ID>, name: &str) -> async_graphql::Result<Option<Uuid>> {
    value.map(|id| parse_id(id, name)).transpose()
}

pub fn parse_optional_timestamp(
    value: Option<String>,
    name: &str,
) -> async_graphql::Result<Option<DateTime<Utc>>> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|_| async_graphql::Error::new(format!("{name} must be RFC3339")))
        })
        .transpose()
}

pub fn parse_optional_entity_kind(
    value: Option<String>,
) -> async_graphql::Result<Option<EntityKind>> {
    value.as_deref().map(parse_entity_kind).transpose()
}

pub fn parse_entity_kind(value: &str) -> async_graphql::Result<EntityKind> {
    match value {
        "human" => Ok(EntityKind::Human),
        "device" => Ok(EntityKind::Device),
        "service" => Ok(EntityKind::Service),
        "workload" => Ok(EntityKind::Workload),
        "application" => Ok(EntityKind::Application),
        other => Err(async_graphql::Error::new(format!(
            "invalid entity kind '{other}'"
        ))),
    }
}

pub fn parse_optional_entity_status(
    value: Option<String>,
) -> async_graphql::Result<Option<EntityStatus>> {
    value.as_deref().map(parse_entity_status).transpose()
}

pub fn parse_optional_subject_kind(
    value: Option<String>,
) -> async_graphql::Result<Option<SubjectKind>> {
    value.as_deref().map(parse_subject_kind).transpose()
}

pub fn parse_subject_kind(value: &str) -> async_graphql::Result<SubjectKind> {
    match value {
        "entity" => Ok(SubjectKind::Entity),
        "group" => Ok(SubjectKind::Group),
        other => Err(async_graphql::Error::new(format!(
            "invalid subject kind '{other}'"
        ))),
    }
}

pub fn parse_grant_kind(value: &str) -> async_graphql::Result<GrantKind> {
    match value {
        "capability" => Ok(GrantKind::Capability),
        "role" => Ok(GrantKind::Role),
        other => Err(async_graphql::Error::new(format!(
            "invalid grant kind '{other}'"
        ))),
    }
}

pub fn parse_effect_or_default(value: Option<String>) -> async_graphql::Result<Effect> {
    value
        .as_deref()
        .map(parse_effect)
        .transpose()
        .map(Option::unwrap_or_default)
}

pub fn parse_optional_audit_outcome(
    value: Option<String>,
) -> async_graphql::Result<Option<AuditOutcome>> {
    value.as_deref().map(parse_audit_outcome).transpose()
}

pub fn parse_optional_credential_kind(
    value: Option<String>,
) -> async_graphql::Result<Option<CredentialKind>> {
    value.as_deref().map(parse_credential_kind).transpose()
}

pub fn parse_scope_kind(value: &str) -> async_graphql::Result<ScopeKind> {
    match value {
        "platform" => Ok(ScopeKind::Platform),
        "tenant" => Ok(ScopeKind::Tenant),
        "object_kind" => Ok(ScopeKind::ObjectKind),
        "object_type" => Ok(ScopeKind::ObjectType),
        "object" => Ok(ScopeKind::Object),
        other => Err(async_graphql::Error::new(format!(
            "invalid scope kind '{other}'"
        ))),
    }
}

fn parse_effect(value: &str) -> async_graphql::Result<Effect> {
    match value {
        "allow" => Ok(Effect::Allow),
        "deny" => Ok(Effect::Deny),
        other => Err(async_graphql::Error::new(format!(
            "invalid effect '{other}'"
        ))),
    }
}

fn parse_audit_outcome(value: &str) -> async_graphql::Result<AuditOutcome> {
    match value {
        "allow" => Ok(AuditOutcome::Allow),
        "deny" => Ok(AuditOutcome::Deny),
        "error" => Ok(AuditOutcome::Error),
        other => Err(async_graphql::Error::new(format!(
            "invalid audit outcome '{other}'"
        ))),
    }
}

fn parse_credential_kind(value: &str) -> async_graphql::Result<CredentialKind> {
    match value {
        "password" => Ok(CredentialKind::Password),
        "api_key" => Ok(CredentialKind::ApiKey),
        "certificate" => Ok(CredentialKind::Certificate),
        other => Err(async_graphql::Error::new(format!(
            "invalid credential kind '{other}'"
        ))),
    }
}

fn parse_entity_status(value: &str) -> async_graphql::Result<EntityStatus> {
    match value {
        "active" => Ok(EntityStatus::Active),
        "inactive" => Ok(EntityStatus::Inactive),
        "suspended" => Ok(EntityStatus::Suspended),
        other => Err(async_graphql::Error::new(format!(
            "invalid entity status '{other}'"
        ))),
    }
}

fn entity_kind_as_str(kind: &EntityKind) -> &'static str {
    match kind {
        EntityKind::Human => "human",
        EntityKind::Device => "device",
        EntityKind::Service => "service",
        EntityKind::Workload => "workload",
        EntityKind::Application => "application",
    }
}

fn entity_status_as_str(status: &EntityStatus) -> &'static str {
    match status {
        EntityStatus::Active => "active",
        EntityStatus::Inactive => "inactive",
        EntityStatus::Suspended => "suspended",
    }
}

pub fn parse_optional_tenant_status(
    value: Option<String>,
) -> async_graphql::Result<Option<TenantStatus>> {
    value.as_deref().map(parse_tenant_status).transpose()
}

fn parse_tenant_status(value: &str) -> async_graphql::Result<TenantStatus> {
    match value {
        "active" => Ok(TenantStatus::Active),
        "inactive" => Ok(TenantStatus::Inactive),
        "frozen" => Ok(TenantStatus::Frozen),
        "deleted" => Ok(TenantStatus::Deleted),
        other => Err(async_graphql::Error::new(format!(
            "invalid tenant status '{other}'"
        ))),
    }
}

fn tenant_status_as_str(status: &TenantStatus) -> &'static str {
    match status {
        TenantStatus::Active => "active",
        TenantStatus::Inactive => "inactive",
        TenantStatus::Frozen => "frozen",
        TenantStatus::Deleted => "deleted",
    }
}

fn subject_kind_as_str(kind: &SubjectKind) -> &'static str {
    match kind {
        SubjectKind::Entity => "entity",
        SubjectKind::Group => "group",
    }
}

fn grant_kind_as_str(kind: &GrantKind) -> &'static str {
    match kind {
        GrantKind::Capability => "capability",
        GrantKind::Role => "role",
    }
}

fn scope_kind_as_str(kind: &ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Platform => "platform",
        ScopeKind::Tenant => "tenant",
        ScopeKind::ObjectKind => "object_kind",
        ScopeKind::ObjectType => "object_type",
        ScopeKind::Object => "object",
    }
}

fn effect_as_str(effect: &Effect) -> &'static str {
    match effect {
        Effect::Allow => "allow",
        Effect::Deny => "deny",
    }
}

fn audit_outcome_as_str(outcome: &AuditOutcome) -> &'static str {
    match outcome {
        AuditOutcome::Allow => "allow",
        AuditOutcome::Deny => "deny",
        AuditOutcome::Error => "error",
    }
}

fn credential_kind_as_str(kind: &CredentialKind) -> &'static str {
    match kind {
        CredentialKind::Password => "password",
        CredentialKind::ApiKey => "api_key",
        CredentialKind::Certificate => "certificate",
    }
}

fn credential_status_as_str(status: &CredentialStatus) -> &'static str {
    match status {
        CredentialStatus::Active => "active",
        CredentialStatus::Revoked => "revoked",
    }
}

fn id(value: Uuid) -> ID {
    ID(value.to_string())
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}
