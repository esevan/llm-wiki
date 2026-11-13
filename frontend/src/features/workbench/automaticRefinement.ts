import type { CaptureDistillationSummary, RefinementSession, ReferenceWorkspace, WorkbenchSnapshot } from '../../types/taskWorkbench';

export type RefinementProgress = { round: number; state: 'working' | 'review' | 'failed' | 'waiting' };
const working = (status?: string) => ['queued', 'running', 'retryable'].includes(status ?? '');
export function refinementProgress(session: RefinementSession, workspace: ReferenceWorkspace): RefinementProgress {
  const round = Math.max(0, session.draftRevision ?? 0);
  if (working(session.responseStatus) || working(workspace.generation?.status) || working(session.previewStatus))
    return { round: round + 1, state: 'working' };
  if (['failed', 'cancelled', 'stale'].includes(workspace.generation?.status ?? session.previewStatus ?? '') || session.responseStatus === 'failed')
    return { round: Math.max(1, round), state: 'failed' };
  if (workspace.preview) return { round: Math.max(1, round), state: 'review' };
  return { round, state: 'waiting' };
}

export function titlePreparationReady(summary?: CaptureDistillationSummary): boolean {
  if (!summary) return true; // Legacy Captures have no title-preparation pipeline.
  const job = summary.distillation;
  if (!job) return !summary.eligible;
  return job.status === 'completed' && job.executionOutcome === 'succeeded' && job.applicationDisposition === 'applied'
    && job.sourceRevision === summary.sourceRevision;
}

type Client = {
  refinement(kind: 'capture' | 'task', id: string): Promise<RefinementSession>;
  refinementStatus(kind: 'capture' | 'task', id: string): Promise<RefinementSession>;
  referenceWorkspace(id: string): Promise<ReferenceWorkspace>;
  generatePreview(id: string, revision: string): Promise<unknown>;
};
/** One pass starts at most one job. Persisted generation/preview records prevent replay. */
export async function advanceAutomaticRefinement(
  client: Client, snapshot: WorkbenchSnapshot, attempted: Set<string>,
  active: () => boolean, report: (key: string, value: RefinementProgress) => void,
  opened: Set<string> = new Set(),
): Promise<boolean> {
  let occupied = false;
  const visibleCaptures = new Set(snapshot.categories.flatMap(category => category.items).filter(item => item.kind === 'capture').map(item => item.id));
  const subjects: Array<{ kind: 'capture' | 'task'; id: string }> = [...snapshot.refiningShortcuts];
  // A delayed/failed board refresh must not hide the job we just started.
  for (const id of opened) {
    if (visibleCaptures.has(id) && !subjects.some(item => item.kind === 'capture' && item.id === id)) subjects.push({ kind: 'capture', id });
  }
  for (const item of subjects) {
    const session = await client.refinementStatus(item.kind, item.id);
    const workspace = await client.referenceWorkspace(session.id);
    const progress = refinementProgress(session, workspace);
    report(`${item.kind}:${item.id}`, progress);
    occupied ||= progress.state === 'working';
  }
  if (occupied || !active()) return false;
  const existing = new Set(snapshot.refiningShortcuts.map(item => `${item.kind}:${item.id}`));
  // Oldest incoming Capture first; an existing manual session is never taken over.
  const candidates = snapshot.categories.flatMap(category => category.items)
    .filter(item => item.kind === 'capture' && titlePreparationReady(item.captureDistillation) && !existing.has(`capture:${item.id}`) && !attempted.has(item.id))
    .sort((a, b) => (a.lastUserActivityAt ?? '').localeCompare(b.lastUserActivityAt ?? ''));
  const item = candidates[0];
  if (!item || !active()) return false;
  attempted.add(item.id);
  try {
    const session = await client.refinement('capture', item.id);
    opened.add(item.id);
    const workspace = await client.referenceWorkspace(session.id);
    report(`capture:${item.id}`, refinementProgress(session, workspace));
    if (!active() || working(session.responseStatus) || working(session.previewStatus) || !titlePreparationReady(session.captureDistillation) || session.state === 'completed' || workspace.preview || workspace.generation || session.previewJobId || session.messages?.length) return true;
    report(`capture:${item.id}`, { round: 1, state: 'working' });
    await client.generatePreview(session.id, `refinement:${session.draftRevision ?? 0}`);
    return true;
  } catch (error) {
    report(`capture:${item.id}`, { round: 1, state: 'failed' });
    throw error;
  }
}
