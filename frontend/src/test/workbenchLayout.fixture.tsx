// Render real workspace components with disposable, in-memory data only.
import { createRoot } from 'react-dom/client';
import type { TaskAggregate, TaskExecutionWorkLogLink, DistillationProjection, WorkbenchItem, WorkbenchSnapshot } from '../types/taskWorkbench';
import { WorkLogDistillation } from '../features/workbench/WorkLogDistillation';
import { WorkbenchView } from '../features/workbench/WorkbenchView';
import { TaskDetail } from '../features/workbench/TaskDetail';
import { DeleteItemDialog } from '../features/workbench/DeleteItemDialog';
import { RefinementPanel } from '../features/workbench/RefinementPanel';
import '../theme/tokens.css';
import '../theme/legacy-components.css';
import '../theme/refresh.css';
import '../theme/conflict-review.css';

const prose = '사용자가 검토하는 긴 작업 설명입니다. Review the task scope, outcome and validation before applying changes. '.repeat(12);
const task: TaskAggregate = { id: 'layout-task', kind: 'task', taskRevision: 1, state: 'task', title: 'Wide workspace / 와이드 작업 공간', detail: prose, outcome: prose, scope: prose, nonGoals: prose, validationCriteria: prose, workLog: [], checklist: [], decisions: [], readinessEntries: [] };
if (new URLSearchParams(location.search).has('alignment')) {
  task.detail = prose.repeat(2);
  task.outcome = 'A clear, verifiable result. 사용자가 확인할 수 있는 짧은 목표입니다.';
  task.scope = 'Review only the Task definition. Task 정의만 검토합니다.';
  task.nonGoals = 'Do not change content or status.\n콘텐츠나 상태를 변경하지 않습니다.';
  task.validationCriteria = 'Labels and text stay together. 라벨과 본문이 함께 표시됩니다.';
}
const crowded = new URLSearchParams(location.search).has('crowded');
const integrated = new URLSearchParams(location.search).has('integrated');
const fixtureItems: WorkbenchItem[] = crowded ? Array.from({length: 24}, (_, index) => [{...task, id: `refining-${index}`, detail: 'Refining task description', title: `Refining ${index}`}, {...task, id: `task-${index}`, detail: 'A readable task description', title: `Task ${index} / 작업`}, {kind: 'capture' as const, id: `capture-${index}`, text: `Capture ${index} / 생각을 정리합니다.`}]).flat() : [task, {kind: 'capture', id: 'layout-capture', text: prose}];
if (integrated) fixtureItems.push(
  {...task, id:'integration-active', state:'in_progress', title:'Active task / 진행 중 작업'},
  {...task, id:'integration-guarded', state:'in_progress', hasActiveExecution:true, title:'Uncertain execution / 확인이 필요한 실행'},
  {...task, id:'integration-held', state:'on_hold', title:'Held task / 보류 작업'},
);
const snapshot: WorkbenchSnapshot = { revision: 1, activeShortcuts: [], refiningShortcuts: crowded ? ['','second-'].flatMap(prefix => Array.from({length: 24}, (_, index) => ({kind: 'task', id: `${prefix}refining-${index}`, draftRevision: 1}))) : [], categories: [{ id: 'general', label: 'General', items: fixtureItems }, ...(crowded ? [{id: 'second', label: 'Second category / 두 번째', items: fixtureItems.map(item => ({...item, id: `second-${item.id}`}))}] : [])] };
const heldCount = new URLSearchParams(location.search).get('held');
if (heldCount !== null) {
  for (const category of snapshot.categories) category.items = category.items.filter(item => item.kind !== 'task' || item.state !== 'on_hold');
  snapshot.categories[0].items.push(...Array.from({length: Number(heldCount)}, (_, index): WorkbenchItem => ({...task, id:`held-${index}`, state:'on_hold', title:`Held task ${index} / 보류한 작업을 이어갑니다.`})));
}
const previews = new URLSearchParams(location.search).has('previews');
if (previews) for (const category of snapshot.categories) for (const item of category.items) if (item.kind === 'task') {
  const index = Number(item.id.match(/(\d+)$/)?.[1] ?? 0);
  item.originCaptureText = index % 3 === 0 ? prose.repeat(3) : index % 3 === 1 ? 'Short original / 짧은 원문' : '';
  item.title = index % 3 === 0 ? 'A deliberately long bilingual task title / 긴 제목을 두 줄로 읽고 상세에서 전체를 확인합니다.'.repeat(4) : item.title;
}
if (previews) snapshot.categories[0].items.push({...task,id:'preview-parent',title:'Parent with a Subtask',originCaptureText:prose}, {...task,id:'preview-child',title:'Child Task',parentTaskId:'preview-parent'});
const savedExecution: TaskExecutionWorkLogLink = {runId:'second-run',sessionId:'same-session',instruction:'Draft a customer reply and check the export. 고객 답변 초안을 작성하고 내보내기를 확인하세요.',status:'failed',provider:'codex',model:'model',evidence:[],artifacts:[],limitations:['Delivery has not been confirmed. 전송은 확인되지 않았습니다.'],syncState:'synced'};
const savedProjection: DistillationProjection = {projectionRevision:1,freshness:'current',result:{claims:[{id:'draft',kind:'performed',statement:'Prepared a reply draft for review. 검토용 답변 초안을 작성했습니다.',actor:'ai',epistemicState:'performed',status:'current',sources:[{type:'run_final_report',id:'second-run',revision:'1',locator:'final_report'}]},{id:'failure',kind:'unresolved',statement:'Export failed; the reply remains unsent. 내보내기에 실패했고 답변은 전송하지 않았습니다.',actor:'tool',epistemicState:'unresolved',status:'current',sources:[]}],workLogView:{sections:[{kind:'performed',claimIds:['draft']},{kind:'unresolved',claimIds:['failure']}]},warnings:[]}};
window.llmWikiApplication = { request: async ({path, method, body: requestBody}) => {
  const id = path.split('/')[2];
  const selected = snapshot.categories.flatMap(category => category.items).find(item => item.id === id);
  if (integrated && path.endsWith('/transitions') && selected?.kind === 'task') {
    const {to} = JSON.parse(String(requestBody));
    if (selected.hasActiveExecution) throw new Error('active_task_execution');
    selected.state = to === 'resume' ? 'in_progress' : to === 'backlog' ? 'task' : to;
    selected.executionBacklog = to === 'backlog';
  }
  if (integrated && method === 'DELETE') for (const category of snapshot.categories) category.items = category.items.filter(item => item.id !== id);
  const aggregate: TaskAggregate = selected?.kind === 'task' ? {...task, ...selected} : {...task};
  if (previews && aggregate.originCaptureText) aggregate.originCapture = {id:'original',text:aggregate.originCaptureText,createdAt:'2026-10-20T10:00:00Z'};
  if (previews && aggregate.id === 'preview-parent') aggregate.hierarchy = {siblings:[],children:[{...task,id:'preview-child',title:'Child Task',parentTaskId:'preview-parent'}]};
  if (integrated) aggregate.workLog = [
    {id:'first-log', body:'Old retained raw request', createdAt:'2026-10-19T10:00:00Z', execution:{...savedExecution,runId:'first-run',instruction:'Review the first draft only.'},distillation:savedProjection},
    {id:'second-log', body:'Later retained raw request', createdAt:'2026-10-20T10:00:00Z', execution:savedExecution,distillation:savedProjection},
  ];
  const body = path === '/workbench' ? snapshot : path.endsWith('/proposals') ? [] : path.endsWith('/reference-workspace') ? {references: [], mentions: []} : path.includes('execution') ? {runs: []} : path.includes('lineage') ? {} : path.includes('refinement') ? {id: 'layout-refinement', messages: [{id: 'question', role: 'user', body: prose}, {id: 'answer', role: 'assistant', body: prose}], proposals: []} : aggregate;
  return {ok: true, status: 200, json: async <T,>() => body as T, text: async () => '', body: null};
} };
const mode = new URLSearchParams(location.search).get('mode');
if (mode === 'worklog') document.documentElement.lang = new URLSearchParams(location.search).get('lang') || 'en';
createRoot(document.getElementById('root')!).render(<>
  {mode === 'worklog' ? <main style={{maxWidth:900, margin:'24px auto', padding:16}}><WorkLogDistillation
    execution={savedExecution}
    projection={savedProjection}
    onOpenOriginal={() => {}} /></main> : <div className="app"><aside className="side"><div className="brand">LLM Wiki</div></aside><main><WorkbenchView active /></main></div>}
  {mode === 'detail' && <TaskDetail taskId="layout-task" onClose={() => {}} onChanged={() => {}} />}
  {mode === 'refinement' && <RefinementPanel kind="task" subjectId="layout-task" onClose={() => {}} onApplied={() => {}} />}
  {mode === 'confirm' && <DeleteItemDialog title="Disposable layout fixture" busy={false} error="" onConfirm={() => {}} onCancel={() => {}} />}
</>);
