import type { InteractionCoverage } from './interactionCoverage';

/** Minimal UI primitives supplied by desktopScenario; no scenario may replace UI actions with API writes. */
export type GlobalScenarioHarness = {
  coverage: InteractionCoverage;
  waitFor(check: () => boolean, label: string): Promise<void>;
  waitForAsync(check: () => Promise<boolean>, label: string): Promise<void>;
  click(element: HTMLElement, label: string): void;
  prepareClick(element: HTMLElement, label: string): Promise<void>;
  enter(element: HTMLInputElement | HTMLTextAreaElement, value: string): void;
  request<T>(path: string, method?: 'GET' | 'POST' | 'PUT' | 'DELETE', body?: object): Promise<T>;
};

function required<T extends HTMLElement>(selector: string, label: string) {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`Missing rendered ${label}: ${selector}`);
  return element;
}

/** F01/F02: changes a real rendered view and locale, with DOM state as evidence. */
export async function runShellNavigationScenario(harness: GlobalScenarioHarness) {
  harness.coverage.observe(document, 'global-shell');
  for (const view of ['search', 'compass', 'ai-setup', 'workbench']) {
    const button = required<HTMLButtonElement>(`[data-control="sidebar-view-${view}"]`, `${view} navigation`);
    harness.coverage.interact(`sidebar-view-${view}`, () => harness.click(button, `${view} navigation`));
    await harness.waitFor(() => document.getElementById(view)?.classList.contains('active') ?? false, `${view} active view`);
    harness.coverage.assertEffect(`sidebar-view-${view}`, () => document.getElementById(view)?.classList.contains('active') ?? false);
  }
  const locale = required<HTMLSelectElement>('[data-control="locale-select"]', 'locale selector');
  harness.coverage.interact('locale-select', () => { locale.value = 'ko'; locale.dispatchEvent(new Event('change', { bubbles: true })); });
  await harness.waitFor(() => document.documentElement.lang === 'ko', 'Korean locale');
  harness.coverage.assertEffect('locale-select', () => document.documentElement.lang === 'ko');
}

/** F36/F38: exercises rendered input + submit paths; callers supply the existing form handler/runtime. */
export function runSearchAndCompassInputScenario(harness: GlobalScenarioHarness) {
  harness.coverage.observe(document, 'global-search-compass');
  const search = required<HTMLInputElement>('[data-control="search-query"]', 'search query');
  harness.coverage.interact('search-query', () => harness.enter(search, 'task evidence'));
  harness.coverage.assertEffect('search-query', () => search.value === 'task evidence');
  const semantic = required<HTMLInputElement>('[data-control="search-semantic"]', 'semantic search toggle');
  harness.coverage.interact('search-semantic', () => semantic.click());
  harness.coverage.assertEffect('search-semantic', () => semantic.checked);
  const goal = required<HTMLInputElement>('[data-control="compass-goal-title"]', 'goal input');
  harness.coverage.interact('compass-goal-title', () => harness.enter(goal, 'Validate interactive controls'));
  harness.coverage.assertEffect('compass-goal-title', () => goal.value.includes('interactive'));
}

