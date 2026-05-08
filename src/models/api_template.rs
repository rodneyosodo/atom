use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum ApiTemplateOperationKind {
    Query,
    Mutation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum ApiTemplateStatus {
    Draft,
    Active,
    Deprecated,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiTemplate {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub operation_kind: ApiTemplateOperationKind,
    pub graphql: String,
    pub variables_schema: Value,
    pub default_variables: Value,
    pub result_selector: Value,
    pub tags: Vec<String>,
    pub status: ApiTemplateStatus,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiTemplate {
    pub tenant_id: Option<Uuid>,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub operation_kind: ApiTemplateOperationKind,
    pub graphql: String,
    #[serde(default = "default_json_object")]
    pub variables_schema: Value,
    #[serde(default = "default_json_object")]
    pub default_variables: Value,
    #[serde(default = "default_json_object")]
    pub result_selector: Value,
    #[serde(default)]
    pub tags: Vec<String>,
    pub status: Option<ApiTemplateStatus>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApiTemplate {
    pub key: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub operation_kind: Option<ApiTemplateOperationKind>,
    pub graphql: Option<String>,
    pub variables_schema: Option<Value>,
    pub default_variables: Option<Value>,
    pub result_selector: Option<Value>,
    pub tags: Option<Vec<String>>,
    pub status: Option<ApiTemplateStatus>,
}

#[derive(Debug, Deserialize)]
pub struct ListApiTemplates {
    pub tenant_id: Option<Uuid>,
    pub status: Option<ApiTemplateStatus>,
    pub tag: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct ApiTemplateList {
    pub items: Vec<ApiTemplate>,
    pub total: i64,
}

pub fn default_json_object() -> Value {
    serde_json::json!({})
}

fn default_limit() -> i64 {
    20
}
