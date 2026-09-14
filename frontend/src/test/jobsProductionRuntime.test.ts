import runtime from '../../public/runtime/jobs.js?raw';
import { expect, it, vi } from 'vitest';

type Job = { id: string; status: string; task_kind: string; entity_type: string; entity_id: string; result_interface: string; error?: { message: string } };

function renderShell() {
  document.body.innerHTML = '<button id="queue-toggle"></button><section id="queue-panel" hidden><header><button></button></header></section><section id="queue-list"></section><button data-view="ai-setup"></button><button data-view="workbench"></button><aside id="queue-toast" hidden><button></button><strong id="queue-toast-title"></strong></aside><button id="alert-toggle"></button><section id="alert-panel" hidden><header><button></button></header></section><span id="alert-badge"></span><section id="alert-list"></section><span id="organize-status"></span>';
}

function startRuntime(api: (path: string, options?: { method?: string }) => Promise<unknown>, options: { t?: (key: string) => string; showNotice?: ReturnType<typeof vi.fn>; onJobEvent?: (refresh: () => void) => void } = {}) {
  const $ = (selector: string) => document.querySelector(selector) as HTMLElement;
  const noop = vi.fn();
  window.llmWikiFormatSystemTime = () => 'now';
  new Function('api', '$', 'esc', 't', 'preview', 'activeLocale', 'chatTarget', 'chatModal', 'itemDetailModal', 'applicationEvents', 'renderExploreWork', 'openSolutionDetail', 'loadBoard', 'showCompletionReviewResult', 'showConflictReviewResult', 'showNotice', 'searchArchivedDocument', 'loadWorkbenchContext', 'openCompletedWorkspace', 'setPreviewTab', runtime)(
    api, $, (value: unknown) => String(value), options.t ?? ((key: string) => key), (value: string) => value, 'en', null, { open: false }, { open: false }, (_url: string, _type: string, refresh: () => void) => { options.onJobEvent?.(refresh); return { signal: new AbortController().signal }; }, noop, noop, noop, noop, noop, options.showNotice ?? noop, noop, noop, noop,
  );
}

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

