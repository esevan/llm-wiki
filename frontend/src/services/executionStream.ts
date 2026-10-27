import type { TaskExecutionRun, TaskExecutionSnapshot } from "../types/taskWorkbench";

export type ExecutionLiveEvent = { kind: "live"; runId: string; liveStatus: TaskExecutionRun["liveStatus"] | null; liveRevision: number; revision: number };
const running = (run: TaskExecutionRun) => ["queued", "running", "awaiting_response"].includes(run.status);

export function mergeExecutionSnapshot(current: TaskExecutionSnapshot | undefined, incoming: TaskExecutionSnapshot): TaskExecutionSnapshot {
  if (current?.revision !== undefined && incoming.revision !== undefined && incoming.revision <= current.revision) return current;
  // A terminal response without an authoritative answer must not erase the
  // last flushed fragment (including failure/cancellation and command output).
  const runs = incoming.runs.map(run => {
    const previous = current?.runs.find(value => value.id === run.id);
    return !running(run) && !run.finalReport && !run.liveStatus && previous?.liveStatus
      ? { ...run, liveStatus: previous.liveStatus } : run;
  });
  return { ...incoming, runs, selectedRun: runs.find(run => run.id === incoming.selectedRun?.id) ?? incoming.selectedRun };
}

export function applyExecutionLive(current: TaskExecutionSnapshot | undefined, event: ExecutionLiveEvent): TaskExecutionSnapshot | undefined {
  if (!current || (current.liveRevision ?? -1) >= event.liveRevision) return current;
  const run = current.runs.find(value => value.id === event.runId);
  if (!run || !running(run)) return current;
  const updated = { ...run, liveStatus: event.liveStatus ?? undefined };
  return { ...current, liveRevision: event.liveRevision, revision: event.revision, runs: current.runs.map(value => value.id === run.id ? updated : value), selectedRun: current.selectedRun?.id === run.id ? updated : current.selectedRun };
}
