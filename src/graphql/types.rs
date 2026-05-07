use async_graphql::{InputObject, Object, ID};
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{
    entity as entity_model,
    enums::{EntityKind, EntityStatus},
    profile as profile_model,
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

pub fn parse_id(value: ID, name: &str) -> async_graphql::Result<Uuid> {
    value
        .as_str()
        .parse()
        .map_err(|_| async_graphql::Error::new(format!("{name} must be a UUID")))
}

pub fn parse_optional_id(value: Option<ID>, name: &str) -> async_graphql::Result<Option<Uuid>> {
    value.map(|id| parse_id(id, name)).transpose()
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

fn id(value: Uuid) -> ID {
    ID(value.to_string())
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}
