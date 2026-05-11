import { Check, Clipboard, Loader2, RefreshCw, XCircle } from "lucide-react";
import { type ReactNode, type SyntheticEvent, useState } from "react";
import { jsonString, parseJson, parseJsonObject } from "../lib/json";
import type { JsonObject, JsonValue } from "../lib/schema";

export function PageHeader({
  eyebrow,
  title,
  children,
}: {
  eyebrow: string;
  title: string;
  children?: ReactNode;
}) {
  return (
    <section className="page-heading">
      <div>
        <p className="eyebrow">{eyebrow}</p>
        <h1>{title}</h1>
      </div>
      {children ? <div className="header-actions">{children}</div> : null}
    </section>
  );
}

export function Panel({
  title,
  eyebrow,
  children,
  actions,
  className = "",
}: {
  title: string;
  eyebrow?: string;
  children: ReactNode;
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <section className={`panel ${className}`}>
      <div className="panel-header">
        <div>
          {eyebrow ? <p className="eyebrow">{eyebrow}</p> : null}
          <h2>{title}</h2>
        </div>
        {actions ? <div className="button-row">{actions}</div> : null}
      </div>
      {children}
    </section>
  );
}

export function StatusBadge({ value }: { value: string | boolean | null | undefined }) {
  const label = value === true ? "allowed" : value === false ? "denied" : value ?? "none";
  const text = String(label);
  const tone =
    text === "active" || text === "success" || text === "allowed" || text === "online"
      ? "ok"
      : text === "disabled" || text === "denied" || text === "error" || text === "deleted"
        ? "error"
        : "warn";
  return <span className={`badge ${tone}`}>{text}</span>;
}

export function ErrorNotice({ message }: { message: string | null }) {
  if (!message) {
    return null;
  }
  return (
    <div className="notice danger">
      <XCircle size={16} aria-hidden="true" />
      <span>{message}</span>
    </div>
  );
}

export function EmptyState({ children }: { children: ReactNode }) {
  return <p className="empty-state">{children}</p>;
}

export function Loading({ label = "Loading" }: { label?: string }) {
  return (
    <div className="status-row muted">
      <Loader2 className="spin" size={16} aria-hidden="true" />
      <span>{label}</span>
    </div>
  );
}

export function CopyButton({ value, label = "Copy" }: { value: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  async function copy() {
    await navigator.clipboard.writeText(value);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  }
  return (
    <button className="button secondary" type="button" onClick={copy}>
      {copied ? <Check size={16} aria-hidden="true" /> : <Clipboard size={16} aria-hidden="true" />}
      <span>{copied ? "Copied" : label}</span>
    </button>
  );
}

export function RefreshButton({
  onClick,
  busy,
  label = "Refresh",
}: {
  onClick: () => void | Promise<void>;
  busy?: boolean;
  label?: string;
}) {
  return (
    <button className="button secondary" type="button" onClick={() => void onClick()} disabled={busy}>
      <RefreshCw className={busy ? "spin" : ""} size={16} aria-hidden="true" />
      <span>{label}</span>
    </button>
  );
}

export function JsonTextarea({
  label,
  value,
  onChange,
  rows = 8,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  rows?: number;
}) {
  return (
    <label>
      <span>{label}</span>
      <textarea value={value} rows={rows} spellCheck={false} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}

export function ResultPanel({
  title,
  value,
  error,
}: {
  title: string;
  value: unknown;
  error?: string | null;
}) {
  return (
    <details className="result-panel" open={Boolean(value) || Boolean(error)}>
      <summary>{title}</summary>
      {error ? <pre className="error-output">{error}</pre> : <pre>{value === undefined ? "" : jsonString(value as JsonValue)}</pre>}
    </details>
  );
}

export function PreviewPanel({
  query,
  variables,
}: {
  query: string;
  variables: JsonObject;
}) {
  return (
    <div className="split">
      <details>
        <summary>GraphQL</summary>
        <pre>{query}</pre>
      </details>
      <details>
        <summary>Variables</summary>
        <pre>{jsonString(variables)}</pre>
      </details>
    </div>
  );
}

export function useAsyncAction() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function run<T>(action: () => Promise<T>): Promise<T | null> {
    setBusy(true);
    setError(null);
    try {
      return await action();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Unexpected error");
      return null;
    } finally {
      setBusy(false);
    }
  }

  return { busy, error, setError, run };
}

export function parseJsonField(value: string, label: string): JsonValue {
  return parseJson(value, label);
}

export function parseJsonObjectField(value: string, label: string): JsonObject {
  return parseJsonObject(value, label);
}

export function preventDefault(handler: () => void | Promise<void>) {
  return (event: SyntheticEvent<HTMLFormElement>) => {
    event.preventDefault();
    void handler();
  };
}
