use std::path::Path;

use axum::{
    http::{header, HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{any, delete, get, get_service, post},
    Extension, Router,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{
    api_endpoints::handlers as api_endpoints, authz::handlers as authz, config::Config, graphql,
    identity::handlers as identity, keys, state::AppState, tenants::handlers as tenants,
};

pub fn create_router(state: AppState) -> Router {
    let graphql_schema = graphql::build_schema(state.clone());
    let cors_origins = state
        .config
        .cors_allowed_origins
        .iter()
        .map(|origin| {
            HeaderValue::from_str(origin)
                .expect("ATOM_CORS_ALLOWED_ORIGINS contains an invalid origin")
        })
        .collect::<Vec<_>>();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(cors_origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT]);

    let app = Router::new()
        // JWKS — unauthenticated, consumed by external verifiers
        .route("/.well-known/jwks.json", get(keys::jwks))
        // Health
        .route("/health", get(identity::health))
        // GraphQL
        .route("/graphql", post(graphql::graphql_handler))
        // Custom API endpoint executor
        .route("/api/custom/*path", any(api_endpoints::custom_endpoint))
        // Auth
        .route("/auth/public-config", get(identity::public_auth_config))
        .route("/auth/signup", post(identity::signup))
        .route("/auth/login", post(identity::login))
        .route("/auth/email/verify", get(identity::verify_email))
        .route("/auth/email/resend", post(identity::resend_verification))
        .route("/auth/oauth/:provider/start", get(identity::oauth_start))
        .route(
            "/auth/oauth/:provider/callback",
            get(identity::oauth_callback),
        )
        .route("/auth/oauth/exchange", post(identity::oauth_exchange))
        .route("/auth/logout", post(identity::logout))
        .route("/auth/sessions/:id", get(identity::get_session))
        .route("/auth/keys/rotate", post(keys::rotate_keys))
        // Entities
        .route(
            "/entities",
            get(identity::list_entities).post(identity::create_entity),
        )
        .route("/entities/:id/access", get(authz::entity_access))
        .route(
            "/entities/:id/effective-capabilities",
            get(authz::effective_capabilities),
        )
        .route("/entities/:id/audit", get(authz::entity_audit_logs))
        .route(
            "/entities/:id",
            get(identity::get_entity)
                .put(identity::update_entity)
                .delete(identity::delete_entity),
        )
        // Profiles
        .route(
            "/profiles",
            get(identity::list_profiles).post(identity::create_profile),
        )
        .route("/profiles/:id", get(identity::get_profile))
        .route(
            "/profiles/:id/versions",
            get(identity::list_profile_versions).post(identity::create_profile_version),
        )
        // Credentials
        .route(
            "/entities/:id/credentials/password",
            post(identity::create_password),
        )
        .route(
            "/entities/:id/credentials/api-keys",
            post(identity::create_api_key),
        )
        .route("/entities/:id/credentials", get(identity::list_credentials))
        .route(
            "/entities/:entity_id/credentials/:cred_id",
            delete(identity::revoke_credential),
        )
        // Groups (on entity)
        .route("/entities/:id/groups", get(identity::get_entity_groups))
        // Ownerships
        .route(
            "/entities/:id/owned",
            get(identity::list_owned).post(identity::add_ownership),
        )
        .route(
            "/entities/:owner_id/owned/:owned_id",
            delete(identity::remove_ownership),
        )
        // Groups
        .route(
            "/groups",
            get(identity::list_groups).post(identity::create_group),
        )
        .route(
            "/groups/:id",
            get(identity::get_group).delete(identity::delete_group),
        )
        .route("/groups/:id/access", get(authz::group_access))
        .route(
            "/groups/:id/members",
            get(identity::list_group_members).post(identity::add_group_member),
        )
        .route(
            "/groups/:group_id/members/:entity_id",
            delete(identity::remove_group_member),
        )
        // Resources
        .route(
            "/resources",
            get(authz::list_resources).post(authz::create_resource),
        )
        .route(
            "/resources/:id",
            get(authz::get_resource)
                .put(authz::update_resource)
                .delete(authz::delete_resource),
        )
        .route("/resources/:id/access", get(authz::resource_access))
        // Roles
        .route("/roles", get(authz::list_roles).post(authz::create_role))
        .route(
            "/roles/:id",
            get(authz::get_role).delete(authz::delete_role),
        )
        .route("/roles/:id/holders", get(authz::role_holders))
        .route(
            "/roles/:id/capabilities",
            get(authz::get_role_capabilities).post(authz::add_role_capability),
        )
        .route(
            "/roles/:role_id/capabilities/:cap_id",
            delete(authz::remove_role_capability),
        )
        // Capabilities
        .route(
            "/capabilities",
            get(authz::list_capabilities).post(authz::create_capability),
        )
        .route(
            "/capabilities/:id",
            get(authz::get_capability).delete(authz::delete_capability),
        )
        // Policy Bindings
        .route(
            "/policies",
            get(authz::list_policies).post(authz::create_policy),
        )
        .route(
            "/policies/:id",
            get(authz::get_policy).delete(authz::delete_policy),
        )
        // Tenants
        .route(
            "/tenants",
            get(tenants::list_tenants).post(tenants::create_tenant),
        )
        .route(
            "/tenants/:id",
            get(tenants::get_tenant)
                .put(tenants::update_tenant)
                .delete(tenants::delete_tenant),
        )
        .route("/tenants/:id/enable", post(tenants::enable_tenant))
        .route("/tenants/:id/disable", post(tenants::disable_tenant))
        .route("/tenants/:id/freeze", post(tenants::freeze_tenant))
        // Authorization check (PDP)
        .route("/authz/check", post(authz::check))
        .route("/authz/check/bulk", post(authz::bulk_check))
        .route("/authz/explain", post(authz::explain))
        // Audit
        .route("/audit", get(authz::audit_logs))
        // Admin hygiene
        .route("/admin/orphan-policies", get(authz::orphan_policies))
        .route(
            "/admin/unprotected-resources",
            get(authz::unprotected_resources),
        )
        .route(
            "/admin/expiring-credentials",
            get(authz::expiring_credentials),
        );

    let app = attach_graphql_console(app, &state.config);

    app.with_state(state)
        .layer(Extension(graphql_schema))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

fn attach_graphql_console(app: Router<AppState>, config: &Config) -> Router<AppState> {
    if !config.graphql_console_enabled {
        return app;
    }

    let dist_dir = Path::new(&config.graphql_console_dist_dir);
    let index = dist_dir.join("index.html");
    if index.is_file() {
        let app = app.nest_service(
            "/graphql/console",
            ServeDir::new(dist_dir)
                .append_index_html_on_directories(true)
                .fallback(ServeFile::new(index)),
        );
        let app = app.route(
            "/graphql/playground",
            get_service(ServeFile::new(console_page_file(dist_dir, "playground"))),
        );

        CONSOLE_PAGE_ROUTES.iter().fold(app, |app, page| {
            app.route(
                &format!("/graphql/console/{page}"),
                get_service(ServeFile::new(console_page_file(dist_dir, page))),
            )
        })
    } else {
        app.route("/graphql/console", get(missing_graphql_console_dist))
            .route("/graphql/console/*path", get(missing_graphql_console_dist))
            .route("/graphql/playground", get(missing_graphql_console_dist))
    }
}

fn console_page_file(dist_dir: &Path, page: &str) -> std::path::PathBuf {
    let page_index = dist_dir.join(page).join("index.html");
    if page_index.is_file() {
        page_index
    } else {
        dist_dir.join("index.html")
    }
}

const CONSOLE_PAGE_ROUTES: &[&str] = &[
    "templates",
    "endpoints",
    "tenants",
    "entities",
    "profiles",
    "resources",
    "policies",
    "authz",
    "explorer",
    "playground",
    "settings",
];

async fn missing_graphql_console_dist() -> impl IntoResponse {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "Atom GraphQL Console is enabled, but console/dist/index.html is missing. Run `pnpm --dir console build` before starting Atom.",
    )
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    use crate::{
        config::{Config, OidcProviderConfig, ADMIN_ENTITY_ID},
        keys::{ActiveKeys, LoadedKey},
        state::AppState,
    };

    use super::create_router;

    #[tokio::test]
    async fn graphql_console_route_is_not_registered_by_default() {
        let app = create_router(test_state(false, "console/dist-missing-for-test"));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/graphql/console")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn graphql_console_returns_503_when_enabled_without_dist() {
        let app = create_router(test_state(true, "console/dist-missing-for-test"));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/graphql/console")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body = String::from_utf8(body.to_vec()).expect("utf8 body");
        assert!(body.contains("console/dist/index.html is missing"));
        assert!(body.contains("pnpm --dir console build"));
    }

    #[tokio::test]
    async fn graphql_console_serves_built_astro_dist_when_available() {
        let dist_dir =
            std::env::temp_dir().join(format!("atom-console-dist-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dist_dir).expect("create temp console dist");
        std::fs::write(
            dist_dir.join("index.html"),
            "<!doctype html><title>Astro console</title>",
        )
        .expect("write temp index");

        let app = create_router(test_state(true, dist_dir.to_str().expect("utf8 path")));

        for uri in [
            "/graphql/console",
            "/graphql/console/templates",
            "/graphql/console/playground",
            "/graphql/playground",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(uri)
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("response");

            assert_eq!(response.status(), StatusCode::OK);
            let body = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body");
            let html = String::from_utf8(body.to_vec()).expect("utf8 body");
            assert!(html.contains("Astro console"));
        }

        let _ = std::fs::remove_dir_all(dist_dir);
    }

    #[tokio::test]
    async fn signup_route_is_disabled_by_default() {
        let app = create_router(test_state(false, "console/dist-missing-for-test"));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/signup")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"alice","email":"alice@example.test","password":"test-password-123"}"#,
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn public_auth_config_reports_enabled_signup_and_providers() {
        let mut state = test_state(false, "console/dist-missing-for-test");
        state.config.signup_enabled = true;
        state.config.dev_allow_unverified_email_login = true;
        state.config.oidc_providers = vec![OidcProviderConfig {
            name: "google".into(),
            issuer: "https://accounts.google.com".into(),
            client_id: "client".into(),
            client_secret: "secret".into(),
            scopes: vec!["openid".into(), "email".into()],
        }];
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/public-config")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let body: serde_json::Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(
            body,
            serde_json::json!({
                "signup_enabled": true,
                "oauth_providers": ["google"],
                "email_verification_required": true,
                "dev_allow_unverified_email_login": true
            })
        );
    }

    fn test_state(graphql_console_enabled: bool, graphql_console_dist_dir: &str) -> AppState {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://atom:atom@localhost/atom_test")
            .expect("create lazy test pool");
        let config = Config {
            database_url: "postgres://atom:atom@localhost/atom_test".into(),
            listen_addr: "127.0.0.1:0".into(),
            grpc_addr: "127.0.0.1:0".into(),
            jwt_expiry_secs: 3600,
            admin_entity_id: ADMIN_ENTITY_ID,
            admin_secret: None,
            signup_enabled: false,
            dev_allow_unverified_email_login: false,
            public_base_url: "http://localhost:8080".into(),
            cors_allowed_origins: vec!["http://localhost:8080".into()],
            email_verification_redirect: "http://localhost:8080/graphql/console/auth/verify-email"
                .into(),
            oauth_success_redirect: "http://localhost:8080".into(),
            oauth_error_redirect: "http://localhost:8080".into(),
            oidc_providers: vec![],
            smtp: None,
            email_verification_expiry_secs: 86_400,
            oauth_state_expiry_secs: 600,
            auth_exchange_code_expiry_secs: 300,
            graphql_console_enabled,
            graphql_console_dist_dir: graphql_console_dist_dir.into(),
        };
        let primary = LoadedKey {
            kid: "test".into(),
            public_key_pem: String::new(),
            private_key_pem: String::new(),
            x_b64: String::new(),
            y_b64: String::new(),
        };
        AppState::new(
            pool,
            config,
            ActiveKeys {
                primary,
                standby: None,
            },
        )
    }
}
