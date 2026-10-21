import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { WorkbenchView } from "./WorkbenchView";
vi.mock("./RefinementPanel", () => ({ RefinementPanel: ({ onApplied }: { onApplied: (event: { taskId: string; revision: number; cause: string }) => void }) => <button onClick={() => onApplied({ taskId: "applied", revision: 2, cause: "preview_adopted" })}>Apply acknowledged preview</button> }));
vi.mock("./TaskDetail", () => ({ TaskDetail: ({ taskId }: { taskId: string }) => <div data-testid="opened-task">{taskId}</div> }));
describe("Workbench refinement application navigation", () => {
  it.each(["capture"] as const)("opens the acknowledged Task after %s Apply and moves it to refined work", async kind => {
    let applied = false;
    window.llmWikiApplication = { request: vi.fn().mockImplementation(() => Promise.resolve({ ok: true, status: 200, text: async () => "", body: null, json: async () => ({ revision: applied ? 2 : 1, activeShortcuts: [], refiningShortcuts: applied ? [] : [{ kind, id: "source", draftRevision: 1 }], categories: [{ id: "General", label: "General", items: applied ? [{ kind: "task", id: "applied", title: "Saved title", taskRevision: 2, refinedRevision: 2, state: "task" }] : [kind === "capture" ? { kind, id: "source", text: "Source work" } : { kind, id: "source", title: "Source work", state: "task", taskRevision: 1 }] }] }) })) };
    render(<WorkbenchView active onTaskApplication={() => { applied = true; }} />);
    await screen.findByText("Source work");
    fireEvent.click(document.querySelector('[data-control="task-shortcut-refine"]')!);
    fireEvent.click(await screen.findByRole("button", { name: "Apply acknowledged preview" }));
    expect(await screen.findByTestId("opened-task")).toHaveTextContent("applied");
    expect(screen.queryByRole("button", { name: "Apply acknowledged preview" })).not.toBeInTheDocument();
    await waitFor(() => expect(document.querySelector('[data-lane="tasks"]')).toHaveTextContent("Saved title"));
    expect(document.querySelector('[data-lane="refining"]')).not.toHaveTextContent("Saved title");
  });
});
