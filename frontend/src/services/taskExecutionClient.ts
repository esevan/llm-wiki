import { Channel, invoke } from "@tauri-apps/api/core";
import type { CodexModelCapability, CodexThreadLinkResult, CodexThreadList, CodexThreadTranscript, TaskExecutionSnapshot, TaskWorkSessionAttachment } from "../types/taskWorkbench";

import { applyExecutionLive, mergeExecutionSnapshot, type ExecutionLiveEvent } from "./executionStream";

type NativeResponse<T> = { status: number; body: T };
type ExecutionInput = { taskId: string; sessionId: string; runId?: string };
const unwrap = async <T>(command: string, input: Record<string, unknown>, onSnapshot?: (snapshot: TaskExecutionSnapshot) => void) => {
  const onEvent = new Channel<{ kind: "snapshot"; snapshot: TaskExecutionSnapshot }>();
  onEvent.onmessage = (event) => { if (event.kind === "snapshot") onSnapshot?.(event.snapshot); };
  const result = await invoke<NativeResponse<T>>(command, { input, onEvent });
  if (result.status < 200 || result.status >= 300) throw new Error((result.body as { error?: { message?: string } })?.error?.message ?? "Execution request failed");
  return result.body;
};
const request = async <T>(command: string, input: Record<string, unknown>) => {
  const result = await invoke<NativeResponse<T>>(command, { input });
  if (result.status < 200 || result.status >= 300) throw new Error((result.body as { error?: { message?: string } })?.error?.message ?? "Codex session request failed");
  return result.body;
};

export type ExecutionSubscription = Promise<TaskExecutionSnapshot> & { dispose: () => Promise<void> };

function subscribe(input: ExecutionInput, onSnapshot?: (snapshot: TaskExecutionSnapshot) => void): ExecutionSubscription {
  const subscriptionId = globalThis.crypto?.randomUUID?.() ?? `execution-subscription-${Date.now()}-${Math.random()}`;
  let disposed = false;
  let current: TaskExecutionSnapshot | undefined;
  const accept = (snapshot: TaskExecutionSnapshot) => {
    const next = mergeExecutionSnapshot(current, snapshot);
    if (next !== current) { current = next; if (!disposed) onSnapshot?.(next); }
    return next;
  };
  const onEvent = new Channel<{ kind: "snapshot"; snapshot: TaskExecutionSnapshot } | ExecutionLiveEvent>();
  onEvent.onmessage = event => {
    if (disposed) return;
    if (event.kind === "snapshot") accept(event.snapshot);
    else if (event.kind === "live") {
      try {
        const next = applyExecutionLive(current, event);
        if (next && next !== current) { current = next; onSnapshot?.(next); }
      } finally {
        void request("task_session_execution_ack", { subscriptionId }).catch(() => { void subscription.dispose(); });
      }
    }
  };
  const ready = invoke<NativeResponse<TaskExecutionSnapshot>>("task_session_execution_subscribe", { input: { ...input, subscriptionId }, onEvent }).then(result => {
    if (result.status < 200 || result.status >= 300) throw new Error((result.body as unknown as { error?: { message?: string } })?.error?.message ?? "Execution request failed");
    return accept(result.body);
  });
  let disposal: Promise<void> | undefined;
  const subscription = Object.assign(ready, { dispose: () => {
    disposed = true;
    // Registration may still be in flight. Wait for it before cancellation so
    // close-before-ready cannot leave a late native receiver behind.
    disposal ??= ready.then(() => request("task_session_execution_unsubscribe", { subscriptionId })).then(() => undefined, () => undefined);
    return disposal;
  } });
  return subscription;
}

export interface CompletionReview {
  taskId: string; taskRevision: number; taskState: string; reviewHash: string; activeOperationId?: string; activeEvidence?: string;
  runs: { id: string; instruction: string; status: string }[];
  queued: { id: string; instruction: string }[];
  responses: { id: string; title: string; kind: string }[];
}
export interface CompletionProgress { state: "stopping" | "cancelled" | "completed"; review: CompletionReview; remaining: CompletionReview; issues?: string[] }
export const taskCompletionClient = {
  preview: (taskId: string) => request<CompletionReview>("task_completion_control", { action: "preview", taskId }),
  control: (input: { action: "begin" | "poll" | "cancel"; taskId: string; operationId: string; expectedTaskRevision?: number; reviewHash?: string; evidence?: string }) => request<CompletionProgress>("task_completion_control", input),
};

export const taskExecutionClient = {
  models: () => request<{ models: CodexModelCapability[] }>("task_session_models", {}),
  queue: (input: ExecutionInput & { action: "enqueue" | "edit" | "remove" | "move" | "pause" | "resume" | "cancel_drain" | "stop_then_drain"; operationId?: string; instruction?: string; attachment?: TaskWorkSessionAttachment; itemId?: string; expectedRevision?: number; direction?: "up" | "down" }) => request<TaskExecutionSnapshot>("task_session_queue", input),
  prepare: (input: ExecutionInput & {resolveReasoning?:boolean}, onSnapshot?: (snapshot: TaskExecutionSnapshot) => void) => unwrap<TaskExecutionSnapshot>("task_session_prepare", input, onSnapshot),
  execute: (input: ExecutionInput & { operationId: string; instruction: string; settingsRevision: string; retryOfRunId?: string; attachment?: TaskWorkSessionAttachment }, onSnapshot?: (snapshot: TaskExecutionSnapshot) => void) => unwrap<TaskExecutionSnapshot>("task_session_execute", input, onSnapshot),
  subscribe,
  interrupt: (input: Required<ExecutionInput>) => unwrap<TaskExecutionSnapshot>("task_session_interrupt", input),
  respond: (input: Required<ExecutionInput> & { requestId: string; response: { decision?: string; answers?: Record<string, { answers: string[] }> } }) => unwrap<TaskExecutionSnapshot>("task_session_formal_response", input),
  syncWorkLog: (input: Required<ExecutionInput>) => unwrap<TaskExecutionSnapshot>("task_session_work_log_sync", input),
  externalThreads: (input: { taskId: string; sessionId?: string; cursor?: string; limit?: number; searchTerm?: string; cwd?: string }) => request<CodexThreadList>("task_session_external_threads_list", input),
  externalThread: (input: { taskId: string; sessionId?: string; threadId?: string; cursor?: string; limit?: number }) => request<CodexThreadTranscript>("task_session_external_thread_read", input),
  linkExternalThread: (input: { taskId: string; sessionId?: string; threadId: string }) => request<CodexThreadLinkResult>("task_session_external_thread_link", input),
};
