use chrono::{Duration, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::AuditRetentionConfig, models::enums::AuditOutcome, state::AppState};

#[derive(Debug, Clone)]
pub struct AuditCleanupSummary {
    pub deleted_rows: i64,
    pub cutoff: chrono::DateTime<Utc>,
}

pub struct AuditEvent<'a> {
    pub actor_entity_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub target_kind: Option<&'a str>,
    pub target_id: Option<Uuid>,
    pub event: &'a str,
    pub outcome: AuditOutcome,
    pub details: Value,
}

pub async fn write(pool: &PgPool, event: AuditEvent<'_>) {
    let result = sqlx::query(
        "INSERT INTO audit_logs (id, actor_entity_id, tenant_id, target_kind, target_id, event, outcome, details)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(event.actor_entity_id)
    .bind(event.tenant_id)
    .bind(event.target_kind)
    .bind(event.target_id)
    .bind(event.event)
    .bind(event.outcome)
    .bind(event.details)
    .execute(pool)
    .await;

    if let Err(e) = result {
        crate::metrics::record_audit_failure();
        tracing::error!("audit write failed event={}: {e}", event.event);
    }
}

pub fn spawn_retention_cleanup(state: AppState) {
    let cfg = state.config.audit_retention;
    if !cfg.enabled {
        tracing::info!("audit retention cleanup disabled");
        return;
    }

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(cfg.cleanup_interval_secs));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            match cleanup_expired(&state.pool, cfg).await {
                Ok(summary) if summary.deleted_rows > 0 => {
                    write(
                        &state.pool,
                        AuditEvent {
                            actor_entity_id: None,
                            tenant_id: None,
                            target_kind: None,
                            target_id: None,
                            event: "audit.retention_cleanup",
                            outcome: AuditOutcome::Allow,
                            details: serde_json::json!({
                                "deleted_rows": summary.deleted_rows,
                                "cutoff": summary.cutoff,
                                "retention_days": cfg.days,
                                "batch_size": cfg.cleanup_batch_size,
                            }),
                        },
                    )
                    .await;
                }
                Ok(_) => {}
                Err(err) => tracing::warn!("audit retention cleanup failed: {err}"),
            }
        }
    });
}

pub async fn cleanup_expired(
    pool: &PgPool,
    cfg: AuditRetentionConfig,
) -> Result<AuditCleanupSummary, sqlx::Error> {
    let cutoff = Utc::now() - Duration::days(cfg.days);
    let mut deleted_rows = 0_i64;

    loop {
        let result = sqlx::query(
            r#"WITH doomed AS (
                   SELECT id
                   FROM audit_logs
                   WHERE created_at < $1
                   ORDER BY created_at ASC
                   LIMIT $2
               )
               DELETE FROM audit_logs
               WHERE id IN (SELECT id FROM doomed)"#,
        )
        .bind(cutoff)
        .bind(cfg.cleanup_batch_size)
        .execute(pool)
        .await?;

        let batch = i64::try_from(result.rows_affected()).unwrap_or(i64::MAX);
        deleted_rows += batch;
        if batch < cfg.cleanup_batch_size {
            break;
        }
    }

    Ok(AuditCleanupSummary {
        deleted_rows,
        cutoff,
    })
}
