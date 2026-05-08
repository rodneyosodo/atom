-- =============================================================
-- API TEMPLATES
--
-- Saved GraphQL API templates are a small metadata layer for the
-- Atom API Builder console. They store GraphQL operations and
-- variables metadata only; they do not inspect raw database tables.
-- =============================================================

CREATE TABLE api_templates (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        REFERENCES tenants(id) ON DELETE CASCADE,
    key              TEXT        NOT NULL,
    name             TEXT        NOT NULL,
    description      TEXT,
    operation_kind   TEXT        NOT NULL CHECK (operation_kind IN ('query', 'mutation')),
    graphql          TEXT        NOT NULL,
    variables_schema JSONB       NOT NULL DEFAULT '{}',
    default_variables JSONB      NOT NULL DEFAULT '{}',
    result_selector  JSONB       NOT NULL DEFAULT '{}',
    tags             TEXT[]      NOT NULL DEFAULT '{}',
    status           TEXT        NOT NULL DEFAULT 'active'
                              CHECK (status IN ('draft', 'active', 'deprecated', 'disabled')),
    created_by       UUID        REFERENCES entities(id) ON DELETE SET NULL,
    updated_by       UUID        REFERENCES entities(id) ON DELETE SET NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_api_templates_global_key
    ON api_templates(key)
    WHERE tenant_id IS NULL;

CREATE UNIQUE INDEX idx_api_templates_tenant_key
    ON api_templates(tenant_id, key)
    WHERE tenant_id IS NOT NULL;

CREATE INDEX idx_api_templates_tenant
    ON api_templates(tenant_id);

CREATE INDEX idx_api_templates_status
    ON api_templates(status);

CREATE INDEX idx_api_templates_tags
    ON api_templates USING GIN(tags);

INSERT INTO api_templates
    (key, name, description, operation_kind, graphql, variables_schema, default_variables, result_selector, tags)
VALUES
    (
        'create_tenant',
        'Create tenant',
        'Create an Atom tenant isolation boundary.',
        'mutation',
        $graphql$
mutation CreateTenant($input: CreateTenantInput!) {
  createTenant(input: $input) {
    id
    name
    route
    status
    tags
    attributes
    createdAt
    updatedAt
  }
}
$graphql$,
        '{"type":"object","required":["input"],"properties":{"input":{"type":"object","required":["name"],"properties":{"name":{"type":"string"},"route":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}},"attributes":{"type":"object"}}}}}'::jsonb,
        '{"input":{"name":"factory-a","route":"factory-a","tags":["example"],"attributes":{}}}'::jsonb,
        '{"path":["createTenant"]}'::jsonb,
        ARRAY['tenant', 'setup']
    ),
    (
        'create_entity_from_profile',
        'Create entity from profile',
        'Create a generic Atom entity using a profile and profile version.',
        'mutation',
        $graphql$
mutation CreateEntityFromProfile($input: CreateEntityInput!) {
  createEntity(input: $input) {
    id
    kind
    profileId
    profileVersionId
    name
    tenantId
    status
    attributes
    createdAt
    updatedAt
  }
}
$graphql$,
        '{"type":"object","required":["input"],"properties":{"input":{"type":"object","required":["profileId","name","attributes"],"properties":{"profileId":{"type":"string"},"profileVersionId":{"type":"string"},"name":{"type":"string"},"tenantId":{"type":"string"},"attributes":{"type":"object"}}}}}'::jsonb,
        '{"input":{"profileId":"paste-profile-id-here","name":"entity-001","attributes":{}}}'::jsonb,
        '{"path":["createEntity"]}'::jsonb,
        ARRAY['entity', 'profile']
    ),
    (
        'create_resource',
        'Create resource',
        'Create a generic protected Atom resource.',
        'mutation',
        $graphql$
mutation CreateResource($input: CreateResourceInput!) {
  createResource(input: $input) {
    id
    kind
    name
    tenantId
    ownerId
    attributes
    createdAt
    updatedAt
  }
}
$graphql$,
        '{"type":"object","required":["input"],"properties":{"input":{"type":"object","required":["kind"],"properties":{"kind":{"type":"string"},"name":{"type":"string"},"tenantId":{"type":"string"},"ownerId":{"type":"string"},"attributes":{"type":"object"}}}}}'::jsonb,
        '{"input":{"kind":"channel","name":"telemetry","attributes":{"topic":"telemetry"}}}'::jsonb,
        '{"path":["createResource"]}'::jsonb,
        ARRAY['resource', 'authz']
    ),
    (
        'create_policy',
        'Create policy',
        'Grant or deny a generic Atom capability or role on a scope.',
        'mutation',
        $graphql$
mutation CreatePolicy($input: CreatePolicyInput!) {
  createPolicy(input: $input) {
    id
    tenantId
    subjectKind
    subjectId
    grantKind
    grantId
    scopeKind
    scopeRef
    effect
    conditions
    createdAt
  }
}
$graphql$,
        '{"type":"object","required":["input"],"properties":{"input":{"type":"object","required":["subjectKind","subjectId","grantKind","grantId","scopeKind"],"properties":{"tenantId":{"type":"string"},"subjectKind":{"type":"string"},"subjectId":{"type":"string"},"grantKind":{"type":"string"},"grantId":{"type":"string"},"scopeKind":{"type":"string"},"scopeRef":{"type":"string"},"effect":{"type":"string"},"conditions":{"type":"object"}}}}}'::jsonb,
        '{"input":{"subjectKind":"entity","subjectId":"paste-subject-id-here","grantKind":"capability","grantId":"paste-capability-id-here","scopeKind":"object","scopeRef":"paste-object-id-here","effect":"allow","conditions":{}}}'::jsonb,
        '{"path":["createPolicy"]}'::jsonb,
        ARRAY['policy', 'authz']
    ),
    (
        'authz_check',
        'Authorization check',
        'Run an Atom online authorization decision.',
        'mutation',
        $graphql$
mutation AuthzCheck($input: AuthzCheckInput!) {
  authzCheck(input: $input) {
    allowed
    reason
    details
  }
}
$graphql$,
        '{"type":"object","required":["input"],"properties":{"input":{"type":"object","required":["subjectId","action"],"properties":{"subjectId":{"type":"string"},"action":{"type":"string"},"resourceId":{"type":"string"},"objectKind":{"type":"string"},"objectId":{"type":"string"},"context":{"type":"object"}}}}}'::jsonb,
        '{"input":{"subjectId":"paste-subject-id-here","action":"read","context":{}}}'::jsonb,
        '{"path":["authzCheck"]}'::jsonb,
        ARRAY['authz', 'check']
    ),
    (
        'create_api_key',
        'Create API key',
        'Create a one-time-revealed Atom API key credential for an entity.',
        'mutation',
        $graphql$
mutation CreateApiKey($entityId: ID!, $input: CreateApiKeyInput!) {
  createApiKey(entityId: $entityId, input: $input) {
    credentialId
    key
    expiresAt
  }
}
$graphql$,
        '{"type":"object","required":["entityId","input"],"properties":{"entityId":{"type":"string"},"input":{"type":"object","properties":{"expiresAt":{"type":"string"},"description":{"type":"string"}}}}}'::jsonb,
        '{"entityId":"paste-entity-id-here","input":{"description":"automation key"}}'::jsonb,
        '{"path":["createApiKey"]}'::jsonb,
        ARRAY['credential', 'api_key']
    );
