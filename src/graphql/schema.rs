use async_graphql::{EmptySubscription, Schema};

use crate::state::AppState;

use super::{
    mutation::{mutation_root, MutationRoot},
    query::QueryRoot,
};

pub type AtomSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(state: AppState) -> AtomSchema {
    Schema::build(QueryRoot, mutation_root(), EmptySubscription)
        .data(state)
        .finish()
}

#[cfg(test)]
mod tests {
    use async_graphql::Request;
    use sqlx::postgres::PgPoolOptions;

    use crate::{
        config::{Config, ADMIN_ENTITY_ID},
        keys::{ActiveKeys, LoadedKey},
        state::AppState,
    };

    use super::build_schema;

    #[tokio::test]
    async fn health_query_returns_ok() {
        let schema = build_schema(test_state());

        let response = schema.execute(Request::new("{ health }")).await;

        assert!(response.errors.is_empty(), "{:?}", response.errors);
        assert_eq!(
            response.data.into_json().unwrap(),
            serde_json::json!({"health": "ok"})
        );
    }

    fn test_state() -> AppState {
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
