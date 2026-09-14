import { createEvent, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { TaskDetail, type DetailSession } from "./TaskDetail";

const task = {
  id: "task-1",
  kind: "task" as const,
  taskRevision: 2,
  state: "in_progress" as const,
  title: "Independent work",
  workLog: [],
  checklist: [],
  decisions: [],
  readinessEntries: [
    { key: "scope", status: "missing" as const },
    { key: "validationCriteria", status: "resolved" as const },
  ],
};
const response = (body: unknown) => ({
  ok: true,
  status: 200,
  json: async () => body,
  text: async () => "",
  body: null,
});

describe("Task detail", () => {
  it("shows Work Log entries newest first with an explicit local date and time", async () => {
    const loggedTask = {
      ...task,
      workLog: [
        { id: "older", body: "Older evidence", createdAt: "2026-01-15T10:30:00Z" },
        { id: "newer", body: "Newer evidence", createdAt: "2026-01-16T18:45:00Z" },
      ],
    };
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(loggedTask)) };
    const { container } = render(<TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />);

    await screen.findByText("Newer evidence");
    const entries = [...container.querySelectorAll<HTMLElement>(".log-entry")];
    expect(entries.map((entry) => entry.querySelector("p")?.textContent)).toEqual([
      "Newer evidence",
      "Older evidence",
    ]);
    expect(entries.map((entry) => entry.querySelector("time")?.textContent)).toEqual([
      expect.stringMatching(/\d/),
      expect.stringMatching(/\d/),
    ]);
    expect(entries.map((entry) => entry.querySelector("time")?.getAttribute("dateTime"))).toEqual([
      "2026-01-16T18:45:00Z",
      "2026-01-15T10:30:00Z",
    ]);
  });

  it("keeps dependent mutations inert until the current revision is rendered", async () => {
    let finishWorkLog!: (value: ReturnType<typeof response>) => void;
    const workLog = new Promise<ReturnType<typeof response>>((resolve) => {
      finishWorkLog = resolve;
    });
    const request = vi.fn().mockImplementation(({ path, method }: { path: string; method?: string }) =>
      path === "/tasks/task-1/work-log" && method === "POST"
        ? workLog
        : Promise.resolve(response(task)),
    );
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );
    const workLogField = await screen.findByLabelText("Work Log entry");
    fireEvent.change(workLogField, { target: { value: "Revision-bound evidence" } });
    fireEvent.click(workLogField.parentElement!.querySelector("button")!);
    const detail = screen.getByLabelText("Independent work");
    await waitFor(() => expect(detail).toHaveAttribute("aria-busy", "true"));

    const decisionField = screen.getByLabelText("Decision");
    fireEvent.change(decisionField, { target: { value: "Must wait" } });
    fireEvent.click(decisionField.parentElement!.querySelector("button")!);
    expect(request.mock.calls.some(([input]) => input.path === "/tasks/task-1/decisions")).toBe(false);

    finishWorkLog(response(task));
    await waitFor(() => expect(detail).toHaveAttribute("aria-busy", "false"));
  });

  it("requires authored completion evidence and permits work before readiness is resolved", async () => {
    const request = vi.fn().mockResolvedValue(response(task));
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );
    await screen.findByText("Independent work");
    fireEvent.click(screen.getByRole("tab", { name: "Review" }));
    expect(screen.getAllByText("Scope")).toHaveLength(1);
    expect(screen.getAllByText("Validation criteria")).toHaveLength(1);
    expect(screen.getByText("Missing")).toBeVisible();
    expect(screen.queryByText("validationCriteria")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Complete Task" }),
    ).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Completion evidence"), {
      target: { value: "Validated in a real workflow" },
    });
    expect(screen.getByRole("button", { name: "Complete Task" })).toBeEnabled();
    fireEvent.click(screen.getByRole("tab", { name: "Work" }));
    fireEvent.change(screen.getByLabelText("Work Log entry"), {
      target: { value: "Started gathering evidence" },
    });
    fireEvent.click(document.querySelector('[data-control="task-worklog-add"]')!);
    await waitFor(() =>
      expect(request).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "/tasks/task-1/work-log",
          method: "POST",
        }),
      ),
    );
  });

  it("adds Work Log entries on Enter and leaves IME Enter for composition", async () => {
    const request = vi.fn().mockResolvedValue(response(task));
    window.llmWikiApplication = { request };
    render(<TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />);
    const field = await screen.findByLabelText("Work Log entry");
    fireEvent.change(field, { target: { value: "Keyboard evidence" } });
    fireEvent.keyDown(field, { key: "Enter" });
    await waitFor(() => expect(request).toHaveBeenCalledWith(expect.objectContaining({ path: "/tasks/task-1/work-log", method: "POST" })));
    const workLogsBeforeIme = request.mock.calls.filter(([arg]) => arg.path === "/tasks/task-1/work-log").length;
    fireEvent.change(field, { target: { value: "Composing text" } });
    fireEvent.keyDown(field, { key: "Enter", isComposing: true });
    expect(field).toHaveValue("Composing text");
    expect(request.mock.calls.filter(([arg]) => arg.path === "/tasks/task-1/work-log")).toHaveLength(workLogsBeforeIme);
  });

  it("attaches an image pasted from the clipboard and submits an image-only Work Log", async () => {
    const request = vi.fn().mockResolvedValue(response(task));
    window.llmWikiApplication = { request };
    render(<TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />);
    const field = await screen.findByLabelText("Work Log entry");
    const image = new File(["screenshot"], "Screenshot.png", { type: "image/png" });
    Object.defineProperty(image, "arrayBuffer", { value: async () => new TextEncoder().encode("screenshot").buffer });
    fireEvent.paste(field, {
      clipboardData: {
        items: [{ type: "image/png", getAsFile: () => image }],
      },
    });

    expect(await screen.findByText("Attach image or file: Screenshot.png")).toBeVisible();
    const workLogAdd = document.querySelector<HTMLButtonElement>('[data-control="task-worklog-add"]')!;
    expect(workLogAdd).toBeEnabled();
    fireEvent.click(workLogAdd);

    await waitFor(() => {
      const workLog = request.mock.calls
        .map(([input]) => input)
        .find((input) => input.path === "/tasks/task-1/work-log" && input.method === "POST");
      expect(JSON.parse(workLog.body)).toMatchObject({
        expectedTaskRevision: 2,
        body: "",
        attachment: { name: "Screenshot.png", mediaType: "image/png" },
      });
    });
  });

  it("leaves plain text paste available and preserves an existing file attachment", async () => {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(task)) };
    render(<TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />);
    const field = await screen.findByLabelText("Work Log entry");
    const existing = new File(["evidence"], "evidence.txt", { type: "text/plain" });
    fireEvent.change(screen.getByLabelText("Attach image or file"), { target: { files: [existing] } });
    const paste = createEvent.paste(field, {
      clipboardData: { items: [{ type: "text/plain", getAsFile: () => null }] },
    });
    fireEvent(field, paste);

    expect(paste.defaultPrevented).toBe(false);
    expect(screen.getByText("Attach image or file: evidence.txt")).toBeVisible();
  });

  it("sends the current Task revision and the checkbox event value", async () => {
    const checklistTask = {
      ...task,
      checklist: [{ id: "check-1", body: "Verify result", checked: false }],
    };
    const request = vi.fn().mockResolvedValue(response(checklistTask));
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );

    fireEvent.click(await screen.findByRole("checkbox", { name: "Verify result" }));

    await waitFor(() => {
      const checklistUpdate = request.mock.calls
        .map(([input]) => input)
        .find(
          (input) =>
            input.path === "/tasks/task-1/checklist/check-1" &&
            input.method === "PUT",
        );
      expect(JSON.parse(checklistUpdate.body)).toMatchObject({
        expectedTaskRevision: 2,
        checked: true,
        body: "Verify result",
      });
    });
  });

  it("maps an authored decision to the append-only decision contract", async () => {
    const request = vi.fn().mockResolvedValue(response(task));
    window.llmWikiApplication = { request };
    render(
      <TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />,
    );
    const decisionField = await screen.findByLabelText("Decision");
    fireEvent.change(decisionField, {
      target: { value: "Ship deliberately" },
    });
    fireEvent.click(decisionField.parentElement!.querySelector("button")!);

    await waitFor(() => {
      const decision = request.mock.calls
        .map(([input]) => input)
        .find((input) => input.path === "/tasks/task-1/decisions");
      expect(JSON.parse(decision.body)).toMatchObject({
        expectedTaskRevision: 2,
        kind: "user",
        payload: { body: "Ship deliberately" },
      });
    });
  });

  it("retains a dirty Task definition while a Work Log refreshes the persisted snapshot", async () => {
    const refreshed = { ...task, taskRevision: 3, workLog: [{ id: "log-1", body: "Independent evidence" }] };
    const request = vi.fn().mockImplementation(({ path, method }: { path: string; method?: string }) =>
      path === "/tasks/task-1/work-log" && method === "POST"
        ? Promise.resolve(response({ id: "task-1", taskRevision: 3 }))
        : Promise.resolve(response(refreshed)),
    );
    window.llmWikiApplication = { request };
    render(<TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />);
    fireEvent.click(await screen.findByRole("tab", { name: "Details" }));
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const title = await screen.findByLabelText("Title");
    fireEvent.change(title, { target: { value: "Keep this edited title" } });
    fireEvent.change(screen.getByLabelText("Work Log entry"), { target: { value: "Independent evidence" } });
    fireEvent.click(document.querySelector('[data-control="task-worklog-add"]')!);
    await waitFor(() => expect(screen.getByLabelText("Title")).toHaveValue("Keep this edited title"));
    expect(screen.getByText("Independent evidence")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await waitFor(() => {
      const revise = request.mock.calls.map(([input]) => input).find((input) => input.path === "/tasks/task-1/revisions");
      expect(JSON.parse(revise.body)).toMatchObject({ expectedTaskRevision: 3, patch: { title: "Keep this edited title" } });
    });
  });

  it("keeps the panel open when a dirty close is cancelled", async () => {
    const onClose = vi.fn();
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(task)) };
    render(<TaskDetail taskId="task-1" onClose={onClose} onChanged={vi.fn()} />);
    fireEvent.click(await screen.findByRole("tab", { name: "Details" }));
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    fireEvent.change(await screen.findByLabelText("Title"), { target: { value: "Unsaved title" } });
    fireEvent.click(screen.getByRole("button", { name: "Close Task detail" }));
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Title")).toHaveValue("Unsaved title");
  });
});

