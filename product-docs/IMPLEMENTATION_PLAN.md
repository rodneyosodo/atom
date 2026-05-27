# Atom Implementation Plan

Tracks the work needed to bring the implementation in line with [PRD.md](./PRD.md).
Milestones are dependency-ordered; each milestone names the PRD IDs it closes
and the test surface it ships with.

> Legacy implementation log: this file records work completed under the older policy/scope model. The authoritative product direction is now [Atom access model](./11-access-model-simplification.md). Future implementation planning should replace scoped policies and overloaded groups with role permission blocks, assignments, Object Groups, and Principal Groups.

## Status

| Milestone | Status |
|---|---|
| M1 — Schema & scope foundations | ✅ Done |
| M2 — Object_type namespacing | ✅ Done |
| M3 — Tenant lifecycle in PDP | ✅ Done |
| M4 — Platform & tenant policy inheritance | ✅ Done |
| M5 — Tenant-admin bootstrap | ✅ Done |
| M6 — ABAC operator & context expansion | ✅ Done |
| M7 — Audit tenanting & login event | ✅ Done |
| M8 — Capability assignment guardrails | ✅ Done |

Legend: ✅ done · 🔄 in progress · ⏳ pending · ⏸ blocked

---

## M1 — Schema & scope foundations

**Goal.** Lay the storage and enum groundwork that every later milestone needs. After M1 the database has the right shape; semantic logic comes in M2–M5.

**Closes (PRD IDs):** ID-10, TEN-15, AM-9, AUD-6 (schema only); structural #1, #3, #6, #7, #8.

**Tasks.**
- [x] Migration `005_scope_and_tenanting.sql`:
  - Added `tenant_id UUID` to `policy_bindings` and `audit_logs`, with `NOT VALID` FK to `tenants(id)` and indexes.
  - Migrated existing `scope_kind` values: `all` → `platform`, `resource_kind` → `object_type` with `resource:` prefix on `scope_ref`, `resource` → `object`. Replaced the CHECK constraint.
  - Created `tenant_memberships` table per PRD shape.
  - Created `capability_assignment_rules` table (with `object_type`, not `resource_kind`).
  - Seeded missing capabilities and re-linked to admin role.
- [x] `models/enums.rs`: `ScopeKind` variants are now `Platform`, `Tenant`, `ObjectKind`, `ObjectType`, `Object`; new `ObjectKind` enum with the 8 canonical kinds + `as_str()` helper.
- [x] `models/policy.rs`: `tenant_id: Option<Uuid>` added to `PolicyBinding` and `CreatePolicyBinding`; doc-comment updated.
- [x] `auth.rs`: `has_global_manage` SQL now matches `scope_kind = 'platform'`.
- [x] `authz/engine.rs`: `ProtectedObject` extended with `coarse_kind`; `scope_matches` rewritten for the five new variants (Tenant matches by direct tenant_id; Platform matches all; ObjectType requires namespaced ref).
- [x] `authz/repo.rs`: every scope-matching SQL JOIN translated to the new vocabulary; `tenant_id` added to inserts/selects on `policy_bindings`.
- [x] `src/lib.rs` introduced so integration tests in `tests/` can import `atom::*`; `main.rs` trimmed to `use atom::*`.
- [x] Existing engine unit tests rewritten for the new enum and signature; new tests added for `Platform`/`Tenant`/`ObjectKind`/`ObjectType`/`Object`.

**Tests shipped — all passing.**
- Unit (`cargo test --bin atom`, no DB): 19 tests pass, including:
  - `scope_kind_serde_round_trip` (all 5 variants)
  - `object_kind_serialises_to_canonical_strings` (incl. `audit_log` snake_case)
  - `scope_matches` per variant: `Platform`, `Tenant`, `ObjectKind`, `ObjectType` (namespaced + bare-rejection), `Object` (incl. None scope_ref)
- Integration (`tests/m1_schema.rs`, `cargo test --test m1_schema -- --ignored`): 9 tests pass.
  - Migration is idempotent.
  - `policy_bindings.tenant_id` and `audit_logs.tenant_id` columns exist.
  - `tenant_memberships` accepts inserts and rejects PK duplicates.
  - `capability_assignment_rules` has `object_type` (not `resource_kind`); rule insert round-trips.
  - Admin seed migrated to `scope_kind = 'platform'`.
  - All 15 canonical capabilities are seeded.
  - CHECK constraint rejects `'all'` and accepts the 5 new values.
- Functional / e2e (`tests/m1_authz.rs`, `cargo test --test m1_authz -- --ignored`): 4 tests pass.
  - Admin's migrated platform binding still authorises after migration (regression).
  - `object_type = resource:channel` matches channels and rejects other resource kinds.
  - `object` scope matches a specific resource UUID and rejects others.
  - `object_kind = resource` matches every resource kind.
