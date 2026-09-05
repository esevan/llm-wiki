import conflictsSource from '../../public/runtime/conflicts.js?raw';
import exploreSource from '../../public/runtime/explore.js?raw';
import manualSource from '../../public/runtime/manual.js?raw';
import searchSource from '../../public/runtime/search-settings.js?raw';
import solutionWorkSource from '../../public/runtime/solution-work.js?raw';
import transitionsSource from '../../public/runtime/transitions.js?raw';
import workbenchSource from '../../public/runtime/workbench.js?raw';
// jsdom is Vitest's runtime dependency; this repository intentionally does not ship its optional types.
// @ts-expect-error -- runtime-only test dependency without a declaration package
import { JSDOM } from 'jsdom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

type AnyFunction = (...args: any[]) => any;
const $ = (selector: string) => document.querySelector(selector) as any;
const flush = () => new Promise(resolve => setTimeout(resolve, 0));
const dialog = (id: string) => `<dialog id="${id}"></dialog>`;

function runModule<T>(source: string, dependencies: Record<string, unknown>, returned: string[]): T {
  return new Function(...Object.keys(dependencies), `${source}\nreturn {${returned.join(',')}};`)(...Object.values(dependencies));
}

function executableDom(html: string, globals: Record<string, unknown>, source: string) {
  const realm = new JSDOM(`<body>${html}</body>`, { runScripts: 'dangerously', url: 'http://runtime.test' });
  Object.assign(realm.window, globals);
  (realm.window as any).$ = (selector: string) => realm.window.document.querySelector(selector);
  realm.window.eval(source);
  return realm;
}

beforeEach(() => {
  document.body.innerHTML = '';
  vi.restoreAllMocks();
  for (const method of ['showModal', 'close']) {
    if (!(HTMLDialogElement.prototype as any)[method]) {
      Object.defineProperty(HTMLDialogElement.prototype, method, {
        configurable: true,
        value() { this.open = method === 'showModal'; },
      });
    }
  }
});

