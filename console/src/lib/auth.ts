import { LOGIN_MUTATION, LOGOUT_MUTATION } from "./snippets";
import { CONSOLE_BASE } from "./routes";
import type { GraphqlEnvelope, LoginResult, PublicAuthConfig, SignupResult } from "./schema";

export const TOKEN_STORAGE_KEY = "atom.graphql.console.token";

type LoginPayload = {
  login: LoginResult;
};

type RestLoginResult = {
  token: string;
  entity_id: string;
  session_id: string;
  expires_at: string;
  email_verified?: boolean | null;
  verification_required?: boolean;
};

type ErrorEnvelope = {
  error?: string;
};

export function getToken(): string | null {
  if (typeof window === "undefined") {
    return null;
  }
  return window.sessionStorage.getItem(TOKEN_STORAGE_KEY);
}

export function setToken(token: string): void {
  if (typeof window === "undefined") {
    return;
  }
  window.sessionStorage.setItem(TOKEN_STORAGE_KEY, token);
  window.dispatchEvent(new CustomEvent("atom-console-auth"));
}

export function clearToken(): void {
  if (typeof window === "undefined") {
    return;
  }
  window.sessionStorage.removeItem(TOKEN_STORAGE_KEY);
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

async function readRestError(response: Response, fallback: string): Promise<string> {
  try {
    const body = (await response.json()) as ErrorEnvelope;
    return body.error || fallback;
  } catch {
    return fallback;
  }
}

function normalizeRestLogin(result: RestLoginResult): LoginResult {
  return {
    token: result.token,
    entityId: result.entity_id,
    sessionId: result.session_id,
    expiresAt: result.expires_at,
    emailVerified: result.email_verified,
    verificationRequired: result.verification_required,
  };
}

export async function getPublicAuthConfig(): Promise<PublicAuthConfig> {
  const response = await fetch("/auth/public-config");
  if (!response.ok) {
    throw new Error(await readRestError(response, `Auth config failed with HTTP ${response.status}`));
  }
  return (await response.json()) as PublicAuthConfig;
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

export async function signup(input: {
  name: string;
  email: string;
  password: string;
}): Promise<SignupResult> {
  const response = await fetch("/auth/signup", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      name: input.name,
      email: input.email,
      password: input.password,
      attributes: {},
    }),
  });
  if (!response.ok) {
    throw new Error(await readRestError(response, `Signup failed with HTTP ${response.status}`));
  }
  return (await response.json()) as SignupResult;
}

export async function resendVerification(email: string): Promise<void> {
  const response = await fetch("/auth/email/resend", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ email }),
  });
  if (!response.ok) {
    throw new Error(await readRestError(response, `Verification resend failed with HTTP ${response.status}`));
  }
}

export function startOAuth(provider: string, returnTo = CONSOLE_BASE): void {
  const params = new URLSearchParams({ return_to: safeReturnTo(returnTo) });
  window.location.assign(`/auth/oauth/${encodeURIComponent(provider)}/start?${params.toString()}`);
}

export async function exchangeOAuthCode(code: string): Promise<LoginResult> {
  const response = await fetch("/auth/oauth/exchange", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ code }),
  });
  if (!response.ok) {
    throw new Error(await readRestError(response, `OAuth exchange failed with HTTP ${response.status}`));
  }
  const result = normalizeRestLogin((await response.json()) as RestLoginResult);
  setToken(result.token);
  return result;
}

export async function verifyEmailToken(token: string): Promise<void> {
  const params = new URLSearchParams({ token });
  const response = await fetch(`/auth/email/verify?${params.toString()}`);
  if (!response.ok) {
    throw new Error(await readRestError(response, `Email verification failed with HTTP ${response.status}`));
  }
}

export function safeReturnTo(value: string | null | undefined): string {
  const candidate = (value || "").trim();
  if (candidate.startsWith("/") && !candidate.startsWith("//") && !candidate.includes("://")) {
    return candidate;
  }
  return CONSOLE_BASE;
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
