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
});
