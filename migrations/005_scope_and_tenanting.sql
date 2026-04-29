-- =============================================================
-- M1 — Schema & scope foundations
--
-- 1. Add tenant_id columns to policy_bindings and audit_logs.
-- 2. Migrate scope_kind values to the canonical PRD set:
--      all          → platform
--      resource_kind → object_type, scope_ref prefixed with "resource:"
--      resource     → object
-- 3. Create tenant_memberships and capability_assignment_rules.
-- 4. Seed missing capabilities and re-link to the admin role.
-- =============================================================

-- ─── policy_bindings.tenant_id (AM-9) ─────────────────────────────────────────
ALTER TABLE policy_bindings ADD COLUMN tenant_id UUID;
ALTER TABLE policy_bindings
    ADD CONSTRAINT policy_bindings_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES tenants(id)
    ON DELETE CASCADE
    NOT VALID;
CREATE INDEX idx_pb_tenant ON policy_bindings(tenant_id);

-- ─── audit_logs.tenant_id (AUD-6) ─────────────────────────────────────────────
ALTER TABLE audit_logs ADD COLUMN tenant_id UUID;
ALTER TABLE audit_logs
    ADD CONSTRAINT audit_logs_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES tenants(id)
    ON DELETE SET NULL
    NOT VALID;
CREATE INDEX idx_audit_tenant ON audit_logs(tenant_id);

-- ─── scope_kind migration (structural #1) ─────────────────────────────────────
-- Drop the old CHECK constraint by its auto-generated name, migrate data,
-- then add a new CHECK constraint with an explicit name.
ALTER TABLE policy_bindings DROP CONSTRAINT IF EXISTS policy_bindings_scope_kind_check;

UPDATE policy_bindings
   SET scope_kind = 'platform'
 WHERE scope_kind = 'all';

UPDATE policy_bindings
   SET scope_kind = 'object_type',
       scope_ref  = CASE
                      WHEN scope_ref IS NULL THEN NULL
                      WHEN scope_ref LIKE '%:%' THEN scope_ref
                      ELSE 'resource:' || scope_ref
                    END
 WHERE scope_kind = 'resource_kind';

UPDATE policy_bindings
   SET scope_kind = 'object'
 WHERE scope_kind = 'resource';

ALTER TABLE policy_bindings
    ADD CONSTRAINT policy_bindings_scope_kind_check
    CHECK (scope_kind IN ('platform', 'tenant', 'object_kind', 'object_type', 'object'));

-- ─── tenant_memberships (ID-10, TEN-15) ───────────────────────────────────────
CREATE TABLE tenant_memberships (
    tenant_id   UUID        NOT NULL REFERENCES tenants(id)  ON DELETE CASCADE,
    entity_id   UUID        NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    status      TEXT        NOT NULL DEFAULT 'active'
                            CHECK (status IN ('active', 'invited', 'suspended', 'left')),
    local_name  TEXT,
    attributes  JSONB       NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, entity_id)
);

CREATE INDEX idx_tenant_memberships_entity ON tenant_memberships(entity_id);
CREATE INDEX idx_tenant_memberships_status ON tenant_memberships(status);

-- ─── capability_assignment_rules (GR-1 prereq) ────────────────────────────────
CREATE TABLE capability_assignment_rules (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID        REFERENCES tenants(id) ON DELETE CASCADE,
    entity_kind     TEXT        NOT NULL
                                CHECK (entity_kind IN ('human', 'device', 'service', 'workload', 'application')),
    capability_name TEXT        NOT NULL,
    object_kind     TEXT        NOT NULL
                                CHECK (object_kind IN ('entity', 'resource', 'group', 'tenant', 'role', 'policy', 'credential', 'audit_log')),
    object_type     TEXT,
    decision        TEXT        NOT NULL
                                CHECK (decision IN ('allow', 'deny', 'require_override')),
    is_absolute     BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_car_tenant ON capability_assignment_rules(tenant_id);
CREATE INDEX idx_car_lookup ON capability_assignment_rules(entity_kind, capability_name, object_kind);

-- ─── Seed missing capabilities (structural #8) ────────────────────────────────
INSERT INTO capabilities (name, resource_kind, description) VALUES
    ('list',                NULL, 'List resources'),
    ('credential.manage',   NULL, 'Manage credentials'),
    ('credential.revoke',   NULL, 'Revoke credentials'),
    ('signing_key.rotate',  NULL, 'Rotate JWT signing keys'),
    ('audit.read',          NULL, 'Read audit logs'),
    ('policy.manage',       NULL, 'Manage policies'),
    ('role.manage',         NULL, 'Manage roles'),
    ('tenant.manage',       NULL, 'Manage tenants (lifecycle and metadata)')
ON CONFLICT (name, resource_kind) DO NOTHING;

-- Keep the seeded admin role linked to every capability, including the new ones.
INSERT INTO role_capabilities (role_id, capability_id)
SELECT '00000000-0000-0000-0000-000000000002', id FROM capabilities
ON CONFLICT DO NOTHING;
