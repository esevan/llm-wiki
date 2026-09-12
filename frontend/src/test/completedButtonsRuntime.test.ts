import archive from '../../public/runtime/archive.js?raw';
import completed from '../../public/runtime/completed-workspace.js?raw';
import explore from '../../public/runtime/explore.js?raw';
import solutionWork from '../../public/runtime/solution-work.js?raw';
import { afterEach, describe, expect, it, vi } from 'vitest';

const flush = () => new Promise((resolve) => setTimeout(resolve, 0));
const $ = (selector: string) => document.querySelector(selector) as HTMLElement;
type RuntimeFunction = (...args: unknown[]) => unknown;
type Runtime = {
  openChat: (type: string, id: string, context?: Record<string, unknown>) => void;
  askNotice: (message: string, title: string, confirm: string) => Promise<boolean>;
  openSolutionWorkDetail: (id: string) => Promise<void>;
  loadWorkbenchContext: () => Promise<void>;
  openCompletedWorkspace: (id: string) => Promise<void>;
  renderKnowledgeLineage: (lineage: unknown) => string;
  waitForJob: RuntimeFunction;
  searchArchivedDocument: RuntimeFunction;
};
const esc = (value: unknown) => String(value || '').replace(/[&<>]/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[character]!));

function dialog(element: HTMLDialogElement) {
  Object.defineProperty(element, 'open', { configurable: true, writable: true, value: false });
  element.showModal = function showModal() { this.open = true; };
  element.close = function close() { this.open = false; this.dispatchEvent(new Event('close')); };
}

function fixture() {
  window.llmWikiFormatSystemTime = (value: string) => value;
  document.body.innerHTML = `<section class="workbench-context"><div id="recent-archive"></div><div id="completed-solutions"></div></section>
    <dialog id="item-detail-modal"><span id="item-detail-type"></span><h2 id="item-detail-title"></h2><div id="item-detail-notes"></div></dialog>
    <dialog id="notice-modal"><h2 id="notice-title"></h2><p id="notice-message"></p><button id="notice-confirm"></button><button id="notice-cancel"></button></dialog>
    <dialog id="chat-modal"><button id="refinement-preview-warning" hidden></button><h2 id="chat-title"></h2><div id="chat-log"></div><form id="chat-form"><label></label><textarea id="chat-message"></textarea></form></dialog>
    <aside id="explore-refinement-preview"><header><small></small></header><button id="preview-detail-tab"></button><button id="preview-context-tab"></button><button id="preview-job-status"></button><span id="explore-preview-status"></span><div id="explore-preview-detail"></div><div id="explore-preview-content"></div></aside>
    <textarea id="feature-nongoals"></textarea><div id="draft-extra-field"></div><form id="draft-form"><input id="draft-type"><input id="draft-id"><textarea id="draft-main"></textarea><textarea id="draft-detail"></textarea><textarea id="draft-extra"></textarea><button id="draft-submit"></button></form><form id="feature-form"><input id="feature-problem"><input id="feature-title"><textarea id="feature-outcome"></textarea></form>`;
  for (const selector of ['#item-detail-modal', '#notice-modal', '#chat-modal']) dialog($(selector) as HTMLDialogElement);
}

