-- Security hardening: explicit authz-check capability and login throttling.

INSERT INTO capabilities (name, resource_kind, description) VALUES
    ('authz.check', NULL, 'Evaluate authorization checks for other subjects')
ON CONFLICT (name, resource_kind) DO NOTHING;

INSERT INTO role_capabilities (role_id, capability_id)
SELECT '00000000-0000-0000-0000-000000000002', c.id
FROM capabilities c
WHERE c.name = 'authz.check' AND c.resource_kind IS NULL
ON CONFLICT DO NOTHING;

CREATE TABLE IF NOT EXISTS auth_login_attempts (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    identifier  TEXT        NOT NULL,
    tenant_id   UUID        REFERENCES tenants(id) ON DELETE CASCADE,
    success     BOOLEAN     NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_auth_login_attempts_throttle
    ON auth_login_attempts(identifier, tenant_id, created_at DESC)
    WHERE success = FALSE;

CREATE INDEX IF NOT EXISTS idx_auth_login_attempts_created
    ON auth_login_attempts(created_at DESC);
