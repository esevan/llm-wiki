// Real RefinementPanel with disposable responses; never contacts native storage.
import { createRoot } from 'react-dom/client';
import { RefinementPanel } from '../features/workbench/RefinementPanel';
import { fixtureVersion, fixtureWorkspace } from '../features/workbench/referencePreviewFixtures';
import '../theme/tokens.css';
import '../theme/legacy-components.css';
import '../theme/refresh.css';
import '../features/workbench/task-workbench.css';
const params = new URLSearchParams(location.search);
document.documentElement.lang = params.get('lang') ?? 'ko';
const state = params.get('state') ?? 'loading';
const ready = ['ready', 'edit', 'history'].includes(state);
const workspace = ready ? fixtureWorkspace(2) : {
  preview: null, findings: [], investigations: [],
  generation: { id: 'fixture-job', status: state === 'failed' ? 'failed' : state === 'empty' ? 'completed' : 'running' },
};
window.llmWikiApplication = { request: async ({ path }) => {
  const body = path.endsWith('/reference-workspace') ? workspace
    : /\/preview-versions\/\d+$/.test(path) ? fixtureVersion(1)
    : path.endsWith('/proposals') ? []
    : path.endsWith('/refinement') ? { id: 'spacing-session', subjectKind: 'capture', subjectId: 'spacing-capture', draftRevision: 1, responseStatus: 'completed', previewStatus: state === 'loading' ? 'running' : 'completed', messages: [] }
    : {};
  return { ok: true, status: 200, json: async <T,>() => body as T, text: async () => '', body: null };
}};
createRoot(document.getElementById('root')!).render(<RefinementPanel kind="capture" subjectId="spacing-capture" subjectTitle="Preview spacing / 프리뷰 간격" onClose={() => {}} />);
if (state === 'edit' || state === 'history') {
  const timer = window.setInterval(() => {
    const control = document.querySelector<HTMLElement>(state === 'edit' ? '[data-control="reference-preview-edit"]' : '[data-control="draft-version-select"]');
    if (!control) return;
    window.clearInterval(timer);
    if (state === 'edit') control.click();
    else { (control as HTMLSelectElement).value = '1'; control.dispatchEvent(new Event('change', { bubbles: true })); }
  }, 50);
}
