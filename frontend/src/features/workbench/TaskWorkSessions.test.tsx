import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Profiler } from "react";
import { TaskWorkSessions, type WorkSessionDraft } from "./TaskWorkSessions";
import type { TaskAggregate, TaskWorkSession, TaskExecutionSnapshot } from "../../types/taskWorkbench";

const execution = vi.hoisted(() => ({
  prepare: vi.fn(),
  subscribe: vi.fn().mockResolvedValue({ runs: [], activeRunId: null, selectedRun: null }),
  execute: vi.fn(), interrupt: vi.fn(), respond: vi.fn(), syncWorkLog: vi.fn(),
  externalThreads: vi.fn().mockResolvedValue({ threads: [] }),
  externalThread: vi.fn(), linkExternalThread: vi.fn(),
}));
vi.mock("../../services/taskExecutionClient", () => ({ taskExecutionClient: execution }));

const task = {
  id: "task-a",
  kind: "task",
  taskRevision: 3,
  state: "in_progress",
  title: "Session work",
  detail: "Canonical detail",
  outcome: "Ship safely",
  problemLinks: [
    { id: "l", problemId: "p1", problemRevision: 2, relationship: "context" },
  ],
} as TaskAggregate;
const session = (id: string, title: string): TaskWorkSession => ({
  id,
  taskId: task.id,
  title,
  provider: "codex",
  model: "gpt-5.6-sol",
  approvalMode: "ask",
  workspacePath: "",
  createdAt: "2026-09-21T10:00:00Z",
  updatedAt: "2026-09-21T10:00:00Z",
});
const response = (body: unknown, ok = true) => ({
  ok,
  status: ok ? 200 : 500,
  json: async () => body,
  text: async () => "",
  body: null,
});