const control = (id: string) => {
  if (id.startsWith("task-revision-")) {
    const details = document.querySelector('[data-control="task-detail-tab-details"]') as HTMLElement | null;
    if (details) fireEvent.click(details);
    const edit = document.querySelector('[data-control="task-definition-edit"]') as HTMLElement | null;
    if (edit) fireEvent.click(edit);
  }
  return document.querySelector<HTMLElement>(`[data-control="${id}"]`)!;
};
describe("Task draft protection", () => {
  it("keeps all definition inputs through independent Work Log and comment refresh", async () => {
    let current = { ...task, detail: "Original", workLog: [{ id: "log", body: "Existing", comments: [] }] };
    const request = vi.fn().mockImplementation(({ method }: { method?: string }) => {
      if (method === "POST") {
        current = { ...current, workLog: [...current.workLog, { id: `added-${current.workLog.length}`, body: "Added", comments: [] }] };
        return Promise.resolve(response({ id: "new-entry" }));
      }
      return Promise.resolve(response(current));
    });
    window.llmWikiApplication = { request };
    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);
    await screen.findByText(task.title);
    for (const name of ["title", "detail", "outcome", "scope", "non-goals", "criteria"]) {
      fireEvent.change(control(`task-revision-${name}`), { target: { value: `Edited ${name}` } });
    }
    fireEvent.change(control("task-worklog-text"), { target: { value: "Add work" } });
    fireEvent.click(control("task-worklog-add"));
    await waitFor(() => expect(document.querySelector(".task-detail")).toHaveAttribute("aria-busy", "false"));
    fireEvent.change(control("task-comment-text"), { target: { value: "Comment" } });
    fireEvent.click(control("task-comment-add"));
    await waitFor(() => expect(request.mock.calls.some(([arg]) => arg.path === "/work-log/log/comments")).toBe(true));
    for (const name of ["title", "detail", "outcome", "scope", "non-goals", "criteria"]) expect(control(`task-revision-${name}`)).toHaveValue(`Edited ${name}`);
    expect(document.querySelector(".task-draft-conflict")).not.toBeInTheDocument();
  });

  it("uses the original revision, retains failed input, then requires explicit overlapping rebase", async () => {
    let current = { ...task, detail: "Original" };
    let conflict = true;
    const request = vi.fn().mockImplementation(({ path, method, body }: { path: string; method?: string; body?: string }) => {
      if (path.endsWith("/revisions") && method === "POST") {
        if (conflict) {
          current = { ...current, taskRevision: 3, detail: "External" };
          return Promise.resolve({ ...response({ detail: "head_conflict: currentRevision=3" }), ok: false, status: 409 });
        }
        const patch = JSON.parse(body!).patch;
        current = { ...current, ...patch, taskRevision: 4 };
        return Promise.resolve(response({ id: task.id, taskRevision: 4, title: current.title, detail: current.detail, outcome: "", scope: "", nonGoals: "", validationCriteria: "" }));
      }
      return Promise.resolve(response(current));
    });
    const close = vi.fn();
    window.llmWikiApplication = { request };
    render(<TaskDetail taskId={task.id} onClose={close} onChanged={vi.fn()} />);
    await screen.findByText(task.title);
    fireEvent.change(control("task-revision-detail"), { target: { value: "Local" } });
    fireEvent.click(control("task-detail-close"));
    fireEvent.click(control("task-draft-guard-save"));
    await screen.findByText("External");
    expect(close).not.toHaveBeenCalled();
    expect(control("task-revision-detail")).toHaveValue("Local");
    expect(JSON.parse(request.mock.calls.find(([arg]) => arg.path.endsWith("/revisions"))![0].body).expectedTaskRevision).toBe(2);
    expect(control("task-draft-guard-save")).toBeDisabled();
    fireEvent.click(control("task-draft-keep-mine"));
    conflict = false;
    fireEvent.click(control("task-draft-guard-save"));
    await waitFor(() => expect(close).toHaveBeenCalledOnce());
    const writes = request.mock.calls.filter(([arg]) => arg.path.endsWith("/revisions"));
    expect(JSON.parse(writes[1][0].body).expectedTaskRevision).toBe(3);
  });

  it("saves normalized response fields and retains aggregate state when the follow-up GET fails", async () => {
    let saved = false;
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => {
      if (path.endsWith("/revisions")) {
        saved = true;
        return Promise.resolve(response({ id: task.id, taskRevision: 3, title: task.title, detail: "Normalized", outcome: "", scope: "", nonGoals: "", validationCriteria: "" }));
      }
      if (saved) return Promise.reject(new Error("Offline"));
      return Promise.resolve(response(task));
    }) };
    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);
    await screen.findByText(task.title);
    fireEvent.change(control("task-revision-detail"), { target: { value: "  Normalized  " } });
    fireEvent.click(control("task-revision-save"));
    await screen.findByText(/Changes saved, but/);
    expect(control("task-revision-detail")).toHaveValue("Normalized");
    expect(document.querySelector(".task-detail")).toHaveAttribute("data-task-state", "in_progress");
    expect(control("task-revision-save")).toBeDisabled();
  });

  it("keeps editing on Escape prompt cancellation and closes only after discard", async () => {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(task)) };
    const close = vi.fn();
    render(<TaskDetail taskId={task.id} onClose={close} onChanged={vi.fn()} />);
    await screen.findByText(task.title);
    fireEvent.change(control("task-revision-detail"), { target: { value: "Uncommitted" } });
    fireEvent.keyDown(control("task-revision-detail"), { key: "Escape" });
    fireEvent.click(control("task-draft-guard-keep-editing"));
    expect(close).not.toHaveBeenCalled();
    expect(control("task-revision-detail")).toHaveValue("Uncommitted");
    fireEvent.click(control("task-detail-close"));
    fireEvent.click(control("task-draft-guard-discard"));
    expect(close).toHaveBeenCalledOnce();
  });
});

