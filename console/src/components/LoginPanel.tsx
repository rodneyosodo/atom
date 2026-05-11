import { KeyRound, LogIn, LogOut } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { SyntheticEvent } from "react";
import { clearToken, getToken, login, logout } from "../lib/auth";
import type { LoginResult } from "../lib/schema";

export function LoginPanel() {
  const [identifier, setIdentifier] = useState("atom-admin");
  const [secret, setSecret] = useState("");
  const [token, setTokenValue] = useState<string | null>(null);
  const [lastLogin, setLastLogin] = useState<LoginResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setTokenValue(getToken());
  }, []);

  const tokenPreview = useMemo(() => {
    if (!token) {
      return "";
    }
    return `${token.slice(0, 16)}...${token.slice(-8)}`;
  }, [token]);

  async function onSubmit(event: SyntheticEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);

    try {
      const result = await login(identifier.trim(), secret);
      setLastLogin(result);
      setTokenValue(result.token);
      setSecret("");
    } catch (loginError) {
      clearToken();
      setTokenValue(null);
      setError(loginError instanceof Error ? loginError.message : "Login failed");
    } finally {
      setBusy(false);
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
          <h2>Login</h2>
        </div>
        <KeyRound size={18} aria-hidden="true" />
      </div>

      <form className="stack" onSubmit={onSubmit}>
        <label>
          <span>Identifier</span>
          <input
            value={identifier}
            onChange={(event) => setIdentifier(event.target.value)}
            autoComplete="username"
            required
          />
        </label>
        <label>
          <span>Secret</span>
          <input
            value={secret}
            onChange={(event) => setSecret(event.target.value)}
            type="password"
            autoComplete="current-password"
            placeholder="ADMIN_SECRET"
            required
          />
        </label>
        {error ? <p className="form-error">{error}</p> : null}
        <button className="button primary" type="submit" disabled={busy}>
          <LogIn size={16} aria-hidden="true" />
          <span>{busy ? "Logging in" : "Login"}</span>
        </button>
      </form>
    </section>
  );
}
