"use client";

import { useQuery } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { graphqlClient } from "@/lib/graphql/client";

const ROLE_CAPABILITIES_QUERY = `
  query RoleCapabilitiesPanel($roleId: ID!) {
    roleCapabilities(roleId: $roleId) { id name resourceKind description }
    role(id: $roleId) {
      permissionBlocks {
        id
        appliesTo
        objectKind
        objectType
        tenantId
        objectId
        groupId
        capabilities { id name resourceKind description }
      }
    }
  }
`;

type GqlCapability = {
  id: string;
  name: string;
  resourceKind: string | null;
  description: string | null;
};

export function RoleCapabilitiesPanel({ roleId }: { roleId: string }) {
  const { data, isFetching, error } = useQuery({
    queryKey: ["role-capabilities-panel", roleId],
    queryFn: ({ signal }) =>
      graphqlClient<{
        roleCapabilities: GqlCapability[];
        role: {
          permissionBlocks: {
            id: string;
            appliesTo: string;
            objectKind: string | null;
            objectType: string | null;
            tenantId: string | null;
            objectId: string | null;
            groupId: string | null;
            capabilities: GqlCapability[];
          }[];
        };
      }>({
        query: ROLE_CAPABILITIES_QUERY,
        variables: { roleId },
        signal,
      }),
    staleTime: 30_000,
  });

  const capabilities = data?.roleCapabilities ?? [];
  const role = data?.role;
  const permissionBlocks = role?.permissionBlocks ?? [];

  return (
    <div className="grid gap-3 rounded-lg border bg-background p-3">
      <div className="text-sm font-medium">Permissions</div>
      {isFetching && capabilities.length === 0 && !role ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : error ? (
        <p className="text-sm text-destructive">{error.message}</p>
      ) : permissionBlocks.length > 0 ? (
        <div className="grid gap-2">
          {permissionBlocks.map((block) => (
            <div className="grid gap-1 rounded-md border p-2" key={block.id}>
              <div className="text-xs font-medium text-muted-foreground">
                {block.appliesTo}
                {block.objectType ? ` · ${block.objectType}` : ""}
                {block.objectKind && !block.objectType
                  ? ` · ${block.objectKind}`
                  : ""}
              </div>
              <div className="flex flex-wrap gap-1.5">
                {block.capabilities.map((cap) => (
                  <Badge
                    key={cap.id}
                    title={cap.description ?? undefined}
                    variant="secondary"
                  >
                    {cap.name}
                    {cap.resourceKind ? (
                      <span className="ml-1 text-muted-foreground">
                        :{cap.resourceKind}
                      </span>
                    ) : null}
                  </Badge>
                ))}
              </div>
            </div>
          ))}
        </div>
      ) : capabilities.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          No permissions assigned to this role.
        </p>
      ) : (
        <div className="flex flex-wrap gap-1.5">
          {capabilities.map((cap) => (
            <Badge
              key={cap.id}
              variant="secondary"
              title={cap.description ?? undefined}
            >
              {cap.name}
              {cap.resourceKind ? (
                <span className="ml-1 text-muted-foreground">
                  :{cap.resourceKind}
                </span>
              ) : null}
            </Badge>
          ))}
        </div>
      )}
    </div>
  );
}
