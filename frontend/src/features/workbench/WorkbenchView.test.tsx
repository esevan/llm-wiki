import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, afterAll, describe, expect, it, vi } from "vitest";
import { WorkbenchView } from "./WorkbenchView";

beforeAll(() => {
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", { configurable: true, value: function (this: HTMLDialogElement) { this.open = true; } });
  Object.defineProperty(HTMLDialogElement.prototype, "close", { configurable: true, value: function (this: HTMLDialogElement) { this.open = false; } });
});
afterAll(() => { Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal"); Reflect.deleteProperty(HTMLDialogElement.prototype, "close"); });

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
  it("renders Subtasks under their parent and focuses only active work", async () => {
    const familySnapshot = { ...snapshot, categories: [{ id: "general", label: "General", items: [
      { kind: "task", id: "parent", title: "Meeting preparation", taskRevision: 2, refinedRevision: 2, state: "task" },
      { kind: "task", id: "active", parentTaskId: "parent", title: "Collect topics", taskRevision: 3, state: "in_progress" },
    ] }] };
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(familySnapshot)) };
    render(<WorkbenchView active />);
    expect(await screen.findByText("Refined - Revision 2")).toBeInTheDocument();
    const disclosure = document.querySelector<HTMLDetailsElement>('[data-control="task-subtasks-expand"]')!;
    expect(disclosure.closest(".canonical-card")).toHaveTextContent("Meeting preparation");
    fireEvent.click(disclosure.querySelector("summary")!);
    expect(disclosure.open).toBe(true);
    expect(disclosure.querySelector(".subtask-children")).toHaveTextContent("Collect topics");
    fireEvent.click(screen.getByRole("button", { name: "Focus active work" }));
    expect(document.querySelector("#workbench")).toHaveAttribute("data-focus-active", "true");
    expect(screen.getByRole("button", { name: "Show all work" })).toHaveAttribute("aria-pressed", "true");
  });

  it("keeps the mounted Task and its unsaved inputs when refining and returning", async () => {
    const aggregate = { id: "active", taskRevision: 3, state: "in_progress", title: "Ship workbench" };
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) =>
      Promise.resolve(response(path === "/workbench" ? snapshot : path.endsWith("/proposals") ? [] : path.endsWith("/refinement") || path.endsWith("/workspace") ? { id: "refine", messages: [] } : aggregate))) };
    render(<WorkbenchView active />);
    fireEvent.click((await screen.findAllByText("Ship workbench"))[0].closest("button")!);
    await waitFor(() => expect(document.querySelector('[data-control="task-detail-refine"]')).toBeInTheDocument());
    const original = document.querySelector(".task-detail");
    const entry = screen.getByLabelText("Work Log entry");
    fireEvent.change(entry, { target: { value: "Keep this evidence draft" } });
    fireEvent.click(document.querySelector('[data-control="task-detail-refine"]')!);
    await screen.findByRole("button", { name: "Close refinement" });
    expect(document.getElementById("workbench")).toHaveAttribute("data-refining-kind", "task");
    expect(document.querySelector(".task-detail")).toBe(original);
    expect(entry).toHaveValue("Keep this evidence draft");
    fireEvent.click(screen.getByRole("button", { name: "Close refinement" }));
    await waitFor(() => expect(document.querySelector(".refinement-panel")).not.toBeInTheDocument());
    expect(document.querySelector(".task-detail")).toBe(original);
    expect(entry).toHaveValue("Keep this evidence draft");
  });

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
    fireEvent.click(screen.getByRole("button", { name: "Close refinement" }));
    await waitFor(() => expect(document.querySelector('[data-control="task-shortcut-refine"]')).toBeInTheDocument());
    expect(saved).toBe(true);
  });

  it("places active Tasks above three exclusive workflow lanes", async () => {
    const board = { ...snapshot, categories: [{ ...snapshot.categories[0], items: [
      ...snapshot.categories[0].items,
      { kind: "capture", id: "inbox", text: "A new thought" },
      { kind: "task", id: "done", title: "Finished work", state: "completed", taskRevision: 1 },
      { kind: "task", id: "shaping", title: "Shape this task", state: "task", taskRevision: 1 },
    ] }], refiningShortcuts: [...snapshot.refiningShortcuts, { kind: "task", id: "shaping", draftRevision: 1 }] };
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(board)) };
    render(<WorkbenchView active />);
    await screen.findByText("Ship workbench");
    expect(screen.getAllByText("Ship workbench")).toHaveLength(1);
    expect(document.querySelector(".workbench-active")).toHaveTextContent("Ship workbench");
    expect(document.querySelector(".workbench-board")).not.toHaveTextContent("Ship workbench");
    const lanes = [...document.querySelectorAll("[data-lane]")];
    expect(lanes.map(lane => lane.getAttribute("data-lane"))).toEqual(["inbox", "refining", "tasks"]);
    expect(lanes[0]).toHaveTextContent("A new thought");
    expect(lanes[0]).not.toHaveTextContent("Keep this thought");
    expect(lanes[1]).toHaveTextContent("Keep this thought");
    expect(lanes[1]).toHaveTextContent("Shape this task");
    expect(lanes[2]).toHaveTextContent("Plan release");
    expect(lanes[2]).not.toHaveTextContent("Shape this task");
    expect(lanes[2].querySelector("details")).not.toHaveAttribute("open");
    expect(lanes[2].querySelector("details")).toHaveTextContent("Finished work");
    expect(document.querySelector(".workbench-main")?.firstElementChild).toHaveClass("workbench-active");
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
    await screen.findByRole("region", { name: "All work" });
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

  it("submits the Workbench entry on Enter while preserving Shift+Enter", async () => {
    const request = vi.fn().mockResolvedValue(response(snapshot));
    window.llmWikiApplication = { request };
    render(<WorkbenchView active />);
    const input = await screen.findByLabelText("Workbench entry");
    fireEvent.change(input, { target: { value: "Enter capture" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(request).toHaveBeenCalledWith(expect.objectContaining({ path: "/captures", method: "POST" })));
    const capturesBeforeShift = request.mock.calls.filter(([arg]) => arg.path === "/captures").length;
    fireEvent.change(input, { target: { value: "line one" } });
    fireEvent.keyDown(input, { key: "Enter", shiftKey: true });
    expect(input).toHaveValue("line one");
    expect(request.mock.calls.filter(([arg]) => arg.path === "/captures")).toHaveLength(capturesBeforeShift);
    fireEvent.keyDown(input, { key: "Enter", isComposing: true });
    expect(request.mock.calls.filter(([arg]) => arg.path === "/captures")).toHaveLength(capturesBeforeShift);
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
      workspaceDock: true,
    });
    delete window.openChat;
  });

  it("requires confirmation before deleting a Capture and refreshes after confirmation", async () => {
    let deleted = false;
    const request = vi.fn().mockImplementation(({ path, method }: { path: string; method?: string }) => {
      if (path === "/workbench") return Promise.resolve(response(deleted ? { ...snapshot, refiningShortcuts: [], categories: [] } : snapshot));
      if (path === "/items/captures/capture" && method === "DELETE") { deleted = true; return Promise.resolve(response(null, true, 204)); }
      return Promise.resolve(response({}));
    });
    window.llmWikiApplication = { request };
    render(<WorkbenchView active />);
    const card = (await screen.findAllByText("Keep this thought")).find((node) => node.closest("article"))!;
    const trigger = card.closest("article")!.querySelector<HTMLButtonElement>('[data-control="task-card-delete"]')!;
    trigger.focus();
    fireEvent.click(trigger);
    expect(screen.getByRole("button", { name: "Keep item" })).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "Keep item" }));
    expect(trigger).toHaveFocus();
    fireEvent.click(trigger);
    fireEvent(screen.getByRole("alertdialog"), new Event("cancel", { bubbles: false, cancelable: true }));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    fireEvent.click(trigger);
    expect(request.mock.calls.some(([arg]) => arg.path.includes("/items/"))).toBe(false);
    expect(screen.getByRole("alertdialog")).toHaveTextContent("Delete this item from the Workbench?");
    fireEvent.click(screen.getByRole("alertdialog").querySelector('[data-control="task-delete-confirm"]')!);
    await waitFor(() => expect(screen.queryByText("Keep this thought")).not.toBeInTheDocument());
    expect(request.mock.calls.filter(([arg]) => arg.method === "DELETE")).toHaveLength(1);
    expect(document.querySelector("#workbench h1")).toHaveFocus();
  });
  it("retains a Task and its active shortcut when deletion fails, then permits retry", async () => {
    let attempts = 0;
    const request = vi.fn().mockImplementation(({ method }: { path: string; method?: string }) => {
      if (method === "DELETE") { attempts++; return Promise.resolve(attempts === 1 ? response({ error: "Delete offline" }, false) : response(null, true, 204)); }
      return Promise.resolve(response(attempts === 2 ? { ...snapshot, activeShortcuts: [], categories: [] } : snapshot));
    });
    window.llmWikiApplication = { request };
    render(<WorkbenchView active />);
    const card = (await screen.findAllByText("Ship workbench")).find((node) => node.closest("article"))!;
    fireEvent.click(card.closest("article")!.querySelector('[data-control="task-card-delete"]')!);
    fireEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: /^Delete$/ }));
    await screen.findByText("Delete offline");
    expect(document.querySelector('[data-control="task-shortcut-open"]')).toBeInTheDocument();
    fireEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: /^Delete$/ }));
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
    expect(document.querySelector('[data-control="task-shortcut-open"]')).not.toBeInTheDocument();
    expect(request.mock.calls.filter(([arg]) => arg.path === "/tasks/active" && arg.method === "DELETE")).toHaveLength(2);
  });

});

