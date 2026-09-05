import runtime from '../../public/runtime/jobs.js?raw';
import { expect, it, vi } from 'vitest';

it('CB-038 renders production Queue and notification actions before dispatching their real handlers', async () => {
  document.body.innerHTML = '<button id="queue-toggle"></button><section id="queue-panel" hidden><header><button></button></header></section><section id="queue-list"></section><aside id="queue-toast" hidden><button></button><strong id="queue-toast-title"></strong></aside><button id="alert-toggle"></button><section id="alert-panel" hidden><header><button></button></header></section><span id="alert-badge"></span><section id="alert-list"></section><span id="organize-status"></span>';
  const api = vi.fn(async (path: string, options?: { method?: string }) => {
    if (path === '/jobs') return { jobs: [
      { id: 'running', status: 'running', task_kind: 'workbench_organization', entity_type: 'workbench', entity_id: 'current', result_interface: 'workbench' },
      { id: 'failed', status: 'failed', task_kind: 'workflow_draft', entity_type: 'captures', entity_id: 'capture', result_interface: 'none' },
      { id: 'ready', status: 'completed', task_kind: 'workflow_draft', entity_type: 'captures', entity_id: 'capture', result_interface: 'workbench' },
    ] };
    if (path === '/notifications') return { unread_count: 1, notifications: [{ id: 'notice', title: 'Ready', job_id: 'ready' }] };
    if (options?.method === 'POST') return {};
    if (path.endsWith('/result')) return { result: { organized: 1 } };
    return {};
  });
  const $ = (selector: string) => document.querySelector(selector) as HTMLElement;
  const noop = vi.fn();
  window.llmWikiFormatSystemTime = () => 'now';
  new Function('api', '$', 'esc', 't', 'preview', 'activeLocale', 'chatTarget', 'chatModal', 'itemDetailModal', 'applicationEvents', 'renderExploreWork', 'openSolutionDetail', 'loadBoard', 'showCompletionReviewResult', 'showConflictReviewResult', 'showNotice', 'searchArchivedDocument', 'loadWorkbenchContext', 'openCompletedWorkspace', 'setPreviewTab', runtime)(
    api, $, (value: unknown) => String(value), (key: string) => key, (value: string) => value, 'en', null, { open: false }, { open: false }, () => ({ signal: new AbortController().signal }), noop, noop, noop, noop, noop, noop, noop, noop, noop,
  );
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(document.querySelectorAll('[data-job-action="cancel"]')).toHaveLength(1);
  expect(document.querySelectorAll('[data-job-action="retry"]')).toHaveLength(1);
  expect(document.querySelector<HTMLButtonElement>('[data-job-action="result"]:not(:disabled)')).not.toBeNull();
  document.querySelector<HTMLButtonElement>('[data-job-action="cancel"]')!.click();
  document.querySelector<HTMLButtonElement>('[data-job-action="retry"]')!.click();
  document.querySelector<HTMLButtonElement>('[data-job-action="result"]:not(:disabled)')!.click();
  document.querySelector<HTMLButtonElement>('[data-notification-action="open"]')!.click();
  document.querySelector<HTMLButtonElement>('[data-notification-action="dismiss"]')!.click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(api).toHaveBeenCalledWith('/jobs/running/cancel', { method: 'POST' });
  expect(api).toHaveBeenCalledWith('/jobs/failed/retry', { method: 'POST' });
  expect(api).toHaveBeenCalledWith('/notifications/notice/read', { method: 'POST' });
  expect(api).toHaveBeenCalledWith('/notifications/notice/dismiss', { method: 'POST' });
  expect(document.getElementById('queue-panel')?.hidden).toBe(true);
  document.querySelector<HTMLButtonElement>('#queue-toggle')!.click();
  expect(document.getElementById('queue-panel')?.hidden).toBe(false);
  document.querySelector<HTMLButtonElement>('#alert-toggle')!.click();
  expect(document.getElementById('alert-panel')?.hidden).toBe(false);
});