- Regression: 9 pre-existing DB tests in `authz::engine::db_tests` and `tenants::repo::tests` still pass.
- Lint: `cargo clippy --tests -- -D warnings` clean; `cargo fmt --check` clean.

---

## M2 — Object_type namespacing

**Goal.** Enforce the `<kind>:<sub-kind>` form across the API and migrate any straggling bare values. Translate legacy `resource_kind` at the HTTP edge.

**Closes:** AZ-17, AZ-19, structural #2, #18.

**Tasks.**
- [x] `src/authz/compat.rs` with `translate_legacy_scope` helper covering the three legacy values + pass-through.
- [x] Custom `Deserialize` for `CreatePolicyBinding` that runs the translator at the API edge (legacy form never reaches storage).
- [x] `validate()` method on `CreatePolicyBinding` enforcing scope_kind/scope_ref consistency (notably: `object_type` must be namespaced).
- [x] Handler `create_policy` invokes `req.validate()` before persisting.
- [x] PDP: `resolve_object` now supports `object_kind = entity` (AZ-17), with the entity row contributing the sub-kind for `object_type`.
- [ ] Audit `details` enrichment with `object_kind` / `object_type` is deferred to M7 (audit tenanting).

**Tests shipped — all passing.**
- Unit (in `src/authz/compat.rs` and `src/models/policy.rs`):
  - `translate_legacy_scope`: 7 cases (`all` / `resource` / `resource_kind` bare and namespaced / no scope_ref / canonical pass-through / unknown pass-through).
  - `CreatePolicyBinding::deserialize`: 4 cases (legacy `all` / `resource_kind` / `resource` / unknown helpful error).
  - `CreatePolicyBinding::validate`: 5 cases (`object_type` bare-rejection / namespaced / tenant non-UUID / tenant valid / object missing / platform OK).
- Integration (`tests/m2_compat.rs`, `--ignored`): 5 tests pass.
  - Legacy `resource_kind=channel` → stored as `object_type=resource:channel`.
  - Legacy `all` → stored as `platform`.
  - Entity-as-object (`object_kind=entity`) authz works for specific UUID + denies others.
  - `object_type=entity:device` matches device entities and rejects services.
  - Admin's platform scope inherits into entity-as-object checks.

---

## M3 — Tenant lifecycle in PDP

**Goal.** PDP denies authz checks for objects inside inactive/frozen/deleted tenants and explains why.

**Closes:** TEN-14, AZ-16, AUD-8.

**Tasks.**
- [x] `AuthzResponse::details: Option<Value>` added (with `allow()` / `deny()` / `deny_with_details()` constructors). gRPC unaffected.
- [x] `check_tenant_lifecycle` helper short-circuits `evaluate` and `explain` when the resolved object's owning tenant is not `active`. Reason text "tenant is &lt;state&gt;" matches PRD examples.
- [x] Resolved tenant rows are now loaded regardless of status so the engine deny can carry state-aware reasons (deleted no longer surfaces as "not found").
- [x] HTTP `check` and `explain` audit handlers merge `response.details` (incl. `tenant_id`, `tenant_status`) into the audit JSON.

**Tests shipped — all passing.**
- Integration (`tests/m3_lifecycle.rs`, `--ignored`): 6 tests pass.
  - inactive / frozen / deleted tenant denies with correct reason + details.
  - resource scoped to a frozen tenant denies (parent-tenant lifecycle propagates).
  - platform resource (`tenant_id = NULL`) is unaffected.
  - `/authz/explain` returns the lifecycle reason with no matched binding.
- Regression: existing `db_tests::deleted_tenant_*` updated to assert the new state-aware reason and now also asserts the structured details payload.

---

## M4 — Platform & tenant policy inheritance

**Goal.** PDP recognises `scope_kind=platform` as top-of-hierarchy (super admin via inheritance) and `scope_kind=tenant` as inheritance into all tenant-scoped objects. Replace the global `RequireManage` with a scope-aware management gate.

**Closes:** TEN-4, TEN-8, TEN-10, AM-4, AM-10, AM-11, AUTH-9, AUTH-10, AZ-10 (full), AZ-18 (full); structural #9, #10.

