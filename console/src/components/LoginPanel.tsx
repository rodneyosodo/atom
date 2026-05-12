import {
  KeyRound,
  LogIn,
  LogOut,
  Mail,
  RefreshCw,
  UserPlus,
  CircleUserRound,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { SyntheticEvent } from "react";
import {
  clearToken,
  getPublicAuthConfig,
  getToken,
  login,
  logout,
  resendVerification,
  signup,
  startOAuth,
} from "../lib/auth";
import { CONSOLE_BASE } from "../lib/routes";
import type { LoginResult, PublicAuthConfig } from "../lib/schema";

type AuthMode = "login" | "signup";

const DEFAULT_AUTH_CONFIG: PublicAuthConfig = {
  signup_enabled: false,
  oauth_providers: [],
  email_verification_required: true,
  dev_allow_unverified_email_login: false,
};

export function LoginPanel() {
  const [mode, setMode] = useState<AuthMode>("login");
  const [identifier, setIdentifier] = useState("");
  const [secret, setSecret] = useState("");
  const [signupName, setSignupName] = useState("");
  const [signupEmail, setSignupEmail] = useState("");
  const [signupPassword, setSignupPassword] = useState("");
  const [pendingEmail, setPendingEmail] = useState<string | null>(null);
  const [token, setTokenValue] = useState<string | null>(null);
  const [lastLogin, setLastLogin] = useState<LoginResult | null>(null);
  const [authConfig, setAuthConfig] = useState<PublicAuthConfig>(DEFAULT_AUTH_CONFIG);
  const [busy, setBusy] = useState(false);
  const [resendBusy, setResendBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    setTokenValue(getToken());
    void getPublicAuthConfig()
      .then(setAuthConfig)
      .catch(() => setAuthConfig(DEFAULT_AUTH_CONFIG));
  }, []);

  const tokenPreview = useMemo(() => {
    if (!token) {
      return "";
    }
    return `${token.slice(0, 16)}...${token.slice(-8)}`;
  }, [token]);

  const googleEnabled = authConfig.oauth_providers.includes("google");
  const verificationBlocked =
    error?.toLowerCase().includes("email verification required") && identifier.includes("@");

  async function onLogin(event: SyntheticEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setMessage(null);

    try {
      const result = await login(identifier.trim(), secret);
      setLastLogin(result);
      setTokenValue(result.token);
      setSecret("");
      setPendingEmail(null);
    } catch (loginError) {
      clearToken();
      setTokenValue(null);
      const message = loginError instanceof Error ? loginError.message : "Login failed";
      setError(message);
      if (message.toLowerCase().includes("email verification required") && identifier.includes("@")) {
        setPendingEmail(identifier.trim());
      }
    } finally {
      setBusy(false);
    }
  }

  async function onSignup(event: SyntheticEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setMessage(null);

    try {
      const result = await signup({
        name: signupName.trim(),
        email: signupEmail.trim(),
        password: signupPassword,
      });
      setPendingEmail(result.email);
      setIdentifier(result.email);
      setSignupPassword("");
      setMode("login");
      setMessage(
        authConfig.dev_allow_unverified_email_login
          ? "Account created. Verification email sent."
          : "Verification email sent.",
      );
    } catch (signupError) {
      setError(signupError instanceof Error ? signupError.message : "Signup failed");
    } finally {
      setBusy(false);
    }
  }

  async function onResend() {
    const email = pendingEmail || identifier.trim() || signupEmail.trim();
    if (!email) {
      return;
    }
    setResendBusy(true);
    setError(null);
    setMessage(null);
    try {
      await resendVerification(email);
      setPendingEmail(email);
      setMessage("Verification email sent.");
    } catch (resendError) {
      setError(resendError instanceof Error ? resendError.message : "Verification resend failed");
    } finally {
      setResendBusy(false);
    }
  }

  async function onLogout() {
    setBusy(true);
    setError(null);
    await logout();
    setTokenValue(null);
    setLastLogin(null);
    setBusy(false);
  }

  if (token) {
    return (
      <section className="panel auth-panel">
        <div className="panel-header">
          <div>
            <p className="eyebrow">Auth status</p>
            <h2>Signed in</h2>
          </div>
          <KeyRound size={18} aria-hidden="true" />
        </div>
        <dl className="token-details">
          <div>
            <dt>Token</dt>
            <dd>{tokenPreview}</dd>
          </div>
          {lastLogin ? (
            <div>
              <dt>Entity</dt>
              <dd>{lastLogin.entityId}</dd>
            </div>
          ) : null}
        </dl>
        <button className="button secondary" type="button" onClick={onLogout} disabled={busy}>
          <LogOut size={16} aria-hidden="true" />
          <span>Logout</span>
        </button>
      </section>
    );
  }

  return (
    <section className="panel auth-panel">
      <div className="panel-header">
        <div>
          <p className="eyebrow">Auth status</p>
          <h2>{mode === "login" ? "Login" : "Sign up"}</h2>
        </div>
        <KeyRound size={18} aria-hidden="true" />
      </div>

      <div className="segmented auth-tabs" role="tablist" aria-label="Authentication mode">
        <button
          className={mode === "login" ? "active" : ""}
          type="button"
          onClick={() => {
            setMode("login");
            setError(null);
          }}
        >
          Login
        </button>
        <button
          className={mode === "signup" ? "active" : ""}
          type="button"
          onClick={() => {
            setMode("signup");
            setError(null);
          }}
          disabled={!authConfig.signup_enabled}
        >
          Sign up
        </button>
      </div>

      {mode === "login" ? (
        <form className="stack" onSubmit={onLogin}>
          <label>
            <span>Email or identifier</span>
            <input
              value={identifier}
              onChange={(event) => setIdentifier(event.target.value)}
              autoComplete="username"
              placeholder="alice@example.com"
              required
            />
          </label>
          <label>
            <span>Password</span>
            <input
              value={secret}
              onChange={(event) => setSecret(event.target.value)}
              type="password"
              autoComplete="current-password"
              required
            />
          </label>
          {error ? <p className="form-error">{error}</p> : null}
          {message ? <p className="form-success">{message}</p> : null}
          <button className="button primary" type="submit" disabled={busy}>
            <LogIn size={16} aria-hidden="true" />
            <span>{busy ? "Logging in" : "Login"}</span>
          </button>
        </form>
      ) : (
        <form className="stack" onSubmit={onSignup}>
          <label>
            <span>Name</span>
            <input
              value={signupName}
              onChange={(event) => setSignupName(event.target.value)}
              autoComplete="name"
              required
            />
          </label>
          <label>
            <span>Email</span>
            <input
              value={signupEmail}
              onChange={(event) => setSignupEmail(event.target.value)}
              autoComplete="email"
              type="email"
              required
            />
          </label>
          <label>
            <span>Password</span>
            <input
              value={signupPassword}
              onChange={(event) => setSignupPassword(event.target.value)}
              type="password"
              autoComplete="new-password"
              required
            />
          </label>
          {error ? <p className="form-error">{error}</p> : null}
          {message ? <p className="form-success">{message}</p> : null}
          <button className="button primary" type="submit" disabled={busy}>
            <UserPlus size={16} aria-hidden="true" />
            <span>{busy ? "Creating" : "Create account"}</span>
          </button>
        </form>
      )}

      {(pendingEmail || verificationBlocked) && (
        <button className="button secondary spaced" type="button" onClick={onResend} disabled={resendBusy}>
          <RefreshCw size={16} aria-hidden="true" className={resendBusy ? "spin" : undefined} />
          <span>{resendBusy ? "Sending" : "Resend verification"}</span>
        </button>
      )}

      {googleEnabled ? (
        <>
          <div className="auth-divider"><span>or</span></div>
          <button
            className="button secondary"
            type="button"
            onClick={() => startOAuth("google", CONSOLE_BASE)}
            disabled={busy}
          >
            <CircleUserRound size={16} aria-hidden="true" />
            <span>Continue with Google</span>
          </button>
        </>
      ) : null}

      {authConfig.signup_enabled ? null : (
        <p className="auth-footnote">
          <Mail size={14} aria-hidden="true" />
          <span>Signup disabled</span>
        </p>
      )}
    </section>
  );
}
