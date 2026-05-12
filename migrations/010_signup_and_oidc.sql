-- =============================================================
-- PUBLIC HUMAN SIGNUP, EMAIL VERIFICATION, AND OIDC IDENTITIES
-- =============================================================

CREATE TABLE entity_emails (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id   UUID        NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    email       TEXT        NOT NULL,
    verified_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (entity_id),
    UNIQUE (email)
);

CREATE INDEX idx_entity_emails_entity ON entity_emails(entity_id);
CREATE INDEX idx_entity_emails_verified ON entity_emails(verified_at);

CREATE TABLE email_verification_tokens (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id   UUID        NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    email_id    UUID        NOT NULL REFERENCES entity_emails(id) ON DELETE CASCADE,
    secret_hash TEXT        NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_email_verification_tokens_entity ON email_verification_tokens(entity_id);
CREATE INDEX idx_email_verification_tokens_active
    ON email_verification_tokens(id)
    WHERE consumed_at IS NULL;

CREATE TABLE oauth_identities (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id      UUID        NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    provider       TEXT        NOT NULL,
    subject        TEXT        NOT NULL,
    email          TEXT        NOT NULL,
    email_verified BOOLEAN     NOT NULL DEFAULT false,
    profile        JSONB       NOT NULL DEFAULT '{}',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, subject)
);

CREATE INDEX idx_oauth_identities_entity ON oauth_identities(entity_id);
CREATE INDEX idx_oauth_identities_email ON oauth_identities(email);

CREATE TABLE oauth_login_states (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    provider      TEXT        NOT NULL,
    state_hash    TEXT        NOT NULL,
    pkce_verifier TEXT        NOT NULL,
    nonce         TEXT        NOT NULL,
    return_to     TEXT,
    expires_at    TIMESTAMPTZ NOT NULL,
    consumed_at   TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_oauth_login_states_active
    ON oauth_login_states(id)
    WHERE consumed_at IS NULL;

CREATE TABLE auth_exchange_codes (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id   UUID        NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    secret_hash TEXT        NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_auth_exchange_codes_active
    ON auth_exchange_codes(id)
    WHERE consumed_at IS NULL;
