pub(crate) const HTML_BEFORE_CSS: &str = r######"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Atom API Builder</title>
  <style>
"######;

pub(crate) const HTML_AFTER_CSS_BEFORE_JS: &str = r######"  </style>
</head>
<body>
  <div class="app-shell">
    <aside class="side-nav">
      <div class="side-scroll">
        <div class="brand">
          <h1>Atom API Builder</h1>
          <div class="subtitle">Hasura-like developer console and API Builder for generic Atom GraphQL.</div>
        </div>

        <section class="nav-section">
          <h2>Start</h2>
          <div class="nav-list">
            <button class="nav-button active" data-screen="start"><strong>What do you want to do?</strong><span>Pick a task</span></button>
            <button class="nav-button" data-screen="guided"><strong>Guided setup</strong><span>Tenant to authz check</span></button>
            <button class="nav-button" data-screen="api-builder"><strong>API Builder</strong><span>Reusable recipes</span></button>
            <button class="nav-button" data-screen="api-endpoints"><strong>API Endpoint Builder</strong><span>Custom HTTP endpoints</span></button>
            <button class="nav-button" data-screen="login"><strong>login helper</strong><span>Get a token</span></button>
          </div>
        </section>

        <section class="nav-section">
          <h2>Build</h2>
          <div class="nav-list">
            <button class="nav-button" data-screen="tenant"><strong>Tenant builder</strong><span>Create an isolation boundary</span></button>
            <button class="nav-button" data-screen="profile"><strong>Profile builder</strong><span>Define subtype and schema</span></button>
            <button class="nav-button" data-screen="entity"><strong>Entity builder</strong><span>People, devices, services</span></button>
            <button class="nav-button" data-screen="resource"><strong>Resource builder</strong><span>Protected objects</span></button>
            <button class="nav-button" data-screen="policy"><strong>Policy builder</strong><span>Allow access</span></button>
            <button class="nav-button" data-screen="authz"><strong>Authz builder</strong><span>Test access</span></button>
            <button class="nav-button" data-screen="credentials"><strong>Credential builder</strong><span>Passwords and API keys</span></button>
          </div>
        </section>

        <section class="nav-section">
          <h2>Advanced</h2>
          <div class="nav-list">
            <button class="nav-button" data-screen="advanced"><strong>Advanced GraphQL</strong><span>operation explorer</span></button>
            <button class="nav-button" data-screen="assistant"><strong>AI Assistant placeholder</strong><span>Copyable prompt only</span></button>
          </div>
        </section>

        <section class="nav-section">
          <h2>Model Rules</h2>
          <p class="help">Atom GraphQL is generic. Tenant, entity, resource, policy, profile, and credential operations are the source of truth.</p>
          <p class="help"><strong>kind</strong> is Atom internal runtime/authz kind. <strong>profile</strong> is the user or domain subtype. <strong>profileVersion</strong> is validation and history only.</p>
        </section>
      </div>
    </aside>

    <main class="main">
      <div class="topbar">
        <label>Endpoint
          <input id="endpoint" value="/graphql" autocomplete="off" />
        </label>
        <label>Authorization Bearer token
          <input id="token" type="password" autocomplete="off" placeholder="Use login helper or paste token" />
        </label>
        <button id="clearToken">Clear token</button>
        <button id="refreshSchema">Refresh schema</button>
        <div id="connectionStatus" class="status-row" style="grid-column: 1 / -1;">
          <span class="badge warn">Schema not loaded</span>
          <span class="badge warn">Not authenticated</span>
        </div>
        <div id="endpointWarning" class="notice danger hidden" style="grid-column: 1 / -1;"></div>
      </div>

      <div class="workspace">
        <section id="screen-start" class="screen active">
          <div class="panel tint">
            <h1>What do you want to do?</h1>
            <p class="help">Use plain mode by default. Each task generates generic Atom GraphQL, shows variables, and keeps the advanced query view collapsed until you need it.</p>
            <p class="help">Includes login helper, operation explorer, entity builder, resource builder, policy builder, authz builder, and AI Assistant placeholder.</p>
          </div>
          <div class="task-grid">
            <button class="task-card" data-screen="guided"><strong>Set up a tenant</strong><span class="help">Create a tenant, entity, resource, policy, and access test in one guided flow.</span></button>
            <button class="task-card" data-screen="entity"><strong>Add a person, device, or service</strong><span class="help">Create an entity from an entity profile and schema-backed attributes.</span></button>
            <button class="task-card" data-screen="resource"><strong>Create a protected object</strong><span class="help">Create a generic resource such as a channel, file, report, or workflow.</span></button>
            <button class="task-card" data-screen="policy"><strong>Allow access</strong><span class="help">Build an allow or deny policy with subject, grant, scope, and conditions.</span></button>
            <button class="task-card" data-screen="authz"><strong>Test access</strong><span class="help">Run an authorization check or privileged/admin explanation.</span></button>
            <button class="task-card" data-screen="credentials"><strong>Manage credentials</strong><span class="help">List credentials, create API keys, set passwords, and revoke credentials.</span></button>
            <button class="task-card" data-screen="advanced"><strong>Explore GraphQL directly</strong><span class="help">Use the GraphQL Operation Explorer in Advanced GraphQL.</span></button>
            <button class="task-card" data-screen="api-builder"><strong>Use API Builder recipes</strong><span class="help">Generate GraphQL, curl, and JavaScript snippets from reusable recipes.</span></button>
          </div>
        </section>

        <section id="screen-login" class="screen">
          <div class="panel">
            <h1>login helper</h1>
            <p class="help">Login is public. Tokens can be stored in localStorage for this browser, and Authorization is sent only to same-origin <code>/graphql</code>.</p>
            <div class="grid-3">
              <label>Identifier<input id="loginIdentifier" value="atom-admin" autocomplete="username" /></label>
              <label>Secret<input id="loginSecret" type="password" value="change-me" autocomplete="current-password" /></label>
              <label>Credential kind<input id="loginKind" value="password" /></label>
            </div>
            <div class="actions">
              <button class="primary" id="runLogin">Run login</button>
              <button id="copyLoginMutation">Copy GraphQL</button>
            </div>
            <div id="loginSummary" class="summary-box">No login request yet.</div>
            <details>
              <summary>Advanced GraphQL preview</summary>
              <pre id="loginGraphql"></pre>
            </details>
            <details>
              <summary>Variables</summary>
              <pre id="loginVariables"></pre>
            </details>
            <details>
              <summary>Raw result</summary>
              <pre id="loginResult"></pre>
            </details>
          </div>
        </section>

        <section id="screen-guided" class="screen">
          <div class="panel tint">
            <h1>Guided setup wizard</h1>
            <p class="help">Create a tenant, add an entity, create a resource, grant access, and test the decision. Each step shows a result summary and suggests the next action.</p>
          </div>
          <div class="wizard-nav">
            <button class="wizard-tab active" data-step="1"><strong>Step 1</strong><br><span class="help">Create tenant</span></button>
            <button class="wizard-tab" data-step="2"><strong>Step 2</strong><br><span class="help">Add entity</span></button>
            <button class="wizard-tab" data-step="3"><strong>Step 3</strong><br><span class="help">Create resource</span></button>
            <button class="wizard-tab" data-step="4"><strong>Step 4</strong><br><span class="help">Grant access</span></button>
            <button class="wizard-tab" data-step="5"><strong>Step 5</strong><br><span class="help">Test access</span></button>
          </div>

          <div id="wizard-step-1" class="panel wizard-step active">
            <h2>Step 1: Create tenant</h2>
            <p class="help">A tenant is an isolation boundary for related identities and protected objects.</p>
            <div class="grid">
              <label>Tenant name<input id="setupTenantName" value="factory-a" /></label>
              <label>Tenant route<input id="setupTenantRoute" value="factory-a" /></label>
            </div>
            <label>Attributes JSON<textarea id="setupTenantAttributes" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button id="previewSetupTenant">Preview</button>
              <button class="primary" id="runSetupTenant">Run createTenant</button>
            </div>
            <div id="setupTenantSummary" class="summary-box">Next: create the tenant, then add an entity inside it.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="setupTenantGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="setupTenantVariables"></pre></details>
            <div class="actions"><button data-step="2" class="next-step">Next: Add entity</button></div>
          </div>

          <div id="wizard-step-2" class="panel wizard-step">
            <h2>Step 2: Add entity</h2>
            <p class="help">An entity is a principal: a person, device, service, workload, or application.</p>
            <div class="grid">
              <label>Tenant ID<input id="setupEntityTenantId" placeholder="created tenant id" /></label>
              <label>Internal kind<select id="setupEntityKind"><option>human</option><option selected>device</option><option>service</option><option>workload</option><option>application</option></select></label>
              <label>Profile<select id="setupEntityProfile"></select></label>
              <label>Profile version<select id="setupEntityProfileVersion"></select></label>
              <label>Name<input id="setupEntityName" value="device-001" /></label>
            </div>
            <div class="actions">
              <button id="loadSetupProfiles">Load profiles</button>
              <span id="setupEntityBadges" class="status-row"></span>
            </div>
            <label>Attributes JSON<textarea id="setupEntityAttributes" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button id="previewSetupEntity">Preview</button>
              <button class="primary" id="runSetupEntity">Run createEntity</button>
            </div>
            <div id="setupEntitySummary" class="summary-box">Next: add an entity from a profile or internal kind, then create a protected resource.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="setupEntityGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="setupEntityVariables"></pre></details>
            <div class="actions"><button data-step="3" class="next-step">Next: Create resource</button></div>
          </div>

          <div id="wizard-step-3" class="panel wizard-step">
            <h2>Step 3: Create resource</h2>
            <p class="help">Resources are things protected by Atom policies.</p>
            <div class="grid">
              <label>Tenant ID<input id="setupResourceTenantId" placeholder="created tenant id" /></label>
              <label>Resource kind<input id="setupResourceKind" value="channel" /></label>
              <label>Name<input id="setupResourceName" value="telemetry" /></label>
            </div>
            <label>Attributes JSON<textarea id="setupResourceAttributes" spellcheck="false">{"topic":"telemetry"}</textarea></label>
            <div class="actions">
              <button id="previewSetupResource">Preview</button>
              <button class="primary" id="runSetupResource">Run createResource</button>
            </div>
            <div id="setupResourceSummary" class="summary-box">Next: create the protected object, then grant an entity access to it.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="setupResourceGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="setupResourceVariables"></pre></details>
            <div class="actions"><button data-step="4" class="next-step">Next: Grant access</button></div>
          </div>

          <div id="wizard-step-4" class="panel wizard-step">
            <h2>Step 4: Grant access</h2>
            <p class="help">An allow policy connects who can do what on which scope. Deny policies override allow policies.</p>
            <div class="grid">
              <label>Tenant ID<input id="setupPolicyTenantId" placeholder="created tenant id" /></label>
              <label>Subject ID<input id="setupPolicySubjectId" placeholder="created entity id" /></label>
              <label>Capability ID<input id="setupPolicyGrantId" placeholder="capability uuid" /></label>
              <label>Resource ID<input id="setupPolicyResourceId" placeholder="created resource id" /></label>
              <label>Effect<select id="setupPolicyEffect"><option selected>allow</option><option>deny</option></select></label>
            </div>
            <label>Conditions JSON<textarea id="setupPolicyConditions" spellcheck="false">{}</textarea></label>
            <div id="setupPolicyHumanSummary" class="summary-box">Allow subject to capability on resource.</div>
            <div class="actions">
              <button id="previewSetupPolicy">Preview</button>
              <button class="primary" id="runSetupPolicy">Run createPolicy</button>
            </div>
            <div id="setupPolicySummary" class="summary-box">Next: run an authorization check with the subject, action, and resource.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="setupPolicyGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="setupPolicyVariables"></pre></details>
            <div class="actions"><button data-step="5" class="next-step">Next: Test access</button></div>
          </div>

          <div id="wizard-step-5" class="panel wizard-step">
            <h2>Step 5: Test access</h2>
            <p class="help">Use the same subject, action, and resource that your application would ask Atom to authorize.</p>
            <div class="grid">
              <label>Subject ID<input id="setupAuthzSubjectId" placeholder="created entity id" /></label>
              <label>Action<input id="setupAuthzAction" value="publish" /></label>
              <label>Resource ID<input id="setupAuthzResourceId" placeholder="created resource id" /></label>
            </div>
            <label>Context JSON<textarea id="setupAuthzContext" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button id="previewSetupAuthz">Preview</button>
              <button class="primary" id="runSetupAuthz">Run authzCheck</button>
            </div>
            <div id="setupAuthzSummary" class="summary-box">Run the check to see Allowed or Denied with a reason.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="setupAuthzGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="setupAuthzVariables"></pre></details>
          </div>
        </section>

        <section id="screen-tenant" class="screen">
          <div class="panel">
            <h1>Tenant builder</h1>
            <p class="help">Create an isolation boundary. Domain-like applications call <code>createTenant</code>.</p>
            <div class="grid">
              <label>Name<input id="tenantName" value="factory-a" /></label>
              <label>Route<input id="tenantRoute" value="factory-a" /></label>
            </div>
            <label>Tags, comma separated<input id="tenantTags" placeholder="iot, production" /></label>
            <label>Attributes JSON<textarea id="tenantAttributes" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button id="previewTenant">Preview</button>
              <button class="primary" id="runTenant">Run createTenant</button>
            </div>
            <div id="tenantSummary" class="summary-box">What happened? No tenant request yet.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="tenantGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="tenantVariables"></pre></details>
            <details><summary>Raw result</summary><pre id="tenantResult"></pre></details>
          </div>
        </section>

        <section id="screen-profile" class="screen">
          <div class="panel">
            <h1>Profile builder</h1>
            <p class="help">A profile is a user/domain subtype and schema. It is separate from Atom internal kind.</p>
            <div class="grid">
              <label>Tenant<select id="profileTenantSelect"></select></label>
              <label>Or tenant ID<input id="profileTenantId" placeholder="optional tenant uuid" /></label>
              <label>Object kind<input id="profileObjectKind" value="entity" /></label>
              <label>Internal kind<select id="profileKind"><option>human</option><option selected>device</option><option>service</option><option>workload</option><option>application</option></select></label>
              <label>Profile key<input id="profileKey" value="client" /></label>
              <label>Display name<input id="profileDisplayName" value="Client" /></label>
              <label>Status<input id="profileStatus" value="active" /></label>
              <label>Description<input id="profileDescription" placeholder="optional description" /></label>
            </div>
            <div class="actions">
              <button id="loadTenantsForProfile">Load tenants</button>
              <button id="previewProfile">Preview createProfile</button>
              <button class="primary" id="runProfile">Run createProfile</button>
            </div>
            <h3>Profile version</h3>
            <p class="help">Profile versions hold JSON Schema for validation and history only.</p>
            <div class="grid">
              <label>Profile ID<input id="profileVersionProfileId" placeholder="profile uuid" /></label>
              <label>Version<input id="profileVersionNumber" type="number" value="1" /></label>
              <label>Status<input id="profileVersionStatus" value="active" /></label>
            </div>
            <label>JSON Schema<textarea id="profileJsonSchema" spellcheck="false">{"type":"object","properties":{"serial_no":{"type":"string"}}}</textarea></label>
            <label>UI Schema<textarea id="profileUiSchema" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button id="previewProfileVersion">Preview createProfileVersion</button>
              <button class="primary" id="runProfileVersion">Run createProfileVersion</button>
              <button id="refreshProfilesAfterProfile">Refresh entity profiles</button>
            </div>
            <div id="profileSummary" class="summary-box">What happened? No profile request yet.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="profileGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="profileVariables"></pre></details>
            <details><summary>Raw result</summary><pre id="profileResult"></pre></details>
          </div>
        </section>

        <section id="screen-entity" class="screen">
          <div class="panel">
            <h1>Entity builder</h1>
            <p class="help">Create people, devices, services, workloads, or applications. Select a profile first; Atom derives internal kind from the selected profile.</p>
            <h3>Profile picker</h3>
            <div class="grid">
              <label>Tenant<select id="entityTenantSelect"></select></label>
              <label>Or tenant ID<input id="entityTenantId" placeholder="optional tenant uuid" /></label>
              <label>Profile kind<select id="entityKind"><option>human</option><option selected>device</option><option>service</option><option>workload</option><option>application</option></select></label>
              <label>Profile<select id="entityProfile"></select></label>
              <label>Profile version<select id="entityProfileVersion"></select></label>
              <label>Name<input id="entityName" placeholder="entity-001" /></label>
            </div>
            <div class="actions">
              <button id="loadTenantsForEntity">Load tenants</button>
              <button id="loadProfiles">Load profiles</button>
              <span id="entityBadges" class="status-row" aria-label="Derived kind badge"></span>
            </div>
            <h3>Schema form</h3>
            <div id="schemaForm" class="grid"></div>
            <label>Attributes JSON fallback<textarea id="entityAttributes" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button id="previewEntity">Preview</button>
              <button class="primary" id="runEntity">Run createEntity</button>
            </div>
            <div id="entitySummary" class="summary-box">What happened? No entity request yet.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="entityGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="entityVariables"></pre></details>
            <details><summary>Raw result</summary><pre id="entityResult"></pre></details>
          </div>
        </section>

        <section id="screen-resource" class="screen">
          <div class="panel">
            <h1>Resource builder</h1>
            <p class="help">Resources are things protected by Atom policies.</p>
            <p class="help">A channel is just <code>createResource(kind: "channel")</code>.</p>
            <div class="grid">
              <label>Tenant<select id="resourceTenantSelect"></select></label>
              <label>Or tenant ID<input id="resourceTenantId" placeholder="optional tenant uuid" /></label>
              <label>Resource kind<select id="resourceKindPreset"><option selected>channel</option><option>file</option><option>report</option><option>workflow</option><option>custom</option></select></label>
              <label>Custom kind<input id="resourceKindCustom" placeholder="custom kind" /></label>
              <label>Name<input id="resourceName" value="telemetry" /></label>
              <label>Owner ID<input id="resourceOwnerId" placeholder="optional entity uuid" /></label>
            </div>
            <label>Attributes JSON<textarea id="resourceAttributes" spellcheck="false">{"topic":"telemetry"}</textarea></label>
            <div class="actions">
              <button id="loadTenantsForResource">Load tenants</button>
              <button id="previewResource">Preview</button>
              <button class="primary" id="runResource">Run createResource</button>
            </div>
            <div id="resourceSummary" class="summary-box">What happened? No resource request yet.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="resourceGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="resourceVariables"></pre></details>
            <details><summary>Raw result</summary><pre id="resourceResult"></pre></details>
          </div>
        </section>

        <section id="screen-policy" class="screen">
          <div class="panel">
            <h1>Policy builder</h1>
            <p class="help">Build a visual policy from who, can do, on, and when. Connection-like applications call <code>createPolicy</code>.</p>
            <div class="builder-sections">
              <div class="mini-panel">
                <h2>WHO</h2>
                <label>Subject kind<select id="policySubjectKind"><option selected>entity</option><option>group</option></select></label>
                <label>Subject selector<select id="policySubjectSelect"></select></label>
                <label>Or manual subject ID<input id="policySubjectId" /></label>
                <div class="actions"><button id="loadPolicySubjects">Load subjects</button></div>
              </div>
              <div class="mini-panel">
                <h2>CAN DO</h2>
                <label>Grant kind<select id="policyGrantKind"><option selected>capability</option><option>role</option></select></label>
                <label>Capability or role<select id="policyGrantSelect"></select></label>
                <label>Or manual grant ID<input id="policyGrantId" /></label>
                <div class="actions"><button id="loadPolicyGrants">Load grants</button></div>
              </div>
              <div class="mini-panel">
                <h2>ON</h2>
                <label>Scope kind<select id="policyScopeKind"><option>platform</option><option>tenant</option><option>object_kind</option><option>object_type</option><option selected>object</option></select></label>
                <label>Tenant<select id="policyTenantSelect"></select></label>
                <label>Resource or entity<select id="policyObjectSelect"></select></label>
                <label>Scope ref<input id="policyScopeRef" placeholder="resource uuid, tenant uuid, or kind" /></label>
                <div class="actions"><button id="loadPolicyObjects">Load objects</button></div>
              </div>
              <div class="mini-panel">
                <h2>WHEN</h2>
                <label>Conditions JSON<textarea id="policyConditions" spellcheck="false">{}</textarea></label>
              </div>
              <div class="mini-panel">
                <h2>EFFECT</h2>
                <label>Effect<select id="policyEffect"><option selected>allow</option><option>deny</option></select></label>
              </div>
            </div>
            <div id="policyHumanSummary" class="summary-box">Allow subject to capability on scope.</div>
            <div class="actions">
              <button id="loadTenantsForPolicy">Load tenants</button>
              <button id="previewPolicy">Preview</button>
              <button class="primary" id="runPolicy">Run createPolicy</button>
              <button id="copyPolicyGraphql">Copy GraphQL</button>
              <button id="copyPolicyCurl">Copy curl</button>
              <button id="copyPolicyJs">Copy JavaScript</button>
              <button id="savePolicyTemplate">Save as API template</button>
              <button id="testThisPolicy">Test this policy</button>
            </div>
            <div id="policySummary" class="summary-box">What happened? No policy request yet.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="policyGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="policyVariables"></pre></details>
            <details><summary>curl</summary><pre id="policyCurl"></pre></details>
            <details><summary>JavaScript fetch</summary><pre id="policyJs"></pre></details>
            <details><summary>Raw result</summary><pre id="policyResult"></pre></details>
          </div>
        </section>

        <section id="screen-authz" class="screen">
          <div class="panel">
            <h1>Authz builder</h1>
            <p class="help">Run an authorization decision against Atom's online PDP. Use authzCheck for normal application paths.</p>
            <div class="grid">
              <label>Subject ID<input id="authzSubjectId" /></label>
              <label>Action<input id="authzAction" value="publish" /></label>
              <label>Resource ID<input id="authzResourceId" /></label>
              <label>Object kind<input id="authzObjectKind" placeholder="optional object kind" /></label>
              <label>Object ID<input id="authzObjectId" placeholder="optional object uuid" /></label>
            </div>
            <label>Context JSON<textarea id="authzContext" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button class="primary" id="runAuthzCheck">Run authzCheck</button>
              <button id="runAuthzExplain">Run authzExplain (privileged/admin)</button>
              <button id="previewAuthz">Preview</button>
            </div>
            <div id="authzNarrative" class="summary-box">Run a check to see an Allowed or Denied badge, reason, details, and next suggested action.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="authzGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="authzVariables"></pre></details>
            <details><summary>Raw result</summary><pre id="authzResult"></pre></details>
          </div>
        </section>

        <section id="screen-credentials" class="screen">
          <div class="panel">
            <h1>Credential builder</h1>
            <p class="help">Manage credentials for an entity. API key plaintext is revealed once by Atom and cannot be recovered later.</p>
            <div class="grid">
              <label>Entity ID<input id="credentialEntityId" /></label>
              <label>Credential ID for revoke<input id="credentialId" /></label>
              <label>API key description<input id="apiKeyDescription" placeholder="automation key" /></label>
              <label>API key expires at<input id="apiKeyExpiresAt" placeholder="optional RFC3339 timestamp" /></label>
              <label>Password<input id="passwordSecret" type="password" placeholder="new password" /></label>
            </div>
            <div class="actions">
              <button id="listCredentials">List credentials</button>
              <button class="primary" id="createApiKey">Create API key</button>
              <button id="createPassword">Create password</button>
              <button id="revokeCredential">Revoke credential</button>
            </div>
            <div id="credentialSummary" class="summary-box">What happened? No credential request yet.</div>
            <details><summary>Advanced GraphQL preview</summary><pre id="credentialGraphql"></pre></details>
            <details><summary>Variables</summary><pre id="credentialVariables"></pre></details>
            <details><summary>Raw result</summary><pre id="credentialResult"></pre></details>
          </div>
        </section>

        <section id="screen-api-builder" class="screen">
          <div class="panel tint">
            <h1>API Builder</h1>
            <p class="help">Load saved API Templates from Atom, or use local recipes when the template metadata API is unavailable.</p>
          </div>
          <div class="panel">
            <div class="response-header status-row">
              <h2 style="margin-bottom: 0;">API Templates</h2>
              <span id="templateStatus" class="badge warn">Not loaded</span>
            </div>
            <div class="grid">
              <label>Tenant ID<input id="templateTenantId" placeholder="optional tenant uuid" /></label>
              <label>Status<select id="templateStatusFilter"><option value="active" selected>active</option><option value="">any</option><option value="draft">draft</option><option value="deprecated">deprecated</option><option value="disabled">disabled</option></select></label>
              <label>Tag<input id="templateTagFilter" placeholder="optional tag" /></label>
            </div>
            <div class="actions">
              <button id="refreshTemplates">Refresh templates</button>
              <button class="primary" id="runTemplate">Run selected template</button>
              <button id="saveTemplate">Save as template</button>
              <button id="updateTemplate">Update template</button>
              <button id="copyTemplateGraphql">Copy GraphQL</button>
              <button id="copyTemplateVariables">Copy variables</button>
              <button id="copyTemplateCurl">Copy curl</button>
              <button id="copyTemplateJs">Copy JavaScript fetch</button>
            </div>
            <div id="templateSummary" class="summary-box">Refresh templates to load saved Atom API Builder metadata.</div>
            <div id="templateSecretWarning" class="notice danger hidden"></div>
            <div class="split" style="margin-top: 12px;">
              <div>
                <h2>Template browser</h2>
                <div id="templateGroups" class="template-groups"></div>
              </div>
              <div>
                <h2>Variables Schema</h2>
                <pre id="templateVariablesSchema"></pre>
              </div>
            </div>
            <details><summary>curl</summary><pre id="templateCurl"></pre></details>
            <details><summary>JavaScript fetch</summary><pre id="templateJs"></pre></details>
            <details><summary>Raw response</summary><pre id="templateResult"></pre></details>
          </div>
          <div class="panel">
            <h2>Local recipe fallback</h2>
            <div class="grid">
              <label>Recipe<select id="recipeSelect"></select></label>
              <label>Saved recipes/examples<select id="savedRecipes"></select></label>
            </div>
            <div id="recipeForm" class="grid" style="margin-top: 10px;"></div>
            <div class="actions">
              <button id="generateRecipe">Generate</button>
              <button class="primary" id="runRecipe">Run it</button>
              <button id="copyRecipeGraphql">Copy GraphQL</button>
              <button id="copyRecipeCurl">Copy curl</button>
              <button id="copyRecipeJs">Copy JavaScript fetch snippet</button>
              <button id="saveRecipe">Save recipe</button>
              <button id="loadRecipe">Load saved</button>
            </div>
            <div id="recipeSummary" class="summary-box">Choose a recipe to start.</div>
            <div class="recipe-preview">
              <div>
                <h2>Generated GraphQL</h2>
                <pre id="recipeGraphql"></pre>
              </div>
              <div>
                <h2>Variables JSON</h2>
                <pre id="recipeVariables"></pre>
              </div>
            </div>
            <details><summary>curl</summary><pre id="recipeCurl"></pre></details>
            <details><summary>JavaScript fetch</summary><pre id="recipeJs"></pre></details>
            <details><summary>Raw result</summary><pre id="recipeResult"></pre></details>
          </div>
        </section>

        <section id="screen-api-endpoints" class="screen">
          <div class="panel tint">
            <h1>Custom API Endpoint</h1>
            <p class="help">API Endpoint Builder for management users. Runs saved GraphQL template metadata behind private REST paths under <code>/api/custom/</code>; REST and GraphQL semantics remain unchanged.</p>
            <div class="status-row">
              <span class="badge">Atom</span>
              <span class="badge warn">caller_context is recommended</span>
              <span class="badge warn">Do not store secrets, tokens, or API keys in templates or default variables.</span>
            </div>
          </div>
          <div class="panel">
            <div class="response-header status-row">
              <h2 style="margin-bottom: 0;">Endpoint list</h2>
              <span id="endpointListStatus" class="badge warn">Not loaded</span>
            </div>
            <div class="grid">
              <label>Tenant ID<input id="endpointTenantId" placeholder="optional tenant uuid" /></label>
              <label>Status filter<select id="endpointStatusFilter"><option value="">any</option><option selected>draft</option><option>active</option><option>disabled</option></select></label>
            </div>
            <div class="actions">
              <button id="loadApiEndpoints">Load endpoints</button>
              <button class="primary" id="startNewApiEndpoint">New endpoint</button>
            </div>
            <label class="hidden">Existing endpoint<select id="endpointSelect"></select></label>
            <div id="endpointList" class="endpoint-list">
              <div class="help">Load endpoints to see method, path, template, auth mode, status, last execution, timestamps, and actions.</div>
            </div>
          </div>

          <div class="panel">
            <div class="response-header status-row">
              <h2 style="margin-bottom: 0;">API Endpoint Builder</h2>
              <span id="endpointBuilderMode" class="badge">Draft</span>
            </div>
            <p class="help">Create or edit a Custom API Endpoint in four steps. Endpoint paths must remain under <code>/api/custom/</code>.</p>
            <div class="endpoint-wizard-nav">
              <button class="endpoint-wizard-tab active" data-endpoint-step="1"><strong>Choose template</strong><span>Pick saved GraphQL</span></button>
              <button class="endpoint-wizard-tab" data-endpoint-step="2"><strong>Configure route</strong><span>Method, path, auth</span></button>
              <button class="endpoint-wizard-tab" data-endpoint-step="3"><strong>Map request to variables</strong><span>Request mapping</span></button>
              <button class="endpoint-wizard-tab" data-endpoint-step="4"><strong>Test and publish</strong><span>Test request</span></button>
            </div>

            <div id="endpoint-step-1" class="endpoint-wizard-step active">
              <h2>Choose template</h2>
              <p class="help">Choose the saved template this endpoint will run. Atom templates keep the GraphQL operation reusable.</p>
              <div class="grid">
                <label>Search<input id="endpointTemplateSearch" placeholder="template name, key, description" /></label>
                <label>Filter by tag<input id="endpointTemplateTagFilter" placeholder="setup, authz, profile" /></label>
                <label class="hidden">Saved API template<select id="endpointTemplateSelect"></select></label>
              </div>
              <div class="actions">
                <button id="loadEndpointTemplates">Load templates</button>
                <button class="primary endpoint-next" data-endpoint-step="2">Next: Configure route</button>
              </div>
              <div id="endpointTemplateChooser" class="template-choice-grid"></div>
              <div class="split" style="margin-top: 12px;">
                <div>
                  <h2>GraphQL preview</h2>
                  <pre id="endpointTemplatePreview"></pre>
                </div>
                <div>
                  <h2>Default variables</h2>
                  <pre id="endpointTemplateDefaults"></pre>
                </div>
              </div>
            </div>

            <div id="endpoint-step-2" class="endpoint-wizard-step">
              <h2>Configure route</h2>
              <p class="help">The route remains private and must start with <code>/api/custom/</code>.</p>
              <div class="notice">caller_context is recommended. It evaluates the saved template as the caller and preserves caller permissions.</div>
              <div id="endpointServiceWarning" class="notice danger hidden">service_context can bypass caller permissions and should be used carefully. It runs this endpoint as the service entity you provide.</div>
              <div class="grid">
                <label>Method<select id="endpointMethod"><option>GET</option><option selected>POST</option><option>PUT</option><option>PATCH</option><option>DELETE</option></select></label>
                <label>Path<input id="endpointPath" value="/api/custom/devices" /></label>
                <label>Key<input id="endpointKey" placeholder="create_device_endpoint" /></label>
                <label>Name<input id="endpointName" placeholder="Create device endpoint" /></label>
                <label>Auth mode<select id="endpointAuthMode"><option selected>caller_context</option><option>service_context</option></select></label>
                <label>Service entity ID<input id="endpointServiceEntityId" placeholder="required for service_context" /></label>
              </div>
              <label>Description<input id="endpointDescription" placeholder="optional description" /></label>
              <div class="actions">
                <button class="endpoint-back" data-endpoint-step="1">Back</button>
                <button class="primary endpoint-next" data-endpoint-step="3">Next: Request mapping</button>
              </div>
            </div>

            <div id="endpoint-step-3" class="endpoint-wizard-step">
              <h2>Map request to variables</h2>
              <p class="help">Build request mapping rows for template variables. Example: <code>input.name &lt;- body.name</code>.</p>
              <h3>Template variables</h3>
              <div id="endpointMappingRows" class="mapping-rows"></div>
              <div class="actions">
                <button id="addEndpointMappingRow">Add mapping row</button>
                <button id="syncEndpointMappingJson">Update JSON editor</button>
              </div>
              <details open><summary>Advanced variablesMapping JSON</summary><textarea id="endpointVariablesMapping" spellcheck="false">{
  "input.name": "$body.name",
  "input.tenantId": "$body.tenantId",
  "input.profileId": "$body.profileId",
  "input.attributes": "$body.attributes",
  "context.actorId": "$auth.entityId"
}</textarea></details>
              <details><summary>requestSchema editor</summary><textarea id="endpointRequestSchema" spellcheck="false">{}</textarea></details>
              <details><summary>responseMapping editor</summary><textarea id="endpointResponseMapping" spellcheck="false">{}</textarea></details>
              <div class="actions">
                <button class="endpoint-back" data-endpoint-step="2">Back</button>
                <button class="primary endpoint-next" data-endpoint-step="4">Next: Test request</button>
              </div>
            </div>

            <div id="endpoint-step-4" class="endpoint-wizard-step">
              <h2>Test and publish</h2>
              <p class="help">Test request uses the saved active endpoint path. Publish endpoint enables the route after review.</p>
              <label>Sample request body<textarea id="endpointSampleBody" spellcheck="false">{
  "name": "device-001",
  "attributes": {}
}</textarea></label>
              <div class="actions">
                <button id="previewApiEndpoint">Preview endpoint</button>
                <button id="createApiEndpoint">Save draft</button>
                <button id="testApiEndpoint">Run test</button>
                <button class="primary" id="enableApiEndpoint">Publish endpoint</button>
                <button id="disableApiEndpoint">Disable selected</button>
                <button id="copyEndpointCurl">Copy curl</button>
                <button id="copyEndpointJs">Copy JavaScript fetch</button>
              </div>
              <div id="endpointBuilderSummary" class="summary-box">Choose template, configure route, map request, then publish endpoint.</div>
              <div class="split" style="margin-top: 12px;">
                <div>
                  <h2>Response JSON</h2>
                  <pre id="endpointTestResult"></pre>
                </div>
                <div>
                  <h2>Generated request</h2>
                  <pre id="endpointGeneratedRequest"></pre>
                </div>
              </div>
              <details><summary>Endpoint preview</summary><pre id="endpointPreview"></pre></details>
              <details><summary>Generated curl</summary><pre id="endpointCurl"></pre></details>
              <details><summary>Generated JavaScript fetch</summary><pre id="endpointJs"></pre></details>
            </div>
          </div>

          <div class="panel">
            <div class="response-header status-row">
              <h2 style="margin-bottom: 0;">Execution logs</h2>
              <span id="endpointLogsStatus" class="badge">No endpoint selected</span>
            </div>
            <p class="help">Recent executions show status, caller, createdAt, error, requestSummary, and responseSummary.</p>
            <div class="actions">
              <button id="viewEndpointLogs">View logs</button>
            </div>
            <div id="endpointLogs" class="endpoint-logs">
              <div class="help">Select an endpoint and choose View logs.</div>
            </div>
          </div>
        </section>

        <section id="screen-advanced" class="screen">
          <div class="panel">
            <h1>Operation Builder</h1>
            <p class="help">Build GraphQL operations from introspection only, then run, copy, or save them as reusable API Templates.</p>
            <div class="grid">
              <label>Search operations and types<input id="schemaSearch" placeholder="filter operations and types" autocomplete="off" /></label>
              <label>Saved examples<select id="savedExamples"></select></label>
            </div>
            <div class="actions">
              <button id="loadExample">Load example</button>
              <button id="saveExample">Save current example</button>
            </div>
            <div id="schemaStatus" class="status-row" style="margin-top: 10px;">
              <span class="badge warn">Loading schema</span>
            </div>
          </div>
          <div class="split">
            <div class="panel">
              <h2>Queries</h2>
              <div id="queryOps" class="operation-list"></div>
              <h2 style="margin-top: 16px;">Mutations</h2>
              <div id="mutationOps" class="operation-list"></div>
            </div>
            <div class="panel">
              <h2>Selected operation</h2>
              <div id="selectedOperation" class="help">Select a query or mutation. Required arguments appear in the generated variables skeleton.</div>
              <h3>Arguments</h3>
              <div id="argumentForm" class="grid"></div>
              <h3>Field selector</h3>
              <div id="returnFields" class="field-list hidden"></div>
              <div class="actions">
                <button id="generateOperation">Generate operation</button>
                <button class="primary" id="runOperation">Run operation</button>
                <button id="copyQuery">Copy GraphQL</button>
                <button id="copyVariables">Copy variables</button>
                <button id="copyAdvancedCurl">Copy curl</button>
                <button id="copyAdvancedJs">Copy JavaScript fetch</button>
                <button id="saveOperationTemplate">Save as template</button>
              </div>
            </div>
          </div>
          <div class="split">
            <div class="panel">
              <h2>GraphQL</h2>
              <textarea id="queryEditor" spellcheck="false"></textarea>
            </div>
            <div class="panel">
              <h2>Variables</h2>
              <textarea id="variablesEditor" spellcheck="false">{}</textarea>
            </div>
          </div>
          <details><summary>curl</summary><pre id="advancedCurl"></pre></details>
          <details><summary>JavaScript fetch</summary><pre id="advancedJs"></pre></details>
          <div class="panel">
            <div class="response-header status-row">
              <h2 style="margin-bottom: 0;">Response viewer</h2>
              <span id="responseStatus" class="badge">No request yet</span>
            </div>
            <div id="responseSummary" class="summary-box">What happened? No advanced request yet.</div>
            <pre id="responseViewer"></pre>
          </div>
        </section>

        <section id="screen-assistant" class="screen">
          <div class="panel">
            <h1>AI Assistant</h1>
            <div class="notice">Prompt generation only. No LLM calls are made. Review generated GraphQL before running.</div>
            <p class="help">Describe a request. The console builds a copyable prompt with Atom model rules, selected context, schema operations, saved templates, and the current operation.</p>
            <label>User request<textarea id="assistantRequest" spellcheck="false" placeholder="Example: create a tenant and a device entity"></textarea></label>
            <div class="actions">
              <button class="assistant-example" data-example="Create a tenant and a device entity">Create a tenant and a device entity</button>
              <button class="assistant-example" data-example="Create a protected resource and allow an entity to access it">Create a protected resource and allow an entity to access it</button>
              <button class="assistant-example" data-example="Explain why an authorization check failed">Explain why an authorization check failed</button>
              <button class="assistant-example" data-example="Generate a reusable API template">Generate a reusable API template</button>
            </div>
            <div class="grid">
              <label class="check-row"><input id="assistantIncludeSchema" type="checkbox" checked /> Include schema summary</label>
              <label class="check-row"><input id="assistantIncludeTemplates" type="checkbox" checked /> Include saved templates</label>
              <label class="check-row"><input id="assistantIncludeCurrentOperation" type="checkbox" checked /> Include current operation</label>
            </div>
            <label>Selected tenant/profile/resource context<textarea id="assistantContext" spellcheck="false">{}</textarea></label>
            <div class="actions">
              <button id="refreshAssistantContext">Refresh context</button>
              <button id="generatePrompt">Generate prompt</button>
              <button id="copyPrompt">Copy prompt</button>
            </div>
            <h2>Copyable prompt</h2>
            <pre id="assistantPrompt"></pre>
            <h2>Expected response format</h2>
            <pre id="assistantExpected">GraphQL operation:
<query or mutation>

variables JSON:
{}

Explanation:
<plain-language explanation and assumptions>

safety notes:
<validation concerns, missing permissions, secret handling, and assumptions></pre>
          </div>
        </section>
      </div>
    </main>

    <aside class="docs-panel">
      <div class="docs-scroll">
        <div class="brand">
          <h1>Schema Docs</h1>
          <div class="subtitle">Core Atom model reference from GraphQL introspection.</div>
        </div>
        <section class="nav-section">
          <dl class="schema-docs">
            <dt>Tenant</dt><dd>Isolation boundary.</dd>
            <dt>Entity</dt><dd>Principal; human/device/service/workload/application.</dd>
            <dt>Resource</dt><dd>Protected object, for example channel/rule/report.</dd>
            <dt>Group</dt><dd>Collection of entities.</dd>
            <dt>Profile</dt><dd>User/domain subtype/schema.</dd>
            <dt>ProfileVersion</dt><dd>JSON Schema validation/history.</dd>
            <dt>Policy</dt><dd>Grants capability/role over scope.</dd>
          </dl>
        </section>
        <section class="nav-section">
          <h2>External Mapping</h2>
          <p class="help">Magistrala or any external system should use generic Atom operations:</p>
          <ul class="help">
            <li>domain -> createTenant</li>
            <li>client -> createEntity with profile client under kind device</li>
            <li>channel -> createResource with kind "channel"</li>
            <li>connection -> createPolicy for publish/subscribe</li>
          </ul>
          <p class="help">Do not add GraphQL aliases for these.</p>
        </section>
        <section class="nav-section">
          <h2>Types</h2>
          <div id="objectTypes" class="type-list"></div>
          <h3>Input types</h3>
          <div id="inputTypes" class="type-list"></div>
          <h3>Enums</h3>
          <div id="enumTypes" class="type-list"></div>
        </section>
      </div>
    </aside>
  </div>

  <script>
"######;

pub(crate) const HTML_AFTER_JS: &str = r######"  </script>
</body>
</html>"######;