**Tasks.**
- [x] `scope_matches`: `Platform` matches any object; `Tenant` matches when `binding.scope_ref == object.tenant_id` (UUID compare). Tenant-owned bindings (`policy_bindings.tenant_id`) are additionally bounded to objects in that tenant so `object_kind`/`object_type` policies cannot leak globally.
- [x] Replaced the global-only gate with `has_capability_in_scope(pool, entity_id, capability_name, scope)` where scope covers `Platform`, `Tenant(uuid)`, and `Object(uuid)`. `has_global_manage` remains as a compatibility wrapper for the `RequireManage` extractor.
- [x] Per-endpoint guards:
  - Tenant lifecycle endpoints now require `tenant.manage` at `Platform`.
  - Tenant-scoped resource/entity/group mutations require `manage` at `Tenant(<owning-tenant>)`; global objects require platform scope.
  - Tenant-scoped role mutations require `role.manage` at `Tenant(<owning-tenant>)`.
  - Tenant-scoped policy mutations require `policy.manage` at `Tenant(<owning-tenant>)`.
  - Credential operations require `credential.manage` on the target entity's tenant, platform for global entities, or an explicit exact-object grant.
  - Admin hygiene endpoints continue to require platform `manage`.
- [x] Tenant-owned policy validation: tenant-owned policies cannot use platform scope, cannot target another tenant via `scope_kind=tenant`, and exact-object scopes are rejected unless the referenced object is known to belong to the policy tenant.

**Tests shipped.**
- Unit (`cargo test`): scope-match table now includes tenant-owned binding boundary checks, alongside the existing platform/tenant/object-kind/object-type/object cases.
- Integration / functional (`tests/m4_inheritance.rs`, `--ignored`, requires `DATABASE_URL`): 5 tests compile and are ready to run:
  - Tenant scope inherits only to resources in the same tenant and not to other tenants or global resources.
  - Seeded platform admin `manage` satisfies tenant-scoped gates by platform inheritance.
  - Tenant `policy.manage` authority does not leak to other tenants or platform scope.
  - Tenant-owned `object_kind` policies are bounded by `policy_bindings.tenant_id`.
  - Tenant-scoped `tenant.manage` does not satisfy the platform tenant lifecycle gate.
- Verification in this workspace: `cargo test`, `cargo fmt --check`, and `cargo clippy --tests -- -D warnings` pass. DB-gated ignored tests were not executed because `DATABASE_URL` is not set in the environment.

---

## M5 — Tenant-admin bootstrap

**Goal.** When a tenant is created, Atom creates the `tenant-admin` role, seeds it with the right capabilities (no `tenant.manage`), binds it to the creator, and adds a `tenant_memberships` row for human creators.

**Closes:** TEN-12, TEN-13, TEN-16; closes ID-10 wiring (the table from M1 gets used).

**Tasks.**
- [x] On `POST /tenants` success, in a single transaction:
  - Insert role `tenant-admin` with `tenant_id = <new tenant>`.
  - Attach capabilities `manage`, `audit.read`, `credential.manage`, `policy.manage`, `role.manage`.
  - Insert policy binding: subject = creator, grant = role, scope = `tenant`, scope_ref = `<new tenant>`, tenant_id = `<new tenant>`.
  - If creator's `kind=human`, insert `tenant_memberships(tenant_id, entity_id, status='active')`.

**Tests shipped.**
- Unit: `tenant_admin_bootstrap_plan_matches_m5_contract` verifies the pure bootstrap plan and that `tenant.manage` is intentionally excluded.
- Integration / functional (`tests/m5_bootstrap.rs`, `--ignored`, requires `DATABASE_URL`): 3 tests compile and are ready to run:
  - Tenant creation bootstraps the role, five capabilities, binding, and human membership.
  - Non-human creators receive the policy binding but no tenant membership row.
  - Creator can immediately `manage` the created tenant and cannot manage another tenant.
- Verification in this workspace after M5: `cargo test`, `cargo fmt --check`, and `cargo clippy --tests -- -D warnings` pass. DB-gated ignored tests were not executed because `DATABASE_URL` is not set in the environment.

---

## M6 — ABAC operator & context expansion

**Goal.** ABAC supports `eq`/`neq`/`contains`/`in`/`gt`/`gte`/`lt`/`lte` operators and the full evaluation context (entity.kind/status, tenant.status, object.kind/type/tenant_id, etc.).

**Closes:** AZ-11 (full), AZ-20, AZ-21 (existing), AZ-22 (existing); structural #4 (full), #5 (already ✅).

**Tasks.**
- [x] New `authz/conditions.rs` evaluator for literal values and operator objects (`eq`, `neq`, `contains`, `in`, `gt`, `gte`, `lt`, `lte`).
- [x] `conditions_match` now consumes the parsed operator form and fails closed for missing fields or unsupported operators.
- [x] `build_context` populates entity (`id`, `kind`, `status`, `tenant_id`, `attributes`), resource compatibility fields, object (`id`, `kind`, `type`, `tenant_id`, `attributes`), tenant (`id`, `status`, `attributes`), and request context fields.

