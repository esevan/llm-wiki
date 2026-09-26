import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  ReferenceAwarePreview,
  type ReferenceAwarePreviewProps,
} from "./ReferenceAwarePreview";
import { fixtureVersion, fixtureWorkspace } from "./referencePreviewFixtures";
const callbacks = (): ReferenceAwarePreviewProps => ({
  workspace: fixtureWorkspace(2),
  onGenerate: vi.fn().mockResolvedValue(undefined),
  onInvestigate: vi.fn().mockResolvedValue(undefined),
  onRestore: vi.fn().mockResolvedValue(undefined),
  onSelect: vi
    .fn()
    .mockImplementation(async (version) => fixtureVersion(version)),
  onCompare: vi.fn().mockImplementation(async (left, right) => ({
    left: fixtureVersion(left),
    right: fixtureVersion(right),
    fields: [{ key: "goal", state: "changed" }],
  })),
  onSave: vi.fn().mockResolvedValue(undefined),
  onApply: vi.fn().mockResolvedValue(undefined),
  onOpenReference: vi.fn().mockImplementation(async (reference) => ({
    ...reference,
    markdown: "# Approval\nExact source body",
  })),
});
describe("Reference-aware preview", () => {
  it("keeps zero-result filtering mounted and opens the exact version", async () => {
    const props = callbacks();
    render(<ReferenceAwarePreview {...props} />);
    const search = screen.getByLabelText("Search references");
    fireEvent.change(search, { target: { value: "missing" } });
    expect(screen.getByText("No matching references")).toBeVisible();
    expect(search).toBeVisible();
    fireEvent.change(search, { target: { value: "Approval" } });
    fireEvent.click(
      screen.getByRole("button", { name: "Approval policy Approval" }),
    );
    await screen.findByText("Exact source body");
    expect(props.onOpenReference).toHaveBeenCalledWith(
      fixtureVersion().references[0],
      2,
    );
  });
  it("fetches complete history and comparison, and prevents historical apply", async () => {
    const props = callbacks();
    render(<ReferenceAwarePreview {...props} />);
    fireEvent.change(screen.getByLabelText("Version"), {
      target: { value: "1" },
    });
    await screen.findByText("Contract preview 1");
    expect(props.onSelect).toHaveBeenCalledWith(1);
    expect(
      screen.queryByRole("button", { name: "Apply to Task" }),
    ).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Later version"), {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Compare versions" }));
    await screen.findByRole("region", { name: "Version comparison" });
    expect(props.onCompare).toHaveBeenCalledWith(1, 2);
    fireEvent.click(screen.getByRole("button", { name: "Restore as new" }));
    await waitFor(() =>
      expect(props.onRestore).toHaveBeenCalledWith(fixtureVersion(1)),
    );
  });
  it("preserves edits against polling and sends the original version/hash to save", async () => {
    const props = callbacks();
    props.onSave = vi
      .fn()
      .mockRejectedValue(new Error("preview_head_conflict"));
    const view = render(<ReferenceAwarePreview {...props} />);
    fireEvent.click(screen.getByRole("button", { name: "Edit preview" }));
    fireEvent.change(screen.getByLabelText("Edit preview: Goal"), {
      target: { value: "User condition" },
    });
    view.rerender(
      <ReferenceAwarePreview {...props} workspace={fixtureWorkspace(3)} />,
    );
    expect(screen.getByLabelText("Edit preview: Goal")).toHaveValue(
      "User condition",
    );
    expect(
      screen.getByText(
        "A newer preview is available. Your edits are preserved.",
      ),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Save preview" }));
    await screen.findByRole("alert");
    expect(props.onSave).toHaveBeenCalledWith(
      fixtureVersion(2),
      expect.objectContaining({ goal: "User condition" }),
    );
    expect(screen.getByLabelText("Edit preview: Goal")).toHaveValue(
      "User condition",
    );
  });
  it("pins the open viewer version while a newer preview arrives", async () => {
    const props = callbacks();
    const view = render(<ReferenceAwarePreview {...props} />);
    fireEvent.click(
      screen.getByRole("button", { name: "Approval policy Approval" }),
    );
    await screen.findByText("Exact source body");
    view.rerender(
      <ReferenceAwarePreview {...props} workspace={fixtureWorkspace(3)} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Next reference" }));
    await waitFor(() =>
      expect(props.onOpenReference).toHaveBeenLastCalledWith(
        fixtureVersion().references[1],
        2,
      ),
    );
  });
  it("keeps reading order and the active filter with 1,000 references", () => {
    const props = callbacks();
    const workspace = fixtureWorkspace();
    const references = Array.from({ length: 1_000 }, (_, index) => ({
      ...fixtureVersion().references[0],
      documentId: `document-${index}`,
      title: `Source ${String(index).padStart(4, "0")}`,
    }));
    workspace.preview!.current!.references = references;
    const view = render(
      <ReferenceAwarePreview {...props} workspace={workspace} />,
    );
    const rows = () =>
      Array.from(
        document.querySelectorAll('[data-control="reference-row-open"]'),
      ).map((node) => node.getAttribute("aria-label"));
    expect(rows()).toHaveLength(20);
    fireEvent.change(screen.getByLabelText("Search references"), {
      target: { value: "Source 099" },
    });
    expect(rows()).toHaveLength(10);
    const updated = structuredClone(workspace);
    updated.preview!.current!.references = [
      { ...references[0], documentId: "new-source", title: "Source 099 new" },
      ...references,
    ];
    view.rerender(<ReferenceAwarePreview {...props} workspace={updated} />);
    expect(screen.getByLabelText("Search references")).toHaveValue(
      "Source 099",
    );
    expect(rows()).toHaveLength(10);
    expect(screen.getByText("New references are ready (1)")).toBeVisible();
    fireEvent.click(
      screen.getByRole("button", { name: "Show new references" }),
    );
    expect(rows().at(-1)).toContain("Source 099 new");
    fireEvent.change(screen.getByLabelText("Reference order"), {
      target: { value: "title" },
    });
    expect(rows()[0]).toContain("Source 099 new");
  });
  it("shows persisted assumption status separately from its basis", () => {
    const props = callbacks();
    props.workspace!.preview!.current!.assumptions = [
      {
        id: "confirmed",
        text: "Recorded condition",
        status: "confirmed",
        basis: "source",
        sourceClaimIds: ["c"],
      },
    ];
    render(<ReferenceAwarePreview {...props} />);
    fireEvent.click(screen.getByText("Assumptions (1)"));
    expect(
      screen.getByText("Confirmed assumption · Source evidence"),
    ).toBeVisible();
    expect(screen.getByText("Recorded condition")).toBeVisible();
  });
});
