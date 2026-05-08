-- =============================================================
-- API ENDPOINTS
--
-- Custom HTTP endpoints execute saved generic Atom GraphQL API
-- templates. They are metadata only: no raw database introspection
-- and no external-system-specific aliases.
-- =============================================================

CREATE TABLE api_endpoints (
    id                 UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID        REFERENCES tenants(id) ON DELETE CASCADE,
    key                TEXT        NOT NULL,
    name               TEXT        NOT NULL,
    description        TEXT,
    method             TEXT        NOT NULL CHECK (method IN ('GET', 'POST', 'PUT', 'PATCH', 'DELETE')),
    path               TEXT        NOT NULL,
    template_id        UUID        NOT NULL REFERENCES api_templates(id) ON DELETE CASCADE,
    auth_mode          TEXT        NOT NULL DEFAULT 'caller_context'
                                      CHECK (auth_mode IN ('caller_context', 'service_context')),
    service_entity_id  UUID        REFERENCES entities(id) ON DELETE SET NULL,
    variables_mapping  JSONB       NOT NULL DEFAULT '{}',
    request_schema     JSONB       NOT NULL DEFAULT '{}',
    response_mapping   JSONB       NOT NULL DEFAULT '{}',
    status             TEXT        NOT NULL DEFAULT 'draft'
                                      CHECK (status IN ('draft', 'active', 'disabled')),
    created_by         UUID        REFERENCES entities(id) ON DELETE SET NULL,
    updated_by         UUID        REFERENCES entities(id) ON DELETE SET NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_api_endpoints_global_key
    ON api_endpoints(key)
    WHERE tenant_id IS NULL;

CREATE UNIQUE INDEX idx_api_endpoints_tenant_key
    ON api_endpoints(tenant_id, key)
    WHERE tenant_id IS NOT NULL;

CREATE UNIQUE INDEX idx_api_endpoints_active_method_path
    ON api_endpoints(method, path)
    WHERE status = 'active';

CREATE INDEX idx_api_endpoints_tenant
    ON api_endpoints(tenant_id);

CREATE INDEX idx_api_endpoints_status
    ON api_endpoints(status);

CREATE TABLE api_endpoint_executions (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    endpoint_id       UUID        REFERENCES api_endpoints(id) ON DELETE SET NULL,
    caller_entity_id  UUID        REFERENCES entities(id) ON DELETE SET NULL,
    status            TEXT        NOT NULL CHECK (status IN ('success', 'error', 'denied')),
    request_summary   JSONB       NOT NULL DEFAULT '{}',
    response_summary  JSONB       NOT NULL DEFAULT '{}',
    error             TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_api_endpoint_executions_endpoint
    ON api_endpoint_executions(endpoint_id, created_at DESC);

CREATE INDEX idx_api_endpoint_executions_caller
    ON api_endpoint_executions(caller_entity_id, created_at DESC);