const draftControl = (id: string) => {
  if (id.startsWith("task-revision-")) {
    const details = document.querySelector('[data-control="task-detail-tab-details"]') as HTMLElement | null;
    if (details) fireEvent.click(details);
    const edit = document.querySelector('[data-control="task-definition-edit"]') as HTMLElement | null;
    if (edit) fireEvent.click(edit);
  }
  return document.querySelector<HTMLElement>(`[data-control="${id}"]`)!;
};
const openTask = (title: string) => fireEvent.click(screen.getAllByText(title).find((node) => node.closest("article"))!.closest("article")!.querySelector("button")!);
describe("guarded Task selection", () => {
  it.each(["keep", "discard", "save", "failure"])("handles %s when switching tasks", async (choice) => {
    let active = { id: "active", kind: "task", taskRevision: 3, state: "in_progress", title: "Ship workbench", detail: "Original", outcome: "", scope: "", nonGoals: "", validationCriteria: "" };
    const request = vi.fn().mockImplementation(({ path, method, body }: { path: string; method?: string; body?: string }) => {
      if (path === "/workbench") return Promise.resolve(response(snapshot));
      if (path.endsWith("/revisions") && method === "POST") {
        if (choice === "failure") return Promise.resolve(response({ detail: "Offline" }, false));
        active = { ...active, ...JSON.parse(body!).patch, taskRevision: 4 };
        return Promise.resolve(response(active));
      }
      return Promise.resolve(response(path === "/tasks/active" ? active : { ...active, id: "ready", title: "Plan release", taskRevision: 1 }));
    });
    window.llmWikiApplication = { request };
    render(<WorkbenchView active />);
    await screen.findByText("Plan release");
    openTask("Ship workbench");
    await waitFor(() => expect(draftControl("task-revision-detail")).toBeInTheDocument());
    fireEvent.change(draftControl("task-revision-detail"), { target: { value: "Local draft" } });
    openTask("Plan release");
    expect(draftControl("task-revision-detail")).toHaveValue("Local draft");
    expect(request.mock.calls.some(([arg]) => arg.path === "/tasks/ready")).toBe(false);
    fireEvent.click(draftControl(choice === "keep" ? "task-draft-guard-keep-editing" : choice === "discard" ? "task-draft-guard-discard" : "task-draft-guard-save"));
    if (choice === "keep" || choice === "failure") {
      if (choice === "failure") await screen.findByText("Offline");
      expect(draftControl("task-revision-detail")).toHaveValue("Local draft");
      expect(request.mock.calls.some(([arg]) => arg.path === "/tasks/ready")).toBe(false);
    } else {
      await waitFor(() => expect(document.querySelector(".task-detail h2")).toHaveTextContent("Plan release"));
      expect(draftControl("task-revision-detail")).toHaveValue(choice === "save" ? "Local draft" : "Original");
    }
    expect(request.mock.calls.filter(([arg]) => arg.path.endsWith("/revisions"))).toHaveLength(choice === "save" || choice === "failure" ? 1 : 0);
  });

  it("guards unsaved detail before deletion and confirms the current Task title", async () => {
    const task = { id: "active", taskRevision: 3, state: "in_progress", title: "Ship workbench", detail: "Original" };
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => Promise.resolve(response(path === "/workbench" ? snapshot : task)));
    window.llmWikiApplication = { request };
    render(<WorkbenchView active />);
    await screen.findByText("Plan release");
    openTask("Ship workbench");
    await waitFor(() => expect(draftControl("task-revision-detail")).toBeInTheDocument());
    fireEvent.change(draftControl("task-revision-detail"), { target: { value: "Unsaved draft" } });
    fireEvent.click(draftControl("task-detail-delete"));
    expect(document.querySelector(".workbench-delete-dialog")).not.toBeInTheDocument();
    fireEvent.click(draftControl("task-draft-guard-keep-editing"));
    expect(draftControl("task-revision-detail")).toHaveValue("Unsaved draft");
    fireEvent.click(draftControl("task-detail-delete"));
    fireEvent.click(draftControl("task-draft-guard-discard"));
    expect(document.querySelector(".workbench-delete-dialog")).toHaveTextContent("Ship workbench");
    fireEvent.click(draftControl("task-delete-cancel"));
    expect(document.querySelector(".task-detail")).toBeInTheDocument();
    expect(request.mock.calls.some(([arg]) => arg.method === "DELETE")).toBe(false);
  });

  it("blocks switches during a mutation and keeps drafts while the route is hidden", async () => {
    let finish!: (value: ReturnType<typeof response>) => void;
    const pending = new Promise<ReturnType<typeof response>>((resolve) => { finish = resolve; });
    const active = { id: "active", taskRevision: 3, state: "in_progress", title: "Ship workbench" };
    const request = vi.fn().mockImplementation(({ path, method }: { path: string; method?: string }) => {
      if (path === "/workbench") return Promise.resolve(response(snapshot));
      if (method === "POST") return pending;
      return Promise.resolve(response(active));
    });
    window.llmWikiApplication = { request };
    const view = render(<WorkbenchView active />);
    await screen.findByText("Plan release");
    openTask("Ship workbench");
    await waitFor(() => expect(draftControl("task-revision-detail")).toBeInTheDocument());
    fireEvent.change(draftControl("task-revision-detail"), { target: { value: "Draft" } });
    view.rerender(<WorkbenchView active={false} />);
    view.rerender(<WorkbenchView active />);
    expect(draftControl("task-revision-detail")).toHaveValue("Draft");
    fireEvent.change(draftControl("task-worklog-text"), { target: { value: "Evidence" } });
    fireEvent.click(draftControl("task-worklog-add"));
    await waitFor(() => expect(document.querySelector(".task-detail")).toHaveAttribute("aria-busy", "true"));
    openTask("Plan release");
    expect(document.querySelector(".task-draft-guard")).not.toBeInTheDocument();
    expect(request.mock.calls.some(([arg]) => arg.path === "/tasks/ready")).toBe(false);
    finish(response({ id: "entry" }));
    await waitFor(() => expect(document.querySelector(".task-detail")).toHaveAttribute("aria-busy", "false"));
    expect(draftControl("task-revision-detail")).toHaveValue("Draft");
  });

  it("ignores an old Task GET after selection changes and restores focus after Escape", async () => {
    let finish!: (value: ReturnType<typeof response>) => void;
    const pending = new Promise<ReturnType<typeof response>>((resolve) => { finish = resolve; });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => {
      if (path === "/workbench") return Promise.resolve(response(snapshot));
      if (path === "/tasks/active") return pending;
      return Promise.resolve(response({ id: "ready", taskRevision: 1, state: "task", title: "Plan release", detail: "Ready detail" }));
    }) };
    render(<WorkbenchView active />);
    await screen.findByText("Plan release");
    openTask("Ship workbench");
    openTask("Plan release");
    await waitFor(() => expect(draftControl("task-revision-detail")).toHaveValue("Ready detail"));
    finish(response({ id: "active", taskRevision: 3, state: "in_progress", title: "Old task", detail: "Wrong" }));
    await waitFor(() => expect(document.querySelector(".task-detail h2")).toHaveFocus());
    expect(draftControl("task-revision-detail")).toHaveValue("Ready detail");
    const trigger = screen.getAllByText("Plan release").find((node) => node.closest("article"))!.closest("article")!.querySelector("button")!;
    fireEvent.keyDown(draftControl("task-revision-detail"), { key: "Escape" });
    expect(document.querySelector(".task-detail")).not.toBeInTheDocument();
    await waitFor(() => expect(trigger).toHaveFocus());
  });
});
