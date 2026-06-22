-- Fix entity tenant scoping: enforce name uniqueness for tenant-less entities,
-- and stop orphaning a deleted tenant's entities into the system namespace.
--
-- entities.tenant_id is nullable and was ON DELETE SET NULL. Two problems:
--
--  1. The old index idx_entities_name_tenant (name, tenant_id) treats NULL
--     tenant_id as distinct, so multiple tenant-less (system) entities with the
--     same name (e.g. two `admin`s) slipped through and produced ambiguous,
--     unresolvable logins.
--
--  2. ON DELETE SET NULL orphaned a deleted tenant's entities into the
--     tenant-less namespace shared with the bootstrap `admin` / `mg-service`
--     entities, causing name collisions and leaking tenant users into the
--     platform-global scope.
--
-- Fix 1: match the project idiom (roles index at 001:431) and collapse NULL
-- tenant_id to a sentinel UUID so a single unique index covers tenant-scoped
-- and system rows alike.
--
-- Fix 2: tenant deletion is already a hard teardown everywhere else (roles,
-- groups, policies, memberships, invitations, api_endpoints all ON DELETE
-- CASCADE), so entities were the inconsistent case. Switch to CASCADE: deleting
-- a tenant deletes its entities (and their credentials, sessions, emails,
-- memberships, which already cascade off entities). Audit trail is preserved:
-- audit_logs.tenant_id and audit_logs.entity_id are ON DELETE SET NULL, so log
-- rows survive with the references nulled. System entities (tenant_id IS NULL)
-- are never touched.

-- Fail loudly if pre-existing duplicates would otherwise break index creation.
DO $$
DECLARE
    dup RECORD;
BEGIN
    SELECT name, count(*) AS n
      INTO dup
      FROM entities
     WHERE tenant_id IS NULL
     GROUP BY name
    HAVING count(*) > 1
     LIMIT 1;

    IF FOUND THEN
        RAISE EXCEPTION
            'cannot enforce global entity name uniqueness: % duplicate tenant-less entities named %; deduplicate before migrating',
            dup.n, dup.name;
    END IF;
END $$;

DROP INDEX idx_entities_name_tenant;

CREATE UNIQUE INDEX idx_entities_name_tenant
    ON entities (
        name,
        COALESCE(tenant_id, '00000000-0000-0000-0000-000000000000'::uuid)
    );

ALTER TABLE entities
    DROP CONSTRAINT entities_tenant_id_fkey,
    ADD CONSTRAINT entities_tenant_id_fkey
        FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;