/** F36/F37: submits the shipping runtime form, then proves page/open/retry effects from rendered state. */
export async function runSearchScenario(harness: GlobalScenarioHarness, seededText: string) {
  harness.coverage.observe(document, 'global-search');
  const query = required<HTMLInputElement>('[data-control="search-query"]', 'search query');
  const semantic = required<HTMLInputElement>('[data-control="search-semantic"]', 'semantic search toggle');
  const form = required<HTMLFormElement>('#search-form', 'search form');
  harness.coverage.interact('search-query', () => harness.enter(query, seededText));
  harness.coverage.assertEffect('search-query', () => query.value === seededText);
  const semanticExpected = !semantic.checked;
  harness.coverage.interact('search-semantic', () => harness.click(semantic, 'toggle semantic search'));
  harness.coverage.assertEffect('search-semantic', () => semantic.checked === semanticExpected);
  harness.coverage.interact('search-submit', () => form.requestSubmit());
  await harness.waitFor(() => document.querySelectorAll('[data-knowledge-path]').length > 0, 'seeded search result');
  harness.coverage.assertEffect('search-submit', () => document.getElementById('results')?.textContent?.includes(seededText) ?? false);
  harness.coverage.observe(document, 'global-search-results');
  const more = required<HTMLButtonElement>('#more-results', 'Show 20 more search results');
  if (more.disabled) throw new Error('Search pagination fixture rendered a disabled Show 20 more action');
  const before = document.querySelectorAll('[data-knowledge-path]').length;
  harness.coverage.interact('search-more', () => harness.click(more, 'Show 20 more search results'));
  await harness.waitFor(() => document.querySelectorAll('[data-knowledge-path]').length > before, 'next search page');
  harness.coverage.assertEffect('search-more', () => document.querySelectorAll('[data-knowledge-path]').length > before);
  const result = required<HTMLElement>('[data-knowledge-path]', 'search result');
  harness.coverage.interact('search-result-open', () => harness.click(result, 'open seeded knowledge result'));
  await harness.waitFor(() => document.getElementById('item-detail-modal')?.hasAttribute('open') ?? false, 'knowledge reader');
  harness.coverage.assertEffect('search-result-open', () => Boolean(document.getElementById('item-detail-title')?.textContent));
  await harness.waitFor(() => Boolean(document.querySelector('[data-retry-knowledge]')), 'knowledge reader failure');
  const retry = required<HTMLElement>('[data-retry-knowledge]', 'knowledge reader retry');
  harness.coverage.observe(document, 'global-search-reader-failure');
  const path = retry.getAttribute('data-retry-knowledge');
  harness.coverage.interact('search-knowledge-retry', () => harness.click(retry, 'retry knowledge reader'));
  await harness.waitFor(() => !document.querySelector(`[data-retry-knowledge="${CSS.escape(path ?? '')}"]`), 'knowledge retry replacement');
  harness.coverage.assertEffect('search-knowledge-retry', () => !document.querySelector(`[data-retry-knowledge="${CSS.escape(path ?? '')}"]`));
}

/** F38: uses the rendered form and independently reads the durable dashboard projection. */
export async function runCompassGoalScenario(harness: GlobalScenarioHarness, title: string) {
  harness.coverage.observe(document, 'global-compass');
  const input = required<HTMLInputElement>('[data-control="compass-goal-title"]', 'goal title');
  const form = required<HTMLFormElement>('#goal-form', 'goal form');
  const submit = required<HTMLButtonElement>('#goal-form button', 'Add goal');
  harness.coverage.interact('compass-goal-title', () => harness.enter(input, title));
  harness.coverage.assertEffect('compass-goal-title', () => input.value === title);
  if (!input.validity.valid) throw new Error('Goal input unexpectedly invalid after entering the fixture title');
  harness.coverage.interact('compass-goal-save', () => harness.click(submit, 'Add goal'));
  await harness.waitFor(() => document.getElementById('dashboard')?.textContent?.includes(title) ?? false, 'rendered created goal');
  const dashboard = await harness.request<{ goals: Array<{ title: string }> }>('/dashboard');
  harness.coverage.assertEffect('compass-goal-save', () => dashboard.goals.some(goal => goal.title === title) && input.value === '');
  // Current shipping Compass has no edit/delete/step controls. This is a deliberate
  // reachability result, not an exclusion: adding any such enabled control makes
  // observe() fail until it has its own manifest id and semantic scenario.
  if (form.querySelector('[data-control*="edit"],[data-control*="delete"],[data-control*="step"]')) throw new Error('Compass edit/delete/step control needs an individual scenario registration');
}