describe('production Workbench button templates', () => {
  it('renders and clicks menus, priority, manual, workflow, archive, flow, and organize actions', async () => {
    document.body.innerHTML = `
      <form id="capture"><input id="capture-text"></form><section id="board"></section>
      <section id="in-progress-solutions"></section><section class="workbench-context"><div id="recent-archive"></div><div id="completed-solutions"></div></section>
      <button id="archive-more"></button><button id="flow-toggle"></button><div id="flow-view" hidden></div>
      <button id="organize">◎</button><div id="organize-status"></div>
      ${dialog('item-detail-modal')}<div id="item-detail-type"></div><div id="item-detail-title"></div><dl id="item-detail-notes"></dl>
    `;
    const board = {
      captures: [{ id: 'c1', text: 'Capture', manual_priority: 0 }],
      problems: [
        { id: 'p1', statement: 'Pending', state: 'proposed', manual_priority: 0 },
        { id: 'p2', statement: 'Approved', state: 'approved', manual_priority: 0 },
      ],
      features: [
        { id: 's1', problem_id: 'p2', title: 'Clear', outcome: 'Ready', state: 'proposed', conflict_state: 'clear', manual_priority: 0 },
        { id: 's2', problem_id: 'p2', title: 'Conflict', outcome: 'Review', state: 'proposed', conflict_state: 'potential_conflict', manual_priority: 0 },
        { id: 's3', problem_id: 'p2', title: 'Active', outcome: 'Doing', state: 'approved', conflict_state: 'clear', manual_priority: 0 },
      ],
    };
    const api = vi.fn(async (path: string) => {
      if (path === '/board') return board;
      if (path.startsWith('/workbench/recent-archive')) return { documents: [{ path: 'done.md', title: 'Done' }] };
      if (path.startsWith('/workbench/completed-solutions')) return { solutions: [{ id: 'done', title: 'Done solution', problem_id: 'p2' }] };
      return {};
    });
    const calls = {
      manual: vi.fn(), transition: vi.fn(), conflict: vi.fn(), move: vi.fn(), handoff: vi.fn(),
      approve: vi.fn(), next: vi.fn(), remove: vi.fn(), archive: vi.fn(), completed: vi.fn(), refresh: vi.fn(async () => undefined), detail: vi.fn(),
    };
    const transitionMenuItem = (id: string, type: string, entityId: string) => `<button onclick="openTransition('${id}','${type}','${entityId}')">Transition ${id}</button>`;
    const setImportant = new Function('$', 'api', 'loadBoard', `${conflictsSource.split('\n').find(line => line.startsWith('async function setImportant('))};return setImportant;`)($, api, vi.fn());
    const runtime = runModule<Record<string, AnyFunction>>(workbenchSource, {
      $, api, esc: String, t: (key: string) => key, transitionMenuItem,
      setImportant,
      copyHandoff: calls.handoff, moveSolution: calls.move, runConflictReview: calls.conflict, openTransition: calls.transition,
      renderQueue: vi.fn(), refreshQueue: calls.refresh, openChat: vi.fn(),
      featureModal: $('dialog'), itemDetailModal: $('#item-detail-modal'), activeLocale: 'en',
      detachKnowledgeTranslationReader: vi.fn(), streamKnowledgeTranslation: vi.fn(),
      applicationRequest: vi.fn(), navigator, alert: vi.fn(), askNotice: vi.fn(), showNotice: vi.fn(),
    }, ['loadBoard', 'searchArchivedDocument', 'openCompletedDetail']);
    runtime.searchArchivedDocument = calls.archive;
    runtime.openCompletedDetail = calls.completed;
    await runtime.loadBoard();
    await flush();

    const firstMenu = document.querySelector<HTMLDetailsElement>('.card-menu')!;
    firstMenu.querySelector('summary')!.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    expect(firstMenu.open).toBe(true);

    const important = [...document.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent === 'Mark important')!;
    const importantRealm = executableDom(important.outerHTML + '<div id="organize-status"></div>', { api, loadBoard: vi.fn() }, conflictsSource.split('\n').find(line => line.startsWith('async function setImportant('))!);
    importantRealm.window.document.querySelector<HTMLButtonElement>('button')!.click();
    await flush();
    expect(api).toHaveBeenCalledWith('/workbench/importance', expect.objectContaining({ method: 'PUT' }));

    const manual = document.querySelector<HTMLButtonElement>('[data-manual-type="problems"]')!;
    const openManualItem = vi.fn();
    const manualClickBinding = manualSource.split('\n').find(line => line.startsWith("$('#board').addEventListener"))!;
    new Function('$', 'openManualItem', manualClickBinding)($, openManualItem);
    manual.click();
    expect(openManualItem).toHaveBeenCalledWith('problems', 'p1');

    const boardClickBinding = conflictsSource.split('\n').find(line => line.startsWith("$('#board').onclick"))!;
    new Function('$', 'approveProblem', 'openNextChat', 'runConflictReview', 'reviewCompletion', 'moveSolution', 'deleteItem', 'openItemDetail', boardClickBinding)($, calls.approve, calls.next, calls.conflict, vi.fn(), calls.move, calls.remove, calls.detail);
    document.querySelector<HTMLButtonElement>('[data-approve-problem]')!.click();
    document.querySelector<HTMLButtonElement>('[data-next-chat-id]')!.click();
    const moveButton = [...document.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent === '▶')!;
    const moveRealm = executableDom(moveButton.outerHTML + '<div id="organize-status"></div>', { api, loadBoard: vi.fn() }, conflictsSource.split('\n').find(line => line.startsWith('async function moveSolution('))!);
    moveRealm.window.document.querySelector<HTMLButtonElement>('button')!.click();
    const conflictButton = [...document.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent === '↯')!;
    const conflictFunction = conflictsSource.slice(conflictsSource.indexOf('async function runConflictReview('), conflictsSource.indexOf('function showConflictReviewResult('));
    const openChat = vi.fn();
    const conflictRealm = executableDom(conflictButton.outerHTML, { openChat, showNotice: vi.fn(), conflictCopy: (_key: string, fallback: string) => fallback }, conflictFunction);
    conflictRealm.window.document.querySelector<HTMLButtonElement>('button')!.click();
    const copyButton = [...document.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent === 'Copy handoff')!;
    const clipboard = { writeText: vi.fn(async () => undefined) };
    const copyRealm = executableDom(copyButton.outerHTML, { applicationRequest: vi.fn(async () => new Response('handoff')), activeLocale: 'en', showNotice: vi.fn() }, solutionWorkSource.split('\n').find(line => line.startsWith('copyHandoff=async function'))!);
    Object.defineProperty(copyRealm.window.navigator, 'clipboard', { configurable: true, value: clipboard });
    copyRealm.window.document.querySelector<HTMLButtonElement>('button')!.click();
    expect([...document.querySelectorAll<HTMLButtonElement>('button')].find(button => button.textContent?.includes('capture_to_problem'))!.getAttribute('onclick')).toContain('openTransition');
    const deleteButton = document.querySelector<HTMLButtonElement>('[data-delete-type="captures"]')!;
    const deleteRealm = executableDom(`<section id="board">${deleteButton.outerHTML}</section>`, {
      api, askNotice: vi.fn(async () => true), loadBoard: vi.fn(), approveProblem: vi.fn(), openNextChat: vi.fn(),
      runConflictReview: vi.fn(), reviewCompletion: vi.fn(), moveSolution: vi.fn(), openItemDetail: vi.fn(), showNotice: vi.fn(),
    }, solutionWorkSource.split('\n').find(line => line.startsWith('deleteItem=async function'))! + '\n' + boardClickBinding);
    deleteRealm.window.document.querySelector<HTMLButtonElement>('button')!.click();
    await flush();
    expect(api).toHaveBeenCalledWith('/features/s1/stage', expect.objectContaining({ method: 'PUT' }));
    expect(openChat).toHaveBeenCalledWith('features', 's2');
    expect(clipboard.writeText).toHaveBeenCalledWith('handoff');
    expect(api).toHaveBeenCalledWith('/items/captures/c1', { method: 'DELETE' });
    expect(calls.approve).toHaveBeenCalledWith('p1', expect.any(HTMLButtonElement));
    expect(calls.next).toHaveBeenCalledWith('problems', 'p2');

    $('.workbench-context [data-archive-path]').click();
    await flush();
    expect(api).toHaveBeenCalledWith('/knowledge?path=done.md&locale=en&translate=false');
    $('.workbench-context [data-completed-id]').click();
    expect($('#item-detail-title').textContent).toBe('Done solution');

    $('#flow-toggle').click();
    expect($('#flow-view').hidden).toBe(false);
    $('#archive-more').click();
    expect($('#archive-more').getAttribute('aria-label')).toContain('fewer');
    $('#organize').click();
    await flush();
    expect(api).toHaveBeenCalledWith('/workbench/organize', { method: 'POST' });
    expect(calls.refresh).toHaveBeenCalled();
  });
});

