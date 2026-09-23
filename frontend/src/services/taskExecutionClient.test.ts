import { Channel, invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { taskExecutionClient } from "./taskExecutionClient";

vi.mock("@tauri-apps/api/core", () => ({
  Channel: vi.fn(function ChannelMock(this: { onmessage?: unknown }) { return this; }),
  invoke: vi.fn(),
}));

const snapshot = {
  runs: [],
  activeRunId: null,
  selectedRun: null,
};

describe("Task execution native client", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(Channel).mockClear();
  });

  it("sends an exact execution identity and forwards only a snapshot event", async () => {
    vi.mocked(invoke).mockResolvedValue({ status: 201, body: snapshot });
    const observed = vi.fn();

    await expect(taskExecutionClient.execute({
      taskId: "task-a",
      sessionId: "session-a",
      operationId: "operation-a",
      instruction: "Run the check",
      settingsRevision: "settings-a",
    }, observed)).resolves.toEqual(snapshot);

    expect(invoke).toHaveBeenCalledWith("task_session_execute", {
      input: {
        taskId: "task-a",
        sessionId: "session-a",
        operationId: "operation-a",
        instruction: "Run the check",
        settingsRevision: "settings-a",
      },
      onEvent: expect.any(Object),
    });
    const channel = vi.mocked(Channel).mock.results[0]?.value as {
      onmessage?: (event: unknown) => void;
    };
    channel.onmessage?.({ kind: "snapshot", snapshot });
    channel.onmessage?.({ kind: "unsafe", raw: "must not cross the boundary" });
    expect(observed).toHaveBeenCalledTimes(1);
    expect(observed).toHaveBeenCalledWith(snapshot);
  });

  it("uses exact run and formal-request identities for control operations", async () => {
    vi.mocked(invoke).mockResolvedValue({ status: 200, body: snapshot });
    const identity = { taskId: "task-a", sessionId: "session-a", runId: "run-a" };

    await taskExecutionClient.interrupt(identity);
    await taskExecutionClient.respond({
      ...identity,
      requestId: "request-a",
      response: { answers: { question: { answers: ["choice"] } } },
    });
    await taskExecutionClient.syncWorkLog(identity);

    expect(invoke).toHaveBeenNthCalledWith(1, "task_session_interrupt", expect.objectContaining({ input: identity }));
    expect(invoke).toHaveBeenNthCalledWith(2, "task_session_formal_response", expect.objectContaining({
      input: { ...identity, requestId: "request-a", response: { answers: { question: { answers: ["choice"] } } } },
    }));
    expect(invoke).toHaveBeenNthCalledWith(3, "task_session_work_log_sync", expect.objectContaining({ input: identity }));
  });

  it("prepares a bound Codex thread only when explicitly requested", async () => {
    vi.mocked(invoke).mockResolvedValue({ status: 200, body: snapshot });

    await taskExecutionClient.prepare({ taskId: "task-a", sessionId: "session-a" });

    expect(invoke).toHaveBeenCalledWith("task_session_prepare", expect.objectContaining({
      input: { taskId: "task-a", sessionId: "session-a" },
    }));
  });

  it("exposes only the safe native error message", async () => {
    vi.mocked(invoke).mockResolvedValue({
      status: 409,
      body: { error: { code: "active_run", message: "This session already has an active Run." } },
    });

    await expect(taskExecutionClient.subscribe({ taskId: "task-a", sessionId: "session-a" }, vi.fn()))
      .rejects.toThrow("This session already has an active Run.");
  });

  it("lists, reads, and links persisted Codex sessions without opening an execution channel", async () => {
    const thread = {
      id: "thread-a", title: "Investigate sync", preview: "Check the sync path", cwd: "/project",
      model: "gpt-5.6-sol", source: "vscode", status: "idle",
      createdAt: "2026-09-20T10:00:00Z", updatedAt: "2026-09-22T10:00:00Z",
    };
    const transcript = { thread, turns: [] };
    vi.mocked(invoke)
      .mockResolvedValueOnce({ status: 200, body: { threads: [thread] } })
      .mockResolvedValueOnce({ status: 200, body: transcript })
      .mockResolvedValueOnce({ status: 200, body: { ...transcript, taskId: "task-a", sessionId: "session-new", threadId: thread.id, linked: true } });

    await expect(taskExecutionClient.externalThreads({ taskId: "task-a", limit: 50 })).resolves.toEqual({ threads: [thread] });
    await expect(taskExecutionClient.externalThread({ taskId: "task-a", threadId: thread.id })).resolves.toEqual(transcript);
    await expect(taskExecutionClient.linkExternalThread({ taskId: "task-a", threadId: thread.id })).resolves.toEqual(expect.objectContaining({ sessionId: "session-new" }));

    expect(invoke).toHaveBeenNthCalledWith(1, "task_session_external_threads_list", { input: { taskId: "task-a", limit: 50 } });
    expect(invoke).toHaveBeenNthCalledWith(2, "task_session_external_thread_read", { input: { taskId: "task-a", threadId: "thread-a" } });
    expect(invoke).toHaveBeenNthCalledWith(3, "task_session_external_thread_link", { input: { taskId: "task-a", threadId: "thread-a" } });
    expect(Channel).not.toHaveBeenCalled();
  });
});