/** F39/F40: fixture-only provider configuration; the secret is never read back or reported. */
export async function runProviderSettingsScenario(harness: GlobalScenarioHarness, fixture: { url: string; model: string; advancedModel: string; key: string }) {
  const form = required<HTMLFormElement>('#provider-form', 'provider form');
  harness.coverage.observe(document, 'provider-settings');
  const set = (id: string, value: string) => {
    const input = required<HTMLInputElement>(`[data-control="${id}"]`, id);
    harness.coverage.interact(id, () => harness.enter(input, value));
    harness.coverage.assertEffect(id, () => input.value === value);
  };
  set('provider-url', fixture.url); set('provider-model', fixture.model); set('provider-advanced-model', fixture.advancedModel);
  set('provider-key', fixture.key);
  const workerCount = required<HTMLInputElement>('[data-control="provider-worker-count"]', 'background worker count');
  harness.coverage.interact('provider-worker-count', () => harness.enter(workerCount, '3'));
  harness.coverage.assertEffect('provider-worker-count', () => workerCount.value === '3');
  const advanced = required<HTMLDetailsElement>('[data-control="provider-advanced-options"]', 'advanced model options');
  harness.coverage.interact('provider-advanced-options', () => harness.click(required<HTMLElement>('[data-control="provider-advanced-options"] summary', 'advanced options summary'), 'Advanced options'));
  harness.coverage.assertEffect('provider-advanced-options', () => advanced.open);
  const toggles = [...document.querySelectorAll<HTMLInputElement>('[data-advanced-task]')];
  if (toggles.length !== 13) throw new Error(`Expected 13 advanced task controls, found ${toggles.length}`);
  // Every model-kind route is independently exercised. A data-driven loop avoids
  // needless Cartesian combinations while still rejecting a missing toggle.
  for (const toggle of toggles) {
    const id = toggle.dataset.control;
    if (!id) throw new Error('Advanced model toggle lacks a data-control id');
    const expected = !toggle.checked;
    harness.coverage.interact(id, () => harness.click(toggle, id));
    harness.coverage.assertEffect(id, () => toggle.checked === expected);
  }
  const save = required<HTMLButtonElement>('#provider-form button[type="submit"], #provider-form button:not([type])', 'Save configuration');
  harness.coverage.interact('provider-save', () => harness.click(save, 'Save fixture provider configuration'));
  await harness.waitFor(() => document.getElementById('provider-status')?.textContent?.includes('Saved') ?? false, 'provider save status');
  const stored = await harness.request<{ base_url: string; model: string; advanced_model?: string; advanced_tasks: Record<string, boolean>; async_worker_count: number }>('/provider/config');
  harness.coverage.assertEffect('provider-save', () => stored.base_url === fixture.url && stored.model === fixture.model && stored.advanced_model === fixture.advancedModel && stored.async_worker_count === 3 && toggles.every(toggle => stored.advanced_tasks[toggle.dataset.advancedTask ?? ''] === toggle.checked) && required<HTMLInputElement>('#provider-key', 'provider key').value === '');
  harness.coverage.assertEffect('provider-worker-count', () => stored.async_worker_count === 3);
  const test = required<HTMLButtonElement>('#provider-test', 'Test connection');
  harness.coverage.interact('provider-test', () => harness.click(test, 'Test fixture provider'));
  await harness.waitFor(() => document.getElementById('provider-status')?.textContent?.includes(fixture.model) ?? false, 'provider model list');
  harness.coverage.assertEffect('provider-test', () => document.getElementById('provider-status')?.textContent?.includes(fixture.model) ?? false);
  form.reset();
}

