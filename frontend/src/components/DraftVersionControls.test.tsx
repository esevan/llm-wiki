import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DraftVersionControls } from "./DraftVersionControls";
describe("Draft version controls", () => {
  it("compares two explicit immutable versions and exposes restore only for history", () => {
    const compare = vi.fn(),
      restore = vi.fn();
    const versions = [
      { version: 1, derivationKind: "generated" as const },
      { version: 2, derivationKind: "edited" as const },
      { version: 3, derivationKind: "restored" as const },
    ];
    render(
      <DraftVersionControls
        versions={versions}
        selected={2}
        currentVersion={3}
        onSelect={vi.fn()}
        onCompare={compare}
        onRestore={restore}
      />,
    );
    fireEvent.change(screen.getByLabelText("Earlier version"), {
      target: { value: "2" },
    });
    expect(
      screen.getByRole("button", { name: "Compare versions" }),
    ).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Later version"), {
      target: { value: "3" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Compare versions" }));
    expect(compare).toHaveBeenCalledWith(2, 3);
    fireEvent.click(screen.getByRole("button", { name: "Restore as new" }));
    expect(restore).toHaveBeenCalledWith(2);
  });
});
