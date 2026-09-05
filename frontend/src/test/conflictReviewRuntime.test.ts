import source from '../../public/runtime/conflicts.js?raw';
import { beforeEach, expect, it, vi } from 'vitest';

const action = source.slice(source.indexOf('async function runConflictReview('), source.indexOf('function showConflictReviewResult('));
const job = (status: string, id = 'existing') => ({ id, task_kind: 'conflict_review', entity_id: 'solution', status });
function setup(jobs: ReturnType<typeof job>[]) {
  const api = vi.fn(async (path: string) => path === '/jobs' ? { jobs } : job('queued', 'new'));
  const open = vi.fn(), refresh = vi.fn(), notice = vi.fn();
  const run = new Function('api', 'openJobResult', 'refreshQueue', 'showNotice', '$', 'itemDetailModal', `
    let activeConflictAbort=null, activeConflictRun='', queueJobs=[];
    const terminalJobStates=new Set(['completed','awaiting_review','failed','cancelled','stale']);
    const renderQueue=()=>{}, conflictCopy=(_key,fallback)=>fallback;
    ${action}; return runConflictReview;
  `)(api, open, refresh, notice, (selector: string) => document.querySelector(selector), { close: vi.fn() });
  return { api, open, refresh, notice, run, button: document.querySelector('button')! };
}
beforeEach(() => { document.body.innerHTML = '<button id="queue-toggle">Review</button><section id="queue-panel" hidden></section>'; });
it('CB-030 reopens a completed review without submitting another job', async () => {
  const h = setup([job('completed')]);
  await h.run('solution', h.button);
  expect(h.open).toHaveBeenCalledWith('existing');
  expect(h.api).toHaveBeenCalledTimes(1);
  expect(h.notice).not.toHaveBeenCalled();
  expect(h.button.disabled).toBe(false);
});
it.each(['queued', 'running', 'retryable'])('CB-030 shows the active %s job even on explicit rerun', async status => {
  const h = setup([job(status), job('completed', 'older')]);
  await h.run('solution', h.button, true);
  expect(h.api).toHaveBeenCalledTimes(1);
  expect(h.open).not.toHaveBeenCalled();
  expect(document.querySelector<HTMLElement>('#queue-panel')!.hidden).toBe(false);
});
it.each([{ jobs: [] }, { jobs: [job('failed')] }])('CB-030 creates a review when no reusable result exists', async ({ jobs }) => {
  const h = setup(jobs);
  await h.run('solution', h.button);
  expect(h.api).toHaveBeenCalledWith('/features/solution/conflict-review', expect.objectContaining({ method: 'POST' }));
});
it('CB-030 allows an explicit fresh review after completion', async () => {
  const h = setup([job('completed')]);
  await h.run('solution', h.button, true);
  expect(h.api).toHaveBeenCalledTimes(2);
  expect(h.open).not.toHaveBeenCalled();
});
it('CB-030 reports lookup failure without blindly submitting another review', async () => {
  const h = setup([]);
  h.api.mockRejectedValueOnce(new Error('Unavailable'));
  await h.run('solution', h.button);
  expect(h.api).toHaveBeenCalledTimes(1);
  expect(h.notice).toHaveBeenCalledWith('Unavailable', 'Could not open Conflict Review');
  expect(h.button.disabled).toBe(false);
});
