import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { WorkbenchView } from "./WorkbenchView";

const snapshot = {
  revision: 1,
  activeShortcuts: [{ kind: "task" as const, id: "active", taskRevision: 3 }],
  refiningShortcuts: [
    { kind: "capture" as const, id: "capture", draftRevision: 2 },
  ],
  categories: [
    {
      id: "general",
      label: "General",
      items: [
        { kind: "capture" as const, id: "capture", text: "Keep this thought" },
        {
          kind: "task" as const,
          id: "active",
          taskRevision: 3,
          state: "in_progress" as const,
          title: "Ship workbench",
        },
        {
          kind: "task" as const,
          id: "ready",
          taskRevision: 1,
          state: "task" as const,
          title: "Plan release",
        },
      ],
    },
  ],
};
const response = (body: unknown, ok = true, status = ok ? 200 : 500) => ({
  ok,
  status,
  json: async () => body,
  text: async () => "",
  body: null,
});

describe("Task Workbench", () => {
  it("refreshes saved refinement shortcuts when the loaded panel closes", async () => {
    let saved = false;
    const request = vi.fn().mockImplementation(({ path, method }: { path: string; method?: string }) => {
      if (path === "/workbench") return Promise.resolve(response({ ...snapshot, refiningShortcuts: saved ? snapshot.refiningShortcuts : [] }));
      if (path.endsWith("/workspace") && method === "PUT") saved = true;
      return Promise.resolve(response(path.endsWith("/proposals") ? [] : { id: "saved", draftRevision: 0, messages: [] }));
    });
    window.llmWikiApplication = { request };
    render(<WorkbenchView active />);
    const capture = await screen.findByText("Keep this thought");
    fireEvent.click(capture.closest("article")!.querySelector('[data-control="task-card-refine"]')!);
    await waitFor(() => expect(document.querySelector(".refinement-panel")).toHaveAttribute("data-refinement-session", "saved"));
    fireEvent.click(screen.getByRole("button", { name: "×" }));
    await waitFor(() => expect(document.querySelector('[data-control="task-shortcut-refine"]')).toBeInTheDocument());
    expect(saved).toBe(true);
  });

  it("orders active and refining shortcuts before canonical categories", async () => {
    window.llmWikiApplication = {
      request: vi.fn().mockResolvedValue(response(snapshot)),
    };
    render(<WorkbenchView active />);
    await waitFor(() =>
      expect(screen.getAllByText("Ship workbench")).toHaveLength(2),
    );
    const page = document.getElementById("workbench")!.textContent!;
    expect(page.indexOf("Active tasks")).toBeLessThan(page.indexOf("Refining"));
    expect(page.indexOf("Refining")).toBeLessThan(page.indexOf("All work"));
    expect(screen.getAllByText("Ship workbench")).toHaveLength(2);
    expect(screen.getByText("Plan release").closest("article")).toHaveTextContent(
      "Task",
    );
    expect(screen.getByText("Plan release").closest("article")).not.toHaveTextContent(
      "In progress",
    );
  });

  it("retains a direct Task draft and selected mode after a save failure", async () => {
    window.llmWikiApplication = {
      request: vi
        .fn()
        .mockImplementation(({ path }: { path: string }) =>
          path === "/workbench"
            ? Promise.resolve(response(snapshot))
            : Promise.resolve(response({ error: "offline" }, false)),
        ),
    };
    render(<WorkbenchView active />);
    await screen.findByText("All work");
    fireEvent.click(screen.getByRole("radio", { name: "Register as a Task" }));
    const input = screen.getByLabelText(
      "Workbench entry",
    ) as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: "Keep my Task draft" } });
    fireEvent.submit(input.closest("form")!);
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("offline"),
    );
    expect(input).toHaveValue("Keep my Task draft");
    expect(
      screen.getByRole("radio", { name: "Register as a Task" }),
    ).toBeChecked();
  });

  it("localizes empty shortcut states in Korean", async () => {
    document.documentElement.lang = "ko";
    window.llmWikiApplication = {
      request: vi.fn().mockResolvedValue(response({
        ...snapshot,
        activeShortcuts: [],
        refiningShortcuts: [],
        categories: [],
      })),
    };
    render(<WorkbenchView active />);
    await screen.findByText("진행 중인 Task가 없습니다.");
    expect(screen.queryByText("No active Tasks yet.")).not.toBeInTheDocument();
    expect(screen.getByText("다듬기를 기다리는 항목이 없습니다.")).toBeInTheDocument();
    document.documentElement.lang = "en";
  });

  it("renders a migrated Problem-only refinement item without presenting it as a Task", async () => {
    const openChat = vi.fn();
    window.openChat = openChat;
    window.llmWikiApplication = {
      request: vi.fn().mockResolvedValue(response({
        ...snapshot,
        activeShortcuts: [],
        refiningShortcuts: [],
        categories: [{
          id: "General",
          label: "General",
          items: [{
            kind: "refinement",
            id: "legacy-problem-p1",
            problemId: "p1",
            problemRevision: 2,
            title: "Preserved migrated Problem",
            sourceKind: "legacy_problem",
          }],
        }],
      })),
    };
    render(<WorkbenchView active />);
    const card = await screen.findByText("Preserved migrated Problem");
    expect(card.closest("article")).toHaveTextContent("Refining");
    expect(card.closest("article")).not.toHaveTextContent("Task");
    fireEvent.click(card.closest("article")!.querySelector("button")!);
    expect(openChat).toHaveBeenCalledWith("problems", "p1", {
      problemRevision: 2,
      sourceTitle: "Preserved migrated Problem",
    });
    delete window.openChat;
  });
});