it("localizes calculated readiness text but preserves authored reasons", async () => {
  document.documentElement.lang = "ko";
  try {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response({ ...task, readinessEntries: [
      { key: "prerequisites", status: "resolved", reason: "No explicit prerequisite", provenance: "calculated" },
      { key: "scope", status: "missing", reason: "User-authored rationale" },
    ] })) };
    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);
    fireEvent.click(await screen.findByRole("tab", { name: "검토" }));
    await screen.findByText("명시된 선행 조건 없음");
    expect(screen.getByText("User-authored rationale")).toBeVisible();
    expect(screen.queryByText("No explicit prerequisite")).not.toBeInTheDocument();
    expect(screen.getByLabelText("범위 사유")).toBeInTheDocument();
  } finally { document.documentElement.lang = "en"; }
});

it("rebases a disjoint refresh before save and lets the user discard overlapping values", async () => {
  let current = { ...task, detail: "Original" };
  let overlap = false;
  const request = vi.fn().mockImplementation(({ path, method, body }: { path: string; method?: string; body?: string }) => {
    if (path.endsWith("/work-log") && method === "POST") {
      current = { ...current, taskRevision: current.taskRevision + 1, title: "External title", detail: overlap ? "External detail" : current.detail };
      return Promise.resolve(response({ id: "entry" }));
    }
    if (path.endsWith("/revisions")) {
      current = { ...current, ...JSON.parse(body!).patch, taskRevision: current.taskRevision + 1 };
      return Promise.resolve(response({ ...current, outcome: "", scope: "", nonGoals: "", validationCriteria: "" }));
    }
    return Promise.resolve(response(current));
  });
  window.llmWikiApplication = { request };
  render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);
  await screen.findByText(task.title);
  fireEvent.change(control("task-revision-detail"), { target: { value: "Local" } });
  fireEvent.change(control("task-worklog-text"), { target: { value: "Evidence" } });
  fireEvent.click(control("task-worklog-add"));
  await screen.findAllByText("External title");
  await waitFor(() => expect(document.querySelector(".task-detail")).toHaveAttribute("aria-busy", "false"));
  fireEvent.click(control("task-revision-save"));
  await waitFor(() => expect(control("task-revision-save")).toBeDisabled());
  expect(JSON.parse(request.mock.calls.find(([arg]) => arg.path.endsWith("/revisions"))![0].body)).toMatchObject({ expectedTaskRevision: 3, patch: { detail: "Local" } });
  await waitFor(() => expect(document.querySelector(".task-detail")).toHaveAttribute("aria-busy", "false"));
  overlap = true;
  fireEvent.change(control("task-revision-detail"), { target: { value: "Another local" } });
  fireEvent.change(control("task-worklog-text"), { target: { value: "More evidence" } });
  fireEvent.click(control("task-worklog-add"));
  await screen.findByText("External detail");
  await waitFor(() => expect(document.querySelector(".task-detail")).toHaveAttribute("aria-busy", "false"));
  fireEvent.click(control("task-draft-use-latest"));
  expect(control("task-revision-detail")).toHaveValue("External detail");
  expect(control("task-revision-save")).toBeDisabled();
});