**Tests shipped.**
- Unit: one test per operator group, mixed condition AND logic, missing-field fail-closed, and context-shape assertions.
- Integration / functional (`tests/m6_conditions.rs`, `--ignored`, requires `DATABASE_URL`): 3 tests compile and are ready to run:
  - `gte` timestamp against request context gates access.
  - Conditions over `object.type`, `tenant.status`, `contains`, `in`, and numeric `gte` apply.
  - Missing expanded context field fails closed.
- Verification in this workspace after M6: `cargo test`, `cargo fmt --check`, and `cargo clippy --tests -- -D warnings` pass. DB-gated ignored tests were not executed because `DATABASE_URL` is not set in the environment.

---

## M7 — Audit tenanting & login event

**Goal.** Audit rows carry `tenant_id` for tenant-scoped events; tenant-admins see only their tenant; a distinct `auth.login` event is emitted on successful login.

**Closes:** AUD-1, AUD-6, AUD-7.

**Tasks.**
- [x] `audit::write()` accepts an optional `tenant_id`. Handlers and services pass known tenant context for logout, credential events, authz check/explain, bulk checks, and login.
- [x] `GET /audit` filters tenant-admin callers to tenant IDs where they hold `audit.read`; platform `audit.read` or platform `manage` can read globally.
- [x] `identity::service::login_password` emits `auth.login` with outcome, entity_id, and tenant_id when available.

**Tests shipped.**
- Integration / functional (`tests/m7_audit.rs`, `--ignored`, requires `DATABASE_URL`): 3 tests compile and are ready to run:
  - `audit::write` persists `tenant_id`.
  - Tenant audit filtering returns only allowed tenant rows.
  - Successful login emits `auth.login` allow with the entity ID in details.
- Verification in this workspace after M7: `cargo test`, `cargo fmt --check`, and `cargo clippy --tests -- -D warnings` pass. DB-gated ignored tests were not executed because `DATABASE_URL` is not set in the environment.

---

## M8 — Capability assignment guardrails

**Goal.** Build out the guardrails subsystem: rules table is exercised, validation hooks fire on policy/role/group changes, audit captures rejects and overrides.

**Closes:** GR-1, GR-2, GR-3 (Should), GR-4, GR-5, GR-6, GR-7, GR-8, GR-9, GR-11, GR-12 (verifiable now), GR-13, GR-14 (Should), GR-15.

**Tasks.**
- [x] `guardrails.rs` module with rule loader, decision evaluator, and precedence ordering. Absolute denies outrank tenant/global allows; unsupported or `require_override` decisions fail closed for now.
- [x] Migration `006_guardrail_defaults.sql` seeds global default rules, including device denies for administrative resource capabilities and publish/subscribe allows on `resource:channel`.
- [x] Validation hooks at:
  - policy create (direct grants, role grants, and group policies by expanding current members);
  - role capability change (expands existing role holders);
  - group membership change (validates policies the new member would inherit).
- [x] `require_override` request flow remains deferred until OQ2 is decided; current behavior rejects with a clear error.

**Tests shipped.**
- Unit: precedence ordering, unmatched assignment behavior, and matching object-type allow.
- Integration / functional (`tests/m8_guardrails.rs`, `--ignored`, requires `DATABASE_URL`): 3 tests compile and are ready to run:
  - Direct grant rejects device `manage` on resources and persists no policy row.
  - Role capability addition rejects when an existing device role holder would inherit denied capability.
  - Group membership rejects a device that would inherit denied policy.
- Verification in this workspace after M8: `cargo test`, `cargo fmt --check`, and `cargo clippy --tests -- -D warnings` pass. DB-gated ignored tests were not executed because `DATABASE_URL` is not set in the environment.

---

## Cross-cutting test scope

Each milestone ships with three test layers:

1. **Unit tests** (`#[cfg(test)] mod tests` next to the code, no DB): covers pure logic and parsing.
2. **Integration tests** (`tests/<milestone>_<area>.rs`, `#[ignore]`, requires `DATABASE_URL`): covers DB schema and queries.
3. **Functional / end-to-end tests** (`tests/<milestone>_e2e.rs`, `#[ignore]`): covers the HTTP/gRPC contract via an in-process Axum router.

Run unit tests with `cargo test`. Run DB-gated tests with `cargo test -- --ignored` once Postgres is reachable.

---

## Open questions tracked separately

The PRD's Open Questions (1–4 after the cleanups) are decided per-milestone:

- OQ1 (large-group validation) — addressed in M8.
- OQ2 (`require_override` flow) — deferred; M8 ships without it.
- OQ3 (session validation cadence) — out of MVP scope; not blocking M1–M8.
- OQ4 (multi-replica migrations) — out of MVP scope; not blocking M1–M8.