/** F41–F43: uses rendered forms; scoped reads confirm each mutation without exposing command secrets. */
export async function runMcpSettingsScenario(harness: GlobalScenarioHarness, name: string, topic: string) {
  const section = required<HTMLElement>('.mcp-connections', 'MCP settings');
  const create = required<HTMLFormElement>('.mcp-connections > form', 'MCP create form');
  const nameInput = required<HTMLInputElement>('.mcp-connections > form input[name="name"]', 'connection name');
  harness.coverage.observe(document, 'mcp-settings');
  harness.coverage.interact('mcp-connection-name', () => harness.enter(nameInput, name));
  harness.coverage.assertEffect('mcp-connection-name', () => nameInput.value === name);
  const topicsInput = required<HTMLInputElement>('[data-control="mcp-connection-topics"]', 'connection topics');
  harness.coverage.interact('mcp-connection-topics', () => harness.enter(topicsInput, topic));
  harness.coverage.assertEffect('mcp-connection-topics', () => topicsInput.value === topic);
  const scopes = [...create.querySelectorAll<HTMLInputElement>('input[name="scope"]')];
  if (scopes.length !== 10) throw new Error(`Expected 10 MCP scope controls, found ${scopes.length}`);
  for (const scope of scopes) {
    const id = scope.dataset.control;
    if (!id) throw new Error('MCP scope lacks a data-control id');
    const expected = !scope.checked;
    harness.coverage.interact(id, () => harness.click(scope, id));
    harness.coverage.assertEffect(id, () => scope.checked === expected);
  }
  const selectedScopes = scopes.filter(scope => scope.checked).map(scope => scope.value).sort();
  harness.coverage.interact('mcp-connection-create', () => create.requestSubmit());
  await harness.waitFor(() => section.textContent?.includes(name) ?? false, 'created MCP connection');
  const connections = await harness.request<{ connections: Array<{ id: string; name: string; scopes: string[]; state: string }> }>('/work-tracking/connections');
  const connection = connections.connections.find(item => item.name === name);
  if (!connection) throw new Error('Created MCP connection was absent from the durable connection list');
  harness.coverage.assertEffect('mcp-connection-create', () => JSON.stringify([...connection.scopes].sort()) === JSON.stringify(selectedScopes));
  harness.coverage.observe(document, 'mcp-created-connection');
  const disclosure = required<HTMLDetailsElement>('[data-control="mcp-connection-access"]', 'connection access disclosure');
  harness.coverage.interact('mcp-connection-access', () => harness.click(required<HTMLElement>('[data-control="mcp-connection-access"] summary', 'connection access summary'), 'Connection access'));
  harness.coverage.assertEffect('mcp-connection-access', () => disclosure.open);
  const copy = disclosure.querySelector<HTMLElement>('button,[data-control*="copy"]');
  if (copy) throw new Error('MCP copy control requires its own UI interaction and clipboard assertion');
  const command = disclosure.querySelector<HTMLElement>('code');
  if (!command?.textContent?.includes(connection.id)) throw new Error('Rendered MCP connection command omitted its scoped connection id');
  const topicDetails = required<HTMLDetailsElement>('[data-control="mcp-topic-details"]', 'topic membership disclosure');
  harness.coverage.interact('mcp-topic-details', () => harness.click(required<HTMLElement>('[data-control="mcp-topic-details"] summary', 'topic membership summary'), 'Topic membership'));
  harness.coverage.assertEffect('mcp-topic-details', () => topicDetails.open);
  const membership = required<HTMLFormElement>('[data-control="mcp-topic-form"]', 'topic membership form');
  const setMembership = (id: string, value: string) => {
    const field = required<HTMLInputElement | HTMLSelectElement>(`[data-control="${id}"]`, id);
    harness.coverage.interact(id, () => {
      if (field instanceof HTMLSelectElement) { field.value = value; field.dispatchEvent(new Event('change', { bubbles: true })); }
      else harness.enter(field, value);
    });
    harness.coverage.assertEffect(id, () => field.value === value);
  };
  setMembership('mcp-topic-id', topic);
  setMembership('mcp-topic-type', 'problems');
  setMembership('mcp-topic-member', 'fixture-member');
  setMembership('mcp-topic-included', 'yes');
  harness.coverage.interact('mcp-topic-save', () => membership.requestSubmit());
  await harness.waitFor(() => document.querySelector('[role="status"]')?.textContent?.length ? true : false, 'topic membership status');
  harness.coverage.assertEffect('mcp-topic-save', () => Boolean(document.querySelector('[role="status"]')?.textContent));
  await harness.waitFor(() => {
    const control = document.querySelector<HTMLButtonElement>('[data-control="mcp-connection-revoke"]');
    return Boolean(control && !control.disabled);
  }, 'enabled Revoke fixture connection');
  const revoke = required<HTMLButtonElement>('[data-control="mcp-connection-revoke"]', 'Revoke connection');
  harness.coverage.interact('mcp-connection-revoke', () => harness.click(revoke, 'Revoke fixture connection'));
  await harness.waitFor(() => !revoke.isConnected, 'revoked connection action removed');
  const revoked = await harness.request<{ connections: Array<{ id: string; state: string }> }>('/work-tracking/connections');
  harness.coverage.assertEffect('mcp-connection-revoke', () => revoked.connections.some(item => item.id === connection.id && item.state === 'revoked') && !revoke.isConnected);
}

/** F50: production alert routing creates the notice; the tested action is the rendered dialog button. */
export async function runNoticeScenario(harness: GlobalScenarioHarness) {
  window.alert('Deterministic packaged notice');
  harness.coverage.observe(document, 'system-notice');
  const notice = required<HTMLButtonElement>('#notice-confirm', 'notice confirmation');
  harness.coverage.interact('notice-confirm', () => harness.click(notice, 'Confirm notice'));
  harness.coverage.assertEffect('notice-confirm', () => !(document.getElementById('notice-modal') as HTMLDialogElement).open);
}

