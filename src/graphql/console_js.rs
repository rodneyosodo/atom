pub(crate) const CONSOLE_JS: &str = r######"    const state = {
      schema: null,
      schemaModel: null,
      typeMap: new Map(),
      selectedOperation: null,
      lastResult: null,
      lastAuthzName: "authzCheck",
      tenants: [],
      profilesBySelect: new Map(),
      versionsBySelect: new Map(),
      resources: [],
      capabilities: [],
      roles: [],
      entities: [],
      groups: [],
      apiTemplates: [],
      selectedApiTemplate: null,
      apiEndpoints: [],
      selectedApiEndpoint: null,
      wizard: {}
    };

    const introspectionQuery = `
query IntrospectionQuery {
  __schema {
    queryType { name }
    mutationType { name }
    types { ...FullType }
  }
}

fragment FullType on __Type {
  kind
  name
  description
  fields(includeDeprecated: true) {
    name
    description
    args { ...InputValue }
    type { ...TypeRef }
    isDeprecated
    deprecationReason
  }
  inputFields { ...InputValue }
  enumValues(includeDeprecated: true) {
    name
    description
    isDeprecated
    deprecationReason
  }
}

fragment InputValue on __InputValue {
  name
  description
  type { ...TypeRef }
  defaultValue
}

fragment TypeRef on __Type {
  kind
  name
  ofType {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
            }
          }
        }
      }
    }
  }
}`;

    const $ = (id) => document.getElementById(id);

    function escapeHtml(value) {
      return String(value ?? "").replace(/[&<>"']/g, (char) => ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        "\"": "&quot;",
        "'": "&#39;"
      })[char]);
    }

    function parseJson(id) {
      const text = $(id).value.trim();
      if (!text) return {};
      try {
        return JSON.parse(text);
      } catch (err) {
        throw new Error(`${humanLabel(id)} contains invalid JSON: ${err.message}`);
      }
    }

    function humanLabel(id) {
      return id.replace(/([A-Z])/g, " $1").replace(/^./, (char) => char.toUpperCase());
    }

    function nullable(value) {
      const text = String(value ?? "").trim();
      return text ? text : null;
    }

    function selectedOrInput(selectId, inputId) {
      return nullable($(inputId).value) || nullable($(selectId).value);
    }

    function commaList(value) {
      return String(value || "").split(",").map((item) => item.trim()).filter(Boolean);
    }

    function endpointSafety() {
      const raw = $("endpoint").value.trim() || "/graphql";
      const url = new URL(raw, window.location.origin);
      const sameOriginGraphql = url.origin === window.location.origin && url.pathname === "/graphql";
      return {
        url,
        canSendAuth: sameOriginGraphql,
        warning: sameOriginGraphql ? "" : "Authorization is disabled unless the endpoint is same-origin /graphql."
      };
    }

    function renderEndpointWarning() {
      const safety = endpointSafety();
      $("endpointWarning").textContent = safety.warning;
      $("endpointWarning").classList.toggle("hidden", !safety.warning);
      return safety;
    }

    function authHeaders(canSendAuth) {
      const token = $("token").value.trim();
      return token && canSendAuth ? { Authorization: `Bearer ${token}` } : {};
    }

    async function requestGraphql(query, variables = {}, useAuth = true) {
      const safety = renderEndpointWarning();
      const res = await fetch(safety.url.href, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...(useAuth ? authHeaders(safety.canSendAuth) : {})
        },
        body: JSON.stringify({ query, variables })
      });
      const text = await res.text();
      try {
        return JSON.parse(text);
      } catch (_) {
        throw new Error(`GraphQL endpoint returned non-JSON status ${res.status}`);
      }
    }

    function unwrapType(type) {
      let node = type;
      while (node && node.ofType) node = node.ofType;
      return node;
    }

    function typeName(type) {
      if (!type) return "";
      if (type.kind === "NON_NULL") return `${typeName(type.ofType)}!`;
      if (type.kind === "LIST") return `[${typeName(type.ofType)}]`;
      return type.name || "";
    }

    function namedType(type) {
      return unwrapType(type)?.name;
    }

    function isNonNull(type) {
      return type?.kind === "NON_NULL";
    }

    function baseType(type) {
      return state.typeMap.get(namedType(type));
    }

    function defaultValueForType(type) {
      const named = namedType(type);
      if (!isNonNull(type)) return null;
      if (["String", "ID"].includes(named)) return "";
      if (["Int", "Float"].includes(named)) return 0;
      if (named === "Boolean") return false;
      if (named === "JSON" || named === "JSONObject") return {};
      const gqlType = state.typeMap.get(named);
      if (gqlType?.kind === "ENUM") return gqlType.enumValues?.[0]?.name || "";
      if (gqlType?.kind === "INPUT_OBJECT") {
        return Object.fromEntries((gqlType.inputFields || []).map((field) => [field.name, defaultValueForType(field.type)]));
      }
      return "";
    }

    function indent(text) {
      return text.split("\n").map((line) => `  ${line}`).join("\n");
    }

    function pascal(name) {
      return name.charAt(0).toUpperCase() + name.slice(1);
    }

    function responseFieldsFor(type) {
      const gqlType = baseType(type);
      return (gqlType?.fields || []).filter((field) => !field.name.startsWith("__"));
    }

    function defaultReturnSelection(type, depth = 0) {
      const fields = responseFieldsFor(type);
      if (!fields.length) return "";
      const scalarFields = fields.filter((field) => ["SCALAR", "ENUM"].includes(unwrapType(field.type)?.kind));
      const selected = scalarFields.length ? scalarFields : fields.slice(0, 8);
      return selected.map((field) => {
        const kind = unwrapType(field.type)?.kind;
        if (["OBJECT", "INTERFACE"].includes(kind) && depth < 1) {
          const nested = defaultReturnSelection(field.type, depth + 1);
          return nested ? `${field.name} {\n${indent(nested)}\n}` : field.name;
        }
        return field.name;
      }).join("\n");
    }

    function operationText(kind, op, selectedFields = null) {
      const args = op.args || [];
      const vars = args.map((arg) => `$${arg.name}: ${typeName(arg.type)}`).join(", ");
      const argText = args.map((arg) => `${arg.name}: $${arg.name}`).join(", ");
      const selection = selectedFields ? selectedFields.join("\n") : defaultReturnSelection(op.type);
      const header = `${kind} ${pascal(op.name)}${vars ? `(${vars})` : ""}`;
      const call = `${op.name}${argText ? `(${argText})` : ""}`;
      if (!selection) return `${header} {\n  ${call}\n}`;
      return `${header} {\n  ${call} {\n${indent(indent(selection))}\n  }\n}`;
    }

    function variableSkeleton(op) {
      return Object.fromEntries((op.args || []).map((arg) => [arg.name, defaultValueForType(arg.type)]));
    }

    function setPathValue(target, path, value) {
      let current = target;
      for (const segment of path.slice(0, -1)) {
        if (!current[segment] || typeof current[segment] !== "object") current[segment] = {};
        current = current[segment];
      }
      current[path[path.length - 1]] = value;
    }

    function inputValueForForm(value, type) {
      const named = namedType(type);
      if (named === "JSON" || named === "JSONObject") return JSON.stringify(value ?? {}, null, 2);
      if (value == null) return "";
      return String(value);
    }

    function inputControlFor(path, labelText, type, value) {
      const named = namedType(type);
      const gqlType = state.typeMap.get(named);
      const label = document.createElement("label");
      label.textContent = `${labelText} (${typeName(type)})`;
      let control;
      if (gqlType?.kind === "ENUM") {
        control = document.createElement("select");
        if (!isNonNull(type)) control.appendChild(new Option("", ""));
        for (const enumValue of gqlType.enumValues || []) control.appendChild(new Option(enumValue.name, enumValue.name));
        control.value = value || "";
      } else if (named === "JSON" || named === "JSONObject") {
        control = document.createElement("textarea");
        control.spellcheck = false;
        control.value = inputValueForForm(value, type);
      } else {
        control = document.createElement("input");
        control.type = named === "Int" || named === "Float" ? "number" : named === "Boolean" ? "checkbox" : "text";
        if (control.type === "checkbox") {
          control.checked = Boolean(value);
        } else {
          control.value = inputValueForForm(value, type);
        }
      }
      control.dataset.varPath = JSON.stringify(path);
      control.dataset.varType = named || "";
      control.dataset.required = String(isNonNull(type));
      control.addEventListener("input", () => regenerateOperation(false));
      control.addEventListener("change", () => regenerateOperation(false));
      label.appendChild(control);
      return label;
    }

    function renderInputObjectFields(target, path, gqlType, value, depth = 0) {
      for (const field of gqlType.inputFields || []) {
        const fieldPath = [...path, field.name];
        const fieldValue = value && typeof value === "object" ? value[field.name] : defaultValueForType(field.type);
        const fieldType = state.typeMap.get(namedType(field.type));
        if (fieldType?.kind === "INPUT_OBJECT" && depth < 2) {
          const section = document.createElement("div");
          section.className = "mini-panel";
          section.innerHTML = `<h3>${escapeHtml(field.name)}</h3>`;
          renderInputObjectFields(section, fieldPath, fieldType, fieldValue || {}, depth + 1);
          target.appendChild(section);
        } else {
          target.appendChild(inputControlFor(fieldPath, field.name, field.type, fieldValue));
        }
      }
    }

    function renderArgumentForm(op) {
      const target = $("argumentForm");
      target.innerHTML = "";
      const skeleton = variableSkeleton(op);
      for (const arg of op.args || []) {
        const gqlType = state.typeMap.get(namedType(arg.type));
        if (gqlType?.kind === "INPUT_OBJECT") {
          const section = document.createElement("div");
          section.className = "mini-panel";
          section.innerHTML = `<h3>${escapeHtml(arg.name)}</h3>`;
          renderInputObjectFields(section, [arg.name], gqlType, skeleton[arg.name] || {});
          target.appendChild(section);
        } else {
          target.appendChild(inputControlFor([arg.name], arg.name, arg.type, skeleton[arg.name]));
        }
      }
      if (!target.children.length) target.innerHTML = `<div class="help">This operation has no arguments.</div>`;
    }

    function valueFromControl(control) {
      const named = control.dataset.varType;
      const required = control.dataset.required === "true";
      if (control.type === "checkbox") return control.checked;
      const raw = control.value.trim();
      if (!raw && !required) return null;
      if (named === "Int") return raw ? parseInt(raw, 10) : 0;
      if (named === "Float") return raw ? Number(raw) : 0;
      if (named === "JSON" || named === "JSONObject") return raw ? JSON.parse(raw) : {};
      return raw;
    }

    function collectArgumentVariables(op) {
      const variables = variableSkeleton(op);
      $("argumentForm").querySelectorAll("[data-var-path]").forEach((control) => {
        setPathValue(variables, JSON.parse(control.dataset.varPath), valueFromControl(control));
      });
      return variables;
    }

    function operationMatches(op, search) {
      if (!search) return true;
      const haystack = [
        op.name,
        typeName(op.type),
        ...(op.args || []).map((arg) => `${arg.name} ${typeName(arg.type)}`)
      ].join(" ").toLowerCase();
      return haystack.includes(search.toLowerCase());
    }

    function renderOperationList(targetId, kind, rootTypeName) {
      const root = state.typeMap.get(rootTypeName);
      const target = $(targetId);
      const search = $("schemaSearch").value.trim();
      target.innerHTML = "";
      for (const op of (root?.fields || []).filter((field) => operationMatches(field, search))) {
        const button = document.createElement("button");
        button.className = "operation-button";
        const args = (op.args || []).map((arg) => `${arg.name}: ${typeName(arg.type)} (${isNonNull(arg.type) ? "required" : "optional"})`).join(", ") || "no arguments";
        button.innerHTML = `<strong>${escapeHtml(op.name)}</strong><span>${escapeHtml(args)} -> ${escapeHtml(typeName(op.type))}</span>`;
        button.addEventListener("click", () => selectOperation(kind, op));
        target.appendChild(button);
      }
      if (!target.children.length) {
        target.innerHTML = `<span class="help">No ${kind} operations match the current search.</span>`;
      }
    }

    function typeMatches(type, search) {
      if (!search) return true;
      return type.name?.toLowerCase().includes(search.toLowerCase());
    }

    function renderTypeLists() {
      const search = $("schemaSearch").value.trim();
      const types = Array.from(state.typeMap.values())
        .filter((type) => type.name && !type.name.startsWith("__") && typeMatches(type, search));
      $("objectTypes").innerHTML = types
        .filter((type) => type.kind === "OBJECT")
        .map((type) => `<span class="pill">${escapeHtml(type.name)}</span>`)
        .join("");
      $("inputTypes").innerHTML = types
        .filter((type) => type.kind === "INPUT_OBJECT")
        .map((type) => `<span class="pill">${escapeHtml(type.name)}</span>`)
        .join("");
      $("enumTypes").innerHTML = types
        .filter((type) => type.kind === "ENUM")
        .map((type) => `<span class="pill">${escapeHtml(type.name)}</span>`)
        .join("");
    }

    function renderSchema() {
      if (!state.schema) return;
      renderOperationList("queryOps", "query", state.schema.queryType.name);
      renderOperationList("mutationOps", "mutation", state.schema.mutationType.name);
      renderTypeLists();
    }

    function selectOperation(kind, op) {
      state.selectedOperation = { kind, op };
      const args = (op.args || []).map((arg) => `<span class="badge ${isNonNull(arg.type) ? "warn" : ""}">${escapeHtml(arg.name)}: ${escapeHtml(typeName(arg.type))}</span>`).join(" ");
      $("selectedOperation").innerHTML = `<strong>${kind} ${escapeHtml(op.name)}</strong><div class="status-row" style="margin-top: 8px;">${args || '<span class="badge">No arguments</span>'}<span class="badge">returns ${escapeHtml(typeName(op.type))}</span></div>`;
      renderArgumentForm(op);
      renderReturnFieldSelector(op);
      regenerateOperation(true);
      showScreen("advanced");
    }

    function renderReturnFieldSelector(op) {
      const target = $("returnFields");
      const fields = responseFieldsFor(op.type);
      target.innerHTML = "";
      target.classList.toggle("hidden", !fields.length);
      if (!fields.length) return;
      const preferred = new Set(["id", "name", "status", "kind", "key", "displayName"]);
      const scalarFields = fields.filter((field) => ["SCALAR", "ENUM"].includes(unwrapType(field.type)?.kind));
      const preferredScalars = scalarFields.filter((field) => preferred.has(field.name));
      const defaultScalars = new Set((preferredScalars.length ? preferredScalars : scalarFields.slice(0, 4)).map((field) => field.name));
      for (const field of fields) {
        const kind = unwrapType(field.type)?.kind;
        if (["SCALAR", "ENUM"].includes(kind)) {
          const label = document.createElement("label");
          const checked = defaultScalars.has(field.name) ? "checked" : "";
          label.innerHTML = `<input type="checkbox" data-field="${escapeHtml(field.name)}" ${checked}> ${escapeHtml(field.name)} <span class="muted">${escapeHtml(typeName(field.type))}</span>`;
          label.querySelector("input").addEventListener("change", () => regenerateOperation(false));
          target.appendChild(label);
          continue;
        }

        const nested = responseFieldsFor(field.type).filter((nestedField) => ["SCALAR", "ENUM"].includes(unwrapType(nestedField.type)?.kind));
        if (!nested.length) continue;
        const section = document.createElement("div");
        section.className = "mini-panel";
        section.innerHTML = `<h3>${escapeHtml(field.name)}</h3>`;
        const preferredNested = nested.filter((nestedField) => preferred.has(nestedField.name));
        const defaultNested = new Set((field.name === "items" ? preferredNested : []).map((nestedField) => nestedField.name));
        for (const nestedField of nested) {
          const label = document.createElement("label");
          const checked = defaultNested.has(nestedField.name) ? "checked" : "";
          label.innerHTML = `<input type="checkbox" data-parent="${escapeHtml(field.name)}" data-field="${escapeHtml(nestedField.name)}" ${checked}> ${escapeHtml(nestedField.name)} <span class="muted">${escapeHtml(typeName(nestedField.type))}</span>`;
          label.querySelector("input").addEventListener("change", () => regenerateOperation(false));
          section.appendChild(label);
        }
        target.appendChild(section);
      }
    }

    function collectSelectedFields() {
      const target = $("returnFields");
      const selected = Array.from(target.querySelectorAll("input[data-field]:checked:not([data-parent])"))
        .map((input) => input.dataset.field);
      const nestedByParent = new Map();
      target.querySelectorAll("input[data-parent]:checked").forEach((input) => {
        const parent = input.dataset.parent;
        if (!nestedByParent.has(parent)) nestedByParent.set(parent, []);
        nestedByParent.get(parent).push(input.dataset.field);
      });
      for (const [parent, fields] of nestedByParent.entries()) {
        selected.push(`${parent} {\n${indent(fields.join("\n"))}\n}`);
      }
      return selected;
    }

    function regenerateOperation(showErrors) {
      const current = state.selectedOperation;
      if (!current) return;
      try {
        const selected = collectSelectedFields();
        const variables = collectArgumentVariables(current.op);
        $("queryEditor").value = operationText(current.kind, current.op, selected.length ? selected : null);
        $("variablesEditor").value = JSON.stringify(variables, null, 2);
        updateTemplateOutputs();
      } catch (err) {
        if (showErrors) {
          $("responseSummary").innerHTML = `<span class="badge error">Builder input error</span> ${escapeHtml(err.message)}`;
        }
      }
    }

    function updateStatus() {
      const schemaLoaded = Boolean(state.schema);
      const tokenPresent = Boolean($("token").value.trim());
      const safety = renderEndpointWarning();
      $("connectionStatus").innerHTML = `
        <span class="badge ${safety.canSendAuth ? "ok" : "warn"}">${escapeHtml(safety.url.pathname || "/graphql")}</span>
        <span class="badge ${schemaLoaded ? "ok" : "warn"}">${schemaLoaded ? "Schema loaded" : "Schema not loaded"}</span>
        <span class="badge ${tokenPresent ? "ok" : "warn"}">${tokenPresent ? "Token ready" : "Not authenticated"}</span>
        <span class="badge ${safety.canSendAuth ? "ok" : "warn"}">${safety.canSendAuth ? "Auth header allowed" : "Auth header blocked"}</span>
      `;
    }

    async function loadSchema() {
      $("schemaStatus").innerHTML = `<span class="badge warn">Loading schema</span>`;
      updateStatus();
      try {
        const result = await requestGraphql(introspectionQuery, {}, false);
        if (result.errors?.length) throw new Error(result.errors.map((err) => err.message).join("; "));
        state.schema = result.data.__schema;
        state.typeMap = new Map(state.schema.types.map((type) => [type.name, type]));
        state.schemaModel = {
          queries: state.typeMap.get(state.schema.queryType.name)?.fields || [],
          mutations: state.typeMap.get(state.schema.mutationType.name)?.fields || [],
          objectTypes: state.schema.types.filter((type) => type.kind === "OBJECT" && !type.name.startsWith("__")),
          inputObjectTypes: state.schema.types.filter((type) => type.kind === "INPUT_OBJECT"),
          enums: state.schema.types.filter((type) => type.kind === "ENUM"),
          scalars: state.schema.types.filter((type) => type.kind === "SCALAR")
        };
        renderSchema();
        $("schemaStatus").innerHTML = `<span class="badge ok">Loaded ${state.schema.types.length} types</span>`;
      } catch (err) {
        $("schemaStatus").innerHTML = `<span class="badge error">${escapeHtml(err.message)}</span>`;
      }
      updateStatus();
    }

    function setPreview(query, variables, queryTarget, variablesTarget) {
      $(queryTarget).textContent = query;
      $(variablesTarget).textContent = JSON.stringify(variables, null, 2);
    }

    function setAdvancedOperation(query, variables) {
      $("queryEditor").value = query;
      $("variablesEditor").value = JSON.stringify(variables, null, 2);
    }

    function summarizeErrors(result) {
      return result.errors?.map((err) => err.message).join("; ") || "";
    }

    function writeRaw(targetId, result) {
      $(targetId).textContent = JSON.stringify(result, null, 2);
      state.lastResult = result;
    }

    function idFrom(value) {
      return value?.id || value?.entityId || value?.credentialId || null;
    }

    function resultSummary(result, dataKey, noun, nextText) {
      const errors = summarizeErrors(result);
      if (errors) return `<span class="badge error">Error</span> ${escapeHtml(errors)}`;
      const value = result.data?.[dataKey];
      const id = idFrom(value);
      const idText = id ? ` ID: <code>${escapeHtml(id)}</code>.` : "";
      return `<span class="badge ok">Success</span> ${escapeHtml(noun)} completed.${idText} ${escapeHtml(nextText || "")}`;
    }

    async function runPreviewed(builder, queryTarget, variablesTarget, resultTarget, summaryTarget, dataKey, noun, nextText, afterSuccess = null) {
      try {
        const built = builder();
        setPreview(built.query, built.variables, queryTarget, variablesTarget);
        setAdvancedOperation(built.query, built.variables);
        const result = await requestGraphql(built.query, built.variables, built.useAuth !== false);
        writeRaw(resultTarget, result);
        $(summaryTarget).innerHTML = resultSummary(result, dataKey, noun, nextText);
        if (!result.errors?.length && afterSuccess) afterSuccess(result);
        return !result.errors?.length;
      } catch (err) {
        $(summaryTarget).innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
        return false;
      }
    }

    function previewOnly(builder, queryTarget, variablesTarget, summaryTarget, text) {
      try {
        const built = builder();
        setPreview(built.query, built.variables, queryTarget, variablesTarget);
        setAdvancedOperation(built.query, built.variables);
        $(summaryTarget).innerHTML = `<span class="badge ok">Preview ready</span> ${escapeHtml(text || "Review generated GraphQL before running.")}`;
      } catch (err) {
        $(summaryTarget).innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    function loginMutation() {
      return `mutation Login($input: LoginInput!) {
  login(input: $input) {
    token
    entityId
    sessionId
    expiresAt
  }
}`;
    }

    function loginBuild() {
      return {
        query: loginMutation(),
        variables: {
          input: {
            identifier: $("loginIdentifier").value,
            secret: $("loginSecret").value,
            kind: $("loginKind").value || "password"
          }
        },
        useAuth: false
      };
    }

    async function runLogin() {
      const built = loginBuild();
      setPreview(built.query, built.variables, "loginGraphql", "loginVariables");
      const safeVariables = JSON.parse(JSON.stringify(built.variables));
      safeVariables.input.secret = safeVariables.input.secret ? "<redacted>" : "";
      $("loginVariables").textContent = JSON.stringify(safeVariables, null, 2);
      try {
        const result = await requestGraphql(built.query, built.variables, false);
        writeRaw("loginResult", result);
        const login = result.data?.login;
        if (login?.token) {
          $("token").value = login.token;
          localStorage.setItem("atom.graphql.console.token", login.token);
          $("loginSummary").innerHTML = `<span class="badge ok">Logged in</span> Entity <code>${escapeHtml(login.entityId)}</code>. Token stored in localStorage for this browser.`;
        } else {
          $("loginSummary").innerHTML = result.errors?.length ? `<span class="badge error">Error</span> ${escapeHtml(summarizeErrors(result))}` : `<span class="badge warn">No token returned</span>`;
        }
      } catch (err) {
        $("loginSummary").innerHTML = `<span class="badge error">Error</span> ${escapeHtml(err.message)}`;
      }
      updateStatus();
    }

    function clearToken() {
      $("token").value = "";
      localStorage.removeItem("atom.graphql.console.token");
      updateStatus();
    }

    function tenantMutation() {
      return `mutation CreateTenant($input: CreateTenantInput!) {
  createTenant(input: $input) {
    id
    name
    route
    status
    tags
    attributes
  }
}`;
    }

    function tenantBuild(prefix = "tenant") {
      return {
        query: tenantMutation(),
        variables: {
          input: {
            name: $(`${prefix}Name`).value,
            route: nullable($(`${prefix}Route`)?.value),
            tags: $(`${prefix}Tags`) ? commaList($(`${prefix}Tags`).value) : [],
            attributes: parseJson(`${prefix}Attributes`)
          }
        }
      };
    }

    function profileMutation() {
      return `mutation CreateProfile($input: CreateProfileInput!) {
  createProfile(input: $input) {
    id
    tenantId
    objectKind
    kind
    key
    displayName
    description
    status
  }
}`;
    }

    function profileVersionMutation() {
      return `mutation CreateProfileVersion($profileId: ID!, $input: CreateProfileVersionInput!) {
  createProfileVersion(profileId: $profileId, input: $input) {
    id
    profileId
    version
    jsonSchema
    uiSchema
    status
  }
}`;
    }

    function profileBuild() {
      return {
        query: profileMutation(),
        variables: {
          input: {
            tenantId: selectedOrInput("profileTenantSelect", "profileTenantId"),
            objectKind: $("profileObjectKind").value || "entity",
            kind: $("profileKind").value,
            key: $("profileKey").value,
            displayName: $("profileDisplayName").value,
            description: nullable($("profileDescription").value),
            status: nullable($("profileStatus").value)
          }
        }
      };
    }

    function profileVersionBuild() {
      return {
        query: profileVersionMutation(),
        variables: {
          profileId: $("profileVersionProfileId").value,
          input: {
            version: Number($("profileVersionNumber").value || 1),
            jsonSchema: parseJson("profileJsonSchema"),
            uiSchema: parseJson("profileUiSchema"),
            status: nullable($("profileVersionStatus").value)
          }
        }
      };
    }

    function entityMutation() {
      return `mutation CreateEntity($input: CreateEntityInput!) {
  createEntity(input: $input) {
    id
    kind
    profileId
    profileVersionId
    name
    tenantId
    status
    attributes
  }
}`;
    }

    function entityBuild(config = {}) {
      const selectId = config.profileSelect || "entityProfile";
      const versionId = config.versionSelect || "entityProfileVersion";
      const profileId = nullable($(selectId).value);
      const input = {
        profileId,
        profileVersionId: nullable($(versionId).value),
        name: $(config.name || "entityName").value,
        tenantId: config.tenantValue ?? selectedOrInput("entityTenantSelect", "entityTenantId"),
        attributes: config.attributesJson ? parseJson(config.attributesJson) : schemaFormAttributes("schemaForm", "entityAttributes")
      };
      if (!input.profileId) {
        delete input.profileId;
        input.kind = $(config.kind || "entityKind").value;
      }
      if (!input.profileVersionId) delete input.profileVersionId;
      return { query: entityMutation(), variables: { input } };
    }

    function resourceMutation() {
      return `mutation CreateResource($input: CreateResourceInput!) {
  createResource(input: $input) {
    id
    kind
    name
    tenantId
    ownerId
    attributes
  }
}`;
    }

    function resourceKind(prefix = "resource") {
      const preset = $(`${prefix}KindPreset`)?.value;
      if (preset && preset !== "custom") return preset;
      return $(`${prefix}KindCustom`)?.value || $(`${prefix}Kind`)?.value || "resource";
    }

    function resourceBuild(prefix = "resource") {
      return {
        query: resourceMutation(),
        variables: {
          input: {
            kind: resourceKind(prefix),
            name: nullable($(`${prefix}Name`).value),
            tenantId: prefix === "resource" ? selectedOrInput("resourceTenantSelect", "resourceTenantId") : nullable($(`${prefix}TenantId`).value),
            ownerId: nullable($(`${prefix}OwnerId`)?.value),
            attributes: parseJson(`${prefix}Attributes`)
          }
        }
      };
    }

    function policyMutation() {
      return `mutation CreatePolicy($input: CreatePolicyInput!) {
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
  }
}`;
    }

    function selectedOption(selectId) {
      return $(selectId).selectedOptions[0];
    }

    function policySubjectId() {
      return nullable($("policySubjectId").value) || nullable($("policySubjectSelect").value);
    }

    function policyGrantId() {
      return nullable($("policyGrantId").value) || nullable($("policyGrantSelect").value);
    }

    function policyScopeRef() {
      const kind = $("policyScopeKind").value;
      if (kind === "platform") return null;
      if (kind === "tenant") return nullable($("policyScopeRef").value) || nullable($("policyTenantSelect").value);
      if (kind === "object") return nullable($("policyScopeRef").value) || nullable($("policyObjectSelect").value);
      return nullable($("policyScopeRef").value);
    }

    function policyBuild() {
      return {
        query: policyMutation(),
        variables: {
          input: {
            tenantId: nullable($("policyTenantSelect").value),
            subjectKind: $("policySubjectKind").value,
            subjectId: policySubjectId(),
            grantKind: $("policyGrantKind").value,
            grantId: policyGrantId(),
            scopeKind: $("policyScopeKind").value,
            scopeRef: policyScopeRef(),
            effect: $("policyEffect").value,
            conditions: parseJson("policyConditions")
          }
        }
      };
    }

    function setupPolicyBuild() {
      return {
        query: policyMutation(),
        variables: {
          input: {
            tenantId: nullable($("setupPolicyTenantId").value),
            subjectKind: "entity",
            subjectId: $("setupPolicySubjectId").value,
            grantKind: "capability",
            grantId: $("setupPolicyGrantId").value,
            scopeKind: "object",
            scopeRef: $("setupPolicyResourceId").value,
            effect: $("setupPolicyEffect").value,
            conditions: parseJson("setupPolicyConditions")
          }
        }
      };
    }

    function authzMutation(name) {
      return `mutation ${pascal(name)}($input: AuthzCheckInput!) {
  ${name}(input: $input) {
    allowed
    reason
    ${name === "authzExplain" ? "subject\n    resource\n    capability\n    matchedBinding\n    evaluatedBindings" : "details"}
  }
}`;
    }

    function authzBuild(name = "authzCheck", prefix = "authz") {
      return {
        query: authzMutation(name),
        variables: {
          input: {
            subjectId: $(`${prefix}SubjectId`).value,
            action: $(`${prefix}Action`).value,
            resourceId: nullable($(`${prefix}ResourceId`).value),
            objectKind: nullable($(`${prefix}ObjectKind`)?.value),
            objectId: nullable($(`${prefix}ObjectId`)?.value),
            context: parseJson(`${prefix}Context`)
          }
        }
      };
    }

    function credentialBuild(kind) {
      if (kind === "list") {
        return {
          query: `query Credentials($entityId: ID!) {
  credentials(entityId: $entityId) {
    total
    items {
      id
      kind
      identifier
      status
      expiresAt
      createdAt
    }
  }
}`,
          variables: { entityId: $("credentialEntityId").value }
        };
      }
      if (kind === "apiKey") {
        return {
          query: `mutation CreateApiKey($entityId: ID!, $input: CreateApiKeyInput!) {
  createApiKey(entityId: $entityId, input: $input) {
    credentialId
    key
    expiresAt
  }
}`,
          variables: {
            entityId: $("credentialEntityId").value,
            input: {
              description: nullable($("apiKeyDescription").value),
              expiresAt: nullable($("apiKeyExpiresAt").value)
            }
          }
        };
      }
      if (kind === "password") {
        return {
          query: `mutation CreatePassword($entityId: ID!, $password: String!) {
  createPassword(entityId: $entityId, password: $password)
}`,
          variables: {
            entityId: $("credentialEntityId").value,
            password: $("passwordSecret").value
          }
        };
      }
      return {
        query: `mutation RevokeCredential($entityId: ID!, $credentialId: ID!) {
  revokeCredential(entityId: $entityId, credentialId: $credentialId)
}`,
        variables: {
          entityId: $("credentialEntityId").value,
          credentialId: $("credentialId").value
        }
      };
    }

    function schemaFormAttributes(formId, fallbackId) {
      const attrs = {};
      for (const input of $(formId).querySelectorAll("[data-attr]")) {
        if (input.value === "") continue;
        if (input.dataset.attrJson === "true") attrs[input.dataset.attr] = JSON.parse(input.value || "null");
        else if (input.type === "number") attrs[input.dataset.attr] = Number(input.value);
        else if (input.type === "checkbox") attrs[input.dataset.attr] = input.checked;
        else attrs[input.dataset.attr] = input.value;
      }
      return Object.keys(attrs).length ? attrs : parseJson(fallbackId);
    }

    async function loadTenants(targetIds = ["profileTenantSelect", "entityTenantSelect", "resourceTenantSelect", "policyTenantSelect"]) {
      const query = `query TenantsForConsole {
  tenants(limit: 200) {
    items {
      id
      name
      route
      status
    }
  }
}`;
      const result = await requestGraphql(query, {});
      if (result.errors?.length) throw new Error(summarizeErrors(result));
      state.tenants = result.data?.tenants?.items || [];
      for (const id of targetIds) fillSelect(id, state.tenants, "Choose tenant", (tenant) => `${tenant.name} (${tenant.status})`, "id");
      return state.tenants;
    }

    function fillSelect(selectId, items, placeholder, labelFn, valueKey = "id") {
      const select = $(selectId);
      if (!select) return;
      select.innerHTML = "";
      const blank = document.createElement("option");
      blank.value = "";
      blank.textContent = placeholder;
      select.appendChild(blank);
      for (const item of items) {
        const option = document.createElement("option");
        option.value = item[valueKey] || "";
        option.textContent = labelFn(item);
        for (const [key, value] of Object.entries(item)) {
          if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
            option.dataset[key] = value;
          }
        }
        select.appendChild(option);
      }
    }

    function selectedLabel(selectId) {
      const option = selectedOption(selectId);
      return option?.value ? option.textContent : "";
    }

    async function loadProfilesFor(selectId, versionSelectId, kindId, tenantSelectId = null, tenantInputId = null) {
      const tenantId = tenantSelectId ? selectedOrInput(tenantSelectId, tenantInputId) : null;
      const query = `query EntityProfiles($tenantId: ID) {
  profiles(objectKind: "entity", tenantId: $tenantId, status: "active", limit: 200) {
    items {
      id
      tenantId
      kind
      key
      displayName
      status
    }
  }
}`;
      const result = await requestGraphql(query, { tenantId });
      if (result.errors?.length) throw new Error(summarizeErrors(result));
      const profiles = result.data?.profiles?.items || [];
      state.profilesBySelect.set(`${selectId}:all`, profiles);
      await renderProfilesForKind(selectId, versionSelectId, kindId);
    }

    async function renderProfilesForKind(selectId, versionSelectId, kindId) {
      const allProfiles = state.profilesBySelect.get(`${selectId}:all`) || [];
      if (!allProfiles.length) {
        updateEntityBadges();
        return;
      }
      const kind = $(kindId).value;
      const profiles = allProfiles.filter((profile) => !kind || profile.kind === kind);
      state.profilesBySelect.set(selectId, profiles);
      fillProfileSelectGrouped(selectId, profiles, "Choose profile");
      await loadProfileVersionsFor(selectId, versionSelectId);
      updateEntityBadges();
    }

    function fillProfileSelectGrouped(selectId, profiles, placeholder) {
      const select = $(selectId);
      select.innerHTML = "";
      const blank = document.createElement("option");
      blank.value = "";
      blank.textContent = placeholder;
      select.appendChild(blank);
      const byKind = new Map();
      for (const profile of profiles) {
        if (!byKind.has(profile.kind)) byKind.set(profile.kind, []);
        byKind.get(profile.kind).push(profile);
      }
      for (const [kind, items] of Array.from(byKind.entries()).sort(([a], [b]) => a.localeCompare(b))) {
        const group = document.createElement("optgroup");
        group.label = kind;
        for (const profile of items.sort((a, b) => a.key.localeCompare(b.key))) {
          const option = document.createElement("option");
          option.value = profile.id;
          option.textContent = `${profile.key} - ${profile.displayName}`;
          for (const [key, value] of Object.entries(profile)) {
            if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
              option.dataset[key] = value;
            }
          }
          group.appendChild(option);
        }
        select.appendChild(group);
      }
    }

    async function loadProfileVersionsFor(profileSelectId, versionSelectId) {
      const profileId = $(profileSelectId).value;
      const select = $(versionSelectId);
      select.innerHTML = `<option value="">Active/latest version</option>`;
      if (!profileId) {
        renderJsonSchemaFormFor(versionSelectId, profileSelectId);
        return;
      }
      const query = `query ProfileVersions($profileId: ID!) {
  profileVersions(profileId: $profileId) {
    id
    version
    jsonSchema
    status
  }
}`;
      const result = await requestGraphql(query, { profileId });
      if (result.errors?.length) throw new Error(summarizeErrors(result));
      const versions = (result.data?.profileVersions || []).sort((a, b) => b.version - a.version);
      state.versionsBySelect.set(versionSelectId, versions);
      for (const version of versions) {
        const option = document.createElement("option");
        option.value = version.id;
        option.textContent = `v${version.version} ${version.status}`;
        option.dataset.schema = JSON.stringify(version.jsonSchema || {});
        option.dataset.status = version.status;
        select.appendChild(option);
      }
      const activeIndex = Array.from(select.options).findIndex((option) => option.dataset.status === "active");
      select.selectedIndex = activeIndex >= 0 ? activeIndex : (select.options.length > 1 ? 1 : 0);
      renderJsonSchemaFormFor(versionSelectId, profileSelectId);
    }

    function renderJsonSchemaFormFor(versionSelectId, profileSelectId) {
      const formId = versionSelectId === "entityProfileVersion" ? "schemaForm" : null;
      if (!formId) return;
      const option = $(versionSelectId).selectedOptions[0];
      const schema = option?.dataset.schema ? JSON.parse(option.dataset.schema) : {};
      const props = schema.properties || {};
      const form = $(formId);
      form.innerHTML = "";
      for (const [name, spec] of Object.entries(props)) {
        const label = document.createElement("label");
        label.textContent = spec.title || name;
        let input;
        if (Array.isArray(spec.enum)) {
          input = document.createElement("select");
          input.innerHTML = `<option value="">Choose ${escapeHtml(name)}</option>` + spec.enum.map((item) => `<option>${escapeHtml(item)}</option>`).join("");
        } else if (spec.type === "object" || spec.type === "array") {
          input = document.createElement("textarea");
          input.spellcheck = false;
          input.value = spec.type === "array" ? "[]" : "{}";
          input.dataset.attrJson = "true";
        } else {
          input = document.createElement("input");
          input.type = spec.type === "number" || spec.type === "integer" ? "number" : spec.type === "boolean" ? "checkbox" : "text";
          input.placeholder = spec.description || spec.type || "";
        }
        input.dataset.attr = name;
        label.appendChild(input);
        form.appendChild(label);
      }
      if (!Object.keys(props).length) {
        form.innerHTML = `<div class="help">This profile version has no simple JSON Schema properties. Use the JSON fallback below.</div>`;
      }
      updateEntityBadges();
    }

    function selectedProfile(selectId = "entityProfile") {
      const profiles = state.profilesBySelect.get(selectId) || [];
      return profiles.find((profile) => profile.id === $(selectId).value);
    }

    function updateEntityBadges() {
      const profile = selectedProfile("entityProfile");
      const version = selectedOption("entityProfileVersion");
      $("entityBadges").innerHTML = `
        <span class="badge ok">Derived kind: ${escapeHtml(profile?.kind || $("entityKind").value)}</span>
        <span class="badge">Profile subtype: ${escapeHtml(profile?.key || "not selected")}</span>
        <span class="badge ${version?.dataset.status === "active" ? "ok" : "warn"}">Version: ${escapeHtml(version?.textContent || "active/latest")}</span>
      `;
      const setupProfile = selectedProfile("setupEntityProfile");
      $("setupEntityBadges").innerHTML = `
        <span class="badge ok">Derived kind: ${escapeHtml(setupProfile?.kind || $("setupEntityKind").value)}</span>
        <span class="badge">Profile subtype: ${escapeHtml(setupProfile?.key || "not selected")}</span>
      `;
    }

    function entitySuccessSummary(result, profileSelectId, nameId, summaryId, nextText) {
      const entity = result.data?.createEntity;
      if (!entity) return;
      const profile = selectedProfile(profileSelectId);
      const name = entity.name || $(nameId).value;
      const kind = entity.kind || profile?.kind || "unknown";
      const profileText = profile ? `${profile.key} (${profile.displayName})` : "not selected";
      $(summaryId).innerHTML = `<span class="badge ok">Created</span> Created entity <strong>${escapeHtml(name)}</strong>. Internal Atom kind is <strong>${escapeHtml(kind)}</strong>. Profile is <strong>${escapeHtml(profileText)}</strong>. ${escapeHtml(nextText || "")}`;
    }

    async function loadPolicySubjects() {
      const tenantId = nullable($("policyTenantSelect").value);
      if ($("policySubjectKind").value === "entity") {
        const query = `query EntitySubjects($tenantId: ID) {
  entities(tenantId: $tenantId, limit: 200) {
    items { id kind name status tenantId }
  }
}`;
        const result = await requestGraphql(query, { tenantId });
        if (result.errors?.length) throw new Error(summarizeErrors(result));
        state.entities = result.data?.entities?.items || [];
        fillSelect("policySubjectSelect", state.entities, "Choose entity", (item) => `${item.name} (${item.kind})`, "id");
      } else {
        const query = `query GroupSubjects($tenantId: ID) {
  groups(tenantId: $tenantId, limit: 200) {
    items { id name tenantId description }
  }
}`;
        const result = await requestGraphql(query, { tenantId });
        if (result.errors?.length) throw new Error(summarizeErrors(result));
        state.groups = result.data?.groups?.items || [];
        fillSelect("policySubjectSelect", state.groups, "Choose group", (item) => item.name, "id");
      }
      updatePolicyHumanSummary();
    }

    async function loadPolicyGrants() {
      if ($("policyGrantKind").value === "capability") {
        const query = `query CapabilitiesForPolicy {
  capabilities {
    items { id name resourceKind description }
  }
}`;
        const result = await requestGraphql(query, {});
        if (result.errors?.length) throw new Error(summarizeErrors(result));
        state.capabilities = result.data?.capabilities?.items || [];
        fillSelect("policyGrantSelect", state.capabilities, "Choose capability", (item) => `${item.name}${item.resourceKind ? ` (${item.resourceKind})` : ""}`, "id");
      } else {
        const query = `query RolesForPolicy($tenantId: ID) {
  roles(tenantId: $tenantId, limit: 200) {
    items { id name tenantId description }
  }
}`;
        const result = await requestGraphql(query, { tenantId: nullable($("policyTenantSelect").value) });
        if (result.errors?.length) throw new Error(summarizeErrors(result));
        state.roles = result.data?.roles?.items || [];
        fillSelect("policyGrantSelect", state.roles, "Choose role", (item) => item.name, "id");
      }
      updatePolicyHumanSummary();
    }

    async function loadPolicyObjects() {
      const query = `query ObjectsForPolicy($tenantId: ID) {
  resources(tenantId: $tenantId, limit: 200) {
    items { id kind name tenantId ownerId }
  }
  entities(tenantId: $tenantId, limit: 200) {
    items { id kind name status tenantId }
  }
}`;
      const result = await requestGraphql(query, { tenantId: nullable($("policyTenantSelect").value) });
      if (result.errors?.length) throw new Error(summarizeErrors(result));
      state.resources = result.data?.resources?.items || [];
      state.entities = result.data?.entities?.items || state.entities;
      fillPolicyObjectSelect();
      updatePolicyHumanSummary();
    }

    function fillPolicyObjectSelect() {
      const select = $("policyObjectSelect");
      select.innerHTML = `<option value="">Choose resource or entity</option>`;
      const groups = [
        ["Resources", state.resources || [], (item) => `${item.name || item.id} (${item.kind})`],
        ["Entities", state.entities || [], (item) => `${item.name || item.id} (${item.kind})`]
      ];
      for (const [label, items, labelFn] of groups) {
        if (!items.length) continue;
        const group = document.createElement("optgroup");
        group.label = label;
        for (const item of items) {
          const option = document.createElement("option");
          option.value = item.id;
          option.textContent = labelFn(item);
          option.dataset.name = item.name || item.id;
          option.dataset.kind = item.kind || "";
          option.dataset.objectCategory = label.toLowerCase();
          group.appendChild(option);
        }
        select.appendChild(group);
      }
    }

    function policyGrantName() {
      const selected = selectedOption("policyGrantSelect");
      return selected?.value ? (selected.dataset.name || selected.textContent) : $("policyGrantKind").value;
    }

    function policySubjectName() {
      return selectedLabel("policySubjectSelect") || policySubjectId() || "subject";
    }

    function policyScopeLabel() {
      const kind = $("policyScopeKind").value;
      const ref = policyScopeRef();
      if (kind === "platform") return "platform";
      if (kind === "object") return `object ${selectedLabel("policyObjectSelect") || ref || "object"}`;
      if (kind === "tenant") return `tenant ${selectedLabel("policyTenantSelect") || ref || "tenant"}`;
      return `${kind} ${ref || "scope"}`;
    }

    function updatePolicyHumanSummary() {
      const effect = $("policyEffect").value === "deny" ? "Deny" : "Allow";
      const subject = policySubjectName();
      const grant = policyGrantName();
      const subjectKind = $("policySubjectKind").value;
      const grantKind = $("policyGrantKind").value;
      $("policyHumanSummary").innerHTML = `${effect} ${escapeHtml(subjectKind)} <strong>${escapeHtml(subject)}</strong> to ${escapeHtml(grantKind)} <strong>${escapeHtml(grant)}</strong> on <strong>${escapeHtml(policyScopeLabel())}</strong>.`;
      $("setupPolicyHumanSummary").innerHTML = `${escapeHtml(effect)} subject to capability on resource <code>${escapeHtml($("setupPolicyResourceId").value || "resource id")}</code>.`;
    }

    function preparePolicyPreview() {
      updatePolicyHumanSummary();
      const built = policyBuild();
      setPreview(built.query, built.variables, "policyGraphql", "policyVariables");
      setAdvancedOperation(built.query, built.variables);
      $("policyCurl").textContent = curlSnippet(built.query, built.variables);
      $("policyJs").textContent = jsSnippet(built.query, built.variables);
      updateTemplateOutputs();
      return built;
    }

    function previewPolicy() {
      try {
        preparePolicyPreview();
        $("policySummary").innerHTML = `<span class="badge ok">Preview ready</span> Review generated createPolicy GraphQL before running.`;
      } catch (err) {
        $("policySummary").innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    function copyPolicyGraphql() {
      try {
        const built = preparePolicyPreview();
        copyText(built.query);
      } catch (err) {
        $("policySummary").innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    function copyPolicyCurl() {
      try {
        preparePolicyPreview();
        copyText($("policyCurl").textContent);
      } catch (err) {
        $("policySummary").innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    function copyPolicyJs() {
      try {
        preparePolicyPreview();
        copyText($("policyJs").textContent);
      } catch (err) {
        $("policySummary").innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    async function savePolicyTemplate() {
      try {
        preparePolicyPreview();
        const tenantId = nullable($("policyTenantSelect").value);
        if (tenantId && !$("templateTenantId").value.trim()) $("templateTenantId").value = tenantId;
        const saved = await saveApiTemplate();
        if (!saved) $("policySummary").innerHTML = `<span class="badge warn">Not saved</span> Template save was cancelled or failed.`;
      } catch (err) {
        $("policySummary").innerHTML = `<span class="badge error">Save failed</span> ${escapeHtml(err.message)}`;
      }
    }

    function testThisPolicy() {
      $("authzSubjectId").value = policySubjectId() || "";
      if ($("policyGrantKind").value === "capability") $("authzAction").value = policyGrantName().split(" ")[0];
      if ($("policyScopeKind").value === "object") $("authzResourceId").value = policyScopeRef() || "";
      showScreen("authz");
      previewOnly(() => authzBuild("authzCheck"), "authzGraphql", "authzVariables", "authzNarrative", "Policy values copied into Authz builder where possible.");
    }

    function renderAuthzNarrative(result, name, targetId = "authzNarrative") {
      const errors = summarizeErrors(result);
      if (errors) {
        $(targetId).innerHTML = `<span class="badge error">Error</span> ${escapeHtml(errors)} <br>Next suggested action: verify the token and input IDs.`;
        return;
      }
      const decision = result.data?.[name];
      if (!decision) {
        $(targetId).innerHTML = `<span class="badge warn">No decision</span> GraphQL returned no ${escapeHtml(name)} payload.`;
        return;
      }
      const badge = decision.allowed ? `<span class="badge ok">Allowed</span>` : `<span class="badge error">Denied</span>`;
      const next = decision.allowed
        ? "Next suggested action: use this same subject/action/resource tuple in your application flow."
        : "Next suggested action: confirm the subject, capability action, resource, policy scope, and any deny policies.";
      const details = decision.details || decision.evaluatedBindings || {};
      $(targetId).innerHTML = `${badge} ${escapeHtml(decision.reason || "No reason returned")}<br><br><strong>Details</strong><pre>${escapeHtml(JSON.stringify(details, null, 2))}</pre><br>${escapeHtml(next)}`;
    }

    async function runAuthz(name, prefix = "authz", resultTarget = "authzResult", narrativeTarget = "authzNarrative") {
      state.lastAuthzName = name;
      const queryTarget = prefix === "authz" ? "authzGraphql" : `${prefix}Graphql`;
      const variablesTarget = prefix === "authz" ? "authzVariables" : `${prefix}Variables`;
      try {
        const built = authzBuild(name, prefix);
        setPreview(built.query, built.variables, queryTarget, variablesTarget);
        setAdvancedOperation(built.query, built.variables);
        const result = await requestGraphql(built.query, built.variables);
        writeRaw(resultTarget, result);
        renderAuthzNarrative(result, name, narrativeTarget);
      } catch (err) {
        $(narrativeTarget).innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    async function runCredential(kind) {
      try {
        const built = credentialBuild(kind);
        const safe = JSON.parse(JSON.stringify(built.variables));
        if (safe.password) safe.password = "<redacted>";
        setPreview(built.query, safe, "credentialGraphql", "credentialVariables");
        setAdvancedOperation(built.query, built.variables);
        const result = await requestGraphql(built.query, built.variables);
        writeRaw("credentialResult", result);
        $("credentialSummary").innerHTML = result.errors?.length
          ? `<span class="badge error">Error</span> ${escapeHtml(summarizeErrors(result))}`
          : `<span class="badge ok">Success</span> Credential operation completed. Review raw result for returned fields.`;
      } catch (err) {
        $("credentialSummary").innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    function showWizardStep(step) {
      document.querySelectorAll(".wizard-step").forEach((item) => item.classList.toggle("active", item.id === `wizard-step-${step}`));
      document.querySelectorAll(".wizard-tab").forEach((item) => item.classList.toggle("active", item.dataset.step === String(step)));
    }

    function copyText(text) {
      navigator.clipboard.writeText(text);
    }

    function snippetPayload(query, variables) {
      return JSON.stringify({ query, variables }, null, 2);
    }

    function curlSnippet(query, variables) {
      return `curl -sS ${endpointSafety().url.href} \\
  -H 'Content-Type: application/json' \\
  -H 'Authorization: Bearer $ATOM_TOKEN' \\
  --data '${snippetPayload(query, variables).replace(/'/g, "'\\''")}'`;
    }

    function jsSnippet(query, variables) {
      return `const response = await fetch("${endpointSafety().url.href}", {
  method: "POST",
  headers: {
    "Content-Type": "application/json",
    "Authorization": "Bearer " + token
  },
  body: JSON.stringify(${snippetPayload(query, variables)})
});

const result = await response.json();`;
    }

    function templateListQuery() {
      return `query ApiTemplates($tenantId: ID, $status: ApiTemplateStatus, $tag: String, $limit: Int = 100, $offset: Int = 0) {
  apiTemplates(tenantId: $tenantId, status: $status, tag: $tag, limit: $limit, offset: $offset) {
    items {
      id
      tenantId
      key
      name
      description
      operationKind
      graphql
      variablesSchema
      defaultVariables
      resultSelector
      tags
      status
      updatedAt
    }
    total
  }
}`;
    }

    function createTemplateMutation() {
      return `mutation CreateApiTemplate($input: CreateApiTemplateInput!) {
  createApiTemplate(input: $input) {
    id
    tenantId
    key
    name
    description
    operationKind
    graphql
    variablesSchema
    defaultVariables
    resultSelector
    tags
    status
    updatedAt
  }
}`;
    }

    function updateTemplateMutation() {
      return `mutation UpdateApiTemplate($id: ID!, $input: UpdateApiTemplateInput!) {
  updateApiTemplate(id: $id, input: $input) {
    id
    tenantId
    key
    name
    description
    operationKind
    graphql
    variablesSchema
    defaultVariables
    resultSelector
    tags
    status
    updatedAt
  }
}`;
    }

    function apiEndpointListQuery() {
      return `query ApiEndpoints($tenantId: ID, $status: String, $limit: Int = 100, $offset: Int = 0) {
  apiEndpoints(tenantId: $tenantId, status: $status, limit: $limit, offset: $offset) {
    items {
      id
      tenantId
      key
      name
      description
      method
      path
      templateId
      authMode
      serviceEntityId
      variablesMapping
      requestSchema
      responseMapping
      status
      updatedAt
    }
    total
  }
}`;
    }

    function createEndpointMutation() {
      return `mutation CreateApiEndpoint($input: CreateApiEndpointInput!) {
  createApiEndpoint(input: $input) {
    id
    key
    name
    method
    path
    templateId
    authMode
    status
  }
}`;
    }

    function enableEndpointMutation() {
      return `mutation EnableApiEndpoint($id: ID!) {
  enableApiEndpoint(id: $id) {
    id
    key
    method
    path
    status
  }
}`;
    }

    function disableEndpointMutation() {
      return `mutation DisableApiEndpoint($id: ID!) {
  disableApiEndpoint(id: $id) {
    id
    key
    method
    path
    status
  }
}`;
    }

    function templateSummaryText(template) {
      const tags = (template.tags || []).join(", ") || "untagged";
      const scope = template.tenantId ? `tenant ${template.tenantId}` : "global";
      return `${template.operationKind} · ${template.status} · ${scope} · ${tags}`;
    }

    async function loadApiTemplates() {
      $("templateStatus").className = "badge warn";
      $("templateStatus").textContent = "Loading";
      try {
        const variables = {
          tenantId: nullable($("templateTenantId").value),
          status: nullable($("templateStatusFilter").value),
          tag: nullable($("templateTagFilter").value),
          limit: 100,
          offset: 0
        };
        const result = await requestGraphql(templateListQuery(), variables);
        if (result.errors?.length) throw new Error(summarizeErrors(result));
        state.apiTemplates = result.data?.apiTemplates?.items || [];
        state.selectedApiTemplate = state.apiTemplates.find((item) => item.id === state.selectedApiTemplate?.id) || null;
        renderTemplateGroups();
        fillEndpointTemplateSelect();
        $("templateStatus").className = "badge ok";
        $("templateStatus").textContent = `${state.apiTemplates.length} templates`;
        $("templateSummary").innerHTML = state.apiTemplates.length
          ? `<span class="badge ok">Loaded</span> Select a template to edit, run, copy, or update.`
          : `<span class="badge warn">No templates</span> Try another tenant, status, or tag filter.`;
      } catch (err) {
        state.apiTemplates = [];
        state.selectedApiTemplate = null;
        renderTemplateGroups();
        fillEndpointTemplateSelect();
        $("templateStatus").className = "badge error";
        $("templateStatus").textContent = "Unavailable";
        $("templateSummary").innerHTML = `<span class="badge error">Template API unavailable</span> ${escapeHtml(err.message)} Local recipe fallback below still works.`;
      }
    }

    function renderTemplateGroups() {
      const target = $("templateGroups");
      target.innerHTML = "";
      if (!state.apiTemplates.length) {
        target.innerHTML = `<div class="help">No API Templates loaded. Saved localStorage recipes remain available in the fallback panel below.</div>`;
        $("templateVariablesSchema").textContent = "";
        updateTemplateOutputs();
        return;
      }

      const groups = new Map();
      for (const template of state.apiTemplates) {
        const tags = template.tags?.length ? template.tags : ["untagged"];
        for (const tag of tags) {
          if (!groups.has(tag)) groups.set(tag, []);
          groups.get(tag).push(template);
        }
      }

      for (const [tag, templates] of Array.from(groups.entries()).sort(([a], [b]) => a.localeCompare(b))) {
        const section = document.createElement("section");
        section.className = "template-group";
        section.innerHTML = `<h3>${escapeHtml(tag)}</h3>`;
        for (const template of templates.sort((a, b) => a.key.localeCompare(b.key))) {
          const button = document.createElement("button");
          button.className = `template-button${template.id === state.selectedApiTemplate?.id ? " active" : ""}`;
          button.innerHTML = `<strong>${escapeHtml(template.name || template.key)}</strong><span>${escapeHtml(template.key)} · ${escapeHtml(templateSummaryText(template))}</span>`;
          button.addEventListener("click", () => selectApiTemplate(template.id));
          section.appendChild(button);
        }
        target.appendChild(section);
      }
    }

    function selectApiTemplate(id) {
      const template = state.apiTemplates.find((item) => item.id === id);
      if (!template) return;
      state.selectedApiTemplate = template;
      $("queryEditor").value = template.graphql || "";
      $("variablesEditor").value = JSON.stringify(template.defaultVariables || {}, null, 2);
      $("templateVariablesSchema").textContent = JSON.stringify(template.variablesSchema || {}, null, 2);
      $("templateSummary").innerHTML = `<span class="badge ok">Selected</span> ${escapeHtml(template.name || template.key)}. Loaded GraphQL and default variables into Advanced GraphQL.`;
      updateTemplateOutputs();
      renderTemplateGroups();
      showScreen("api-builder");
    }

    function currentEditorVariables() {
      const text = $("variablesEditor").value.trim();
      if (!text) return {};
      return JSON.parse(text);
    }

    function selectedTemplateResultPath() {
      const selector = state.selectedApiTemplate?.resultSelector || {};
      return Array.isArray(selector.path) ? selector.path : [];
    }

    function valueAtPath(value, path) {
      let current = value;
      for (const segment of path) {
        if (current == null) return undefined;
        current = current[segment];
      }
      return current;
    }

    function resultSummaryForTemplate(result) {
      const errors = summarizeErrors(result);
      if (errors) return `<span class="badge error">Error</span> ${escapeHtml(errors)}`;
      const path = selectedTemplateResultPath();
      if (path.length) {
        const selected = valueAtPath(result.data, path);
        return `<span class="badge ok">Success</span> Result selector <code>${escapeHtml(path.join("."))}</code>: <pre>${escapeHtml(JSON.stringify(selected, null, 2))}</pre>`;
      }
      return `<span class="badge ok">Success</span> Template operation completed. Review raw response below.`;
    }

    function isSensitiveKey(key) {
      return /password|secret|token|api[_-]?key|apikey|authorization/i.test(key);
    }

    function sensitiveVariablePaths(value, prefix = []) {
      if (!value || typeof value !== "object") return [];
      const paths = [];
      for (const [key, nested] of Object.entries(value)) {
        const path = [...prefix, key];
        if (isSensitiveKey(key)) paths.push(path.join("."));
        paths.push(...sensitiveVariablePaths(nested, path));
      }
      return paths;
    }

    function redactSensitiveVariables(value) {
      if (Array.isArray(value)) return value.map(redactSensitiveVariables);
      if (!value || typeof value !== "object") return value;
      return Object.fromEntries(Object.entries(value).map(([key, nested]) => [
        key,
        isSensitiveKey(key) ? "<redacted>" : redactSensitiveVariables(nested)
      ]));
    }

    function updateTemplateSecretWarning() {
      let variables = {};
      try {
        variables = currentEditorVariables();
      } catch (_) {
        $("templateSecretWarning").classList.add("hidden");
        return [];
      }
      const paths = sensitiveVariablePaths(variables);
      $("templateSecretWarning").classList.toggle("hidden", !paths.length);
      $("templateSecretWarning").textContent = paths.length
        ? `Sensitive-looking variable keys detected: ${paths.join(", ")}. These values are redacted before saving a template.`
        : "";
      return paths;
    }

    function updateTemplateOutputs() {
      updateTemplateSecretWarning();
      try {
        const variables = currentEditorVariables();
        const curl = curlSnippet($("queryEditor").value, variables);
        const js = jsSnippet($("queryEditor").value, variables);
        $("templateCurl").textContent = curl;
        $("templateJs").textContent = js;
        $("advancedCurl").textContent = curl;
        $("advancedJs").textContent = js;
      } catch (err) {
        const message = `Variables JSON error: ${err.message}`;
        $("templateCurl").textContent = message;
        $("templateJs").textContent = message;
        $("advancedCurl").textContent = message;
        $("advancedJs").textContent = message;
      }
    }

    async function runSelectedTemplate() {
      try {
        const query = $("queryEditor").value;
        const variables = currentEditorVariables();
        updateTemplateOutputs();
        const result = await requestGraphql(query, variables);
        writeRaw("templateResult", result);
        $("templateSummary").innerHTML = resultSummaryForTemplate(result);
      } catch (err) {
        $("templateSummary").innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    function detectOperationKind(query) {
      return /^\s*(query\b|\{)/i.test(query) ? "query" : "mutation";
    }

    function promptWithDefault(label, fallback = "") {
      const value = prompt(label, fallback || "");
      return value == null ? null : value.trim();
    }

    function templateInputFromEditor(existing = null) {
      const query = $("queryEditor").value.trim();
      if (!query) throw new Error("GraphQL editor is empty");
      const variables = currentEditorVariables();
      const redactedVariables = redactSensitiveVariables(variables);
      const key = promptWithDefault("Template key", existing?.key || "");
      if (!key) return null;
      const name = promptWithDefault("Template name", existing?.name || key);
      if (!name) return null;
      const description = promptWithDefault("Template description", existing?.description || "");
      if (description == null) return null;
      const tagsText = promptWithDefault("Tags, comma separated", (existing?.tags || []).join(", "));
      if (tagsText == null) return null;
      const tenantId = existing?.tenantId || nullable($("templateTenantId").value);
      return {
        tenantId,
        key,
        name,
        description: nullable(description),
        operationKind: detectOperationKind(query),
        graphql: query,
        variablesSchema: existing?.variablesSchema || {},
        defaultVariables: redactedVariables,
        resultSelector: existing?.resultSelector || {},
        tags: commaList(tagsText),
        status: existing?.status || "active"
      };
    }

    async function saveApiTemplate() {
      try {
        const input = templateInputFromEditor();
        if (!input) return false;
        const result = await requestGraphql(createTemplateMutation(), { input });
        if (result.errors?.length) throw new Error(summarizeErrors(result));
        const template = result.data.createApiTemplate;
        state.selectedApiTemplate = template;
        $("templateSummary").innerHTML = `<span class="badge ok">Saved</span> Template <code>${escapeHtml(template.key)}</code> created.`;
        await loadApiTemplates();
        selectApiTemplate(template.id);
        return true;
      } catch (err) {
        $("templateSummary").innerHTML = `<span class="badge error">Save failed</span> ${escapeHtml(err.message)}`;
        return false;
      }
    }

    async function updateApiTemplate() {
      try {
        if (!state.selectedApiTemplate) throw new Error("Select a template before updating");
        const input = templateInputFromEditor(state.selectedApiTemplate);
        if (!input) return;
        delete input.tenantId;
        const result = await requestGraphql(updateTemplateMutation(), { id: state.selectedApiTemplate.id, input });
        if (result.errors?.length) throw new Error(summarizeErrors(result));
        const template = result.data.updateApiTemplate;
        state.selectedApiTemplate = template;
        $("templateSummary").innerHTML = `<span class="badge ok">Updated</span> Template <code>${escapeHtml(template.key)}</code> updated.`;
        await loadApiTemplates();
        selectApiTemplate(template.id);
      } catch (err) {
        $("templateSummary").innerHTML = `<span class="badge error">Update failed</span> ${escapeHtml(err.message)}`;
      }
    }

    function fillEndpointTemplateSelect() {
      const select = $("endpointTemplateSelect");
      if (!select) return;
      select.innerHTML = `<option value="">Choose saved API template</option>`;
      for (const template of (state.apiTemplates || []).sort((a, b) => a.key.localeCompare(b.key))) {
        const option = document.createElement("option");
        option.value = template.id;
        option.textContent = `${template.key} - ${template.name || template.key}`;
        select.appendChild(option);
      }
    }

    async function loadEndpointTemplates() {
      if (!state.apiTemplates.length) await loadApiTemplates();
      fillEndpointTemplateSelect();
      $("endpointBuilderSummary").innerHTML = `<span class="badge ok">Templates loaded</span> Choose a template, then configure method and path.`;
    }

    async function loadApiEndpoints() {
      try {
        const result = await requestGraphql(apiEndpointListQuery(), {
          tenantId: nullable($("endpointTenantId").value),
          status: nullable($("endpointStatusFilter").value),
          limit: 100,
          offset: 0
        });
        if (result.errors?.length) throw new Error(summarizeErrors(result));
        state.apiEndpoints = result.data?.apiEndpoints?.items || [];
        const select = $("endpointSelect");
        select.innerHTML = `<option value="">Choose endpoint</option>`;
        for (const endpoint of state.apiEndpoints) {
          const option = document.createElement("option");
          option.value = endpoint.id;
          option.textContent = `${endpoint.method} ${endpoint.path} - ${endpoint.status}`;
          select.appendChild(option);
        }
        $("endpointBuilderSummary").innerHTML = `<span class="badge ok">Loaded</span> ${state.apiEndpoints.length} endpoints loaded.`;
      } catch (err) {
        $("endpointBuilderSummary").innerHTML = `<span class="badge error">Load failed</span> ${escapeHtml(err.message)}`;
      }
    }

    function selectApiEndpoint() {
      const endpoint = state.apiEndpoints.find((item) => item.id === $("endpointSelect").value);
      if (!endpoint) return;
      state.selectedApiEndpoint = endpoint;
      $("endpointTenantId").value = endpoint.tenantId || "";
      $("endpointKey").value = endpoint.key || "";
      $("endpointName").value = endpoint.name || "";
      $("endpointDescription").value = endpoint.description || "";
      $("endpointMethod").value = endpoint.method || "POST";
      $("endpointPath").value = endpoint.path || "/api/custom/";
      $("endpointTemplateSelect").value = endpoint.templateId || "";
      $("endpointAuthMode").value = endpoint.authMode || "caller_context";
      $("endpointServiceEntityId").value = endpoint.serviceEntityId || "";
      $("endpointVariablesMapping").value = JSON.stringify(endpoint.variablesMapping || {}, null, 2);
      $("endpointRequestSchema").value = JSON.stringify(endpoint.requestSchema || {}, null, 2);
      $("endpointResponseMapping").value = JSON.stringify(endpoint.responseMapping || {}, null, 2);
      previewApiEndpoint();
    }

    function apiEndpointInput(status = "draft") {
      const path = $("endpointPath").value.trim();
      if (!path.startsWith("/api/custom/")) throw new Error("Path must start with /api/custom/");
      return {
        tenantId: nullable($("endpointTenantId").value),
        key: $("endpointKey").value.trim(),
        name: $("endpointName").value.trim(),
        description: nullable($("endpointDescription").value),
        method: $("endpointMethod").value,
        path,
        templateId: $("endpointTemplateSelect").value,
        authMode: $("endpointAuthMode").value,
        serviceEntityId: nullable($("endpointServiceEntityId").value),
        variablesMapping: parseJson("endpointVariablesMapping"),
        requestSchema: parseJson("endpointRequestSchema"),
        responseMapping: parseJson("endpointResponseMapping"),
        status
      };
    }

    function previewApiEndpoint() {
      try {
        const input = apiEndpointInput(state.selectedApiEndpoint?.status || "draft");
        $("endpointPreview").textContent = JSON.stringify({
          endpoint: `${input.method} ${input.path}`,
          templateId: input.templateId,
          authMode: input.authMode,
          variablesMapping: input.variablesMapping,
          requestSchema: input.requestSchema,
          responseMapping: input.responseMapping
        }, null, 2);
        $("endpointBuilderSummary").innerHTML = `<span class="badge ok">Preview ready</span> ${escapeHtml(input.method)} ${escapeHtml(input.path)} runs saved GraphQL template <code>${escapeHtml(input.templateId || "template id")}</code>.`;
      } catch (err) {
        $("endpointBuilderSummary").innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    async function createApiEndpoint() {
      try {
        const input = apiEndpointInput("draft");
        const result = await requestGraphql(createEndpointMutation(), { input });
        if (result.errors?.length) throw new Error(summarizeErrors(result));
        const endpoint = result.data.createApiEndpoint;
        $("endpointBuilderSummary").innerHTML = `<span class="badge ok">Created</span> Draft endpoint <code>${escapeHtml(endpoint.method)} ${escapeHtml(endpoint.path)}</code> created.`;
        await loadApiEndpoints();
        $("endpointSelect").value = endpoint.id;
        selectApiEndpoint();
      } catch (err) {
        $("endpointBuilderSummary").innerHTML = `<span class="badge error">Create failed</span> ${escapeHtml(err.message)}`;
      }
    }

    async function setSelectedApiEndpointStatus(mutationFn, label) {
      try {
        const id = $("endpointSelect").value;
        if (!id) throw new Error("Select an endpoint first");
        const result = await requestGraphql(mutationFn(), { id });
        if (result.errors?.length) throw new Error(summarizeErrors(result));
        $("endpointBuilderSummary").innerHTML = `<span class="badge ok">${escapeHtml(label)}</span> Endpoint ${escapeHtml(label.toLowerCase())}.`;
        await loadApiEndpoints();
      } catch (err) {
        $("endpointBuilderSummary").innerHTML = `<span class="badge error">${escapeHtml(label)} failed</span> ${escapeHtml(err.message)}`;
      }
    }

    async function testApiEndpoint() {
      try {
        const input = apiEndpointInput(state.selectedApiEndpoint?.status || "draft");
        const body = parseJson("endpointSampleBody");
        const result = await fetch(input.path, {
          method: input.method,
          headers: {
            "Content-Type": "application/json",
            ...authHeaders(true)
          },
          body: ["GET", "DELETE"].includes(input.method) ? undefined : JSON.stringify(body)
        });
        const text = await result.text();
        $("endpointTestResult").textContent = text;
        $("endpointBuilderSummary").innerHTML = `<span class="badge ${result.ok ? "ok" : "error"}">HTTP ${result.status}</span> Custom endpoint test completed.`;
      } catch (err) {
        $("endpointBuilderSummary").innerHTML = `<span class="badge error">Test failed</span> ${escapeHtml(err.message)}`;
      }
    }

    const recipes = {
      createTenant: {
        title: "Create tenant",
        fields: [
          ["name", "Tenant name", "factory-a"],
          ["route", "Route", "factory-a"],
          ["attributes", "Attributes JSON", "{}"]
        ],
        build(values) {
          return {
            query: tenantMutation(),
            variables: { input: { name: values.name, route: nullable(values.route), attributes: JSON.parse(values.attributes || "{}") } }
          };
        }
      },
      createProfile: {
        title: "Create profile",
        fields: [
          ["tenantId", "Tenant ID", ""],
          ["objectKind", "Object kind", "entity"],
          ["kind", "Internal kind", "device"],
          ["key", "Profile key", "client"],
          ["displayName", "Display name", "Client"]
        ],
        build(values) {
          return {
            query: profileMutation(),
            variables: { input: { tenantId: nullable(values.tenantId), objectKind: values.objectKind, kind: values.kind, key: values.key, displayName: values.displayName, description: null, status: "active" } }
          };
        }
      },
      createEntityFromProfile: {
        title: "Create entity from profile",
        fields: [
          ["tenantId", "Tenant ID", ""],
          ["profileId", "Profile ID", ""],
          ["profileVersionId", "Profile version ID", ""],
          ["name", "Entity name", "device-001"],
          ["attributes", "Attributes JSON", "{}"]
        ],
        build(values) {
          const input = { tenantId: nullable(values.tenantId), profileId: values.profileId, profileVersionId: nullable(values.profileVersionId), name: values.name, attributes: JSON.parse(values.attributes || "{}") };
          if (!input.profileVersionId) delete input.profileVersionId;
          return { query: entityMutation(), variables: { input } };
        }
      },
      createResource: {
        title: "Create resource",
        fields: [
          ["tenantId", "Tenant ID", ""],
          ["kind", "Resource kind", "channel"],
          ["name", "Resource name", "telemetry"],
          ["attributes", "Attributes JSON", "{\"topic\":\"telemetry\"}"]
        ],
        build(values) {
          return { query: resourceMutation(), variables: { input: { tenantId: nullable(values.tenantId), kind: values.kind, name: nullable(values.name), ownerId: null, attributes: JSON.parse(values.attributes || "{}") } } };
        }
      },
      grantCapability: {
        title: "Grant capability",
        fields: [
          ["tenantId", "Tenant ID", ""],
          ["subjectKind", "Subject kind", "entity"],
          ["subjectId", "Subject ID", ""],
          ["capabilityId", "Capability ID", ""],
          ["scopeKind", "Scope kind", "object"],
          ["scopeRef", "Scope ref", ""],
          ["conditions", "Conditions JSON", "{}"]
        ],
        build(values) {
          return { query: policyMutation(), variables: { input: { tenantId: nullable(values.tenantId), subjectKind: values.subjectKind, subjectId: values.subjectId, grantKind: "capability", grantId: values.capabilityId, scopeKind: values.scopeKind, scopeRef: nullable(values.scopeRef), effect: "allow", conditions: JSON.parse(values.conditions || "{}") } } };
        }
      },
      createApiKey: {
        title: "Create API key",
        fields: [
          ["entityId", "Entity ID", ""],
          ["description", "Description", "automation key"],
          ["expiresAt", "Expires at", ""]
        ],
        build(values) {
          return { query: credentialBuild("apiKey").query, variables: { entityId: values.entityId, input: { description: nullable(values.description), expiresAt: nullable(values.expiresAt) } } };
        }
      },
      runAuthzCheck: {
        title: "Run authz check",
        fields: [
          ["subjectId", "Subject ID", ""],
          ["action", "Action", "publish"],
          ["resourceId", "Resource ID", ""],
          ["context", "Context JSON", "{}"]
        ],
        build(values) {
          return { query: authzMutation("authzCheck"), variables: { input: { subjectId: values.subjectId, action: values.action, resourceId: nullable(values.resourceId), objectKind: null, objectId: null, context: JSON.parse(values.context || "{}") } } };
        }
      }
    };

    function renderRecipeList() {
      $("recipeSelect").innerHTML = Object.entries(recipes).map(([key, recipe]) => `<option value="${key}">${escapeHtml(recipe.title)}</option>`).join("");
      renderRecipeForm();
      renderSavedRecipes();
    }

    function renderRecipeForm() {
      const recipe = recipes[$("recipeSelect").value];
      $("recipeForm").innerHTML = recipe.fields.map(([key, label, value]) => {
        if (key.toLowerCase().includes("json") || key === "attributes" || key === "conditions" || key === "context") {
          return `<label>${escapeHtml(label)}<textarea id="recipeField-${escapeHtml(key)}" spellcheck="false">${escapeHtml(value)}</textarea></label>`;
        }
        return `<label>${escapeHtml(label)}<input id="recipeField-${escapeHtml(key)}" value="${escapeHtml(value)}" /></label>`;
      }).join("");
      generateRecipe();
    }

    function recipeValues() {
      const recipe = recipes[$("recipeSelect").value];
      return Object.fromEntries(recipe.fields.map(([key]) => [key, $(`recipeField-${key}`).value]));
    }

    function buildRecipe() {
      return recipes[$("recipeSelect").value].build(recipeValues());
    }

    function generateRecipe() {
      try {
        const built = buildRecipe();
        $("recipeGraphql").textContent = built.query;
        $("recipeVariables").textContent = JSON.stringify(built.variables, null, 2);
        $("recipeCurl").textContent = curlSnippet(built.query, built.variables);
        $("recipeJs").textContent = jsSnippet(built.query, built.variables);
        $("recipeSummary").innerHTML = `<span class="badge ok">Generated</span> Review GraphQL and variables before running.`;
      } catch (err) {
        $("recipeSummary").innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    async function runRecipe() {
      try {
        const built = buildRecipe();
        const result = await requestGraphql(built.query, built.variables);
        writeRaw("recipeResult", result);
        $("recipeSummary").innerHTML = result.errors?.length
          ? `<span class="badge error">Error</span> ${escapeHtml(summarizeErrors(result))}`
          : `<span class="badge ok">Success</span> Recipe ran successfully.`;
      } catch (err) {
        $("recipeSummary").innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    function renderSavedRecipes() {
      const saved = JSON.parse(localStorage.getItem("atom.graphql.console.recipes") || "{}");
      $("savedRecipes").innerHTML = Object.keys(saved).map((name) => `<option>${escapeHtml(name)}</option>`).join("");
    }

    function saveRecipe() {
      generateRecipe();
      const name = prompt("Recipe name");
      if (!name) return;
      const saved = JSON.parse(localStorage.getItem("atom.graphql.console.recipes") || "{}");
      saved[name] = {
        recipe: $("recipeSelect").value,
        values: recipeValues(),
        query: $("recipeGraphql").textContent,
        variables: $("recipeVariables").textContent
      };
      localStorage.setItem("atom.graphql.console.recipes", JSON.stringify(saved));
      renderSavedRecipes();
    }

    function loadSavedRecipe() {
      const saved = JSON.parse(localStorage.getItem("atom.graphql.console.recipes") || "{}");
      const item = saved[$("savedRecipes").value];
      if (!item) return;
      $("recipeSelect").value = item.recipe;
      renderRecipeForm();
      for (const [key, value] of Object.entries(item.values || {})) {
        const field = $(`recipeField-${key}`);
        if (field) field.value = value;
      }
      generateRecipe();
    }

    function renderSavedExamples() {
      const examples = JSON.parse(localStorage.getItem("atom.graphql.console.examples") || "{}");
      $("savedExamples").innerHTML = Object.keys(examples).map((name) => `<option>${escapeHtml(name)}</option>`).join("");
    }

    function saveExample() {
      const name = prompt("Example name");
      if (!name) return;
      const examples = JSON.parse(localStorage.getItem("atom.graphql.console.examples") || "{}");
      examples[name] = { query: $("queryEditor").value, variables: $("variablesEditor").value };
      localStorage.setItem("atom.graphql.console.examples", JSON.stringify(examples));
      renderSavedExamples();
    }

    function loadExample() {
      const examples = JSON.parse(localStorage.getItem("atom.graphql.console.examples") || "{}");
      const selected = examples[$("savedExamples").value];
      if (!selected) return;
      $("queryEditor").value = selected.query;
      $("variablesEditor").value = selected.variables;
      showScreen("advanced");
    }

    async function runCurrent() {
      try {
        const result = await requestGraphql($("queryEditor").value, JSON.parse($("variablesEditor").value || "{}"));
        writeRaw("responseViewer", result);
        $("responseStatus").className = result.errors?.length ? "badge error" : "badge ok";
        $("responseStatus").textContent = result.errors?.length ? "GraphQL error" : "Success";
        $("responseSummary").innerHTML = result.errors?.length
          ? `<span class="badge error">Error</span> ${escapeHtml(summarizeErrors(result))}`
          : `<span class="badge ok">Success</span> Operation completed.`;
      } catch (err) {
        $("responseStatus").className = "badge error";
        $("responseStatus").textContent = "Input error";
        $("responseSummary").innerHTML = `<span class="badge error">Input error</span> ${escapeHtml(err.message)}`;
      }
    }

    function schemaSummaryForPrompt() {
      const queries = (state.typeMap.get(state.schema?.queryType?.name)?.fields || []).map((field) => field.name);
      const mutations = (state.typeMap.get(state.schema?.mutationType?.name)?.fields || []).map((field) => field.name);
      const enums = Array.from(state.typeMap.values()).filter((type) => type.kind === "ENUM").map((type) => `${type.name}: ${(type.enumValues || []).map((item) => item.name).join(", ")}`);
      const inputTypes = (state.schemaModel?.inputObjects || []).map((type) => type.name);
      const objectTypes = (state.schemaModel?.objects || []).map((type) => type.name).filter((name) => !name.startsWith("__"));
      return `Available GraphQL operations from introspection:
Queries: ${queries.join(", ") || "none loaded"}
Mutations: ${mutations.join(", ") || "none loaded"}
Input object types: ${inputTypes.join(", ") || "none loaded"}
Object types: ${objectTypes.join(", ") || "none loaded"}
Enums:
${enums.join("\n") || "none loaded"}`;
    }

    function assistantSelectedContext() {
      return {
        tenantId: selectedOrInput("entityTenantSelect", "entityTenantId") || nullable($("resourceTenantId").value) || nullable($("policyTenantSelect").value),
        tenantName: selectedLabel("entityTenantSelect") || selectedLabel("resourceTenantSelect") || selectedLabel("policyTenantSelect"),
        entityKind: $("entityKind").value,
        profileId: nullable($("entityProfile").value),
        profile: selectedLabel("entityProfile"),
        profileVersionId: nullable($("entityProfileVersion").value),
        profileVersion: selectedLabel("entityProfileVersion"),
        resourceKind: resourceKind("resource"),
        resourceId: nullable($("authzResourceId").value) || nullable($("policyObjectSelect").value),
        resource: selectedLabel("policyObjectSelect"),
        subjectKind: $("policySubjectKind").value,
        subjectId: nullable($("authzSubjectId").value) || policySubjectId(),
        subject: selectedLabel("policySubjectSelect"),
        grantKind: $("policyGrantKind").value,
        grantId: policyGrantId(),
        grant: selectedLabel("policyGrantSelect"),
        scopeKind: $("policyScopeKind").value,
        scopeRef: policyScopeRef()
      };
    }

    function refreshAssistantContext() {
      $("assistantContext").value = JSON.stringify(assistantSelectedContext(), null, 2);
    }

    function assistantContextFromEditor() {
      const text = $("assistantContext").value.trim();
      return text ? JSON.parse(text) : assistantSelectedContext();
    }

    function relevantApiTemplatesForPrompt() {
      const request = $("assistantRequest").value.toLowerCase();
      const words = request.split(/[^a-z0-9_]+/).filter((word) => word.length > 2);
      const templates = state.apiTemplates || [];
      const scored = templates.map((template) => {
        const haystack = [
          template.key,
          template.name,
          template.description,
          ...(template.tags || [])
        ].join(" ").toLowerCase();
        const score = words.filter((word) => haystack.includes(word)).length;
        return { template, score };
      });
      const relevant = scored.filter((item) => item.score > 0).sort((a, b) => b.score - a.score).map((item) => item.template);
      return (relevant.length ? relevant : templates).slice(0, 8);
    }

    function savedTemplatesForPrompt() {
      const templates = relevantApiTemplatesForPrompt();
      if (!templates.length) return "No saved API templates are currently loaded.";
      return templates.map((template) => `- ${template.key}: ${template.name || template.key}
  operationKind: ${template.operationKind}
  tags: ${(template.tags || []).join(", ") || "none"}
  description: ${template.description || "none"}
  graphql:
${indent(template.graphql || "")}
  defaultVariables:
${indent(JSON.stringify(template.defaultVariables || {}, null, 2))}`).join("\n\n");
    }

    function currentOperationForPrompt() {
      const query = $("queryEditor").value.trim();
      const variables = $("variablesEditor").value.trim() || "{}";
      if (!query) return "No current operation is loaded.";
      return `GraphQL:
${query}

variables JSON:
${variables}`;
    }

    function generatePrompt() {
      try {
        const sections = [`You are helping generate generic Atom GraphQL.

Atom model rules:
- Use GraphQL introspection and Atom GraphQL operations only.
- Use createTenant for domain-like isolation boundaries.
- Use createEntity for people, devices, services, workloads, and applications.
- Use createResource for protected objects, including kind "channel" when needed.
- Use createPolicy for access grants and connections.
- kind is Atom internal runtime/authz kind.
- profile is the user or domain subtype/schema.
- profileVersion is validation and history only.
- Do not invent external-system GraphQL aliases.
`];

        if ($("assistantIncludeSchema").checked) sections.push(schemaSummaryForPrompt());
        sections.push(`Selected tenant/profile/resource context:
${JSON.stringify(assistantContextFromEditor(), null, 2)}`);
        if ($("assistantIncludeTemplates").checked) sections.push(`Relevant saved API templates:
${savedTemplatesForPrompt()}`);
        if ($("assistantIncludeCurrentOperation").checked) sections.push(`Current operation:
${currentOperationForPrompt()}`);
        sections.push(`User request:
${$("assistantRequest").value}`);
        sections.push(`Expected LLM output format:
GraphQL operation:
<query or mutation>

variables JSON:
{}

Explanation:
<plain-language explanation and assumptions>

safety notes:
<validation concerns, missing permissions, secret handling, and assumptions>`);

        $("assistantPrompt").textContent = sections.join("\n\n");
      } catch (err) {
        $("assistantPrompt").textContent = `Could not generate prompt: ${err.message}`;
      }
    }

    function showScreen(name) {
      document.querySelectorAll(".screen").forEach((screen) => screen.classList.toggle("active", screen.id === `screen-${name}`));
      document.querySelectorAll(".nav-button").forEach((button) => button.classList.toggle("active", button.dataset.screen === name));
    }

    function wire(id, event, handler) {
      const node = $(id);
      if (node) node.addEventListener(event, handler);
    }

    document.querySelectorAll("[data-screen]").forEach((button) => button.addEventListener("click", () => showScreen(button.dataset.screen)));
    document.querySelectorAll(".wizard-tab, .next-step").forEach((button) => button.addEventListener("click", () => showWizardStep(button.dataset.step)));
    document.querySelectorAll(".assistant-example").forEach((button) => button.addEventListener("click", () => {
      $("assistantRequest").value = button.dataset.example;
      refreshAssistantContext();
      generatePrompt();
    }));

    wire("endpoint", "input", updateStatus);
    wire("token", "input", updateStatus);
    wire("refreshSchema", "click", loadSchema);
    wire("clearToken", "click", clearToken);
    wire("schemaSearch", "input", renderSchema);
    wire("queryEditor", "input", updateTemplateOutputs);
    wire("variablesEditor", "input", updateTemplateOutputs);
    wire("generateOperation", "click", () => regenerateOperation(true));
    wire("copyQuery", "click", () => copyText($("queryEditor").value));
    wire("copyVariables", "click", () => copyText($("variablesEditor").value));
    wire("copyAdvancedCurl", "click", () => copyText($("advancedCurl").textContent));
    wire("copyAdvancedJs", "click", () => copyText($("advancedJs").textContent));
    wire("saveOperationTemplate", "click", saveApiTemplate);
    wire("runOperation", "click", runCurrent);
    wire("saveExample", "click", saveExample);
    wire("loadExample", "click", loadExample);

    wire("runLogin", "click", runLogin);
    wire("copyLoginMutation", "click", () => copyText(loginMutation()));

    wire("previewSetupTenant", "click", () => previewOnly(() => tenantBuild("setupTenant"), "setupTenantGraphql", "setupTenantVariables", "setupTenantSummary", "Next: run createTenant."));
    wire("runSetupTenant", "click", () => runPreviewed(() => tenantBuild("setupTenant"), "setupTenantGraphql", "setupTenantVariables", "tenantResult", "setupTenantSummary", "createTenant", "Tenant", "Next: add an entity.", (result) => {
      const tenant = result.data.createTenant;
      state.wizard.tenantId = tenant.id;
      ["setupEntityTenantId", "setupResourceTenantId", "setupPolicyTenantId"].forEach((id) => $(id).value = tenant.id);
    }));
    wire("loadSetupProfiles", "click", () => loadProfilesFor("setupEntityProfile", "setupEntityProfileVersion", "setupEntityKind").catch((err) => $("setupEntitySummary").textContent = err.message));
    wire("setupEntityKind", "change", () => renderProfilesForKind("setupEntityProfile", "setupEntityProfileVersion", "setupEntityKind").catch((err) => $("setupEntitySummary").textContent = err.message));
    wire("setupEntityProfile", "change", () => loadProfileVersionsFor("setupEntityProfile", "setupEntityProfileVersion").catch((err) => $("setupEntitySummary").textContent = err.message));
    wire("previewSetupEntity", "click", () => previewOnly(() => entityBuild({ profileSelect: "setupEntityProfile", versionSelect: "setupEntityProfileVersion", kind: "setupEntityKind", name: "setupEntityName", tenantValue: nullable($("setupEntityTenantId").value), attributesJson: "setupEntityAttributes" }), "setupEntityGraphql", "setupEntityVariables", "setupEntitySummary", "Next: run createEntity."));
    wire("runSetupEntity", "click", () => runPreviewed(() => entityBuild({ profileSelect: "setupEntityProfile", versionSelect: "setupEntityProfileVersion", kind: "setupEntityKind", name: "setupEntityName", tenantValue: nullable($("setupEntityTenantId").value), attributesJson: "setupEntityAttributes" }), "setupEntityGraphql", "setupEntityVariables", "entityResult", "setupEntitySummary", "createEntity", "Entity", "Next: create a protected resource.", (result) => {
      const entity = result.data.createEntity;
      state.wizard.entityId = entity.id;
      ["setupPolicySubjectId", "setupAuthzSubjectId"].forEach((id) => $(id).value = entity.id);
      entitySuccessSummary(result, "setupEntityProfile", "setupEntityName", "setupEntitySummary", "Next: create a protected resource.");
    }));
    wire("previewSetupResource", "click", () => previewOnly(() => resourceBuild("setupResource"), "setupResourceGraphql", "setupResourceVariables", "setupResourceSummary", "Next: run createResource."));
    wire("runSetupResource", "click", () => runPreviewed(() => resourceBuild("setupResource"), "setupResourceGraphql", "setupResourceVariables", "resourceResult", "setupResourceSummary", "createResource", "Resource", "Next: grant access.", (result) => {
      const resource = result.data.createResource;
      state.wizard.resourceId = resource.id;
      ["setupPolicyResourceId", "setupAuthzResourceId"].forEach((id) => $(id).value = resource.id);
    }));
    wire("previewSetupPolicy", "click", () => { updatePolicyHumanSummary(); previewOnly(setupPolicyBuild, "setupPolicyGraphql", "setupPolicyVariables", "setupPolicySummary", "Next: run createPolicy."); });
    wire("runSetupPolicy", "click", () => runPreviewed(setupPolicyBuild, "setupPolicyGraphql", "setupPolicyVariables", "policyResult", "setupPolicySummary", "createPolicy", "Policy", "Next: test access."));
    wire("previewSetupAuthz", "click", () => previewOnly(() => authzBuild("authzCheck", "setupAuthz"), "setupAuthzGraphql", "setupAuthzVariables", "setupAuthzSummary", "Next: run authzCheck."));
    wire("runSetupAuthz", "click", () => runAuthz("authzCheck", "setupAuthz", "authzResult", "setupAuthzSummary"));

    wire("loadTenantsForProfile", "click", () => loadTenants(["profileTenantSelect"]).catch((err) => $("profileSummary").textContent = err.message));
    wire("previewProfile", "click", () => previewOnly(profileBuild, "profileGraphql", "profileVariables", "profileSummary", "Next: run createProfile."));
    wire("runProfile", "click", () => runPreviewed(profileBuild, "profileGraphql", "profileVariables", "profileResult", "profileSummary", "createProfile", "Profile", "Next: create a profile version.", (result) => {
      $("profileVersionProfileId").value = result.data.createProfile.id;
    }));
    wire("previewProfileVersion", "click", () => previewOnly(profileVersionBuild, "profileGraphql", "profileVariables", "profileSummary", "Next: run createProfileVersion."));
    wire("runProfileVersion", "click", () => runPreviewed(profileVersionBuild, "profileGraphql", "profileVariables", "profileResult", "profileSummary", "createProfileVersion", "Profile version", "Next: use Entity builder."));
    wire("refreshProfilesAfterProfile", "click", () => loadProfilesFor("entityProfile", "entityProfileVersion", "entityKind", "entityTenantSelect", "entityTenantId").catch((err) => $("profileSummary").textContent = err.message));

    wire("loadTenantsForEntity", "click", () => loadTenants(["entityTenantSelect"]).catch((err) => $("entitySummary").textContent = err.message));
    wire("loadProfiles", "click", () => loadProfilesFor("entityProfile", "entityProfileVersion", "entityKind", "entityTenantSelect", "entityTenantId").catch((err) => $("entitySummary").textContent = err.message));
    wire("entityKind", "change", () => renderProfilesForKind("entityProfile", "entityProfileVersion", "entityKind").catch((err) => $("entitySummary").textContent = err.message));
    wire("entityProfile", "change", () => loadProfileVersionsFor("entityProfile", "entityProfileVersion").catch((err) => $("entitySummary").textContent = err.message));
    wire("entityProfileVersion", "change", () => renderJsonSchemaFormFor("entityProfileVersion", "entityProfile"));
    wire("previewEntity", "click", () => previewOnly(entityBuild, "entityGraphql", "entityVariables", "entitySummary", "Next: run createEntity."));
    wire("runEntity", "click", () => runPreviewed(entityBuild, "entityGraphql", "entityVariables", "entityResult", "entitySummary", "createEntity", "Entity", "Next: create a resource or credential.", (result) => {
      entitySuccessSummary(result, "entityProfile", "entityName", "entitySummary", "Next: create a resource or credential.");
    }));

    wire("previewTenant", "click", () => previewOnly(tenantBuild, "tenantGraphql", "tenantVariables", "tenantSummary", "Next: run createTenant."));
    wire("runTenant", "click", () => runPreviewed(tenantBuild, "tenantGraphql", "tenantVariables", "tenantResult", "tenantSummary", "createTenant", "Tenant", "Next: add entities and resources."));

    wire("loadTenantsForResource", "click", () => loadTenants(["resourceTenantSelect"]).catch((err) => $("resourceSummary").textContent = err.message));
    wire("previewResource", "click", () => previewOnly(resourceBuild, "resourceGraphql", "resourceVariables", "resourceSummary", "Next: run createResource."));
    wire("runResource", "click", () => runPreviewed(resourceBuild, "resourceGraphql", "resourceVariables", "resourceResult", "resourceSummary", "createResource", "Resource", "Next: grant access with Policy builder."));

    wire("loadTenantsForPolicy", "click", () => loadTenants(["policyTenantSelect"]).catch((err) => $("policySummary").textContent = err.message));
    wire("loadPolicySubjects", "click", () => loadPolicySubjects().catch((err) => $("policySummary").textContent = err.message));
    wire("loadPolicyGrants", "click", () => loadPolicyGrants().catch((err) => $("policySummary").textContent = err.message));
    wire("loadPolicyObjects", "click", () => loadPolicyObjects().catch((err) => $("policySummary").textContent = err.message));
    ["policySubjectKind", "policySubjectSelect", "policySubjectId", "policyGrantKind", "policyGrantSelect", "policyGrantId", "policyScopeKind", "policyScopeRef", "policyObjectSelect", "policyConditions", "policyEffect", "setupPolicyResourceId", "setupPolicyEffect"].forEach((id) => wire(id, "input", updatePolicyHumanSummary));
    ["policySubjectKind", "policyGrantKind", "policyScopeKind", "policyObjectSelect", "policyEffect"].forEach((id) => wire(id, "change", updatePolicyHumanSummary));
    wire("previewPolicy", "click", previewPolicy);
    wire("runPolicy", "click", () => runPreviewed(preparePolicyPreview, "policyGraphql", "policyVariables", "policyResult", "policySummary", "createPolicy", "Policy", "Next: test this policy."));
    wire("copyPolicyGraphql", "click", copyPolicyGraphql);
    wire("copyPolicyCurl", "click", copyPolicyCurl);
    wire("copyPolicyJs", "click", copyPolicyJs);
    wire("savePolicyTemplate", "click", savePolicyTemplate);
    wire("testThisPolicy", "click", testThisPolicy);

    wire("previewAuthz", "click", () => previewOnly(() => authzBuild(state.lastAuthzName), "authzGraphql", "authzVariables", "authzNarrative", "Review generated GraphQL before running."));
    wire("runAuthzCheck", "click", () => runAuthz("authzCheck"));
    wire("runAuthzExplain", "click", () => runAuthz("authzExplain"));

    wire("listCredentials", "click", () => runCredential("list"));
    wire("createApiKey", "click", () => runCredential("apiKey"));
    wire("createPassword", "click", () => runCredential("password"));
    wire("revokeCredential", "click", () => runCredential("revoke"));

    wire("recipeSelect", "change", renderRecipeForm);
    wire("generateRecipe", "click", generateRecipe);
    wire("runRecipe", "click", runRecipe);
    wire("copyRecipeGraphql", "click", () => copyText($("recipeGraphql").textContent));
    wire("copyRecipeCurl", "click", () => copyText($("recipeCurl").textContent));
    wire("copyRecipeJs", "click", () => copyText($("recipeJs").textContent));
    wire("saveRecipe", "click", saveRecipe);
    wire("loadRecipe", "click", loadSavedRecipe);

    wire("refreshTemplates", "click", loadApiTemplates);
    wire("templateTenantId", "change", loadApiTemplates);
    wire("templateStatusFilter", "change", loadApiTemplates);
    wire("templateTagFilter", "change", loadApiTemplates);
    wire("runTemplate", "click", runSelectedTemplate);
    wire("saveTemplate", "click", saveApiTemplate);
    wire("updateTemplate", "click", updateApiTemplate);
    wire("copyTemplateGraphql", "click", () => copyText($("queryEditor").value));
    wire("copyTemplateVariables", "click", () => copyText($("variablesEditor").value));
    wire("copyTemplateCurl", "click", () => copyText($("templateCurl").textContent));
    wire("copyTemplateJs", "click", () => copyText($("templateJs").textContent));

    wire("loadEndpointTemplates", "click", () => loadEndpointTemplates().catch((err) => $("endpointBuilderSummary").textContent = err.message));
    wire("loadApiEndpoints", "click", loadApiEndpoints);
    wire("endpointSelect", "change", selectApiEndpoint);
    wire("previewApiEndpoint", "click", previewApiEndpoint);
    wire("createApiEndpoint", "click", createApiEndpoint);
    wire("enableApiEndpoint", "click", () => setSelectedApiEndpointStatus(enableEndpointMutation, "Enabled"));
    wire("disableApiEndpoint", "click", () => setSelectedApiEndpointStatus(disableEndpointMutation, "Disabled"));
    wire("testApiEndpoint", "click", testApiEndpoint);

    wire("refreshAssistantContext", "click", refreshAssistantContext);
    wire("generatePrompt", "click", generatePrompt);
    wire("copyPrompt", "click", () => copyText($("assistantPrompt").textContent));

    const storedToken = localStorage.getItem("atom.graphql.console.token");
    if (storedToken) $("token").value = storedToken;
    renderSavedExamples();
    renderRecipeList();
    renderTemplateGroups();
    updateTemplateOutputs();
    updateStatus();
    refreshAssistantContext();
    updateEntityBadges();
    updatePolicyHumanSummary();
    setPreview(
      loginMutation(),
      { input: { identifier: $("loginIdentifier").value, secret: "<redacted>", kind: $("loginKind").value || "password" } },
      "loginGraphql",
      "loginVariables"
    );
    loadSchema();
    loadApiTemplates();
"######;
