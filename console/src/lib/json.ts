import type { JsonObject, JsonValue } from "./schema";

export function jsonString(value: JsonValue | undefined, fallback = "{}"): string {
  if (value === undefined) {
    return fallback;
  }
  return JSON.stringify(value, null, 2);
}

export function parseJsonObject(value: string, label: string): JsonObject {
  const parsed = parseJson(value, label);
  if (!isJsonObject(parsed)) {
    throw new Error(`${label} must be a JSON object`);
  }
  return parsed;
}

export function parseJson(value: string, label: string): JsonValue {
  const trimmed = value.trim();
  if (!trimmed) {
    return {};
  }
  try {
    return JSON.parse(trimmed) as JsonValue;
  } catch (error) {
    const message = error instanceof Error ? error.message : "invalid JSON";
    throw new Error(`${label}: ${message}`);
  }
}

export function isJsonObject(value: JsonValue): value is JsonObject {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function compactNullable(value: string): string | null {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

export function tagsFromText(value: string): string[] {
  return value
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean);
}

export function textFromTags(value: string[]): string {
  return value.join(", ");
}

export function getPath(value: JsonValue, path: string[]): JsonValue | undefined {
  return path.reduce<JsonValue | undefined>((current, part) => {
    if (current === undefined || !isJsonObject(current)) {
      return undefined;
    }
    return current[part];
  }, value);
}
