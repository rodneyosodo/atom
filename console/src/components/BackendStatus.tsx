import { Activity, AlertTriangle, CheckCircle2 } from "lucide-react";
import { useEffect, useState } from "react";
import { gql } from "../lib/graphql";
import { HEALTH_QUERY } from "../lib/snippets";

type Status = "checking" | "online" | "offline";

export function BackendStatus() {
  const [status, setStatus] = useState<Status>("checking");
  const [message, setMessage] = useState("Checking GraphQL health");

  useEffect(() => {
    let mounted = true;

    gql<{ health: string }>(HEALTH_QUERY, undefined, { auth: false })
      .then((data) => {
        if (!mounted) {
          return;
        }
        setStatus(data.health === "ok" ? "online" : "offline");
        setMessage(data.health === "ok" ? "GraphQL online" : "Unexpected health response");
      })
      .catch((error: unknown) => {
        if (!mounted) {
          return;
        }
        setStatus("offline");
        setMessage(error instanceof Error ? error.message : "GraphQL unavailable");
      });

    return () => {
      mounted = false;
    };
  }, []);

  const Icon = status === "online" ? CheckCircle2 : status === "offline" ? AlertTriangle : Activity;

  return (
    <div className={`status-pill status-${status}`}>
      <Icon size={16} aria-hidden="true" />
      <span>{message}</span>
    </div>
  );
}
