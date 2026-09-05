import source from '../../public/runtime/conflicts.js?raw';
import { beforeEach, expect, it, vi } from 'vitest';

const action = source.slice(source.indexOf('async function runConflictReview('), source.indexOf('function showConflictReviewResult('));

function setup(beginTrackedConflictReview: ReturnType<typeof vi.fn> = vi.fn(async () => undefined)) {
  const openChat = vi.fn();
  const notice = vi.fn();
  Object.assign(window, { beginTrackedConflictReview });
  const run = new Function('openChat', 'showNotice', 'conflictCopy', `
    ${action}; return runConflictReview;
  `)(openChat, notice, (_key: string, fallback: string) => fallback);
  return { openChat, notice, beginTrackedConflictReview, run, button: document.querySelector('button')! };
}

beforeEach(() => {
  document.body.innerHTML = '<button>Review</button>';
  delete (window as Window & { beginTrackedConflictReview?: unknown }).beginTrackedConflictReview;
});

it('opens Conflict Review in the current Chat instead of submitting a hidden job', async () => {
  const h = setup();
  await h.run('solution', h.button, true);
  expect(h.openChat).toHaveBeenCalledWith('features', 'solution');
  expect(h.beginTrackedConflictReview).toHaveBeenCalledWith('solution');
  expect(h.notice).not.toHaveBeenCalled();
  expect(h.button.disabled).toBe(false);
  expect(h.button.hasAttribute('aria-busy')).toBe(false);
});

it('still opens Chat when tracking orchestration is unavailable', async () => {
  const h = setup();
  delete (window as Window & { beginTrackedConflictReview?: unknown }).beginTrackedConflictReview;
  await h.run('solution', h.button);
  expect(h.openChat).toHaveBeenCalledWith('features', 'solution');
  expect(h.beginTrackedConflictReview).not.toHaveBeenCalled();
  expect(h.notice).not.toHaveBeenCalled();
});

it('reports current-Chat orchestration failures and restores the action', async () => {
  const h = setup(vi.fn(async () => { throw new Error('Unavailable'); }));
  await h.run('solution', h.button);
  expect(h.notice).toHaveBeenCalledWith('Unavailable', 'Could not open Conflict Review');
  expect(h.button.disabled).toBe(false);
  expect(h.button.hasAttribute('aria-busy')).toBe(false);
});
