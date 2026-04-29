-- =============================================================
-- M8 — Capability assignment guardrail defaults
-- =============================================================

INSERT INTO capability_assignment_rules
    (entity_kind, capability_name, object_kind, object_type, decision, is_absolute)
VALUES
    ('device', 'manage', 'resource', NULL, 'deny', TRUE),
    ('device', 'delete', 'resource', NULL, 'deny', TRUE),
    ('device', 'write',  'resource', NULL, 'deny', TRUE),
    ('device', 'publish', 'resource', 'resource:channel', 'allow', FALSE),
    ('device', 'subscribe', 'resource', 'resource:channel', 'allow', FALSE),
    ('human', 'manage', 'resource', NULL, 'allow', FALSE),
    ('human', 'manage', 'entity', NULL, 'allow', FALSE),
    ('human', 'manage', 'group', NULL, 'allow', FALSE),
    ('human', 'policy.manage', 'policy', NULL, 'allow', FALSE),
    ('human', 'role.manage', 'role', NULL, 'allow', FALSE),
    ('service', 'manage', 'resource', NULL, 'allow', FALSE),
    ('service', 'policy.manage', 'policy', NULL, 'allow', FALSE),
    ('service', 'role.manage', 'role', NULL, 'allow', FALSE)
ON CONFLICT DO NOTHING;
