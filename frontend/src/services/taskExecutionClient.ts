import { Channel, invoke } from "@tauri-apps/api/core";
import type { TaskExecutionSnapshot } from "../types/taskWorkbench";

type NativeResponse<T> = { status: number; body: T };
type ExecutionInput = { taskId: string; sessionId: string; runId?: string };
const unwrap = async <T>(command: string, input: Record<string, unknown>, onSnapshot?: (snapshot: TaskExecutionSnapshot) => void) => {
  const onEvent = new Channel<{ kind: "snapshot"; snapshot: TaskExecutionSnapshot }>();
  onEvent.onmessage = (event) => { if (event.kind === "snapshot") onSnapshot?.(event.snapshot); };
  const result = await invoke<NativeResponse<T>>(command, { input, onEvent });
  if (result.status < 200 || result.status >= 300) throw new Error((result.body as { error?: { message?: string } })?.error?.message ?? "Execution request failed");
  return result.body;
};

export const taskExecutionClient = {
  prepare: (input: ExecutionInput, onSnapshot?: (snapshot: TaskExecutionSnapshot) => void) => unwrap<TaskExecutionSnapshot>("task_session_prepare", input, onSnapshot),
  execute: (input: ExecutionInput & { operationId: string; instruction: string; settingsRevision: string; retryOfRunId?: string }, onSnapshot?: (snapshot: TaskExecutionSnapshot) => void) => unwrap<TaskExecutionSnapshot>("task_session_execute", input, onSnapshot),
  subscribe: (input: ExecutionInput, onSnapshot?: (snapshot: TaskExecutionSnapshot) => void) => unwrap<TaskExecutionSnapshot>("task_session_execution_subscribe", input, onSnapshot),
  interrupt: (input: Required<ExecutionInput>) => unwrap<TaskExecutionSnapshot>("task_session_interrupt", input),
  respond: (input: Required<ExecutionInput> & { requestId: string; response: { decision?: string; answers?: Record<string, { answers: string[] }> } }) => unwrap<TaskExecutionSnapshot>("task_session_formal_response", input),
  syncWorkLog: (input: Required<ExecutionInput>) => unwrap<TaskExecutionSnapshot>("task_session_work_log_sync", input),
};