function load(api: ReturnType<typeof vi.fn>): Runtime {
  const context: Record<string, unknown> = {
    api, $, esc, localTime: (value: string) => value, activeLocale: 'en', archiveLimit: 5, pendingProgressImage: null, previewActiveTab: 'context', explorePreviewRequest: 0,
    chatTarget: null, refinementPreviewDraft: null, activeRefinementPreview: null, activeKnowledgeTranslation: null, knowledgeReadRequest: 0,
    itemDetailModal: $('#item-detail-modal'), chatModal: $('#chat-modal'), draftModal: document.createElement('dialog'),
    categoryFor: () => 'General', trackedProgressMarkup: () => '', t: (key: string) => key,
    menuButton: () => '', moreMenu: () => '', priorityButton: () => '', transitionMenuItem: () => '', manualButton: () => '', removeButton: () => '', iconButton: () => '', conflictCopy: () => '',
    showDraftReview: vi.fn(), loadBoard: vi.fn().mockResolvedValue(undefined), refreshQueue: vi.fn(), searchArchivedDocument: vi.fn(),
    regenerateCompletionPlaybook: vi.fn(), deleteCompletionPlaybook: vi.fn(), openChat: vi.fn(), openNextChat: vi.fn(), openCompletedDetail: vi.fn(),
    supersedeRefinementPreview: vi.fn(), clearRefinementPreviewWarning: vi.fn(), chatTargetMatches: (target: unknown) => JSON.stringify(context.chatTarget) === JSON.stringify(target),
    setPreviewJobState: vi.fn(), refinementContextMarkup: () => '', renderRefinementDraft: vi.fn(), showRefinementPreviewWarning: vi.fn(), showDraftGeneration: () => document.createElement('div'), showReadyForReview: vi.fn(),
    setAiActionState: vi.fn(), waitForJob: vi.fn(), loadExploreRefinementPreview: vi.fn(), initializeLocale: () => Promise.resolve(), loadAllTransitions: vi.fn(),
    knowledgeDocumentMarkup: () => '', setKnowledgeProgress: vi.fn(), applicationRequest: vi.fn(), openItemDetail: vi.fn(), openSolutionDetail: vi.fn(),
  };
  const scope = new Proxy(context, { has: (_target, key) => key !== 'scope', get: (target, key) => key in target ? target[key as string] : (globalThis as unknown as Record<string, unknown>)[key as string], set: (target, key, value) => { target[key as string] = value; return true; } });
  const execute = (source: string, expose = '') => new Function('scope', `with(scope){${source}\n${expose}}`)(scope);
  execute(`${explore}\n${solutionWork}\nscope.openSolutionWorkDetail=refreshSolutionWorkDetail;\n${archive}\n${completed}`, 'scope.loadWorkbenchContext=loadWorkbenchContext;scope.regenerateCompletionPlaybook=regenerateCompletionPlaybook;scope.deleteCompletionPlaybook=deleteCompletionPlaybook;scope.openCompletedWorkspace=openCompletedWorkspace;scope.openChat=openChat;scope.renderKnowledgeLineage=renderKnowledgeLineage;scope.askNotice=askNotice;scope.showNotice=showNotice;');
  return context as unknown as Runtime;
}

afterEach(() => { vi.restoreAllMocks(); document.body.innerHTML = ''; const globals = window as unknown as Window & Record<string, unknown>; delete globals.boardItems; delete globals.workbenchBoard; delete globals.completedSolutions; });