describe('production manual, search, and provider controls', () => {
  it('submits feature and reviewed draft forms from their rendered primary buttons', async () => {
    document.body.innerHTML = `
      ${dialog('feature-modal')}${dialog('draft-modal')}
      <form id="feature-form"><input id="feature-problem" value="p"><input id="feature-title" value="Solution"><textarea id="feature-outcome">Outcome</textarea><textarea id="feature-nongoals">None</textarea><textarea id="feature-validation">- [ ] Verified</textarea><button>Save proposal</button></form>
      <form id="draft-form"><input id="draft-type" value="problems"><input id="draft-id" value="p"><input id="draft-main" value="Draft"><textarea id="draft-detail">Outcome</textarea><textarea id="draft-extra">None</textarea><textarea id="draft-validation">- [ ] Verified</textarea><button id="draft-submit">Finalize</button></form>
    `;
    const api = vi.fn(async () => ({}));
    const loadBoard = vi.fn();
    const showNotice = vi.fn();
    const alert = vi.fn();
    const handlers = solutionWorkSource.split('\n').filter(line => line.startsWith("$('#draft-form').onsubmit") || line.startsWith("$('#feature-form').onsubmit")).join('\n');
    new Function('$', 'api', 'draftModal', 'featureModal', 'loadBoard', 'showNotice', 'alert', handlers)($, api, $('#draft-modal'), $('#feature-modal'), loadBoard, showNotice, alert);
    ($('#feature-form button') as HTMLButtonElement).click();
    await flush();
    expect(api).toHaveBeenCalledWith('/problems/p/features', expect.objectContaining({ body: expect.stringContaining('validation_criteria') }));
    ($('#draft-submit') as HTMLButtonElement).click();
    await flush();
    expect(api).toHaveBeenCalledTimes(2);
    expect(loadBoard).toHaveBeenCalledTimes(2);
    expect($('#draft-submit').disabled).toBe(false);
  });

  it('submits manual edits through the rendered save button and restores it after failure', async () => {
    document.body.innerHTML = `${dialog('manual-modal')}<form id="manual-form"><input id="manual-type" value="problems"><input id="manual-id" value="p"><input id="manual-title" value="Title"><textarea id="manual-detail">Detail</textarea><input id="manual-localized-only" type="checkbox"><button aria-label="Save manually">Save</button></form><section id="board"></section><form id="feature-form"><input id="feature-problem"><input id="feature-title"><textarea id="feature-outcome"></textarea><textarea id="feature-nongoals"></textarea></form>`;
    const api = vi.fn().mockResolvedValueOnce({}).mockRejectedValueOnce(new Error('offline'));
    const notice = vi.fn();
    runModule(manualSource, { $, api, manualModal: $('#manual-modal'), activeLocale: 'en', scheduleLocaleApply: vi.fn(), showNotice: notice, loadBoard: vi.fn(), applicationRequest: vi.fn(), navigator, alert: vi.fn(), window }, []);
    const form = $('#manual-form') as HTMLFormElement;
    const save = form.querySelector('button')!;
    save.click();
    await flush();
    expect(api).toHaveBeenCalledWith('/items/problems/p', expect.objectContaining({ method: 'PUT' }));
    save.click();
    await flush();
    expect(notice).toHaveBeenCalledWith('offline', 'Manual update failed');
    expect(save.disabled).toBe(false);
  });

  it('clicks search more/open/retry and provider save/test with success and error outcomes', async () => {
    document.body.innerHTML = `
      <form id="search-form"><input id="query" value="wiki"><input id="semantic" type="checkbox"><button>Search</button></form><div id="results"></div>
      <div id="item-detail-notes"><button data-retry-knowledge="retry.md"></button></div>
      <form id="goal-form"><input id="goal-title" value="Ship audit"><button>Save goal</button></form><div id="dashboard"></div>
      <form id="provider-form"><details></details><input id="provider-url" value="http://local"><input id="provider-model"><input id="provider-advanced-model"><input id="provider-key" value="secret"><input data-advanced-task="review" type="checkbox"><button>Save</button></form>
      <button id="provider-test"></button><div id="provider-status"></div>
    `;
    const twenty = Array.from({ length: 20 }, (_, index) => ({ path: `p${index}.md`, title: `T${index}`, snippet: '' }));
    const api = vi.fn(async (path: string) => {
      if (path === '/provider/config') return { base_url: '', model: '', advanced_tasks: {} };
      if (path.startsWith('/search')) return { results: twenty };
      if (path === '/provider/test') return { models: ['model-a'] };
      if (path === '/dashboard') return { goals: [], events: [] };
      if (path === '/provider/config') return {};
      return { api_key_configured: true, advanced_tasks: {} };
    });
    const open = vi.fn();
    runModule(searchSource, { $, api, esc: String, t: (key: string) => key, searchArchivedDocument: open }, []);
    await flush();
    ($('#search-form button') as HTMLButtonElement).click();
    await flush();
    $('#more-results').click();
    await flush();
    $('[data-knowledge-path]').click();
    $('[data-retry-knowledge]').click();
    expect(api.mock.calls.filter(([path]) => String(path).startsWith('/search')).length).toBe(2);
    expect(open).toHaveBeenCalledWith('p0.md');
    expect(open).toHaveBeenCalledWith('retry.md');
    ($('#goal-form button') as HTMLButtonElement).click();
    await flush();
    expect(api).toHaveBeenCalledWith('/goals', expect.objectContaining({ method: 'POST' }));
    ($('#provider-form button') as HTMLButtonElement).click();
    await flush();
    $('#provider-test').click();
    await flush();
    expect(api).toHaveBeenCalledWith('/provider/config', expect.objectContaining({ method: 'PUT' }));
    expect($('#provider-status').textContent).toContain('model-a');

    api.mockRejectedValueOnce(new Error('provider down'));
    $('#provider-test').click();
    await flush();
    expect($('#provider-status').textContent).toContain('provider down');
  });
});