/** F53–F55. `seed` is E2E-only native fixture setup; every operation below is a UI click. */
export type QueueNotificationFixture = { queued: string; running: string; failed: string; missingKey: string; completed: string; notificationOpen: string; notificationDismiss: string };
export async function runQueueNotificationActionsScenario(harness: GlobalScenarioHarness, seed: () => Promise<QueueNotificationFixture>, providerUrl?: string) {
  type Job = { id: string; status: string; error?: { code?: string } | null };
  type JobResult = { job_id: string; status: string; result: { path?: string; source_hash?: string; translated?: boolean } };
  type Notifications = { notifications: Array<{ id: string; read_at: string | null }> };
  harness.coverage.observe(document, 'queue-notification-shell');
  const alert = required<HTMLButtonElement>('#alert-toggle', 'notifications toggle');
  harness.coverage.interact('notification-toggle', () => harness.click(alert, 'open empty notifications'));
  await harness.waitFor(() => Boolean(document.querySelector('#alert-list .queue-empty')), 'initial empty notifications');
  harness.coverage.assertEffect('notification-toggle', () => required<HTMLElement>('#alert-panel', 'notification panel').hidden === false);
  harness.coverage.observe(document, 'empty-notification-panel');
  const initialNotificationClose = required<HTMLButtonElement>('[data-control="notification-panel-close"]', 'notification panel close');
  harness.coverage.interact('notification-panel-close', () => harness.click(initialNotificationClose, 'close empty notification panel'));
  harness.coverage.assertEffect('notification-panel-close', () => required<HTMLElement>('#alert-panel', 'notification panel').hidden && alert.getAttribute('aria-expanded') === 'false');
  const ids = await seed();
  const decideConflict = async (state: 'conflicted' | 'clear', noteText: string, scenario: string) => {
    harness.coverage.observe(document, scenario);
    const note = required<HTMLTextAreaElement>('[data-control="conflict-decision-note"]', 'conflict decision note');
    harness.coverage.interact('conflict-decision-note', () => harness.enter(note, noteText));
    const decision = required<HTMLButtonElement>(`[data-control="conflict-decision-${state}"]`, `${state} conflict decision`);
    harness.coverage.interact(`conflict-decision-${state}`, () => harness.click(decision, `${state} conflict decision`));
    await harness.waitFor(() => !(document.getElementById('item-detail-modal') as HTMLDialogElement).open, `${state} conflict decision close`);
    const saved = await harness.request<{ conflict_state: string }>('/items/features/fixture-solution');
    harness.coverage.assertEffect('conflict-decision-note', () => note.value === noteText && saved.conflict_state === state);
    harness.coverage.assertEffect(`conflict-decision-${state}`, () => saved.conflict_state === state && !(document.getElementById('item-detail-modal') as HTMLDialogElement).open);
  };
  const queue = required<HTMLButtonElement>('#queue-toggle', 'queue toggle'); harness.coverage.interact('queue-toggle', () => harness.click(queue, 'open queue'));
  await harness.waitFor(() => Boolean(document.querySelector(`#queue-list [data-job-id="${ids.queued}"]`)), 'seeded queued job');
  harness.coverage.assertEffect('queue-toggle', () => required<HTMLElement>('#queue-panel', 'queue panel').hidden === false && queue.getAttribute('aria-expanded') === 'true');
  harness.coverage.observe(document, 'queue-actions');
  const setup = required<HTMLButtonElement>(`#queue-list [data-job-id="${ids.missingKey}"] [data-job-action="setup"]`, 'missing API key setup action');
  harness.coverage.interact('queue-open-ai-setup', () => harness.click(setup, 'open AI setup for missing key job'));
  await harness.waitFor(() => document.getElementById('ai-setup')?.classList.contains('active') === true, 'AI setup from Queue');
  harness.coverage.assertEffect('queue-open-ai-setup', () => document.getElementById('ai-setup')?.classList.contains('active') === true && Boolean(document.querySelector(`#queue-list [data-job-id="${ids.missingKey}"]`)));
  if (providerUrl) {
    const set = (id: string, value: string) => harness.enter(required<HTMLInputElement>(`#${id}`, id), value);
    set('provider-url', providerUrl);
    set('provider-model', 'deterministic-test-model');
    set('provider-key', 'desktop-e2e-key');
    harness.click(required<HTMLElement>('[data-control="provider-save"]', 'save AI setup'), 'save AI setup for retry');
    await harness.waitFor(() => document.getElementById('provider-status')?.textContent?.includes('Saved') === true, 'saved AI setup before retry');
  }
  harness.click(required<HTMLElement>('[data-view="workbench"]', 'Workbench navigation'), 'return to Workbench before retry');
  await harness.waitFor(() => document.getElementById('workbench')?.classList.contains('active') === true, 'Workbench after saving AI setup');
  const queuePanel = required<HTMLElement>('#queue-panel', 'queue panel after AI setup');
  if (queuePanel.hidden) harness.click(queue, 'reopen Queue after saving AI setup');
  await harness.waitFor(() => queuePanel.hidden === false, 'visible Queue after saving AI setup');
  const missingRetry = required<HTMLButtonElement>(`#queue-list [data-job-id="${ids.missingKey}"] [data-job-action="retry"]`, 'missing API key retry action');
  harness.coverage.interact('queue-retry', () => harness.click(missingRetry, 'retry the same missing-key job'));
  let recovered: Job | undefined;
  await harness.waitForAsync(async () => {
    recovered = await harness.request<Job>(`/jobs/${ids.missingKey}`);
    return recovered.status === 'completed';
  }, 'missing-key knowledge translation completes after saving AI setup');
  const completedResult = await harness.request<JobResult>(`/jobs/${ids.missingKey}/result`);
  const translation = await harness.request<{ path?: string; translated?: boolean; served_locale?: string; cache_status?: string; source_hash?: string; markdown?: string }>('/knowledge?path=queue-recovery.md&locale=ko');
  harness.coverage.assertEffect('queue-retry', () =>
    recovered?.id === ids.missingKey &&
    recovered.status === 'completed' &&
    completedResult.job_id === ids.missingKey &&
    completedResult.status === 'completed' &&
    completedResult.result.path === 'queue-recovery.md' &&
    completedResult.result.translated === true &&
    translation.path === 'queue-recovery.md' &&
    translation.translated === true &&
    translation.served_locale === 'ko' &&
    translation.cache_status === 'hit' &&
    translation.source_hash === completedResult.result.source_hash &&
    translation.markdown?.includes('# 기존 맥락') === true,
  );
  for (const [id, action, job] of [['queue-cancel', 'cancel', ids.queued], ['queue-retry', 'retry', ids.failed]] as const) {
    const button = required<HTMLButtonElement>(`#queue-list [data-job-id="${job}"] [data-job-action="${action}"]`, `${action} job action`);
    harness.coverage.interact(id, () => harness.click(button, id));
    let persisted: Job | undefined;
    await harness.waitForAsync(async () => {
      persisted = await harness.request<Job>(`/jobs/${job}`);
      return action === 'cancel'
        ? persisted.status === 'cancelled'
        : persisted.status === 'failed' && persisted.error?.code === 'application_error';
    }, `${id} durable completion`);
    harness.coverage.assertEffect(id, () => action === 'cancel'
      ? persisted?.status === 'cancelled'
      : persisted?.status === 'failed' && persisted.error?.code === 'application_error');
  }
  const runningCancel = required<HTMLButtonElement>(`#queue-list [data-job-id="${ids.running}"] [data-job-action="cancel"]`, 'running job cancel');
  harness.coverage.interact('queue-cancel', () => harness.click(runningCancel, 'cancel running job'));
  let cancelledRunning: Job | undefined;
  await harness.waitForAsync(async () => {
    cancelledRunning = await harness.request<Job>(`/jobs/${ids.running}`);
    return cancelledRunning.status === 'cancelled';
  }, 'running job durable cancellation');
  harness.coverage.assertEffect('queue-cancel', () => cancelledRunning?.status === 'cancelled');
  const queueClose = required<HTMLButtonElement>('[data-control="queue-panel-close"]', 'queue panel close');
  harness.coverage.interact('queue-panel-close', () => harness.click(queueClose, 'close queue panel'));
  harness.coverage.assertEffect('queue-panel-close', () => required<HTMLElement>('#queue-panel', 'queue panel').hidden && queue.getAttribute('aria-expanded') === 'false');
  harness.coverage.interact('queue-toggle', () => harness.click(queue, 'reopen queue for completed result'));
  await harness.waitFor(() => required<HTMLElement>('#queue-toast', 'queue toast').hidden === false, 'completed result notification toast');
  harness.coverage.observe(document, 'notification-toast');
  const toastDismiss = required<HTMLButtonElement>('[data-control="queue-toast-dismiss"]', 'toast dismiss');
  await harness.prepareClick(toastDismiss, 'dismiss completed result toast');
  harness.coverage.interact('queue-toast-dismiss', () => harness.click(toastDismiss, 'dismiss completed result toast'));
  await harness.waitFor(() => required<HTMLElement>('#queue-toast', 'queue toast').hidden, 'dismissed completed result notification toast');
  harness.coverage.assertEffect('queue-toast-dismiss', () => required<HTMLElement>('#queue-toast', 'queue toast').hidden);
  await harness.waitFor(() => Boolean(document.querySelector(`#queue-list [data-job-id="${ids.completed}"] [data-job-action="result"]`)), 'completed job after reopening queue');
  const resultButton = required<HTMLButtonElement>(`#queue-list [data-job-id="${ids.completed}"] [data-job-action="result"]`, 'completed job result');
  await harness.prepareClick(resultButton, 'open completed job result');
  harness.coverage.interact('queue-open-result', () => harness.click(resultButton, 'open completed job result'));
  await harness.waitFor(() => Boolean(document.querySelector('#item-detail-modal[open]')), 'completed job result detail');
  harness.coverage.assertEffect('queue-open-result', () => Boolean(document.querySelector('#item-detail-modal[open]')));
  harness.coverage.observe(document, 'queue-result-detail');
  await decideConflict('conflicted', 'Queue result reviewed with preserved conflict evidence', 'queue-result-conflict-decision');
  harness.coverage.interact('queue-toggle', () => harness.click(queue, 'reopen queue to verify detail close'));
  await harness.waitFor(() => Boolean(document.querySelector(`#queue-list [data-job-id="${ids.completed}"] [data-job-action="result"]`)), 'completed result for detail close');
  const detailResultButton = required<HTMLButtonElement>(`#queue-list [data-job-id="${ids.completed}"] [data-job-action="result"]`, 'completed result for detail close');
  harness.coverage.interact('queue-open-result', () => harness.click(detailResultButton, 'reopen completed job result'));
  await harness.waitFor(() => Boolean(document.querySelector('#item-detail-modal[open]')), 'reopened completed job result detail');
  const queueDetailClose = required<HTMLButtonElement>('#item-detail-close', 'queue result detail close');
  harness.coverage.interact('detail-close', () => harness.click(queueDetailClose, 'close queue result'));
  harness.coverage.assertEffect('detail-close', () => !(document.getElementById('item-detail-modal') as HTMLDialogElement).open);
  harness.coverage.interact('notification-toggle', () => harness.click(alert, 'open notifications'));
  await harness.waitFor(() => Boolean(document.querySelector(`#alert-list [data-notification-id="${ids.notificationOpen}"]`)), 'seeded unread notification');
  harness.coverage.observe(document, 'notification-actions');
  for (const action of ['dismiss', 'open'] as const) {
    const notification = action === 'open' ? ids.notificationOpen : ids.notificationDismiss;
    const button = required<HTMLButtonElement>(`#alert-list [data-notification-id="${notification}"] [data-notification-action="${action}"]`, `${action} notification`);
    const id = `notification-${action}`;
    harness.coverage.interact(id, () => harness.click(button, `${action} notification`));
    let notifications: Notifications | undefined;
    await harness.waitForAsync(async () => {
      notifications = await harness.request<Notifications>('/notifications');
      const item = notifications.notifications.find(candidate => candidate.id === notification);
      return action === 'open'
        ? Boolean(item?.read_at) && Boolean(document.querySelector('#item-detail-modal[open]'))
        : !item && !button.isConnected;
    }, `${action} notification durable effect`);
    harness.coverage.assertEffect(id, () => {
      const item = notifications?.notifications.find(candidate => candidate.id === notification);
      return action === 'open'
        ? Boolean(item?.read_at) && Boolean(document.querySelector('#item-detail-modal[open]'))
        : !item && !button.isConnected;
    });
    if (action === 'open') {
      harness.coverage.observe(document, 'notification-result-detail');
      await decideConflict('clear', 'Notification result reviewed and cleared', 'notification-result-conflict-decision');
    }
  }
}

