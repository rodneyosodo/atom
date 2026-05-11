import { LOGIN_MUTATION, LOGOUT_MUTATION } from "./snippets";
import type { GraphqlEnvelope, LoginResult } from "./schema";

export const TOKEN_STORAGE_KEY = "atom.graphql.console.token";

type LoginPayload = {
  login: LoginResult;
};

export function getToken(): string | null {
  if (typeof window === "undefined") {
    return null;
  }
  return window.localStorage.getItem(TOKEN_STORAGE_KEY);
}

export function setToken(token: string): void {
  if (typeof window === "undefined") {
    return;
  }
  window.localStorage.setItem(TOKEN_STORAGE_KEY, token);
  window.dispatchEvent(new CustomEvent("atom-console-auth"));
}

export function clearToken(): void {
  if (typeof window === "undefined") {
    return;
  }
  window.localStorage.removeItem(TOKEN_STORAGE_KEY);
  window.dispatchEvent(new CustomEvent("atom-console-auth"));
}

export function shouldAttachAuthorization(input: string | URL): boolean {
  if (typeof window === "undefined") {
    return false;
  }

  const url = new URL(input.toString(), window.location.origin);
  if (url.origin !== window.location.origin) {
    return false;
  }

  return url.pathname === "/graphql" || url.pathname.startsWith("/api/custom/");
}

export function authorizationHeaderFor(input: string | URL): Record<string, string> {
  const token = getToken();
  if (!token || !shouldAttachAuthorization(input)) {
    return {};
  }
  return { Authorization: `Bearer ${token}` };
}

export async function login(identifier: string, secret: string): Promise<LoginResult> {
  const response = await fetch("/graphql", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      query: LOGIN_MUTATION,
      variables: {
        input: {
          identifier,
          secret,
          kind: "password",
        },
      },
    }),
  });

  const envelope = (await response.json()) as GraphqlEnvelope<LoginPayload>;
  if (!response.ok) {
    throw new Error(`Login failed with HTTP ${response.status}`);
  }
  if (envelope.errors?.length) {
    throw new Error(envelope.errors.map((error) => error.message).join("\n"));
  }
  if (!envelope.data?.login?.token) {
    throw new Error("Login response did not include a token");
  }

  setToken(envelope.data.login.token);
  return envelope.data.login;
}

export async function logout(): Promise<boolean> {
  const token = getToken();
  if (!token) {
    clearToken();
    return true;
  }

  let remoteLogoutSucceeded = true;
  try {
    const response = await fetch("/graphql", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        ...authorizationHeaderFor("/graphql"),
      },
      body: JSON.stringify({ query: LOGOUT_MUTATION }),
    });
    const envelope = (await response.json()) as GraphqlEnvelope<{ logout: boolean }>;
    remoteLogoutSucceeded = response.ok && !envelope.errors?.length;
  } catch {
    remoteLogoutSucceeded = false;
  } finally {
    clearToken();
  }

  return remoteLogoutSucceeded;
}
