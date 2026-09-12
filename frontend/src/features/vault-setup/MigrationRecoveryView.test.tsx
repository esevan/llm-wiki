import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { MigrationRecoveryView } from "./MigrationRecoveryView";

const recovery = {
  stage: "schema_migration",
  safeError: "migration_failed",
  manifestFile: "state.migration-v7.manifest.json",
  retryAvailable: true,
  restoreAvailable: true,
  createdAt: "2026-09-05T00:00:00Z",
};

describe("migration recovery", () => {
  it("blocks ordinary work and requires an explicit verified restore or retry choice", () => {
    const onRestore = vi.fn(),
      onRetry = vi.fn();
    render(
      <MigrationRecoveryView
        recovery={recovery}
        busy={false}
        error=""
        onRestore={onRestore}
        onRetry={onRetry}
      />,
    );
    expect(
      screen.getByRole("dialog", { name: /recovery is needed/i }),
    ).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: /restore verified backup/i }),
    );
    fireEvent.click(screen.getByRole("button", { name: /retry migration/i }));
    expect(onRestore).toHaveBeenCalledOnce();
    expect(onRetry).toHaveBeenCalledOnce();
  });
});