/** Startup surfaces run in distinct application processes; aggregate their id reports after all six fixtures. */
export async function runFirstRunIntroNavigationScenario(harness: GlobalScenarioHarness) {
  harness.coverage.observe(document, 'first-run-intro-navigation');
  const scene = required<HTMLButtonElement>('[data-control="intro-scene"]:not([aria-current="step"])', 'non-current intro progress dot');
  harness.coverage.interact('intro-scene', () => harness.click(scene, 'change intro scene'));
  await harness.waitFor(() => scene.getAttribute('aria-current') === 'step', 'selected intro scene');
  harness.coverage.assertEffect('intro-scene', () => scene.getAttribute('aria-current') === 'step');
  const before = document.querySelector('[data-control="intro-scene"][aria-current="step"]')?.getAttribute('aria-label');
  const next = required<HTMLButtonElement>('[data-control="intro-next"]', 'intro Continue');
  harness.coverage.interact('intro-next', () => harness.click(next, 'advance intro'));
  await harness.waitFor(() => document.querySelector('[data-control="intro-scene"][aria-current="step"]')?.getAttribute('aria-label') !== before, 'advanced intro scene');
  harness.coverage.assertEffect('intro-next', () => document.querySelector('[data-control="intro-scene"][aria-current="step"]')?.getAttribute('aria-label') !== before);
  const skip = required<HTMLButtonElement>('[data-control="intro-skip"]', 'intro Skip');
  harness.coverage.interact('intro-skip', () => harness.click(skip, 'finish intro'));
  await harness.waitFor(() => skip.disabled || !skip.isConnected, 'intro completion start');
  harness.coverage.assertEffect('intro-skip', () => skip.disabled || !skip.isConnected);
}

