// Render real workspace components with disposable, in-memory data only.
import { createRoot } from 'react-dom/client';
import { WorkbenchView } from '../features/workbench/WorkbenchView';
import { TaskDetail } from '../features/workbench/TaskDetail';
import { DeleteItemDialog } from '../features/workbench/DeleteItemDialog';
import { RefinementPanel } from '../features/workbench/RefinementPanel';
import '../theme/tokens.css';
import '../theme/legacy-components.css';
import '../theme/refresh.css';
import '../theme/conflict-review.css';

const prose = '사용자가 검토하는 긴 작업 설명입니다. Review the task scope, outcome and validation before applying changes. '.repeat(12);
const task = { id: 'layout-task', kind: 'task', taskRevision: 1, state: 'task', title: 'Wide workspace / 와이드 작업 공간', detail: prose, outcome: prose, scope: prose, nonGoals: prose, validationCriteria: prose, workLog: [], checklist: [], decisions: [], readinessEntries: [] };
const snapshot = { revision: 1, activeShortcuts: [], refiningShortcuts: [], categories: [{ id: 'general', label: 'General', items: [task, { kind: 'capture', id: 'layout-capture', text: prose }] }] };
window.llmWikiApplication = { request: async ({path}) => {
  const body = path === '/workbench' ? snapshot : path.endsWith('/proposals') ? [] : path.endsWith('/reference-workspace') ? {references: [], mentions: []} : path.includes('execution') ? {runs: []} : path.includes('lineage') ? {} : path.includes('refinement') ? {id: 'layout-refinement', messages: [{id: 'question', role: 'user', body: prose}, {id: 'answer', role: 'assistant', body: prose}], proposals: []} : task;
  return {ok: true, status: 200, json: async <T,>() => body as T, text: async () => '', body: null};
} };
const mode = new URLSearchParams(location.search).get('mode');
createRoot(document.getElementById('root')!).render(<>
  <div className="app"><aside className="side"><div className="brand">LLM Wiki</div></aside><main><WorkbenchView active /></main></div>
  {mode === 'detail' && <TaskDetail taskId="layout-task" onClose={() => {}} onChanged={() => {}} />}
  {mode === 'refinement' && <RefinementPanel kind="task" subjectId="layout-task" onClose={() => {}} onApplied={() => {}} />}
  {mode === 'confirm' && <DeleteItemDialog title="Disposable layout fixture" busy={false} error="" onConfirm={() => {}} onCancel={() => {}} />}
</>);