describe("Task work sessions", () => {
  beforeEach(() => {
    execution.subscribe.mockReset().mockResolvedValue({ runs: [], activeRunId: null, selectedRun: null });
    execution.externalThreads.mockReset().mockResolvedValue({ threads: [] });
    execution.externalThread.mockReset().mockRejectedValue(new Error("not linked"));
    execution.linkExternalThread.mockReset();
  });

  it("keeps imported activity in provider order and de-duplicates turns already saved as local Runs", async () => {
    const saved = { ...session("linked", "Imported work"), workspacePath: "/project" };
    const localRun = { id: "run-local", taskId: task.id, sessionId: "linked", instruction: "Local prompt", status: "succeeded" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", threadId: "thread-one", turnId: "turn-local", startedAt: "2026-09-22T11:00:00Z", evidence: [], formalRequests: [], workLogSyncState: "synced" as const, revision: 1, finalReport: "Local answer" };
    execution.subscribe.mockResolvedValue({ runs: [localRun], activeRunId: null, selectedRun: localRun });
    execution.externalThread.mockResolvedValue({
      thread: { id: "thread-one", title: "Imported work", preview: "Earlier work", cwd: "/project", model: "gpt-5.6-sol", source: "vscode", status: "idle", createdAt: "2026-09-20T10:00:00Z", updatedAt: "2026-09-22T10:00:00Z", linkedTaskId: task.id, linkedSessionId: "linked" },
      turns: [
        { id: "turn-old", status: "completed", createdAt: "2026-09-22T10:00:00Z", messages: [], activity: [], items: [
          { type: "message", id: "old-user", role: "user", body: "Old prompt", order: 0 },
          { type: "activity", id: "old-command", kind: "commandExecution", label: "check", status: "completed", command: "npm test", output: "passed", exitCode: 0, order: 1 },
          { type: "message", id: "old-answer", role: "assistant", body: "Old answer", order: 2 },
        ] },
        { id: "turn-local", status: "completed", createdAt: "2026-09-22T11:00:00Z", messages: [{ id: "duplicate", role: "assistant", body: "Duplicate local answer" }], activity: [], items: [{ type: "message", id: "duplicate", role: "assistant", body: "Duplicate local answer", order: 0 }] },
        { id: "turn-after", status: "completed", createdAt: "2026-09-22T12:00:00Z", messages: [{ id: "after-user", role: "user", body: "VS Code follow-up" }], activity: [], items: [{ type: "message", id: "after-user", role: "user", body: "VS Code follow-up", order: 0 }] },
      ],
    });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };

    render(<TaskWorkSessions task={task} />);
    const oldPrompt = await screen.findByText("Old prompt");
    const command = screen.getByText("Run command");
    const oldAnswer = screen.getByText("Old answer");
    expect(oldPrompt.compareDocumentPosition(command) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(command.compareDocumentPosition(oldAnswer) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    await waitFor(() => expect(screen.queryByText("Duplicate local answer")).not.toBeInTheDocument());
    const localPrompt = screen.getByText("Local prompt");
    const laterPrompt = screen.getByText("VS Code follow-up");
    expect(oldAnswer.compareDocumentPosition(localPrompt) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(localPrompt.compareDocumentPosition(laterPrompt) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.getByText("Local answer")).toBeVisible();
  });

  it("keeps the session visible when legacy file-change paths contain provider objects", async () => {
    const saved = { ...session("one", "Legacy file change"), workspacePath: "/project" };
    const run = {
      id: "run-file-change", taskId: task.id, sessionId: "one", instruction: "Update the file",
      status: "succeeded" as const, stopRequested: false, provider: "codex" as const,
      model: "gpt-5.6-sol", workspacePath: "/project", formalRequests: [],
      workLogSyncState: "synced" as const, revision: 1,
      evidence: [{
        id: "change", kind: "fileChange", label: "fileChange", status: "completed",
        summary: "Updated one file",
        paths: [{ kind: { type: "update" }, path: "/project/src/main.ts" }] as unknown as string[],
        exitCode: null as unknown as number,
      }],
    };
    execution.subscribe.mockResolvedValue({ runs: [run], activeRunId: null, selectedRun: run });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };

    render(<TaskWorkSessions task={task} />);

    expect(await screen.findByRole("heading", { name: "Legacy file change" })).toBeVisible();
    expect(await screen.findByText("Updated one file")).toBeInTheDocument();
    expect(screen.getByText("completed")).toBeInTheDocument();
    expect(screen.queryByText("exit null")).not.toBeInTheDocument();
    expect(screen.queryByText("/project/src/main.ts")).not.toBeInTheDocument();
  });

  it("follows new output only while the conversation is near the bottom", async () => {
    const saved = { ...session("one", "Follow output"), workspacePath: "/project" };
    let deliver: (snapshot: TaskExecutionSnapshot) => void = () => {};
    execution.subscribe.mockImplementation((_input, onSnapshot) => {
      deliver = onSnapshot;
      return Promise.resolve({ runs: [], activeRunId: null, selectedRun: null });
    });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    const view = render(<TaskWorkSessions task={task} />);
    await screen.findByRole("heading", { name: "Follow output" });
    const timeline = view.container.querySelector<HTMLOListElement>(".task-session-entries")!;
    Object.defineProperty(timeline, "scrollHeight", { configurable: true, value: 1000 });
    Object.defineProperty(timeline, "clientHeight", { configurable: true, value: 300 });
    timeline.scrollTop = 690;
    fireEvent.scroll(timeline);
    const running = { id: "run-live", taskId: task.id, sessionId: "one", instruction: "Watch", status: "running" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], formalRequests: [], workLogSyncState: "synced" as const, revision: 1, liveStatus: { kind: "agentMessage", status: "running" as const, text: "First update" } };
    await act(async () => deliver({ revision: 1, runs: [running], activeRunId: running.id, selectedRun: running }));
    expect(timeline.scrollTop).toBe(1000);

    timeline.scrollTop = 200;
    fireEvent.scroll(timeline);
    await act(async () => deliver({ revision: 2, runs: [{ ...running, revision: 2, liveStatus: { ...running.liveStatus, text: "Second update" } }], activeRunId: running.id, selectedRun: running }));
    expect(timeline.scrollTop).toBe(200);
  });

  it("links an existing VS Code session to an empty Task and renders its readable transcript", async () => {
    const thread = { id: "thread-one", title: "Fix sync", preview: "Trace the failing sync", cwd: "/project", model: "gpt-5.6-sol", source: "vscode", status: "idle", createdAt: "2026-09-21T10:00:00Z", updatedAt: "2026-09-22T10:00:00Z" };
    const created = { ...session("linked", "Fix sync"), workspacePath: "/project" };
    const transcript = { thread: { ...thread, linkedTaskId: task.id, linkedSessionId: "linked" }, turns: [{ id: "turn-one", status: "completed", messages: [{ id: "user-one", role: "user", body: "Run the checks" }, { id: "assistant-one", role: "assistant", body: "## Result\n\nThe check **passed**." }], activity: [{ id: "command-one", kind: "commandExecution", label: "npm test", status: "completed", command: "npm test", output: "12 passed", exitCode: 0 }] }] };
    let linked = false;
    execution.externalThreads.mockResolvedValueOnce({ threads: [thread] }).mockResolvedValue({ threads: [{ ...thread, linkedTaskId: task.id, linkedSessionId: "linked" }] });
    execution.linkExternalThread.mockImplementation(async () => {
      linked = true;
      return { ...transcript, taskId: task.id, sessionId: "linked", threadId: thread.id, linked: true };
    });
    execution.externalThread.mockResolvedValue(transcript);
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => {
      if (path.endsWith("work-sessions")) return Promise.resolve(response({ sessions: linked ? [created] : [] }));
      return Promise.resolve(response({ session: created, entries: [] }));
    }) };

    render(<TaskWorkSessions task={task} />);
    expect(await screen.findByText("Create a session when you are ready to record focused work.")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Link Codex session" }));
    fireEvent.click(await screen.findByRole("button", { name: "Link to Task" }));

    await waitFor(() => expect(execution.linkExternalThread).toHaveBeenCalledWith({ taskId: "task-a", sessionId: undefined, threadId: "thread-one" }));
    expect(await screen.findByRole("heading", { name: "Result" })).toBeVisible();
    expect(screen.getByText(/12 passed/)).not.toBeVisible();
    fireEvent.click(screen.getByText("Run command"));
    expect(screen.getByText(/12 passed/)).toBeVisible();
  });

  it("starts Codex only from the explicit Run action and leaves an unsupported attachment unsent", async () => {
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const prepared = { runs: [], activeRunId: null, selectedRun: null, effectiveConfig: { model: "gpt-5.6-sol", cwd: "/project", approvalPolicy: "ask", approvalsReviewer: "user", sandbox: "workspace-write", provenance: "preflight", settingsRevision: "settings-1", ready: true, capabilities: { structuredUserInput: true } } };
    execution.prepare.mockResolvedValue(prepared);
    execution.execute.mockResolvedValue(prepared);
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) =>
      path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] })),
    ) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    fireEvent.change(await screen.findByLabelText("Instruction or note"), { target: { value: "Run the focused check" } });
    fireEvent.click(screen.getByRole("button", { name: "Prepare conversation" }));
    await waitFor(() => expect(execution.prepare).toHaveBeenCalledWith({ taskId: "task-a", sessionId: "one" }, expect.any(Function)));
    fireEvent.click(screen.getByRole("button", { name: "Run with Codex" }));
    await waitFor(() => expect(execution.execute).toHaveBeenCalledWith(expect.objectContaining({ taskId: "task-a", sessionId: "one", instruction: "Run the focused check", settingsRevision: "settings-1" }), expect.any(Function)));
    expect(window.llmWikiApplication.request).not.toHaveBeenCalledWith(expect.objectContaining({ path: expect.stringContaining("/entries") }));
  });

  it("runs on Enter while preserving Shift+Enter and Korean IME composition", async () => {
    execution.execute.mockClear();
    const saved = { ...session("one", "Keyboard run"), workspacePath: "/project" };
    const prepared = { runs: [], activeRunId: null, selectedRun: null, effectiveConfig: { model: "gpt-5.6-sol", cwd: "/project", approvalPolicy: "ask", approvalsReviewer: "user", sandbox: "workspace-write", provenance: "preflight", settingsRevision: "settings-1", ready: true, capabilities: { structuredUserInput: true } } };
    execution.prepare.mockResolvedValue(prepared);
    execution.execute.mockResolvedValue(prepared);
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    const composer = await screen.findByLabelText("Instruction or note");
    fireEvent.click(screen.getByRole("button", { name: "Prepare conversation" }));
    await waitFor(() => expect(execution.prepare).toHaveBeenCalled());

    fireEvent.change(composer, { target: { value: "한국어 입력" } });
    expect(fireEvent.keyDown(composer, { key: "Enter", isComposing: true })).toBe(true);
    expect(fireEvent.keyDown(composer, { key: "Enter", keyCode: 229 })).toBe(true);
    expect(fireEvent.keyDown(composer, { key: "Enter", shiftKey: true })).toBe(true);
    expect(composer).toHaveValue("한국어 입력");
    expect(execution.execute).not.toHaveBeenCalled();

    expect(fireEvent.keyDown(composer, { key: "Enter" })).toBe(false);
    await waitFor(() => expect(execution.execute).toHaveBeenCalledWith(expect.objectContaining({
      taskId: "task-a",
      sessionId: "one",
      instruction: "한국어 입력",
    }), expect.any(Function)));
    expect(execution.execute).toHaveBeenCalledTimes(1);
  });

  it("renders stored Codex prose as Markdown once and keeps command logs preformatted", async () => {
    const saved = { ...session("one", "Rendered output"), workspacePath: "/project" };
    const finalReport = "## Final result\n\nThe task is **done**.";
    const run = {
      id: "run-markdown", taskId: task.id, sessionId: "one", instruction: "Render output",
      status: "succeeded" as const, stopRequested: false, provider: "codex" as const,
      model: "gpt-5.6-sol", workspacePath: "/project", formalRequests: [],
      workLogSyncState: "synced" as const, revision: 1, finalReport,
      evidence: [
        { id: "progress", kind: "agentMessage", label: "response", status: "completed", summary: "### Progress\n\nStill **working**." },
        { id: "duplicate-final", kind: "agentMessage", label: "response", status: "completed", summary: finalReport },
        { id: "command", kind: "commandExecution", label: "command", status: "completed", command: "npm test", summary: "## Not Markdown\n\n**raw output**", exitCode: 0 },
      ],
    };
    execution.subscribe.mockResolvedValue({ runs: [run], activeRunId: null, selectedRun: run });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });

    expect(await screen.findByRole("heading", { name: "Progress" })).toBeVisible();
    expect(screen.getByText("working").tagName).toBe("STRONG");
    expect(screen.getAllByRole("heading", { name: "Final result" })).toHaveLength(1);
    expect(screen.getByText("done").tagName).toBe("STRONG");
    expect(screen.queryByRole("heading", { name: "Not Markdown" })).not.toBeInTheDocument();
    const commandLog = screen.getByText("Run command").closest("details")?.querySelector("pre");
    expect(commandLog).toHaveTextContent("npm test");
    expect(commandLog).toHaveTextContent("## Not Markdown");
    expect(commandLog).toHaveTextContent("**raw output**");
  });

  it("de-duplicates a final Codex message whose evidence summary was truncated", async () => {
    const saved = { ...session("one", "Long output"), workspacePath: "/project" };
    const finalReport = `## Long final\n\n${"a".repeat(2100)}`;
    const run = {
      id: "run-long", taskId: task.id, sessionId: "one", instruction: "Render long output",
      status: "succeeded" as const, stopRequested: false, provider: "codex" as const,
      model: "gpt-5.6-sol", workspacePath: "/project", formalRequests: [],
      workLogSyncState: "synced" as const, revision: 1, finalReport,
      evidence: [{ id: "truncated-final", kind: "agentMessage", label: "response", status: "completed", summary: `${finalReport.slice(0, 2000)}…` }],
    };
    execution.subscribe.mockResolvedValue({ runs: [run], activeRunId: null, selectedRun: run });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });

    expect(await screen.findAllByRole("heading", { name: "Long final" })).toHaveLength(1);
  });

  it("passes an attached image to the explicit Codex Run", async () => {
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const prepared = { runs: [], activeRunId: null, selectedRun: null, effectiveConfig: { model: "gpt-5.6-sol", cwd: "/project", approvalPolicy: "ask", approvalsReviewer: "user", sandbox: "workspace-write", provenance: "preflight", settingsRevision: "settings-1", ready: true, capabilities: { structuredUserInput: true } } };
    execution.prepare.mockResolvedValue(prepared);
    execution.execute.mockResolvedValue(prepared);
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    fireEvent.change(await screen.findByLabelText("Instruction or note"), { target: { value: "Inspect this screenshot" } });
    fireEvent.change(document.querySelector<HTMLInputElement>('[data-control="task-session-attachment"]')!, { target: { files: [new File(["png"], "screen.png", { type: "image/png" })] } });
    await waitFor(() => expect(screen.getByAltText("screen.png")).toBeVisible());
    fireEvent.click(screen.getByRole("button", { name: "Prepare conversation" }));
    await waitFor(() => expect(execution.prepare).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "Run with Codex" }));
    await waitFor(() => expect(execution.execute).toHaveBeenCalledWith(expect.objectContaining({ attachment: { name: "screen.png", mediaType: "image/png", data: "cG5n" } }), expect.any(Function)));
  });

  it("submits the actual selected structured answer once and retains it after an error", async () => {
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const snapshot = { activeRunId: "run-1", selectedRun: { id: "run-1", taskId: task.id, sessionId: "one", instruction: "Ask", status: "awaiting_response", stopRequested: false, provider: "codex", model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], workLogSyncState: "synced", revision: 1, formalRequests: [{ id: "request-1", kind: "user_input", status: "pending", isBlocking: true, title: "Choose", prompt: "Pick one", choices: [], questions: [{ id: "question-1", header: "Target", prompt: "Which?", options: [{ value: "safe", label: "Safe" }], allowOther: false, isSecret: false }] }] }, runs: [], effectiveConfig: undefined };
    execution.subscribe.mockResolvedValue(snapshot);
    execution.respond.mockRejectedValueOnce(new Error("offline"));
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    const draftStore = new Map<string, WorkSessionDraft>();
    const view = render(<TaskWorkSessions task={task} draftStore={draftStore} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    await screen.findByText("Pick one");
    fireEvent.click(screen.getByRole("button", { name: "Safe" }));
    fireEvent.click(screen.getByRole("button", { name: "Submit answers" }));
    await waitFor(() => expect(execution.respond).toHaveBeenCalledWith(expect.objectContaining({ requestId: "request-1", response: { answers: { "question-1": { answers: ["safe"] } } } })));
    expect(await screen.findByRole("alert")).toHaveTextContent("offline");
    expect(screen.getByRole("button", { name: "Safe" })).toHaveAttribute("aria-pressed", "true");
    fireEvent.change(screen.getByLabelText("Instruction or note"), { target: { value: "separate note" } });
    expect(draftStore.get("one")?.formalAnswers).toEqual({ "run-1/request-1/question-1": ["safe"] });
    view.unmount();
    render(<TaskWorkSessions task={task} draftStore={draftStore} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    expect(await screen.findByRole("button", { name: "Safe" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByLabelText("Instruction or note")).toHaveValue("separate note");
  });
  it("keeps current Run controls pinned when a historical Run is selected", async () => {
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const historical = { id: "run-old", taskId: task.id, sessionId: "one", instruction: "old work", status: "succeeded" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], formalRequests: [], workLogSyncState: "synced" as const, revision: 1, finalReport: "Old result" };
    const current = { id: "run-current", taskId: task.id, sessionId: "one", instruction: "current work", status: "awaiting_response" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], workLogSyncState: "synced" as const, revision: 2, formalRequests: [{ id: "approval-current", kind: "command_approval" as const, status: "pending" as const, isBlocking: true, title: "Approve command", prompt: "Run it?", choices: [{ value: "approve", label: "Proceed" }], questions: [] }] };
    const snapshot = { runs: [current, historical], activeRunId: current.id, selectedRun: current };
    execution.subscribe.mockResolvedValue(snapshot);
    execution.respond.mockResolvedValue(snapshot);
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };

    render(<TaskWorkSessions task={task} />);
    fireEvent.click(await screen.findByRole("button", { name: /Finished · old work/ }));
    expect(screen.getByRole("button", { name: "Stop" })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Proceed" }));
    await waitFor(() => expect(execution.respond).toHaveBeenCalledWith(expect.objectContaining({ runId: "run-current", requestId: "approval-current" })));
  });
  it("keeps completed Runs selectable and makes retry load an instruction without dispatching it", async () => {
    execution.execute.mockClear();
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const run = (id: string, status: "succeeded" | "failed", instruction: string) => ({ id, taskId: task.id, sessionId: "one", instruction, status, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], formalRequests: [], workLogSyncState: "synced" as const, revision: 1, finalReport: status === "succeeded" ? "Done" : undefined });
    execution.subscribe.mockResolvedValue({ activeRunId: null, selectedRun: null, runs: [run("new", "succeeded", "new result"), run("old", "failed", "retry this")], effectiveConfig: undefined });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    expect(await screen.findByText("Run history")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: /Failed · retry this/ }));
    expect(await screen.findByRole("button", { name: "Use instruction again" })).toBeVisible();
    fireEvent.change(screen.getByLabelText("Instruction or note"), { target: { value: "unrelated draft" } });
    expect(screen.getByRole("button", { name: "Use instruction again" })).toBeDisabled();
    expect(screen.getByLabelText("Instruction or note")).toHaveValue("unrelated draft");
    fireEvent.change(screen.getByLabelText("Instruction or note"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Use instruction again" }));
    expect(screen.getByLabelText("Instruction or note")).toHaveValue("retry this");
    expect(execution.execute).not.toHaveBeenCalled();
  });
  it("keeps a newer composer draft when an earlier Run submission resolves", async () => {
    execution.execute.mockClear();
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const prepared = { runs: [], activeRunId: null, selectedRun: null, effectiveConfig: { model: "gpt-5.6-sol", cwd: "/project", approvalPolicy: "ask", approvalsReviewer: "user", sandbox: "workspace-write", provenance: "preflight", settingsRevision: "settings-1", ready: true, capabilities: { structuredUserInput: true } } };
    let resolveRun!: (value: typeof prepared) => void;
    execution.prepare.mockResolvedValue(prepared);
    execution.execute.mockReturnValue(new Promise((resolve) => { resolveRun = resolve; }));
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    fireEvent.change(await screen.findByLabelText("Instruction or note"), { target: { value: "submitted" } });
    fireEvent.click(screen.getByRole("button", { name: "Prepare conversation" }));
    await waitFor(() => expect(execution.prepare).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "Run with Codex" }));
    fireEvent.change(screen.getByLabelText("Instruction or note"), { target: { value: "newer draft" } });
    resolveRun(prepared);
    await waitFor(() => expect(screen.getByLabelText("Instruction or note")).toHaveValue("newer draft"));
  });
  it("reuses an execution operation identity after a response-loss retry", async () => {
    execution.execute.mockClear();
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const prepared = { runs: [], activeRunId: null, selectedRun: null, effectiveConfig: { model: "gpt-5.6-sol", cwd: "/project", approvalPolicy: "ask", approvalsReviewer: "user", sandbox: "workspace-write", provenance: "preflight", settingsRevision: "settings-1", ready: true, capabilities: { structuredUserInput: true } } };
    execution.prepare.mockResolvedValue(prepared);
    execution.execute.mockRejectedValue(new Error("response lost"));
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    fireEvent.change(await screen.findByLabelText("Instruction or note"), { target: { value: "retry safely" } });
    fireEvent.click(screen.getByRole("button", { name: "Prepare conversation" }));
    await waitFor(() => expect(execution.prepare).toHaveBeenCalled());
    fireEvent.click(screen.getByRole("button", { name: "Run with Codex" }));
    await screen.findByRole("alert");
    fireEvent.click(screen.getByRole("button", { name: "Run with Codex" }));
    await waitFor(() => expect(execution.execute).toHaveBeenCalledTimes(2));
    expect(execution.execute.mock.calls[0][0].operationId).toBe(execution.execute.mock.calls[1][0].operationId);
  });
  it("repairs a failed Work Log projection without starting another Codex Run", async () => {
    execution.execute.mockClear();
    execution.syncWorkLog.mockClear();
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const failed = { id: "run-1", taskId: task.id, sessionId: "one", instruction: "Run instruction", status: "succeeded" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], formalRequests: [], userEntryId: "linked-note", workLogEntryId: "log-1", workLogSyncState: "failed" as const, workLogSyncError: "projection unavailable", revision: 2 };
    const snapshot = { runs: [failed], activeRunId: null, selectedRun: failed };
    execution.subscribe.mockResolvedValue(snapshot);
    execution.syncWorkLog.mockResolvedValue({ ...snapshot, runs: [{ ...failed, workLogSyncState: "synced" }], selectedRun: { ...failed, workLogSyncState: "synced" } });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [{ id: "linked-note", author: "user", kind: "note", body: "linked note", createdAt: "2026-09-22T00:00:00Z" }, { id: "separate-note", author: "user", kind: "note", body: "keep visible", createdAt: "2026-09-22T00:01:00Z" }] }))) };
    const draftStore = new Map<string, WorkSessionDraft>();
    render(<TaskWorkSessions task={task} draftStore={draftStore} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    expect(await screen.findByText(/Work Log sync failed/)).toBeVisible();
    expect(screen.queryByText("linked note")).not.toBeInTheDocument();
    expect(screen.getByText("keep visible")).toBeVisible();
    fireEvent.change(screen.getByLabelText("Instruction or note"), { target: { value: "keep this draft" } });
    fireEvent.click(screen.getByRole("button", { name: "Retry Work Log sync" }));
    await waitFor(() => expect(execution.syncWorkLog).toHaveBeenCalledWith({ taskId: "task-a", sessionId: "one", runId: "run-1" }));
    expect(execution.execute).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Instruction or note")).toHaveValue("keep this draft");
  });
  it("keeps note work available for a nonblocking formal request", async () => {
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const run = { id: "run-1", taskId: task.id, sessionId: "one", instruction: "continue", status: "running" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], workLogSyncState: "synced" as const, revision: 1, formalRequests: [{ id: "request-1", kind: "user_input" as const, status: "pending" as const, isBlocking: false, title: "Optional choice", prompt: "Choose later", choices: [], questions: [{ id: "question-1", header: "Choice", prompt: "Optional", options: [], allowOther: false, isSecret: false }] }] };
    execution.subscribe.mockResolvedValue({ runs: [run], activeRunId: "run-1", selectedRun: run });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    expect(await screen.findByText("Codex can continue while you answer this request.")).toBeVisible();
    expect(screen.getByLabelText("Instruction or note")).toBeEnabled();
    expect(screen.getByRole("button", { name: "Save note" })).toBeDisabled();
  });
  it("does not submit mixed secret requests and renders stale proposed answers as read-only", async () => {
    execution.respond.mockClear();
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const secret = { id: "secret-run", taskId: task.id, sessionId: "one", instruction: "secret", status: "awaiting_response" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], workLogSyncState: "synced" as const, revision: 1, formalRequests: [{ id: "mixed", kind: "user_input" as const, status: "pending" as const, isBlocking: true, title: "Mixed", prompt: "Need both", choices: [], questions: [{ id: "public", header: "Public", prompt: "Public", options: [], allowOther: false, isSecret: false }, { id: "secret", header: "Secret", prompt: "Secret", options: [], allowOther: false, isSecret: true }] }, { id: "stale", kind: "user_input" as const, status: "stale" as const, isBlocking: false, title: "Stale", prompt: "Old", choices: [], questions: [{ id: "old", header: "Old", prompt: "Old", options: [], allowOther: false, isSecret: false }], proposedResponse: { answers: { old: { answers: ["saved answer"] } } } }] };
    execution.subscribe.mockResolvedValue({ runs: [secret], activeRunId: "secret-run", selectedRun: secret });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    expect(await screen.findByText("This request needs secret handling in Codex and cannot be answered here.")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Submit answers" })).not.toBeInTheDocument();
    expect(screen.getByText(/saved answer/)).toBeVisible();
    expect(execution.respond).not.toHaveBeenCalled();
  });
  it("maps every provider-safe live activity to a readable progress state", async () => {
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const activities = [
      ["commandExecution", "Command"], ["fileChange", "File change"],
      ["agentMessage", "Codex response"], ["reasoning", "Reasoning"],
      ["webSearch", "Web search"], ["mcpToolCall", "Tool call"],
    ] as const;
    const runs = activities.map(([kind], index) => ({
      id: `run-${index}`, taskId: task.id, sessionId: "one", instruction: kind,
      status: "running" as const, stopRequested: false, provider: "codex" as const,
      model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], formalRequests: [],
      workLogSyncState: "synced" as const, revision: index + 1,
      liveStatus: { kind, status: index % 2 ? "completed" as const : "running" as const },
    }));
    execution.subscribe.mockResolvedValue({ runs, activeRunId: null, selectedRun: runs[0] });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    for (const [kind, label] of activities) {
      fireEvent.click(await screen.findByRole("button", { name: new RegExp(kind) }));
      expect(screen.getByRole("status", { name: "" })).toHaveTextContent(`${label} · ${kind === "fileChange" || kind === "reasoning" || kind === "mcpToolCall" ? "Finished" : "Running"}`);
    }
  });

  it("submits all nonsecret question answers together, including optional detail, and preserves keyboard focus through request states", async () => {
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const request = {
      id: "questions", kind: "user_input" as const, status: "pending" as const, isBlocking: true,
      title: "Plan the work", prompt: "Answer both", choices: [], questions: [
        { id: "approach", header: "Approach", prompt: "Choose", options: [{ value: "safe", label: "Safe" }], allowOther: false, isSecret: false },
        { id: "detail", header: "Additional detail", prompt: "Add context", options: [{ value: "known", label: "Known" }], allowOther: true, isSecret: false },
      ],
    };
    const run = { id: "run-questions", taskId: task.id, sessionId: "one", instruction: "ask", status: "awaiting_response" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], workLogSyncState: "synced" as const, revision: 1, formalRequests: [request] };
    let finish!: (value: TaskExecutionSnapshot) => void;
    execution.subscribe.mockResolvedValue({ runs: [run], activeRunId: run.id, selectedRun: run });
    execution.respond.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    await screen.findByText("Answer both");
    fireEvent.click(screen.getByRole("button", { name: "Submit answers" }));
    expect(document.querySelector('[data-question-id="approach"]')).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "Safe" }));
    fireEvent.change(screen.getByLabelText("Additional detail"), { target: { value: "Use the existing fixture" } });
    fireEvent.click(screen.getByRole("button", { name: "Submit answers" }));
    await waitFor(() => expect(execution.respond).toHaveBeenCalledWith(expect.objectContaining({
      requestId: "questions",
      response: { answers: {
        approach: { answers: ["safe"] },
        detail: { answers: ["Use the existing fixture"] },
      } },
    })));
    expect(screen.getByRole("button", { name: "Submit answers" })).toBeDisabled();
    expect(screen.getByLabelText("Additional detail")).toBeDisabled();
    await act(async () => finish({
      runs: [{ ...run, formalRequests: [{ ...request, status: "answered", response: { answers: { approach: { answers: ["safe"] }, detail: { answers: ["Use the existing fixture"] } } } }] }],
      activeRunId: run.id, selectedRun: { ...run, formalRequests: [{ ...request, status: "answered", response: { answers: { approach: { answers: ["safe"] }, detail: { answers: ["Use the existing fixture"] } } } }] },
    }));
    expect(await screen.findByText("Response recorded: safe; Use the existing fixture")).toBeVisible();
  });

  it("keeps uncertain terminal Runs distinct and explains when no final report was saved", async () => {
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const uncertain = (["interrupted", "needs_attention"] as const).map((status, index) => ({
      id: `uncertain-${index}`, taskId: task.id, sessionId: "one", instruction: `${status} work`, status,
      stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project",
      evidence: [], formalRequests: [], workLogSyncState: "synced" as const, revision: index + 1,
    }));
    execution.subscribe.mockResolvedValue({ runs: uncertain, activeRunId: null, selectedRun: uncertain[0] });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    for (const [index, status] of ["interrupted", "needs_attention"].entries()) {
      fireEvent.click(await screen.findByRole("button", { name: new RegExp(status) }));
      expect(screen.getByRole("status", { name: "" })).toHaveTextContent("Outcome unknown — work may already have happened");
      expect(screen.getAllByText("Codex did not provide a final report.")).toHaveLength(2);
      expect(screen.getByRole("button", { name: "Use instruction again" })).toBeEnabled();
      expect(screen.getByRole("heading", { name: `Run status: ${index === 0 ? "Interrupted" : "Needs attention"}` })).toBeVisible();
    }
  });
  it("keeps saved terminal requests read-only rather than presenting a stale response as actionable", async () => {
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const terminal = { id: "run-terminal", taskId: task.id, sessionId: "one", instruction: "interrupted", status: "needs_attention" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], workLogSyncState: "synced" as const, revision: 1, formalRequests: [{ id: "approval", kind: "command_approval" as const, status: "pending" as const, isBlocking: true, title: "Proceed", prompt: "Should this continue?", choices: [{ value: "approve", label: "Proceed" }], questions: [] }] };
    execution.respond.mockClear();
    execution.subscribe.mockResolvedValue({ runs: [terminal], activeRunId: null, selectedRun: terminal });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    expect(await screen.findByText(/This request is no longer actionable/)).toBeVisible();
    expect(screen.queryByRole("button", { name: "Proceed" })).not.toBeInTheDocument();
    expect(screen.getByRole("status", { name: "" })).toHaveTextContent("Outcome unknown — work may already have happened");
    expect(execution.respond).not.toHaveBeenCalled();
  });
  it("reconnects to the saved active Run within one second after the session surface reopens", async () => {
    const saved = { ...session("one", "Investigation"), workspacePath: "/project" };
    const active = { id: "run-reconnect", taskId: task.id, sessionId: "one", instruction: "Continue safely", status: "running" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], formalRequests: [], workLogSyncState: "synced" as const, revision: 3 };
    execution.subscribe.mockClear();
    execution.subscribe.mockResolvedValue({ runs: [active], activeRunId: active.id, selectedRun: active });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions: [saved] })) : Promise.resolve(response({ session: saved, entries: [] }))) };
    const first = render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    await waitFor(() => expect(execution.subscribe).toHaveBeenCalledWith({ taskId: task.id, sessionId: "one", runId: undefined }, expect.any(Function)));
    first.unmount();
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    await waitFor(() => expect(execution.subscribe.mock.calls.length).toBeGreaterThanOrEqual(2), { timeout: 1000 });
    expect(await screen.findByText("Continue safely")).toBeVisible();
  });
  it("does not regress to older snapshots or show a prior session while the next loads", async () => {
    const sessions = [session("one", "First"), session("two", "Second")];
    const completed = { id: "run-a", taskId: task.id, sessionId: "one", instruction: "private A instruction", status: "succeeded" as const, stopRequested: false, provider: "codex" as const, model: "gpt-5.6-sol", workspacePath: "/project", evidence: [], formalRequests: [], workLogSyncState: "synced" as const, revision: 2 };
    const latest: TaskExecutionSnapshot = { revision: 2, runs: [completed], activeRunId: null, selectedRun: completed };
    let deliver: (snapshot: TaskExecutionSnapshot) => void = () => {};
    execution.subscribe.mockImplementation(({ sessionId }, onSnapshot) => {
      if (sessionId === "one") { deliver = onSnapshot; return Promise.resolve(latest); }
      return new Promise(() => {});
    });
    window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => path.endsWith("work-sessions") ? Promise.resolve(response({ sessions })) : Promise.resolve(response({ session: sessions.find((value) => path.includes(value.id)), entries: [] }))) };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "one" } });
    expect(await screen.findByText("Run status: Finished")).toBeVisible();
    act(() => deliver({ ...latest, revision: 1, selectedRun: { ...completed, status: "running", revision: 1 }, runs: [{ ...completed, status: "running", revision: 1 }], activeRunId: "run-a" }));
    expect(screen.getByText("Run status: Finished")).toBeVisible();
    fireEvent.change(screen.getByRole("combobox", { name: "Session" }), { target: { value: "two" } });
    expect(screen.queryByText("private A instruction")).not.toBeInTheDocument();
    act(() => deliver(latest));
    expect(screen.queryByText("private A instruction")).not.toBeInTheDocument();
    execution.subscribe.mockResolvedValue({ runs: [], activeRunId: null, selectedRun: null });
  });
  it("does not create a session while listing an empty Task", async () => {
    const request = vi.fn().mockResolvedValue(response({ sessions: [] }));
    window.llmWikiApplication = { request };
    render(<TaskWorkSessions task={task} />);
    expect(await screen.findByText(/Create a session when/)).toBeVisible();
    expect(request).toHaveBeenCalledTimes(1);
    expect(request.mock.calls[0][0]).toMatchObject({
      path: "/tasks/task-a/work-sessions",
      method: "GET",
    });
  });

  it("keeps drafts isolated while switching sessions and retains one operation identity on retry", async () => {
    const sessions = [session("one", "First"), session("two", "Second")];
    let failures = 0;
    const writes: string[] = [];
    const request = vi
      .fn()
      .mockImplementation(
        ({
          path,
          method,
          body,
        }: {
          path: string;
          method?: string;
          body?: string;
        }) => {
          if (path === "/tasks/task-a/work-sessions" && method === "GET")
            return Promise.resolve(response({ sessions }));
          if (path.endsWith("/entries")) {
            writes.push(body ?? "");
            failures += 1;
            return Promise.resolve(response({ detail: "offline" }, false));
          }
          const id = path.includes("/one") ? "one" : "two";
          return Promise.resolve(
            response({
              session: sessions.find((value) => value.id === id),
              entries: [],
            }),
          );
        },
      );
    window.llmWikiApplication = { request };
    render(<TaskWorkSessions task={task} />);
    const picker = await screen.findByLabelText("Session");
    fireEvent.change(picker, { target: { value: "one" } });
    const composer = await screen.findByLabelText("Instruction or note");
    fireEvent.change(composer, { target: { value: "first draft" } });
    fireEvent.change(picker, { target: { value: "two" } });
    await waitFor(() =>
      expect(screen.getByLabelText("Instruction or note")).toHaveValue(""),
    );
    fireEvent.change(screen.getByLabelText("Instruction or note"), {
      target: { value: "second draft" },
    });
    fireEvent.change(picker, { target: { value: "one" } });
    await waitFor(() =>
      expect(screen.getByLabelText("Instruction or note")).toHaveValue("first draft"),
    );
    fireEvent.click(screen.getByRole("button", { name: "Save note" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("still here");
    expect(screen.getByLabelText("Instruction or note")).toHaveValue("first draft");
    fireEvent.click(screen.getByRole("button", { name: "Save note" }));
    await waitFor(() => expect(failures).toBe(2));
    expect(JSON.parse(writes[0]).operationId).toBe(
      JSON.parse(writes[1]).operationId,
    );
    fireEvent.change(screen.getByLabelText("Instruction or note"), {
      target: { value: "edited after failure" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save note" }));
    await waitFor(() => expect(failures).toBe(3));
    expect(JSON.parse(writes[2]).operationId).not.toBe(
      JSON.parse(writes[1]).operationId,
    );
  });

  it("restores settings and saved attachments without changing canonical Task context", async () => {
    const saved = session("one", "Investigation");
    saved.model = "gpt-5.6-terra";
    saved.workspacePath = "/project";
    const request = vi.fn().mockImplementation(({ path }: { path: string }) =>
      path.endsWith("work-sessions")
        ? Promise.resolve(response({ sessions: [saved] }))
        : Promise.resolve(
            response({
              session: saved,
              entries: [
                {
                  id: "e",
                  author: "user",
                  kind: "note",
                  body: "Evidence",
                  attachment: {
                    name: "pixel.png",
                    mediaType: "image/png",
                    data: "aGVsbG8=",
                  },
                  createdAt: "2026-09-21T10:00:00Z",
                },
              ],
            }),
          ),
    );
    window.llmWikiApplication = { request };
    render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), {
      target: { value: "one" },
    });
    expect(await screen.findByText("Evidence")).toBeVisible();
    expect(screen.getByLabelText("Model")).toHaveValue("gpt-5.6-terra");
    expect(screen.getByLabelText("Project folder path")).toHaveValue(
      "/project",
    );
    expect(screen.getByRole("img", { name: "pixel.png" })).toBeVisible();
    expect(
      screen.getByRole("link", { name: /Download attachment/ }),
    ).toHaveAttribute("download", "pixel.png");
    fireEvent.click(screen.getByText("Current Task context"));
    expect(screen.getByText("Canonical detail")).toBeVisible();
    expect(screen.getByText(/Problem p1/)).toBeVisible();
  });

  it("ignores a stale session read after another session is selected", async () => {
    const sessions = [session("one", "First"), session("two", "Second")];
    let resolveOne!: (value: ReturnType<typeof response>) => void;
    let resolveTwo!: (value: ReturnType<typeof response>) => void;
    const one = new Promise<ReturnType<typeof response>>((resolve) => {
      resolveOne = resolve;
    });
    const two = new Promise<ReturnType<typeof response>>((resolve) => {
      resolveTwo = resolve;
    });
    window.llmWikiApplication = {
      request: vi.fn().mockImplementation(({ path }: { path: string }) => {
        if (path.endsWith("work-sessions"))
          return Promise.resolve(response({ sessions }));
        return path.endsWith("/one") ? one : two;
      }),
    };
    render(<TaskWorkSessions task={task} />);
    const picker = await screen.findByLabelText("Session");
    fireEvent.change(picker, { target: { value: "one" } });
    fireEvent.change(picker, { target: { value: "two" } });
    resolveTwo(
      response({
        session: sessions[1],
        entries: [
          {
            id: "two-entry",
            author: "user",
            kind: "note",
            body: "second only",
            createdAt: "2026-09-21T10:00:00Z",
          },
        ],
      }),
    );
    expect(await screen.findByText("second only")).toBeVisible();
    resolveOne(
      response({
        session: sessions[0],
        entries: [
          {
            id: "one-entry",
            author: "user",
            kind: "note",
            body: "stale first",
            createdAt: "2026-09-21T10:00:00Z",
          },
        ],
      }),
    );
    await waitFor(() =>
      expect(screen.queryByText("stale first")).not.toBeInTheDocument(),
    );
    expect(picker).toHaveValue("two");
  });

  it("does not attach a file whose read finishes after switching sessions", async () => {
    const sessions = [session("one", "First"), session("two", "Second")];
    let finishRead: (() => void) | undefined;
    class DeferredFileReader {
      result = "data:image/png;base64,aGVsbG8=";
      error = null;
      onload: null | (() => void) = null;
      onerror: null | (() => void) = null;
      readAsDataURL() {
        finishRead = () => this.onload?.();
      }
    }
    vi.stubGlobal("FileReader", DeferredFileReader);
    window.llmWikiApplication = {
      request: vi.fn().mockImplementation(({ path }: { path: string }) =>
        path.endsWith("work-sessions")
          ? Promise.resolve(response({ sessions }))
          : Promise.resolve(
              response({
                session: path.endsWith("/one") ? sessions[0] : sessions[1],
                entries: [],
              }),
            ),
      ),
    };
    render(<TaskWorkSessions task={task} />);
    const picker = await screen.findByLabelText("Session");
    fireEvent.change(picker, { target: { value: "one" } });
    await screen.findByLabelText("Instruction or note");
    const file = new File(["image"], "late.png", { type: "image/png" });
    fireEvent.change(screen.getByLabelText("Attach file"), {
      target: { files: [file] },
    });
    fireEvent.change(picker, { target: { value: "two" } });
    await waitFor(() =>
      expect(screen.getByLabelText("Session")).toHaveValue("two"),
    );
    finishRead?.();
    await waitFor(() =>
      expect(screen.queryByText(/late\.png/)).not.toBeInTheDocument(),
    );
    vi.unstubAllGlobals();
  });

  it("drops a settings completion after the component changes Task", async () => {
    const saved = session("one", "First");
    let resolveSave!: (value: ReturnType<typeof response>) => void;
    const pendingSave = new Promise<ReturnType<typeof response>>((resolve) => {
      resolveSave = resolve;
    });
    const taskB = { ...task, id: "task-b", title: "Task B" };
    window.llmWikiApplication = {
      request: vi
        .fn()
        .mockImplementation(
          ({ path, method }: { path: string; method?: string }) => {
            if (method === "PUT") return pendingSave;
            if (path === "/tasks/task-a/work-sessions")
              return Promise.resolve(response({ sessions: [saved] }));
            if (path === "/tasks/task-b/work-sessions")
              return Promise.resolve(response({ sessions: [] }));
            return Promise.resolve(response({ session: saved, entries: [] }));
          },
        ),
    };
    const view = render(<TaskWorkSessions task={task} />);
    fireEvent.change(await screen.findByLabelText("Session"), {
      target: { value: "one" },
    });
    await screen.findByLabelText("Model");
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));
    expect(screen.getAllByLabelText("Session")[0]).toBeDisabled();
    view.rerender(<TaskWorkSessions task={taskB} />);
    resolveSave(response(saved));
    expect(await screen.findByText(/Create a session when/)).toBeVisible();
    expect(screen.getByRole("button", { name: "New session" })).toBeEnabled();
    expect(screen.queryByText("Settings saved")).not.toBeInTheDocument();
  });

  it("projects a normal saved session within the 100 ms render budget after data arrives", async () => {
    const saved = session("one", "Profiled");
    const durations: number[] = [];
    const entries = Array.from({ length: 50 }, (_, index) => ({
      id: `e-${index}`,
      author: "user",
      kind: "note",
      body: `Saved record ${index}`,
      createdAt: "2026-09-21T10:00:00Z",
    }));
    window.llmWikiApplication = {
      request: vi
        .fn()
        .mockImplementation(({ path }: { path: string }) =>
          path.endsWith("work-sessions")
            ? Promise.resolve(response({ sessions: [saved] }))
            : Promise.resolve(response({ session: saved, entries })),
        ),
    };
    render(
      <Profiler
        id="sessions"
        onRender={(_id, _phase, duration) => durations.push(duration)}
      >
        <TaskWorkSessions task={task} />
      </Profiler>,
    );
    fireEvent.change(await screen.findByLabelText("Session"), {
      target: { value: "one" },
    });
    expect(await screen.findByText("Saved record 49")).toBeVisible();
    const sorted = [...durations].sort((a, b) => a - b);
    const p95 = sorted[Math.max(0, Math.ceil(sorted.length * 0.95) - 1)];
    expect(p95).toBeLessThan(100);
  });
});
