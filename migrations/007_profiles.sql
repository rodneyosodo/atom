-- =============================================================
-- PROFILES
--
-- Profiles are user/domain-customizable subtypes and schema layers.
-- They do not replace the internal runtime/authz kind stored on
-- entities.kind.
-- =============================================================
CREATE TABLE profiles (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID,
    object_kind  TEXT        NOT NULL CHECK (object_kind IN ('entity', 'resource', 'group', 'tenant', 'credential')),
    kind         TEXT        NOT NULL,
    key          TEXT        NOT NULL,
    display_name TEXT        NOT NULL,
    description  TEXT,
    status       TEXT        NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'deprecated', 'disabled')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (
        object_kind <> 'entity'
        OR kind IN ('human', 'device', 'service', 'workload', 'application')
    )
);

CREATE UNIQUE INDEX idx_profiles_global_unique
    ON profiles(object_kind, kind, key)
    WHERE tenant_id IS NULL;

CREATE UNIQUE INDEX idx_profiles_tenant_unique
    ON profiles(tenant_id, object_kind, kind, key)
    WHERE tenant_id IS NOT NULL;

CREATE INDEX idx_profiles_lookup
    ON profiles(object_kind, kind, key, tenant_id);

CREATE TABLE profile_versions (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    profile_id  UUID        NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    version     INTEGER     NOT NULL,
    json_schema JSONB       NOT NULL DEFAULT '{}',
    ui_schema   JSONB       NOT NULL DEFAULT '{}',
    status      TEXT        NOT NULL DEFAULT 'active' CHECK (status IN ('draft', 'active', 'deprecated', 'disabled')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(profile_id, version)
);

CREATE INDEX idx_profile_versions_profile
    ON profile_versions(profile_id);

ALTER TABLE entities
    ADD COLUMN profile_id UUID NULL REFERENCES profiles(id),
    ADD COLUMN profile_version_id UUID NULL REFERENCES profile_versions(id);

CREATE INDEX idx_entities_profile
    ON entities(profile_id);

CREATE INDEX idx_entities_profile_version
    ON entities(profile_version_id);

WITH seeded_profiles AS (
    INSERT INTO profiles (object_kind, kind, key, display_name)
    VALUES
        ('entity', 'device',      'client',          'Client'),
        ('entity', 'device',      'gateway',         'Gateway'),
        ('entity', 'device',      'water_meter',     'Water Meter'),
        ('entity', 'human',       'user',            'User'),
        ('entity', 'service',     'service_account', 'Service Account'),
        ('entity', 'workload',    'workload',        'Workload'),
        ('entity', 'application', 'application',     'Application')
    RETURNING id
)
INSERT INTO profile_versions (profile_id, version, json_schema, ui_schema)
SELECT id, 1, '{}'::jsonb, '{}'::jsonb
FROM seeded_profiles;
