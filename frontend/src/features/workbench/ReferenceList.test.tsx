import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ReferenceList } from "./ReferenceList";
import type {
  ExactReferenceBinding,
  ReferenceUsageFact,
} from "../../types/taskWorkbench";
const reference = (
  id: string,
  section = "Approval",
): ExactReferenceBinding => ({
  documentId: id,
  documentVersion: "v1",
  title: `Source ${id}`,
  section,
  status: "current",
  aspect: "applicability",
  informationType: "knowledge",
});
const rows = () =>
  screen.queryAllByRole("button", { name: /^Source .* (Approval|Decision)$/ });
describe("Grouped reference rail", () => {
  it("pages 1,000 documents, groups sections, and opens the exact section with page navigation context", () => {
    const references = Array.from({ length: 1_000 }, (_, index) =>
      reference(String(index).padStart(4, "0")),
    );
    references.splice(1, 0, reference("0000", "Decision"));
    const open = vi.fn();
    render(<ReferenceList references={references} onOpen={open} />);
    expect(document.querySelectorAll(".reference-document-group")).toHaveLength(
      20,
    );
    expect(rows()).toHaveLength(21);
    expect(screen.getByText("Page 1 / 50 · 1000 documents")).toBeVisible();
    fireEvent.click(
      screen.getByRole("button", { name: "Source 0000 Decision" }),
    );
    expect(open).toHaveBeenCalledWith(
      references[1],
      expect.arrayContaining([references[0], references[1]]),
    );
    expect(open.mock.calls[0][1]).toHaveLength(21);
    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    expect(screen.getByText("Page 2 / 50 · 1000 documents")).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Source 0020 Approval" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Previous page" }));
    expect(
      screen.getByRole("button", { name: "Source 0000 Decision" }),
    ).toBeVisible();
  });
  it("combines metadata and actual-use filters without promoting mere discovery", () => {
    const used = reference("used"),
      discovered = reference("discovered");
    const idea = {
      ...reference("idea"),
      informationType: "idea",
      aspect: "exploration",
    };
    const interactions: ReferenceUsageFact[] = [
      { ...used, kind: "used", contextRevision: 1 },
      { ...idea, kind: "viewed", contextRevision: 1 },
    ];
    render(
      <ReferenceList
        references={[used, discovered, idea]}
        interactions={interactions}
        onOpen={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByLabelText("Reference use"), {
      target: { value: "used" },
    });
    expect(rows()).toHaveLength(1);
    expect(rows()[0]).toHaveAccessibleName("Source used Approval");
    fireEvent.change(screen.getByLabelText("Reference use"), {
      target: { value: "discovered" },
    });
    expect(rows()[0]).toHaveAccessibleName("Source discovered Approval");
    fireEvent.change(screen.getByLabelText("Reference use"), {
      target: { value: "all" },
    });
    fireEvent.change(screen.getByLabelText("Information type"), {
      target: { value: "idea" },
    });
    fireEvent.change(screen.getByLabelText("Evidence aspect"), {
      target: { value: "exploration" },
    });
    expect(rows()).toHaveLength(1);
    fireEvent.change(screen.getByLabelText("Search references"), {
      target: { value: "absent" },
    });
    expect(screen.getByText("No matching references")).toBeVisible();
    expect(screen.getByLabelText("Information type")).toHaveValue("idea");
  });
  it("holds new documents and sections pending without moving focus or the selected page", () => {
    const references = Array.from({ length: 25 }, (_, index) =>
      reference(String(index).padStart(2, "0")),
    );
    const view = render(
      <ReferenceList references={references} onOpen={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    const opener = screen.getByRole("button", { name: "Source 20 Approval" });
    opener.focus();
    view.rerender(
      <ReferenceList
        references={[
          reference("00", "Decision"),
          reference("arrival"),
          ...references,
        ]}
        onOpen={vi.fn()}
      />,
    );
    expect(opener).toHaveFocus();
    expect(screen.getByText("Page 2 / 2 · 25 documents")).toBeVisible();
    expect(
      screen.queryByRole("button", { name: "Source arrival Approval" }),
    ).not.toBeInTheDocument();
    expect(screen.getByText("New references are ready (2)")).toBeVisible();
    fireEvent.click(
      screen.getByRole("button", { name: "Show new references" }),
    );
    expect(screen.getByText("Page 2 / 2 · 26 documents")).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Source arrival Approval" }),
    ).toBeVisible();
    fireEvent.change(screen.getByLabelText("Reference order"), {
      target: { value: "title" },
    });
    expect(screen.getByText("Page 1 / 2 · 26 documents")).toBeVisible();
    const group = document.querySelector(
      ".reference-document-group",
    ) as HTMLElement;
    expect(
      within(group).getByRole("button", { name: "Source 00 Decision" }),
    ).toBeVisible();
  });
});