it('CB-038 renders production Queue and notification actions before dispatching their real handlers', async () => {
  renderShell();
  const api = vi.fn(async (path: string, options?: { method?: string }) => {
    if (path === '/jobs') return { jobs: [
      { id: 'running', status: 'running', task_kind: 'workbench_organization', entity_type: 'workbench', entity_id: 'current', result_interface: 'workbench' },
      { id: 'failed', status: 'failed', task_kind: 'workflow_draft', entity_type: 'captures', entity_id: 'capture', result_interface: 'none', error: { message: 'Configure an API key in AI setup before using AI' } },
      { id: 'ready', status: 'completed', task_kind: 'workflow_draft', entity_type: 'captures', entity_id: 'capture', result_interface: 'workbench' },
    ] };
    if (path === '/notifications') return { unread_count: 1, notifications: [{ id: 'notice', title: 'Ready', job_id: 'ready' }] };
    if (options?.method === 'POST') return {};
    if (path.endsWith('/result')) return { result: { organized: 1 } };
    return {};
  });
  startRuntime(api);
  await tick();
  expect(document.querySelectorAll('[data-job-action="cancel"]')).toHaveLength(1);
  expect(document.querySelectorAll('[data-job-action="retry"]')).toHaveLength(1);
  expect(document.querySelector('[data-job-action="setup"]')).toHaveTextContent('Open AI setup');
  expect(document.querySelector('.queue-provider-help')).toHaveTextContent('No API key is configured. Save your connection details in AI setup, then retry.');
  expect(document.querySelector<HTMLButtonElement>('[data-job-action="result"]:not(:disabled)')).not.toBeNull();
  document.querySelector<HTMLButtonElement>('[data-job-action="cancel"]')!.click();
  document.querySelector<HTMLButtonElement>('[data-job-action="retry"]')!.click();
  const setup = document.querySelector<HTMLButtonElement>('[data-job-action="setup"]')!;
  const navigate = vi.fn();
  document.querySelector('[data-view="ai-setup"]')!.addEventListener('click', navigate);
  setup.click();
  expect(navigate).toHaveBeenCalledOnce();
  document.querySelector<HTMLButtonElement>('[data-job-action="result"]:not(:disabled)')!.click();
  document.querySelector<HTMLButtonElement>('[data-notification-action="open"]')!.click();
  document.querySelector<HTMLButtonElement>('[data-notification-action="dismiss"]')!.click();
  await tick();
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

it('CB-038 localizes the known native missing-key recovery copy in Korean', async () => {
  renderShell();
  const translations: Record<string, string> = {
    'queue.provider_missing': 'API 키가 설정되지 않았습니다. AI 설정에서 연결 정보를 저장한 뒤 다시 시도하세요.',
    'queue.open_ai_setup': 'AI 설정 열기',
    'queue.task.knowledge_translation': 'Knowledge 번역',
  };
  startRuntime(async (path: string) => path === '/jobs'
    ? { jobs: [{ id: 'missing', status: 'failed', task_kind: 'knowledge_translation', entity_type: 'knowledge', entity_id: 'queue-recovery.md', result_interface: 'knowledge_document', error: { message: 'Configure an API key in AI setup before using AI' } }] }
    : { unread_count: 0, notifications: [] }, { t: (key) => translations[key] ?? key });
  await tick();
  expect(document.querySelector('.queue-provider-help')).toHaveTextContent(translations['queue.provider_missing']);
  expect(document.querySelector('[data-job-action="setup"]')).toHaveTextContent(translations['queue.open_ai_setup']);
});

it('CB-038 opens the exact completed Knowledge draft returned by its Queue job', async () => {
  renderShell();
  const navigate = vi.fn();
  document.querySelector('[data-view="workbench"]')!.addEventListener('click', navigate);
  const openDraft = vi.fn();
  window.llmWikiOpenKnowledgeDraft = openDraft;
  startRuntime(async (path: string) => {
    if (path === '/jobs') return { jobs: [{ id: 'knowledge-1', status: 'completed', task_kind: 'knowledge_draft', entity_type: 'tasks', entity_id: 'task-1', result_interface: 'task_knowledge_draft' }] };
    if (path === '/notifications') return { unread_count: 0, notifications: [] };
    if (path === '/jobs/knowledge-1/result') return { result: { taskId: 'task-1', draftRevision: 2, bodyMarkdown: '# Exact draft', contentHash: 'body-2', sourceHash: 'source-2', state: 'draft' } };
    return {};
  });
  await tick();
  expect(document.querySelector('[data-job-action="result"]')).toHaveTextContent('Open result page');
  document.querySelector<HTMLButtonElement>('[data-job-action="result"]')!.click();
  await tick();
  expect(navigate).toHaveBeenCalledOnce();
  expect(document.querySelector<HTMLButtonElement>('#queue-panel')!.hidden).toBe(true);
  expect(openDraft).toHaveBeenCalledWith(expect.objectContaining({ taskId: 'task-1', draftRevision: 2, bodyMarkdown: '# Exact draft' }));
});

it('CB-038 prevents duplicate retry while a job-event repaint occurs', async () => {
  renderShell();
  let releaseRetry: (() => void) | undefined;
  let refreshFromEvent: (() => void) | undefined;
  const failed: Job = { id: 'failed', status: 'failed', task_kind: 'knowledge_translation', entity_type: 'knowledge', entity_id: 'queue-recovery.md', result_interface: 'knowledge_document', error: { message: 'Configure an API key in AI setup before using AI' } };
  const api = vi.fn((path: string, options?: { method?: string }) => {
    if (path === '/jobs') return Promise.resolve({ jobs: [failed] });
    if (path === '/notifications') return Promise.resolve({ unread_count: 0, notifications: [] });
    if (path === '/jobs/failed/retry' && options?.method === 'POST') return new Promise((resolve) => { releaseRetry = () => resolve({}); });
    return Promise.resolve({});
  });
  startRuntime(api, { onJobEvent: (refresh) => { refreshFromEvent = refresh; } });
  await tick();
  document.querySelector<HTMLButtonElement>('[data-job-action="retry"]')!.click();
  expect(document.querySelector<HTMLButtonElement>('[data-job-action="retry"]')!.disabled).toBe(true);
  refreshFromEvent?.();
  await tick();
  const repainted = document.querySelector<HTMLButtonElement>('[data-job-action="retry"]')!;
  expect(repainted.disabled).toBe(true);
  repainted.click();
  expect(api.mock.calls.filter(([path, request]) => path === '/jobs/failed/retry' && request?.method === 'POST')).toHaveLength(1);
  releaseRetry?.();
  await tick(); await tick();
  expect(document.querySelector<HTMLButtonElement>('[data-job-action="retry"]')!.disabled).toBe(false);
});

it('CB-038 restores a failed job retry when both retry and its queue refresh fail', async () => {
  renderShell();
  const failed: Job = { id: 'failed', status: 'failed', task_kind: 'workflow_draft', entity_type: 'captures', entity_id: 'capture', result_interface: 'none', error: { message: 'deterministic fixture failure' } };
  const notice = vi.fn();
  let jobReads = 0;
  const api = vi.fn(async (path: string, options?: { method?: string }) => {
    if (path === '/jobs') {
      jobReads += 1;
      if (jobReads > 1) throw new Error('queue refresh unavailable');
      return { jobs: [failed] };
    }
    if (path === '/notifications') return { unread_count: 0, notifications: [] };
    if (path === '/jobs/failed/retry' && options?.method === 'POST') throw new Error('retry unavailable');
    return {};
  });
  startRuntime(api, { showNotice: notice });
  await tick();
  document.querySelector<HTMLButtonElement>('[data-job-action="retry"]')!.click();
  await tick(); await tick();
  expect(document.querySelector<HTMLButtonElement>('[data-job-action="retry"]')!.disabled).toBe(false);
  expect(document.querySelector('[data-job-id="failed"]')).toHaveTextContent('deterministic fixture failure');
  expect(notice).toHaveBeenCalledWith('Could not retry this job. Try again.', 'retry unavailable');
});
