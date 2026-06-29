import type { ColumnDef } from "@tanstack/react-table";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DataTable } from "@/components/ui/data-table";

const mocks = vi.hoisted(() => ({
  replace: vi.fn(),
}));

vi.mock("next/navigation", () => ({
  usePathname: () => "/roles",
  useRouter: () => ({ replace: mocks.replace }),
  useSearchParams: () => new URLSearchParams(),
}));

type Row = {
  name: string;
};

const columns: ColumnDef<Row>[] = [
  {
    accessorKey: "name",
    header: "Name",
  },
];

describe("DataTable", () => {
  beforeEach(() => {
    mocks.replace.mockReset();
  });

  it("renders without looping when no filters are provided", () => {
    render(
      <DataTable
        columns={columns}
        data={[{ name: "atom-admin" }]}
        limit={10}
        page={1}
        paramKey="roles"
        total={1}
      />,
    );

    expect(screen.getByText("atom-admin")).toBeInTheDocument();
    expect(mocks.replace).not.toHaveBeenCalled();
  });

  it("renders visible labels for dropdown filters", () => {
    render(
      <DataTable
        columns={columns}
        data={[{ name: "sensor-gateway-01" }]}
        filters={[
          {
            key: "kind",
            label: "Kind",
            type: "select",
            options: [{ label: "Device", value: "device" }],
          },
          {
            key: "tenantId",
            label: "Tenant",
            type: "select",
            options: [{ label: "Factory A", value: "tenant-1" }],
          },
        ]}
        limit={10}
        page={1}
        paramKey="entities"
        statusFilter={{ enabled: true, options: ["active", "inactive"] }}
        total={1}
      />,
    );

    expect(screen.getByText("Status")).toBeInTheDocument();
    expect(screen.getByText("Kind")).toBeInTheDocument();
    expect(screen.getByText("Tenant")).toBeInTheDocument();
  });
});