export async function runFirstRunIntroRetryScenario(harness: GlobalScenarioHarness) {
  harness.coverage.observe(document, 'first-run-intro-retry');
  const skip = required<HTMLButtonElement>('[data-control="intro-skip"]', 'intro Skip before retry');
  harness.coverage.interact('intro-skip', () => harness.click(skip, 'trigger one-shot intro completion failure'));
  await harness.waitFor(() => Boolean(document.querySelector('[data-control="intro-retry"]')), 'intro retry after fixture failure');
  harness.coverage.assertEffect('intro-skip', () => Boolean(document.querySelector('[data-control="intro-retry"]')));
  harness.coverage.observe(document, 'first-run-intro-retry-visible');
  const retry = required<HTMLButtonElement>('[data-control="intro-retry"]', 'intro completion retry');
  const previousAlert = document.querySelector('[role="alert"]')?.textContent;
  harness.coverage.interact('intro-retry', () => harness.click(retry, 'retry intro completion'));
  await harness.waitFor(() => retry.disabled || !retry.isConnected || document.querySelector('[role="alert"]')?.textContent !== previousAlert, 'intro retry result');
  harness.coverage.assertEffect('intro-retry', () => retry.disabled || !retry.isConnected || document.querySelector('[role="alert"]')?.textContent !== previousAlert);
}

