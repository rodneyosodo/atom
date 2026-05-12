import { CheckCircle2, KeyRound, Loader2, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import {
  exchangeOAuthCode,
  safeReturnTo,
  verifyEmailToken,
} from "../lib/auth";
import { CONSOLE_BASE } from "../lib/routes";

type PageState = "loading" | "success" | "error";

function queryValue(name: string): string | null {
  if (typeof window === "undefined") {
    return null;
  }
  return new URLSearchParams(window.location.search).get(name);
}

export function AuthCallbackPage() {
  const [state, setState] = useState<PageState>("loading");
  const [message, setMessage] = useState("Completing login");

  useEffect(() => {
    const error = queryValue("error");
    if (error) {
      setState("error");
      setMessage(error);
      return;
    }

    const code = queryValue("code");
    if (!code) {
      setState("error");
      setMessage("Missing OAuth exchange code.");
      return;
    }

    const returnTo = safeReturnTo(queryValue("return_to") ?? CONSOLE_BASE);
    void exchangeOAuthCode(code)
      .then(() => {
        setState("success");
        setMessage("Login complete.");
        window.location.assign(returnTo);
      })
      .catch((caught) => {
        setState("error");
        setMessage(caught instanceof Error ? caught.message : "OAuth login failed.");
      });
  }, []);

  return <AuthStatusPanel state={state} title="OAuth login" message={message} />;
}

export function EmailVerificationPage() {
  const [state, setState] = useState<PageState>("loading");
  const [message, setMessage] = useState("Verifying email");

  useEffect(() => {
    const token = queryValue("token");
    if (!token) {
      setState("error");
      setMessage("Missing verification token.");
      return;
    }

    void verifyEmailToken(token)
      .then(() => {
        setState("success");
        setMessage("Email verified.");
      })
      .catch((caught) => {
        setState("error");
        setMessage(caught instanceof Error ? caught.message : "Email verification failed.");
      });
  }, []);

  return <AuthStatusPanel state={state} title="Email verification" message={message} />;
}

function AuthStatusPanel({
  state,
  title,
  message,
}: {
  state: PageState;
  title: string;
  message: string;
}) {
  const Icon = state === "loading" ? Loader2 : state === "success" ? CheckCircle2 : XCircle;

  return (
    <div className="page-stack">
      <section className="page-heading">
        <div>
          <p className="eyebrow">Auth</p>
          <h1>{title}</h1>
        </div>
      </section>
      <section className="panel auth-status-panel">
        <div className={`auth-status-icon ${state}`}>
          <Icon size={28} aria-hidden="true" className={state === "loading" ? "spin" : undefined} />
        </div>
        <div>
          <h2>{message}</h2>
          <div className="button-row spaced">
            <a className="button primary" href={CONSOLE_BASE}>
              <KeyRound size={16} aria-hidden="true" />
              <span>Console</span>
            </a>
          </div>
        </div>
      </section>
    </div>
  );
}
