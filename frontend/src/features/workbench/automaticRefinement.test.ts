import { describe, expect, it, vi } from 'vitest';
import { advanceAutomaticRefinement, refinementProgress, titlePreparationReady } from './automaticRefinement';
import type { CaptureDistillationSummary, WorkbenchSnapshot } from '../../types/taskWorkbench';
const snapshot: WorkbenchSnapshot = { revision: 1, activeShortcuts: [], refiningShortcuts: [], categories: [{ id: 'x', label: 'x', items: [
  { kind: 'capture', id: 'new', text: 'new', lastUserActivityAt: '2026-02-01' },
  { kind: 'capture', id: 'old', text: 'old', lastUserActivityAt: '2026-01-01' },
] }] };
const client = () => ({ refinement: vi.fn().mockResolvedValue({ id: 's', draftRevision: 0 }), refinementStatus: vi.fn().mockResolvedValue({ id: 's', draftRevision: 1 }), referenceWorkspace: vi.fn().mockResolvedValue({}), generatePreview: vi.fn().mockResolvedValue({}) });
describe('automatic incoming refinement', () => {
  it('starts only the oldest item and never repeats an attempt on a stale board', async () => {
    const api = client(), attempted = new Set<string>();
    await advanceAutomaticRefinement(api, snapshot, attempted, () => true, vi.fn());
    expect(api.refinement).toHaveBeenCalledExactlyOnceWith('capture', 'old');
    expect(api.generatePreview).toHaveBeenCalledExactlyOnceWith('s', 'refinement:0');
    await advanceAutomaticRefinement(api, snapshot, attempted, () => false, vi.fn());
    expect(api.generatePreview).toHaveBeenCalledTimes(1);
  });
  it('waits for an existing active job instead of starting another item', async () => {
    const api = client(); api.referenceWorkspace.mockResolvedValue({ generation: { status: 'running' } });
    await advanceAutomaticRefinement(api, { ...snapshot, refiningShortcuts: [{ kind: 'capture', id: 'other', draftRevision: 1 }] }, new Set(), () => true, vi.fn());
    expect(api.generatePreview).not.toHaveBeenCalled(); expect(api.refinement).not.toHaveBeenCalled();
  });
  it.each([{ preview: {} }, { generation: { status: 'failed' } }, { generation: { status: 'completed' } }])('does not overwrite a persisted result or retry a failure: %j', async workspace => {
    const api = client(); api.referenceWorkspace.mockResolvedValue(workspace);
    await advanceAutomaticRefinement(api, snapshot, new Set(), () => true, vi.fn());
    expect(api.generatePreview).not.toHaveBeenCalled();
  });
  it('stops starting new work after navigation while a request is in flight', async () => {
    const api = client(); let active = true;
    api.refinement.mockImplementation(async () => { active = false; return { id: 's' }; });
    await advanceAutomaticRefinement(api, snapshot, new Set(), () => active, vi.fn());
    expect(api.generatePreview).not.toHaveBeenCalled();
  });
  it('shows round and review state without applying a proposal', () => {
    expect(refinementProgress({ id: 's', draftRevision: 2 }, { generation: { status: 'running' } })).toEqual({ round: 3, state: 'working' });
    expect(refinementProgress({ id: 's', draftRevision: 2 }, { preview: { id: 'p', currentVersion: 2, contextRevision: 0, versions: [] } })).toEqual({ round: 2, state: 'review' });
    expect(refinementProgress({ id: 's', draftRevision: 0 }, { generation: { status: 'failed' } })).toEqual({ round: 1, state: 'failed' });
  });
});

describe('title preparation precedes full refinement', () => {
  const summary = (status: string, disposition = 'applied', sourceRevision = 'source:1') => ({
    eligible: true, sourceRevision: 'source:1', distillation: { status, executionOutcome: 'succeeded', applicationDisposition: disposition, sourceRevision },
  } as CaptureDistillationSummary);
  it.each(['queued', 'running', 'retryable', 'failed', 'cancelled'])('waits for title job %s', status => {
    expect(titlePreparationReady(summary(status))).toBe(false);
  });
  it('requires applied output bound to the current source', () => {
    expect(titlePreparationReady(summary('completed'))).toBe(true);
    expect(titlePreparationReady(summary('completed', 'superseded'))).toBe(false);
    expect(titlePreparationReady(summary('completed', 'applied', 'old-source'))).toBe(false);
    expect(titlePreparationReady({ eligible: true } as CaptureDistillationSummary)).toBe(false);
  });
  it('does not open a refinement session while the older title job is still running', async () => {
    const api = client();
    const board: WorkbenchSnapshot = { ...snapshot, categories: [{ id: 'x', label: 'x', items: [{ kind: 'capture', id: 'c', text: 'original', captureDistillation: summary('running') }] }] };
    await advanceAutomaticRefinement(api, board, new Set(), () => true, vi.fn());
    expect(api.refinement).not.toHaveBeenCalled();
    expect(api.generatePreview).not.toHaveBeenCalled();
  });
});

it('keeps a newly opened job serialized even before the board refresh arrives', async () => {
  const api = client(), attempted = new Set<string>(), opened = new Set<string>();
  await advanceAutomaticRefinement(api, snapshot, attempted, () => true, vi.fn(), opened);
  api.referenceWorkspace.mockResolvedValue({ generation: { status: 'running' } });
  await advanceAutomaticRefinement(api, snapshot, attempted, () => true, vi.fn(), opened);
  expect(api.generatePreview).toHaveBeenCalledTimes(1);
  expect(api.refinement).toHaveBeenCalledTimes(1);
});
it('moves past a failed session-open without repeatedly requesting its missing session', async () => {
  const api = client(), attempted = new Set<string>(), opened = new Set<string>();
  api.refinement.mockRejectedValueOnce(new Error('unavailable'));
  await expect(advanceAutomaticRefinement(api, snapshot, attempted, () => true, vi.fn(), opened)).rejects.toThrow('unavailable');
  await advanceAutomaticRefinement(api, snapshot, attempted, () => true, vi.fn(), opened);
  expect(api.refinementStatus).not.toHaveBeenCalled();
  expect(api.refinement).toHaveBeenLastCalledWith('capture', 'new');
});