export async function runVaultChooseScenario(harness: GlobalScenarioHarness) {
  await harness.waitFor(() => {
    const control = document.querySelector<HTMLButtonElement>('[data-control="vault-setup-choose"]');
    return Boolean(control && !control.disabled);
  }, 'enabled Choose Vault folder');
  harness.coverage.observe(document, 'vault-setup-choose');
  const choose = required<HTMLButtonElement>('[data-control="vault-setup-choose"]', 'Choose Vault folder');
  harness.coverage.interact('vault-setup-choose', () => harness.click(choose, 'Choose Vault folder'));
  await harness.waitFor(() => choose.disabled && choose.getAttribute('aria-busy') === 'true', 'native Vault picker request');
  harness.coverage.assertEffect('vault-setup-choose', () => choose.disabled && choose.getAttribute('aria-busy') === 'true');
}

export async function runVaultRetryScenario(harness: GlobalScenarioHarness) {
  await harness.waitFor(() => Boolean(document.querySelector('[data-control="vault-setup-retry"]')), 'Vault setup error controls');
  harness.coverage.observe(document, 'vault-setup-error');
  const retry = required<HTMLButtonElement>('[data-control="vault-setup-retry"]', 'Vault setup retry');
  const previousAlert = document.querySelector('[role="alert"]')?.textContent;
  harness.coverage.interact('vault-setup-retry', () => harness.click(retry, 'Retry Vault setup'));
  await harness.waitFor(() => !retry.isConnected || document.querySelector('[role="alert"]')?.textContent !== previousAlert, 'Vault retry result');
  harness.coverage.assertEffect('vault-setup-retry', () => !retry.isConnected || document.querySelector('[role="alert"]')?.textContent !== previousAlert);
}

async function runMigrationActionScenario(harness: GlobalScenarioHarness, id: 'migration-recovery-restore' | 'migration-recovery-retry') {
  await harness.waitFor(() => {
    const control = document.querySelector<HTMLButtonElement>(`[data-control="${id}"]`);
    return Boolean(control && !control.disabled);
  }, `enabled ${id}`);
  harness.coverage.observe(document, id);
  const control = required<HTMLButtonElement>(`[data-control="${id}"]`, id);
  const previousAlert = document.querySelector('.vault-setup-card [role="alert"]')?.textContent;
  harness.coverage.interact(id, () => harness.click(control, id));
  await harness.waitFor(() => control.disabled || !control.isConnected || document.querySelector('.vault-setup-card [role="alert"]')?.textContent !== previousAlert, `${id} result`);
  harness.coverage.assertEffect(id, () => control.disabled || !control.isConnected || document.querySelector('.vault-setup-card [role="alert"]')?.textContent !== previousAlert);
}

export const runMigrationRestoreScenario = (harness: GlobalScenarioHarness) => runMigrationActionScenario(harness, 'migration-recovery-restore');
export const runMigrationRetryScenario = (harness: GlobalScenarioHarness) => runMigrationActionScenario(harness, 'migration-recovery-retry');