it("keeps Knowledge publication bound to the persisted source hash", async () => {
    const knowledgeTask = {
      ...task,
      state: "completed" as const,
      publication: {
        state: "published",
        draftRevision: 3,
        contentHash: "body-3",
        sourceHash: "source-2",
      },
    };
    const request = vi.fn().mockResolvedValue(response(knowledgeTask));
    window.llmWikiApplication = { request };
    render(<TaskDetail taskId="task-1" onClose={vi.fn()} onChanged={vi.fn()} />);

    await screen.findByText("Publish approved draft");
    fireEvent.click(document.querySelector('[data-control="task-knowledge-publish"]')!);
    await waitFor(() => {
      const publish = request.mock.calls
        .map(([input]) => input)
        .find((input) => input.path.endsWith("/publish"));
      expect(JSON.parse(publish.body)).toMatchObject({
        expectedContentHash: "body-3",
        expectedSourceHash: "source-2",
      });
    });
  });

describe("Task detail tabs and workbench sessions", () => {
  it.each([
    ["task", "Details"],
    ["completed", "Review"],
  ] as const)("opens %s Tasks on the %s tab", async (state, tabName) => {
    window.llmWikiApplication = {
      request: vi.fn().mockResolvedValue(response({ ...task, state })),
    };

    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);

    await waitFor(() =>
      expect(screen.getByRole("tab", { name: tabName })).toHaveAttribute("aria-selected", "true"),
    );
  });

  it("shows definition prose first and only exposes fields while editing", async () => {
    window.llmWikiApplication = {
      request: vi.fn().mockResolvedValue(response({
        ...task,
        state: "task",
        detail: "A concise definition for the work.",
        outcome: "A usable result.",
      })),
    };

    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);

    await screen.findByText("A concise definition for the work.");
    expect(screen.queryByLabelText("Title")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Detail")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    expect(screen.getByLabelText("Title")).toHaveValue(task.title);
    expect(screen.getByLabelText("Detail")).toHaveValue("A concise definition for the work.");
  });

  it("retains the chosen tab after a readiness mutation refreshes the Task", async () => {
    const request = vi.fn().mockResolvedValue(response(task));
    window.llmWikiApplication = { request };
    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);

    fireEvent.click(await screen.findByRole("tab", { name: "Review" }));
    const reason = await screen.findByLabelText("Scope Reason");
    fireEvent.change(reason, { target: { value: "Not relevant to this discovery task" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Not applicable" })[0]);

    await waitFor(() =>
      expect(request).toHaveBeenCalledWith(expect.objectContaining({ method: "POST" })),
    );
    await waitFor(() =>
      expect(screen.getByRole("tab", { name: "Review" })).toHaveAttribute("aria-selected", "true"),
    );
  });

  it("restores an unsent work entry and attachment when the same Task remounts", async () => {
    const sessions = new Map<string, DetailSession>();
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(task)) };
    const first = render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} sessions={sessions} />);
    const entry = await screen.findByLabelText("Work Log entry");
    const attachment = new File(["evidence"], "evidence.txt", { type: "text/plain" });
    fireEvent.change(entry, { target: { value: "Unsent evidence" } });
    fireEvent.change(screen.getByLabelText("Attach image or file"), { target: { files: [attachment] } });
    await screen.findByText("Attach image or file: evidence.txt");
    await waitFor(() => expect(sessions.get(task.id)?.entry).toBe("Unsent evidence"));
    first.unmount();

    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} sessions={sessions} />);
    expect(await screen.findByLabelText("Work Log entry")).toHaveValue("Unsent evidence");
    expect(screen.getByText("Attach image or file: evidence.txt")).toBeVisible();
  });

  it("keeps workbench session drafts isolated by Task", async () => {
    const sessions = new Map<string, DetailSession>();
    const secondTask = { ...task, id: "task-2", title: "Separate work" };
    window.llmWikiApplication = {
      request: vi.fn().mockImplementation(({ path }: { path: string }) =>
        Promise.resolve(response(path.endsWith("task-2") ? secondTask : task)),
      ),
    };
    const first = render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} sessions={sessions} />);
    fireEvent.change(await screen.findByLabelText("Work Log entry"), { target: { value: "Task one draft" } });
    await waitFor(() => expect(sessions.get(task.id)?.entry).toBe("Task one draft"));
    first.unmount();

    render(<TaskDetail taskId={secondTask.id} onClose={vi.fn()} onChanged={vi.fn()} sessions={sessions} />);
    expect(await screen.findByLabelText("Work Log entry")).toHaveValue("");
    expect(screen.queryByText("Task one draft")).not.toBeInTheDocument();
  });

  it("discards a definition draft before closing so reopening starts clean", async () => {
    const sessions = new Map<string, DetailSession>();
    const close = vi.fn();
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response({ ...task, detail: "Original detail" })) };
    const first = render(<TaskDetail taskId={task.id} onClose={close} onChanged={vi.fn()} sessions={sessions} />);
    fireEvent.click(await screen.findByRole("tab", { name: "Details" }));
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    fireEvent.change(screen.getByLabelText("Detail"), { target: { value: "Discard this edit" } });
    fireEvent.click(screen.getByRole("button", { name: "Close Task detail" }));
    fireEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(close).toHaveBeenCalledOnce();
    first.unmount();

    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} sessions={sessions} />);
    await screen.findByText("Original detail");
    expect(screen.queryByLabelText("Detail")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    expect(screen.getByLabelText("Detail")).toHaveValue("Original detail");
  });

  it("moves the tab selection with arrow, Home, and End keys", async () => {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(task)) };
    render(<TaskDetail taskId={task.id} onClose={vi.fn()} onChanged={vi.fn()} />);

    const work = await screen.findByRole("tab", { name: "Work" });
    fireEvent.keyDown(work, { key: "ArrowRight" });
    await waitFor(() => expect(screen.getByRole("tab", { name: "Details" })).toHaveAttribute("aria-selected", "true"));
    fireEvent.keyDown(screen.getByRole("tab", { name: "Details" }), { key: "End" });
    await waitFor(() => expect(screen.getByRole("tab", { name: "Review" })).toHaveAttribute("aria-selected", "true"));
    fireEvent.keyDown(screen.getByRole("tab", { name: "Review" }), { key: "Home" });
    await waitFor(() => expect(screen.getByRole("tab", { name: "Work" })).toHaveAttribute("aria-selected", "true"));
    fireEvent.keyDown(screen.getByRole("tab", { name: "Work" }), { key: "ArrowLeft" });
    await waitFor(() => expect(screen.getByRole("tab", { name: "Review" })).toHaveAttribute("aria-selected", "true"));
  });
});