describe('production completed-work button bindings', () => {
  it('forwards migrated Problem revision context through the completed-work chat override', async () => {
    fixture();
    const selected = vi.fn(async () => undefined);
    (window as unknown as Window & Record<string, unknown>).selectTrackedWork = selected;
    const runtime = load(vi.fn(async (path: string) => {
      if (path.startsWith('/workbench/recent-archive')) return { documents: [] };
      if (path.startsWith('/workbench/completed-solutions')) return { solutions: [] };
      return { entries: [] };
    }));
    runtime.openChat('problems', 'problem-1', { problemRevision: 3, sourceTitle: 'Preserved Problem' });
    expect(selected).toHaveBeenCalledWith(expect.objectContaining({
      type: 'problems',
      id: 'problem-1',
      problemRevision: 3,
      sourceTitle: 'Preserved Problem',
    }));
    await flush();
    delete (window as unknown as Window & Record<string, unknown>).selectTrackedWork;
  });

  it('resolves the production confirmation notice on both cancel and confirm clicks', async () => {
    fixture();
    const api = vi.fn(async (path: string) => path.startsWith('/workbench/recent-archive') ? { documents: [] } : { solutions: [] });
    const runtime = load(api);

    const cancelled = runtime.askNotice('Remove it?', 'Confirm removal', 'Remove');
    ($('#notice-cancel') as HTMLButtonElement).click();
    await expect(cancelled).resolves.toBe(false);
    const confirmed = runtime.askNotice('Remove it?', 'Confirm removal', 'Remove');
    ($('#notice-confirm') as HTMLButtonElement).click();
    await expect(confirmed).resolves.toBe(true);
    expect($('#notice-title')).toHaveTextContent('Confirm removal');
  });

  it('saves work logs, comments, checklist additions, and checkbox changes from production detail markup', async () => {
    fixture();
    const api = vi.fn(async (path: string) => {
      if (path.startsWith('/workbench/recent-archive')) return { documents: [] };
      if (path.startsWith('/workbench/completed-solutions')) return { solutions: [] };
      if (path === '/features/s/progress') return { entries: [{ id: 'entry', body: 'Existing', created_at: '2026-01-01', comments: [] }], checklist: [{ id: 'check', body: 'Verify', checked: 0 }] };
      return { id: 'image-entry' };
    });
    const runtime = load(api);
    (window as unknown as Window & Record<string, unknown>).boardItems = { 'features:s': { id: 's', title: 'Solution', state: 'approved', problem_id: 'p', outcome: 'Done', non_goals: '', conflict_state: 'clear' } };
    const workbenchBoard = { problems: [{ id: 'p', statement: 'Problem', state: 'approved' }], features: [] };
    window.workbenchBoard = workbenchBoard;
    await runtime.openSolutionWorkDetail('s');

    let notes = $('#item-detail-notes');
    (notes.querySelector('#progress-body') as HTMLTextAreaElement).value = 'Shipped';
    notes.querySelector<HTMLButtonElement>('#progress-composer button')!.click();
    await flush();
    notes = $('#item-detail-notes');
    const comment = notes.querySelector<HTMLInputElement>('[data-comment-entry] input')!;
    comment.value = 'Looks good'; comment.closest<HTMLFormElement>('form')!.querySelector<HTMLButtonElement>('button')!.click();
    await flush();
    notes = $('#item-detail-notes');
    const add = notes.querySelector<HTMLInputElement>('#checklist-composer input')!;
    add.value = 'Verify release'; add.closest<HTMLFormElement>('form')!.querySelector<HTMLButtonElement>('button')!.click();
    await flush();
    notes = $('#item-detail-notes');
    const check = notes.querySelector<HTMLInputElement>('[data-check-id]')!;
    check.click();
    await flush();

    expect(api).toHaveBeenCalledWith('/features/s/progress', { method: 'POST', body: JSON.stringify({ body: 'Shipped', image_data: '', image_media_type: '' }) });
    expect(api).toHaveBeenCalledWith('/progress/entry/comments', { method: 'POST', body: JSON.stringify({ body: 'Looks good' }) });
    expect(api).toHaveBeenCalledWith('/features/s/checklist', { method: 'POST', body: JSON.stringify({ body: 'Verify release' }) });
    expect(api).toHaveBeenCalledWith('/checklist/check', { method: 'PUT', body: JSON.stringify({ body: 'Verify', checked: true }) });
  });

  it('starts image summarization from the production-rendered image action', async () => {
    fixture();
    let summarized = false, finishJob!: () => void;
    const api = vi.fn(async (path: string) => {
      if (path.startsWith('/workbench/recent-archive')) return { documents: [] };
      if (path.startsWith('/workbench/completed-solutions')) return { solutions: [] };
      if (path === '/features/s/progress') return { entries: [{ id: 'image-entry', body: '', image_data: 'AA==', image_media_type: 'image/png', image_summary: summarized ? 'A release screenshot' : '', created_at: '2026-01-01', comments: [] }], checklist: [] };
      if (path === '/progress/image-entry/summarize-image') return { id: 'image-job' };
      return {};
    });
    const runtime = load(api);
    (window as unknown as Window & Record<string, unknown>).boardItems = { 'features:s': { id: 's', title: 'Solution', state: 'approved', problem_id: 'p', outcome: 'Done', non_goals: '', conflict_state: 'clear' } };
    const workbenchBoard = { problems: [{ id: 'p', statement: 'Problem', state: 'approved' }], features: [] };
    window.workbenchBoard = workbenchBoard;
    await runtime.openSolutionWorkDetail('s');
    runtime.waitForJob = vi.fn(() => new Promise<void>((resolve) => { finishJob = () => { summarized = true; resolve(); }; }));

    const button = $('#item-detail-notes').querySelector<HTMLButtonElement>('[data-summarize-entry]')!;
    button.click();
    await flush();
    expect(button).toBeDisabled();
    button.click();
    expect(api.mock.calls.filter(([path]) => path === '/progress/image-entry/summarize-image')).toHaveLength(1);
    finishJob();
    await flush();

    expect(api).toHaveBeenCalledWith('/progress/image-entry/summarize-image', { method: 'POST' });
    expect(runtime.waitForJob).toHaveBeenCalledWith('image-job');
    expect(api.mock.calls.filter(([path]) => path === '/features/s/progress')).toHaveLength(2);
    expect($('#item-detail-notes')).toHaveTextContent('A release screenshot');
  });

  it('restores the summarize-image action after a request failure and shows the app notice', async () => {
    fixture();
    const api = vi.fn(async (path: string) => {
      if (path.startsWith('/workbench/recent-archive')) return { documents: [] };
      if (path.startsWith('/workbench/completed-solutions')) return { solutions: [] };
      if (path === '/features/s/progress') return { entries: [{ id: 'image-entry', body: '', image_data: 'AA==', image_media_type: 'image/png', created_at: '2026-01-01', comments: [] }], checklist: [] };
      if (path === '/progress/image-entry/summarize-image') throw new Error('service unavailable');
      return {};
    });
    const runtime = load(api);
    (window as unknown as Window & Record<string, unknown>).boardItems = { 'features:s': { id: 's', title: 'Solution', state: 'approved', problem_id: 'p', outcome: 'Done', non_goals: '', conflict_state: 'clear' } };
    const workbenchBoard = { problems: [{ id: 'p', statement: 'Problem', state: 'approved' }], features: [] };
    window.workbenchBoard = workbenchBoard;
    await runtime.openSolutionWorkDetail('s');
    const button = $('#item-detail-notes').querySelector<HTMLButtonElement>('[data-summarize-entry]')!;
    button.click();
    await flush();

    expect(button).not.toBeDisabled();
    expect($('#notice-title')).toHaveTextContent('Could not summarize image');
    expect($('#notice-message')).toHaveTextContent('service unavailable');
  });

  it('clicks Work-tab save, comment, checklist, and image-summary controls from rendered refinement markup', async () => {
    fixture();
    let summarized = false, finishJob!: () => void;
    const api = vi.fn(async (path: string) => {
      if (path.startsWith('/workbench/recent-archive')) return { documents: [] };
      if (path.startsWith('/workbench/completed-solutions')) return { solutions: [] };
      if (path === '/features/s/refinement-context') return { has_context: true, entries: [] };
      if (path === '/features/s/progress') return { entries: [{ id: 'image-entry', body: '', image_data: 'AA==', image_media_type: 'image/png', image_summary: summarized ? 'Work-tab summary' : '', created_at: '2026-01-01', comments: [] }], checklist: [{ id: 'check', body: 'Verify', checked: 0 }] };
      if (path === '/progress/image-entry/summarize-image') return { id: 'image-job' };
      return {};
    });
    const runtime = load(api);
    (window as unknown as Window & Record<string, unknown>).boardItems = { 'features:s': { id: 's', title: 'Solution', state: 'approved', problem_id: 'p', outcome: 'Done', non_goals: '', conflict_state: 'clear' } };
    const workbenchBoard = { problems: [{ id: 'p', statement: 'Problem', state: 'approved' }], features: [] };
    window.workbenchBoard = workbenchBoard;
    runtime.openChat('features', 's');
    await flush(); await flush();
    ($('#preview-work-tab') as HTMLButtonElement).click();
    let panel = $('#explore-preview-work');
    (panel.querySelector('#progress-body') as HTMLTextAreaElement).value = 'Work tab update';
    panel.querySelector<HTMLButtonElement>('#progress-composer button')!.click();
    await flush();
    panel = $('#explore-preview-work');
    const comment = panel.querySelector<HTMLInputElement>('[data-comment-entry] input')!;
    comment.value = 'Work tab comment'; comment.closest<HTMLFormElement>('form')!.querySelector<HTMLButtonElement>('button')!.click();
    await flush();
    panel = $('#explore-preview-work');
    const add = panel.querySelector<HTMLInputElement>('#checklist-composer input')!;
    add.value = 'Work tab criterion'; add.closest<HTMLFormElement>('form')!.querySelector<HTMLButtonElement>('button')!.click();
    await flush();
    panel = $('#explore-preview-work');
    panel.querySelector<HTMLInputElement>('[data-check-id]')!.click();
    await flush();
    panel = $('#explore-preview-work');
    runtime.waitForJob = vi.fn(() => new Promise<void>((resolve) => { finishJob = () => { summarized = true; resolve(); }; }));
    const summarize = panel.querySelector<HTMLButtonElement>('[data-summarize-entry]')!;
    summarize.click();
    await flush();
    expect(summarize).toBeDisabled();
    finishJob();
    await flush();

    expect(api).toHaveBeenCalledWith('/features/s/progress', { method: 'POST', body: JSON.stringify({ body: 'Work tab update', image_data: '', image_media_type: '' }) });
    expect(api).toHaveBeenCalledWith('/progress/image-entry/comments', { method: 'POST', body: JSON.stringify({ body: 'Work tab comment' }) });
    expect(api).toHaveBeenCalledWith('/features/s/checklist', { method: 'POST', body: JSON.stringify({ body: 'Work tab criterion' }) });
    expect(api).toHaveBeenCalledWith('/checklist/check', { method: 'PUT', body: JSON.stringify({ body: 'Verify', checked: true }) });
    expect(api).toHaveBeenCalledWith('/progress/image-entry/summarize-image', { method: 'POST' });
    expect(runtime.waitForJob).toHaveBeenCalledWith('image-job');
    expect($('#explore-preview-work')).toHaveTextContent('Work-tab summary');
  });

  it('clicks archive-row regenerate and delete controls and keeps the missing-file icon disabled', async () => {
    fixture();
    const api = vi.fn(async (path: string, options?: { method?: string }) => {
      if (path.startsWith('/workbench/recent-archive')) return { documents: [] };
      if (path.startsWith('/workbench/completed-solutions')) return { solutions: [
        { id: 'available', problem_id: 'p', title: 'Available', archive_status: 'available', completion_playbook_path: 'Knowledge/available.md' },
        { id: 'missing', problem_id: 'm', title: 'Missing', archive_status: 'missing', completion_playbook_path: '' },
      ] };
      if (path === '/problems/p/completion-playbook/regenerate') return {};
      if (path === '/problems/p/completion-playbook' && options?.method === 'DELETE') return {};
      return {};
    });
    const runtime = load(api);
    await runtime.loadWorkbenchContext();
    const missing = document.querySelector<HTMLButtonElement>('.archive-file-icon.missing')!;
    expect(missing).toBeDisabled();
    document.querySelector<HTMLButtonElement>('.archive-regenerate-icon')!.click();
    ($('#notice-confirm') as HTMLButtonElement).click();
    await flush();
    expect(api).toHaveBeenCalledWith('/problems/p/completion-playbook/regenerate', { method: 'POST' });
    document.querySelector<HTMLButtonElement>('.archive-delete-icon')!.click();
    ($('#notice-confirm') as HTMLButtonElement).click();
    await flush();
    expect(api).toHaveBeenCalledWith('/problems/p/completion-playbook', { method: 'DELETE' });
  });

  it('uses completed workspace tabs, archive actions, follow-up, lineage references, and corrections from production markup', async () => {
    fixture();
    const lineage = {
      snapshot_id: 'snapshot',
      lineage: { stages: [{ title: 'Capture', kind: 'capture', claim_id: 'claim', record_type: 'captures', record_id: 'capture', live_available: true }], transitions: [] },
      claims: { claim: { id: 'claim', text: 'AI interpretation', classification: 'inferred', current_revision_id: 'revision', evidence_ids: ['evidence'] } },
      evidence: { evidence: { id: 'evidence', source_type: 'capture', source_id: 'capture', field_name: 'text', source_hash: 'hash' } },
      decision_changes: [{ claim_id: 'claim' }], conflicts: [],
    };
    const api = vi.fn(async (path: string, options?: { method?: string }) => {
      if (path.startsWith('/workbench/recent-archive')) return { documents: [] };
      if (path.startsWith('/workbench/completed-solutions')) return { solutions: [{ id: 's', problem_id: 'p', title: 'Completed solution', outcome: 'Done', archive_status: 'available', completion_playbook_path: 'Knowledge/done.md' }] };
      if (path === '/features/s/refinement-context') return { entries: [] };
      if (path === '/features/s/progress') return { entries: [], checklist: [] };
      if (path === '/features/s/lineage') return lineage;
      if (path === '/features/s/lineage/evidence/evidence') return { source_type: 'capture', field_name: 'text', excerpt: 'Preserved evidence', captured_at: '2026-01-01', live_record: { available: true } };
      if (path === '/items/captures/capture') return { kind: 'capture', title: 'Live Capture', detail: 'Current source', created_at: '2026-01-01' };
      if (path === '/features/s/follow-up-problem') return { id: 'follow-up' };
      if (path === '/features/s/lineage/claims/claim/corrections') return { document_sync: { status: 'current' } };
      if (path === '/problems/p/completion-playbook/regenerate') return {};
      if (path === '/problems/p/completion-playbook' && options?.method === 'DELETE') return {};
      return {};
    });
    const runtime = load(api);
    await flush();
    await runtime.openCompletedWorkspace('s');
    await flush();

    ($('#preview-work-tab') as HTMLButtonElement).click();
    expect($('#explore-preview-work').hidden).toBe(false);
    ($('#preview-context-tab') as HTMLButtonElement).click();
    expect($('#explore-preview-content').hidden).toBe(false);
    ($('#preview-detail-tab') as HTMLButtonElement).click();
    expect($('#explore-preview-detail').hidden).toBe(false);
    ($('#preview-archive-tab') as HTMLButtonElement).click();
    expect($('#explore-preview-archive').hidden).toBe(false);
    ($('#explore-preview-archive').querySelector('[data-completed-action="open"]') as HTMLButtonElement).click();
    expect(runtime.searchArchivedDocument).toHaveBeenCalledWith('Knowledge/done.md');
    ($('#explore-preview-archive').querySelector('[data-completed-action="regenerate"]') as HTMLButtonElement).click();
    ($('#notice-confirm') as HTMLButtonElement).click();
    await flush();
    expect(api).toHaveBeenCalledWith('/problems/p/completion-playbook/regenerate', { method: 'POST' });
    ($('#explore-preview-archive').querySelector('[data-completed-action="delete"]') as HTMLButtonElement).click();
    ($('#notice-confirm') as HTMLButtonElement).click();
    await flush();
    expect(api).toHaveBeenCalledWith('/problems/p/completion-playbook', { method: 'DELETE' });

    ($('#create-follow-up-problem') as HTMLButtonElement).click();
    await flush();
    expect(api).toHaveBeenCalledWith('/features/s/follow-up-problem', { method: 'POST' });
    expect(($('#chat-modal') as HTMLDialogElement).open).toBe(true);
    expect($('#chat-title')).toHaveTextContent('Explore this Problem');

    await runtime.openCompletedWorkspace('s');
    $('#explore-preview-content').innerHTML = runtime.renderKnowledgeLineage(lineage);
    ($('#explore-preview-content').querySelector('[data-lineage-evidence]') as HTMLButtonElement).click();
    await flush();
    expect(api).toHaveBeenCalledWith('/features/s/lineage/evidence/evidence');
    expect($('#explore-preview-content')).toHaveTextContent('Preserved evidence');
    ($('#explore-preview-content').querySelector('.lineage-reference-close') as HTMLButtonElement).click();
    expect($('#explore-preview-content').querySelector('.lineage-reference-popover')).toBeNull();
    ($('#explore-preview-content').querySelector('[data-lineage-record]') as HTMLElement).click();
    await flush();
    expect(api).toHaveBeenCalledWith('/items/captures/capture');
    expect($('#item-detail-title')).toHaveTextContent('Live Capture');
    const correction = $('#explore-preview-content').querySelector<HTMLFormElement>('.lineage-correction')!;
    correction.querySelector<HTMLTextAreaElement>('textarea')!.value = 'Corrected interpretation';
    correction.querySelector<HTMLButtonElement>('button')!.click();
    await flush();
    expect(api).toHaveBeenCalledWith('/features/s/lineage/claims/claim/corrections', {
      method: 'POST', body: JSON.stringify({ text: 'Corrected interpretation', reason: 'User correction from completed Solution Lineage', current_revision_id: 'revision' }),
    });
    expect($('#notice-title')).toHaveTextContent('Lineage corrected');
  });
});