describe('production Explore controls', () => {
  it('clicks draft, tabs, apply, chat send, and preview retry through the rendered handlers', async () => {
    document.body.innerHTML = `
      <dialog id="chat-modal"><section class="modal"><button id="refinement-preview-warning" hidden>!</button><h2 id="chat-title"></h2><div id="chat-log"></div><form id="chat-form"><label></label><textarea id="chat-message"></textarea><button class="primary">Send</button></form></section></dialog>
      <aside id="explore-refinement-preview"><button id="preview-job-status" data-state="idle"></button><span id="explore-preview-status"></span><button id="preview-detail-tab"></button><button id="preview-context-tab"></button><div id="explore-preview-detail"></div><div id="explore-preview-content"></div></aside>
    `;
    const api = vi.fn(async (path: string) => path.endsWith('/refinement-context') ? ({ has_context: true, entries: [{ label: 'Context', text: 'Saved' }] }) : ({}));
    const draftWithAI = vi.fn();
    const applicationRequest = vi.fn(async () => new Response('data: ✅ Ready. Your AI refinement is ready to review.\n\nevent: done\ndata: done\n\n'));
    const close = vi.fn();
    const loadBoard = vi.fn();
    const clearWarning = () => { $('#refinement-preview-warning').hidden = true; };
    const showWarning = () => { $('#refinement-preview-warning').hidden = false; };
    const runtime = runModule<Record<string, AnyFunction>>(exploreSource, {
      $, esc: String, api, chatModal: $('#chat-modal'), refinementPreviewDraft: null, previewActiveTab: 'context',
      supersedeRefinementPreview: vi.fn(), clearRefinementPreviewWarning: clearWarning, showRefinementPreviewWarning: showWarning,
      draftWithAI, applicationRequest, activeLocale: 'en', startBackgroundRefinement: vi.fn(), closeRefinementModal: close,
      loadBoard,
    }, ['openChat', 'renderRefinementDraft', 'setPreviewJobState']);

    runtime.openChat('problems', 'p');
    await flush();
    expect($('#explore-preview-content').textContent).toContain('Saved');

    $('#chat-log').insertAdjacentHTML('beforeend', '<button data-draft-type="problems" data-draft-id="p">Draft</button>');
    $('#chat-log [data-draft-type]').click();
    expect(draftWithAI).toHaveBeenCalledWith('problems', 'p', expect.any(HTMLButtonElement));

    runtime.renderRefinementDraft('problems', 'p', { title: 'Sharper', detail: 'Context:\nMore detail', mode: 'refine', applied: false });
    $('#preview-detail-tab').click();
    expect($('#explore-preview-detail').hidden).toBe(false);
    $('#preview-context-tab').click();
    expect($('#explore-preview-content').hidden).toBe(false);
    $('#preview-detail-tab').click();
    $('#apply-refinement-preview').click();
    await flush();
    expect(api).toHaveBeenCalledWith('/items/problems/p', expect.objectContaining({ method: 'PUT' }));
    expect($('#explore-preview-status').textContent).toBe('APPLIED');
    expect(loadBoard).toHaveBeenCalled();

    runtime.openChat('problems', 'p');
    await flush();
    $('#chat-log [data-chat-prompt]').click();
    await flush();
    expect(applicationRequest).toHaveBeenCalledWith('/problems/p/chat', expect.objectContaining({ method: 'POST' }));

    runtime.setPreviewJobState('error', 'Retry preview');
    const prior = api.mock.calls.length;
    $('#preview-job-status').click();
    await flush();
    expect(api.mock.calls.length).toBeGreaterThan(prior);
    expect($('#explore-preview-content').hidden).toBe(false);

    api.mockRejectedValueOnce(new Error('context unavailable'));
    runtime.openChat('features', 's');
    await flush();
    expect($('#refinement-preview-warning').hidden).toBe(false);
    const beforeWarningRetry = api.mock.calls.length;
    api.mockResolvedValueOnce({ has_context: true, entries: [{ label: 'Context', text: 'Recovered' }] });
    $('#refinement-preview-warning').click();
    await flush();
    expect(api.mock.calls.length).toBeGreaterThan(beforeWarningRetry);
    expect($('#refinement-preview-warning').hidden).toBe(true);
  });
});

