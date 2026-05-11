import type { JsonObject, JsonValue } from "../lib/schema";

type SchemaObject = {
  type?: string;
  properties?: Record<string, SchemaObject>;
  required?: string[];
  enum?: JsonValue[];
  title?: string;
  description?: string;
};

function isObject(value: JsonValue): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asSchemaObject(value: JsonValue): SchemaObject {
  return isObject(value) ? (value as SchemaObject) : {};
}

function getValue(value: JsonObject, path: string[]): JsonValue {
  return path.reduce<JsonValue>((current, part) => {
    if (!isObject(current)) {
      return "";
    }
    return current[part] ?? "";
  }, value);
}

function setValue(value: JsonObject, path: string[], next: JsonValue): JsonObject {
  const clone: JsonObject = { ...value };
  let cursor = clone;
  for (const part of path.slice(0, -1)) {
    const existing = cursor[part];
    const child = isObject(existing) ? { ...existing } : {};
    cursor[part] = child;
    cursor = child;
  }
  cursor[path[path.length - 1] ?? ""] = next;
  return clone;
}

function valueForInput(value: JsonValue): string {
  if (value === null || value === undefined) {
    return "";
  }
  if (typeof value === "object") {
    return JSON.stringify(value);
  }
  return String(value);
}

export function JsonSchemaForm({
  schema,
  value,
  onChange,
}: {
  schema: JsonValue;
  value: JsonObject;
  onChange: (value: JsonObject) => void;
}) {
  if (!isObject(schema) || !isObject(schema.properties ?? null)) {
    return null;
  }
  const properties = schema.properties as Record<string, JsonValue>;

  return (
    <div className="schema-form">
      {Object.entries(properties).map(([key, property]) => (
        <SchemaField
          key={key}
          name={key}
          path={[key]}
          schema={asSchemaObject(property)}
          root={value}
          required={Array.isArray(schema.required) && schema.required.includes(key)}
          onChange={onChange}
        />
      ))}
    </div>
  );
}

function SchemaField({
  name,
  path,
  schema,
  root,
  required,
  onChange,
}: {
  name: string;
  path: string[];
  schema: SchemaObject;
  root: JsonObject;
  required: boolean;
  onChange: (value: JsonObject) => void;
}) {
  const label = schema.title ?? name;
  const current = getValue(root, path);

  if (schema.type === "object" && schema.properties) {
    return (
      <fieldset>
        <legend>{label}</legend>
        {Object.entries(schema.properties).map(([childName, child]) => (
          <SchemaField
            key={childName}
            name={childName}
            path={[...path, childName]}
            schema={child}
            root={root}
            required={Array.isArray(schema.required) && schema.required.includes(childName)}
            onChange={onChange}
          />
        ))}
      </fieldset>
    );
  }

  if (schema.enum?.length) {
    return (
      <label>
        <span>
          {label}
          {required ? " *" : ""}
        </span>
        <select
          value={valueForInput(current)}
          onChange={(event) => onChange(setValue(root, path, event.target.value))}
          required={required}
        >
          <option value="">Choose value</option>
          {schema.enum.map((item) => (
            <option key={String(item)} value={String(item)}>
              {String(item)}
            </option>
          ))}
        </select>
      </label>
    );
  }

  if (schema.type === "boolean") {
    return (
      <label className="inline-check">
        <input
          type="checkbox"
          checked={Boolean(current)}
          onChange={(event) => onChange(setValue(root, path, event.target.checked))}
        />
        <span>
          {label}
          {required ? " *" : ""}
        </span>
      </label>
    );
  }

  if (schema.type === "number" || schema.type === "integer") {
    return (
      <label>
        <span>
          {label}
          {required ? " *" : ""}
        </span>
        <input
          type="number"
          step={schema.type === "integer" ? "1" : "any"}
          value={valueForInput(current)}
          onChange={(event) => {
            const raw = event.target.value;
            onChange(setValue(root, path, raw === "" ? "" : Number(raw)));
          }}
          required={required}
        />
      </label>
    );
  }

  return (
    <label>
      <span>
        {label}
        {required ? " *" : ""}
      </span>
      <input
        value={valueForInput(current)}
        onChange={(event) => onChange(setValue(root, path, event.target.value))}
        required={required}
      />
      {schema.description ? <small>{schema.description}</small> : null}
    </label>
  );
}
