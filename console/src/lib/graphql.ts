import { authorizationHeaderFor } from "./auth";
import type { GraphqlEnvelope, JsonObject, JsonValue } from "./schema";

export type GqlOptions = {
  auth?: boolean;
  endpoint?: "/graphql" | "/api/custom";
};

export async function gql<TData = JsonObject>(
  query: string,
  variables?: JsonObject,
  options: GqlOptions = {},
): Promise<TData> {
  const endpoint = options.endpoint ?? "/graphql";
  const authHeaders = options.auth === false ? {} : authorizationHeaderFor(endpoint);
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...authHeaders,
    },
    body: JSON.stringify({
      query,
      variables: variables ?? {},
    }),
  });

  const envelope = (await response.json()) as GraphqlEnvelope<TData>;
  if (!response.ok) {
    throw new Error(`GraphQL request failed with HTTP ${response.status}`);
  }
  if (envelope.errors?.length) {
    throw new Error(envelope.errors.map((error) => error.message).join("\n"));
  }
  if (!envelope.data) {
    throw new Error("GraphQL response did not include data");
  }

  return envelope.data;
}

export async function rawGql<TData = JsonObject>(
  query: string,
  variables?: JsonObject,
  options: GqlOptions = {},
): Promise<GraphqlEnvelope<TData>> {
  const endpoint = options.endpoint ?? "/graphql";
  const authHeaders = options.auth === false ? {} : authorizationHeaderFor(endpoint);
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...authHeaders,
    },
    body: JSON.stringify({
      query,
      variables: variables ?? {},
    }),
  });

  const envelope = (await response.json()) as GraphqlEnvelope<TData>;
  if (!response.ok) {
    return {
      errors: [{ message: `GraphQL request failed with HTTP ${response.status}` }, ...(envelope.errors ?? [])],
      data: envelope.data,
    };
  }
  return envelope;
}

export async function customEndpoint(
  method: string,
  path: string,
  body?: JsonValue,
): Promise<{ status: number; ok: boolean; text: string }> {
  const response = await fetch(path, {
    method,
    headers: {
      "Content-Type": "application/json",
      ...authorizationHeaderFor(path),
    },
    body: ["GET", "DELETE"].includes(method) ? undefined : JSON.stringify(body ?? {}),
  });

  return {
    status: response.status,
    ok: response.ok,
    text: await response.text(),
  };
}