describe('production transition and conflict decision controls', () => {
  it('opens and submits each manual transition template and reports API failures', async () => {
    document.body.innerHTML = `${dialog('transition-modal')}<form id="transition-form"><input id="transition-id"><input id="transition-entity-type"><input id="transition-entity-id"><div id="transition-fields"></div><button id="transition-submit">Continue</button></form><div id="transition-title"></div><div id="transition-description"></div>`;
    const transitions = ['capture_to_problem', 'problem_to_solution', 'solution_to_approved', 'solution_to_completed'].map(id => ({ id, label: id, fields: [] }));
    const api = vi.fn(async (path: string) => path === '/transitions' ? { transitions } : ({ completed: true }));
    const notice = vi.fn();
    const runtime = runModule<Record<string, AnyFunction>>(transitionsSource, { $, api, esc: String, showNotice: notice, loadBoard: vi.fn() }, ['openTransition']);
    for (const id of transitions.map(item => item.id)) {
      await runtime.openTransition(id, 'features', 's');
      ($('#transition-submit') as HTMLButtonElement).click();
      await flush();
      expect(api).toHaveBeenCalledWith('/transitions/features/s', expect.objectContaining({ method: 'POST' }));
    }
    api.mockRejectedValueOnce(new Error('transition down'));
    await runtime.openTransition('solution_to_completed', 'features', 's');
    ($('#transition-submit') as HTMLButtonElement).click();
    await flush();
    expect(notice).toHaveBeenCalledWith('transition down', 'Manual transition failed');
    expect($('#transition-submit').disabled).toBe(false);
  });

  it('clicks no-conflict decisions, every per-conflict resolution, continue, and failure recovery', async () => {
    const realm = new JSDOM(`<body>${dialog('item-detail-modal')}<div id="item-detail-type"></div><div id="item-detail-title"></div><div id="item-detail-notes"></div><button id="item-detail-close"></button><section id="board"></section><section id="queue-panel"></section><button id="queue-toggle"></button><div id="organize-status"></div></body>`, { runScripts: 'dangerously', url: 'http://runtime.test' });
    const win = realm.window as any;
    const doc = win.document as Document;
    const select = (selector: string) => doc.querySelector(selector) as any;
    const detailModal = select('#item-detail-modal') as HTMLDialogElement;
    Object.defineProperty(detailModal, 'open', { configurable: true, writable: true, value: true });
    detailModal.close = vi.fn(function close(this: HTMLDialogElement) { this.open = false; });
    const api = vi.fn(async (path: string) => path.includes('/resolutions') ? ({ requires_revision: false }) : ({}));
    Object.assign(win, {
      $: select, api, esc: String, queueCopy: (_key: string, fallback: string, vars?: Record<string, unknown>) => vars ? Object.entries(vars).reduce((text, [key, value]) => text.replace(`{${key}}`, String(value)), fallback) : fallback,
      itemDetailModal: detailModal, loadBoard: vi.fn(), showNotice: vi.fn(), t: (key: string) => key,
      alert: vi.fn(), confirm: vi.fn(() => true), openChat: vi.fn(), jobLabel: String,
      detachKnowledgeTranslationReader: vi.fn(),
    });
    win.eval(conflictsSource);

    select('#item-detail-notes').innerHTML = win.conflictReviewMarkup({ run_id: 'run', conflicts: [], recommended_state: 'clear' }, 's', false);
    select('[data-conflict-decision="clear"]').click();
    await flush();
    expect(api).toHaveBeenCalledWith('/features/s/conflict', expect.objectContaining({ method: 'PUT' }));

    select('#item-detail-notes').innerHTML = win.conflictReviewMarkup({ run_id: 'run', conflicts: [], recommended_state: 'clear' }, 's', false);
    select('[data-conflict-decision="conflicted"]').click();
    await flush();
    expect(api).toHaveBeenCalledWith('/features/s/conflict', expect.objectContaining({ body: expect.stringContaining('"state":"conflicted"') }));

    api.mockRejectedValueOnce(new Error('decision failed'));
    select('#item-detail-notes').innerHTML = win.conflictReviewMarkup({ run_id: 'run', conflicts: [], recommended_state: 'clear' }, 's', false);
    const failedClear = select('[data-conflict-decision="clear"]') as HTMLButtonElement;
    failedClear.click();
    await flush();
    expect(failedClear.disabled).toBe(false);
    expect(failedClear.hasAttribute('aria-busy')).toBe(false);

    select('#item-detail-notes').innerHTML = win.conflictReviewMarkup({ run_id: 'run', conflicts: [
      { id: 'a', title: 'A', summary: 'A', recommendation: 'revise', evidence: [] },
      { id: 'b', title: 'B', summary: 'B', recommendation: 'accept', evidence: [] },
    ] }, 's', false);
    const cards = [...doc.querySelectorAll<HTMLElement>('.conflict-card')];
    const apply = cards[0].querySelector<HTMLInputElement>('input[value="apply_recommendation"]')!;
    apply.click();
    const ignore = cards[1].querySelector<HTMLInputElement>('input[value="accept_conflict"]')!;
    const rationale = cards[1].querySelector<HTMLTextAreaElement>('textarea')!;
    ignore.click();
    rationale.value = 'Accepted exception';
    rationale.dispatchEvent(new win.Event('input', { bubbles: true }));
    const continueButton = select('#conflict-review-continue') as HTMLButtonElement;
    expect(continueButton.disabled).toBe(false);
    continueButton.click();
    await flush();
    expect(api).toHaveBeenCalledWith('/conflict-reviews/run/resolutions', expect.objectContaining({ method: 'PUT' }));

    api.mockRejectedValueOnce(new Error('save failed'));
    select('#item-detail-notes').innerHTML = win.conflictReviewMarkup({ run_id: 'run2', conflicts: [{ id: 'a', title: 'A', summary: 'A', recommendation: 'revise', evidence: [] }] }, 's', false);
    const retryApply = select('.conflict-card input[value="apply_recommendation"]') as HTMLInputElement;
    retryApply.click();
    select('#conflict-review-continue').click();
    await flush();
    expect(select('#conflict-review-error').textContent).toBe('save failed');
    expect(select('#conflict-review-continue').disabled).toBe(false);

    select('#item-detail-close').click();
    expect(detailModal.close).toHaveBeenCalled();
  });
});
