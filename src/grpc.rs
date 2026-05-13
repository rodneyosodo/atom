use std::net::SocketAddr;

use tonic::{metadata::MetadataMap, transport::Server, Request, Response, Status};
use uuid::Uuid;

use crate::{
    audit,
    auth::{authenticate_token, AuthContext},
    authz::{access, engine},
    models::{enums::AuditOutcome, policy::AuthzRequest},
    state::AppState,
};

// Generated code from proto/atom.proto
pub mod proto {
    tonic::include_proto!("atom.v1");
}

use proto::{
    auth_service_server::{AuthService, AuthServiceServer},
    authz_service_server::{AuthzService, AuthzServiceServer},
    AuthenticateRequest, AuthenticateResponse, CheckRequest, CheckResponse,
};

// ─── AuthzService ─────────────────────────────────────────────────────────────

struct AtomAuthz {
    state: AppState,
}

#[tonic::async_trait]
impl AuthzService for AtomAuthz {
    async fn check(
        &self,
        request: Request<CheckRequest>,
    ) -> Result<Response<CheckResponse>, Status> {
        let auth = auth_context_from_metadata(&self.state, request.metadata()).await?;
        let req = request.into_inner();

        let subject_id = Uuid::parse_str(&req.subject_id)
            .map_err(|_| Status::invalid_argument("invalid subject_id: expected UUID"))?;

        let resource_id = if req.resource_id.is_empty() {
            None
        } else {
            Some(
                Uuid::parse_str(&req.resource_id)
                    .map_err(|_| Status::invalid_argument("invalid resource_id: expected UUID"))?,
            )
        };

        let object_kind = if req.object_kind.is_empty() {
            None
        } else {
            Some(req.object_kind)
        };
        let object_id = if req.object_id.is_empty() {
            None
        } else {
            Some(
                Uuid::parse_str(&req.object_id)
                    .map_err(|_| Status::invalid_argument("invalid object_id: expected UUID"))?,
            )
        };

        let context = if req.context.is_empty() {
            serde_json::Value::Object(Default::default())
        } else {
            serde_json::to_value(&req.context).unwrap_or_default()
        };

        let authz_req = AuthzRequest {
            subject_id,
            action: req.action,
            resource_id,
            object_kind,
            object_id,
            context,
        };

        let tenant_id = access::authz_request_tenant_id(&self.state.pool, &authz_req)
            .await
            .map_err(Status::from)?;
        access::require_authz_check_access(
            &self.state.pool,
            &auth,
            authz_req.subject_id,
            tenant_id,
        )
        .await
        .map_err(Status::from)?;

        let resp = engine::evaluate(&self.state.pool, &authz_req)
            .await
            .map_err(Status::from)?;
        audit::write(
            &self.state.pool,
            Some(auth.entity_id),
            tenant_id,
            "authz.check",
            if resp.allowed {
                AuditOutcome::Allow
            } else {
                AuditOutcome::Deny
            },
            serde_json::json!({
                "subject_id": authz_req.subject_id,
                "action": authz_req.action,
                "resource_id": authz_req.resource_id,
                "object_kind": authz_req.object_kind,
                "object_id": authz_req.object_id,
                "reason": resp.reason,
                "transport": "grpc",
            }),
        )
        .await;

        Ok(Response::new(CheckResponse {
            allowed: resp.allowed,
            reason: resp.reason,
        }))
    }
}

// ─── AuthService ──────────────────────────────────────────────────────────────

async fn auth_context_from_metadata(
    state: &AppState,
    metadata: &MetadataMap,
) -> Result<AuthContext, Status> {
    let header = metadata
        .get("authorization")
        .ok_or_else(|| Status::unauthenticated("missing authorization metadata"))?
        .to_str()
        .map_err(|_| Status::unauthenticated("invalid authorization metadata"))?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| Status::unauthenticated("expected Bearer token"))?;
    authenticate_token(state, token).await.map_err(Status::from)
}

struct AtomAuth {
    state: AppState,
}

#[tonic::async_trait]
impl AuthService for AtomAuth {
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let token = request.into_inner().token;

        let ctx = authenticate_token(&self.state, &token)
            .await
            .map_err(Status::from)?;

        Ok(Response::new(AuthenticateResponse {
            entity_id: ctx.entity_id.to_string(),
            tenant_id: ctx.tenant_id.map(|t| t.to_string()).unwrap_or_default(),
            session_id: ctx.session_id.map(|s| s.to_string()).unwrap_or_default(),
        }))
    }
}

// ─── Server ───────────────────────────────────────────────────────────────────

pub async fn serve(addr: SocketAddr, state: AppState) -> anyhow::Result<()> {
    tracing::info!("grpc listening on {addr}");

    Server::builder()
        .add_service(AuthzServiceServer::new(AtomAuthz {
            state: state.clone(),
        }))
        .add_service(AuthServiceServer::new(AtomAuth { state }))
        .serve(addr)
        .await?;

    Ok(())
}
