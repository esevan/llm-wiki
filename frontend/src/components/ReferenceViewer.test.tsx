import { useState } from "react";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ReferenceViewer, type ReferenceDocument } from "./ReferenceViewer";
import { fixtureVersion } from "../features/workbench/referencePreviewFixtures";
const references = fixtureVersion().references;
function Harness({
  read,
  sourceReferences = references,
}: {
  read: (reference: (typeof references)[number]) => Promise<ReferenceDocument>;
  sourceReferences?: typeof references;
}) {
  const [binding, setBinding] = useState(sourceReferences[0]);
  const [open, setOpen] = useState(false);
  return (
    <>
      <button onClick={() => setOpen(true)}>Open source</button>
      <ReferenceViewer
        open={open}
        binding={binding}
        references={sourceReferences}
        read={read}
        onNavigate={setBinding}
        onClose={() => setOpen(false)}
      />
    </>
  );
}
describe("Reference viewer", () => {
  it("keeps back/forward history exact and restores the opener on Escape", async () => {
    const read = vi.fn(async (reference: (typeof references)[number]) => ({
      ...reference,
      markdown: `# ${reference.section}\n${reference.documentId} body`,
    }));
    render(<Harness read={read} />);
    const opener = screen.getByRole("button", { name: "Open source" });
    opener.focus();
    fireEvent.click(opener);
    await screen.findByText("approval body");
    fireEvent.click(screen.getByRole("button", { name: "Next reference" }));
    await screen.findByText("rollout body");
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    await screen.findByText("approval body");
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Forward" }));
    await screen.findByText("rollout body");
    expect(screen.getByRole("button", { name: "Forward" })).toBeDisabled();
    fireEvent.keyDown(document, { key: "Escape" });
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    await waitFor(() => expect(opener).toHaveFocus());
  });
  it("ignores a late source response and exposes a recoverable read failure", async () => {
    let resolveFirst: (value: ReferenceDocument) => void = () => undefined;
    const read = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<ReferenceDocument>((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockRejectedValueOnce(new Error("version unavailable"))
      .mockResolvedValue({
        ...references[1],
        markdown: "Recovered exact source",
      });
    render(<Harness read={read} />);
    fireEvent.click(screen.getByRole("button", { name: "Open source" }));
    fireEvent.click(screen.getByRole("button", { name: "Next reference" }));
    await screen.findByRole("alert");
    await act(async () =>
      resolveFirst({ ...references[0], markdown: "Wrong late source" }),
    );
    expect(screen.queryByText("Wrong late source")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await screen.findByText("Recovered exact source");
  });
  it("resolves relative links to an exact saved section", async () => {
    const sourceReferences = references.map((reference) => ({
      ...reference,
      path: `Knowledge/${reference.path}`,
    }));
    sourceReferences[1].section = "Policy > Rollout";
    const read = vi.fn(async (reference: (typeof references)[number]) => ({
      ...reference,
      markdown:
        reference.documentId === "approval"
          ? "[Read rollout](./rollout.md#rollout)"
          : "Exact linked rollout section",
    }));
    render(<Harness read={read} sourceReferences={sourceReferences} />);
    fireEvent.click(screen.getByRole("button", { name: "Open source" }));
    fireEvent.click(await screen.findByRole("link", { name: "Read rollout" }));
    await screen.findByText("Exact linked rollout section");
    expect(read).toHaveBeenLastCalledWith(sourceReferences[1]);
  });
});
