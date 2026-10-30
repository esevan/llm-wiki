import { Profiler } from 'react';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { App } from './App';
import { fixtureWorkspace } from '../features/workbench/referencePreviewFixtures';
vi.mock('../features/workbench/useModalInteraction', () => ({ useModalInteraction: () => {} }));
vi.mock('../features/settings/SettingsView', () => ({ SettingsView: () => null }));
vi.mock('../features/search/SearchView', () => ({ SearchView: () => null }));
const reply = (body: unknown, status = 200) => ({ ok: status < 400, status, json: async <T,>() => body as T, text: async () => '', body: null });
function fixture(state = 'task', fail = false, kind = 'task', sessionState = 'active', profileManual = false) {
  let previewReady=!profileManual,legacyReady=false,legacyReads=0;
  let applied = false, hold = false;
  let release!: () => void;
  const task = () => ({ kind:'task', id:'source', title:applied ? 'Applied work' : 'Source work', state, taskRevision:applied ? 2 : 1, refinedRevision:applied ? 2 : undefined, detail:profileManual?'사용자 조사와 판단 기준 / Research and decision criteria. '.repeat(20):'', workLog:[], checklist:[], decisions:[], readinessEntries:[] });
  const board = () => ({ revision:applied ? 2 : 1, activeShortcuts:state === 'in_progress' ? [{kind:'task',id:'source',taskRevision:1}] : [], refiningShortcuts:!applied && state === 'task' ? [{kind,id:'source',draftRevision:1}] : [], categories:[{id:'General',label:'General',items:[kind === "capture" && !applied ? {kind:"capture",id:"source",text:"Source capture"} : task()]}] });
  const workspace = fixtureWorkspace(1);
  const request = vi.fn(async ({path}: {path:string}) => {
    if (path === '/workbench') {
      const snapshot = board();
      if (hold) {hold = false; await new Promise<void>(resolve => {release=resolve;});}
      return reply(snapshot);
    }
    if (path === '/tasks/source') return reply(task());
    if (path.endsWith('/refinement')) return reply({id:'session',taskId:kind === 'task' ? 'source' : undefined,state:sessionState,currentDraftRevision:1,inputDraft:'',messages:[],responseStatus:'completed',previewStatus:previewReady?'completed':'running'});
    if (path.endsWith('/reference-workspace')) return reply(previewReady?workspace:{...workspace,preview:undefined,generation:{status:'running'}});
    if (path.endsWith('/proposals')) { if(profileManual&&legacyReady){legacyReads++;return reply([{id:'mirrored',type:'task_patch',payload:{taskId:'source',expectedTaskRevision:1,patch:{title:'Intermediate legacy form'}}}]);}return reply([]);}
    if (path.endsWith('/apply')) {
      if (fail) return reply({error:'preview_head_conflict'},409);
      applied = true;
      return reply({task:{id:'source',taskRevision:2},transitionCause:'preview_adopted',version:1});
    }
    if (path.endsWith('/lineage')) return reply({nodes:[],edges:[],unresolved:[]});
    if (path.endsWith('/knowledge')) return reply({taskId:'source',pointers:{},versions:[],ideas:[]});
    if (path.endsWith('/references')) return reply({items:[]});
    return reply({connections:[]});
  });
  window.llmWikiApplication = {request};
  return {request, revealLegacy:()=>{legacyReady=true;},legacyReadCount:()=>legacyReads, revealPreview:()=>{previewReady=true;window.dispatchEvent(new CustomEvent('llm-wiki:queue-changed',{detail:[{id:'profile-job',entity_id:'session',task_kind:'refinement_preview',status:'completed'}]}));}, holdNext:()=>{hold=true;}, release:()=>release(), applied:()=>applied};
}
async function open() {
  await waitFor(() => expect(document.querySelector('[data-control="task-shortcut-refine"], [data-control="task-card-open"]')).not.toBeNull());
  fireEvent.click(document.querySelector('[data-control="task-shortcut-refine"], [data-control="task-card-open"]')!);
  fireEvent.click(await screen.findByRole("tab",{name:"Refine"}));
  return screen.findByRole('button',{name:'Apply to Task'});
}
describe('App Task application and lane refresh',()=>{
  it('ignores a pre-Apply board response that arrives after the acknowledged refined board',async()=>{
    const f=fixture(); render(<App/>); const apply=await open();
    f.holdNext(); act(()=>window.dispatchEvent(new Event('llm-wiki:task-workbench-refresh')));
    fireEvent.click(apply);
    await waitFor(()=>expect(document.querySelector('[data-lane="tasks"]')).toHaveTextContent('Applied work'));
    await act(async()=>f.release());
    expect(document.querySelector('[data-lane="tasks"]')).toHaveTextContent('Applied work');
    expect(document.querySelector('[data-lane="refining"]')).not.toHaveTextContent('Source work');
  });
  it('does not move the card when Apply fails',async()=>{
    const f=fixture('task',true); render(<App/>); fireEvent.click(await open());
    await waitFor(()=>expect(f.request.mock.calls.some(([input])=>input.path.endsWith('/apply'))).toBe(true));
    await waitFor(()=>expect(document.querySelector('[data-control=reference-preview-apply]')).toBeEnabled());
    expect(f.applied()).toBe(false);
    expect(document.querySelector('[data-lane="refining"]')).toHaveTextContent('Source work');
  });
  it.each(['in_progress','on_hold'])('keeps %s execution state after refinement Apply',async state=>{
    const f=fixture(state); render(<App/>); fireEvent.click(await open());
    await waitFor(()=>expect(f.applied()).toBe(true));
    await waitFor(()=>expect(document.querySelector('#workbench')).toHaveTextContent('Applied work'));
    expect(document.querySelector('[data-lane="tasks"]')?.textContent ?? '').not.toContain('Applied work');
  });
});


describe('App unified Task workspace', () => {
  it('uses one dialog, guarded keyboard tabs, and keeps the unapplied proposal separate', async () => {
    fixture(); render(<App/>); await open();
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
    expect(document.querySelector('.refinement-panel')).toHaveAttribute('role','region');
    expect(document.querySelector('[data-control="task-detail-refine"]')).toBeNull();
    expect(screen.getByRole('heading',{name:'Source work',level:2})).toBeVisible();
    expect(document.querySelector(".task-definition-fallback")).toBeNull();
    expect(document.querySelector("#task-tab-details > .task-definition-panel")).toBeNull();
    fireEvent.click(document.querySelector('[data-control="reference-preview-edit"]')!);
    fireEvent.change(document.querySelector('[data-control="reference-preview-field"]')!,{target:{value:'Unapplied proposal'}});
    fireEvent.keyDown(screen.getByRole('tab',{name:'Refine'}),{key:'Home'});
    await waitFor(()=>expect(screen.getByRole('tab',{name:'Refine'})).toHaveAttribute('aria-selected','true'));
    expect(screen.getByRole('heading',{name:'Source work',level:2})).toBeVisible();
    fireEvent.click(document.querySelector('[data-control="task-detail-close"]')!);
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
    fireEvent.click(document.querySelector('[data-control="reference-preview-cancel"]')!);
    fireEvent.keyDown(screen.getByRole('tab',{name:'Refine'}),{key:'Home'});
    await waitFor(()=>expect(screen.getByRole('tab',{name:'Work'})).toHaveAttribute('aria-selected','true'));
    expect(document.querySelector('.refinement-panel')).toBeNull();
    expect(screen.getByRole('tab',{name:'Work'})).toHaveFocus();
  });
  it('shows Capture refinement alone until Apply creates the Task workspace',async()=>{
    const f=fixture('task',false,'capture');render(<App/>);
    await waitFor(()=>expect(document.querySelector('[data-control="task-shortcut-refine"], [data-control="task-card-open"]')).not.toBeNull());
    fireEvent.click(document.querySelector('[data-control="task-shortcut-refine"], [data-control="task-card-open"]')!);
    const apply=await screen.findByRole('button',{name:'Apply to Task'});
    expect(screen.queryByRole('tab',{name:'Work'})).toBeNull();
    expect(document.querySelector('.task-detail')).toBeNull();
    fireEvent.click(apply);
    await waitFor(()=>expect(f.applied()).toBe(true));
    expect(await screen.findByRole('tab',{name:'Work'})).toBeVisible();
    expect(screen.getByRole('tab',{name:'Sessions'})).toBeVisible();
    expect(screen.getByRole('tab',{name:'Review'})).toBeVisible();
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
  });
  it('retains an unsent conversation through tab changes and card reentry',async()=>{
    fixture();render(<App/>);await open();
    const message=document.querySelector<HTMLTextAreaElement>('[data-control="refinement-message"]')!;
    fireEvent.change(message,{target:{value:'Keep this local question'}});
    fireEvent.click(screen.getByRole('tab',{name:'Work'}));
    await waitFor(()=>expect(screen.getByRole('tab',{name:'Work'})).toHaveAttribute('aria-selected','true'));
    fireEvent.click(screen.getByRole('tab',{name:'Refine'}));
    await waitFor(()=>expect(document.querySelector('[data-control="refinement-message"]')).toHaveValue('Keep this local question'));
    fireEvent.click(document.querySelector('[data-control="task-detail-close"]')!);
    await waitFor(()=>expect(document.querySelector('.task-detail')).toBeNull());
    await open();
    expect(document.querySelector('[data-control="refinement-message"]')).toHaveValue('Keep this local question');
  });
});

it('reopens a completed refinement session as a preview within the Task tabs',async()=>{
  fixture('task',false,'task','completed');render(<App/>);await open();
  expect(screen.getByRole('button',{name:'Apply to Task'})).toBeVisible();
  expect(screen.queryByRole('tab',{name:'Work status'})).toBeNull();
  expect(screen.getAllByRole('dialog')).toHaveLength(1);
});


describe('manual to AI preview profiling fixture',()=>{
  for(const locale of ['en','ko']) it(`measures actual App commits and preserves modal/composer identity (${locale})`,async()=>{
    document.documentElement.lang=locale;
    const f=fixture('task',false,'task','active',true);
    const commits:{phase:string;duration:number}[]=[];
    render(<Profiler id="preview-fixture" onRender={(_id,phase,duration)=>commits.push({phase,duration})}><App/></Profiler>);
    await waitFor(()=>expect(document.querySelector('[data-control="task-shortcut-refine"], [data-control="task-card-open"]')).not.toBeNull());
    fireEvent.click(document.querySelector('[data-control="task-shortcut-refine"], [data-control="task-card-open"]')!);
    await waitFor(()=>expect(document.querySelector('[data-control="task-detail-tab-details"]')).not.toBeNull());
    fireEvent.click(document.querySelector('[data-control="task-detail-tab-details"]')!);
    await waitFor(()=>expect(document.querySelector('[data-control="task-definition-fallback"]')).not.toBeNull());
    const manual=document.querySelector('[data-control="task-definition-fallback"]')!;
    const panel=document.querySelector('.refinement-panel')!;
    const composer=document.querySelector<HTMLTextAreaElement>('[data-control="refinement-message"]')!;
    fireEvent.change(composer,{target:{value:'Preserve my unsent question / 작성 중인 질문'}});
    const legacyStart=commits.length;act(()=>f.revealLegacy());
    await waitFor(()=>expect(f.legacyReadCount()).toBeGreaterThan(0),{timeout:3000});
    await act(async()=>{await Promise.resolve();});
    expect(manual.isConnected).toBe(true);
    expect(document.querySelector('[data-control="refinement-proposal-accept"]')).toBeNull();
    expect(screen.queryByText('Intermediate legacy form')).toBeNull();
    const legacyCommits=commits.length-legacyStart;
    const initial=commits.length;const started=performance.now();
    act(()=>f.revealPreview());
    await waitFor(()=>expect(document.querySelector('[data-control="reference-preview-apply"]')).not.toBeNull(),{timeout:4000});
    const elapsed=performance.now()-started;
    expect(manual.isConnected).toBe(false);expect(document.querySelector('.refinement-panel')).toBe(panel);
    expect(document.querySelector('[data-control="refinement-message"]')).toBe(composer);expect(composer.value).toBe('Preserve my unsent question / 작성 중인 질문');
    const transition=commits.slice(initial);
    console.log(`PreviewTransition jsdom fixture locale=${locale} initialCommits=${initial} mirroredProposalCommits=${legacyCommits} manualPreservedUntilCanonical=true transitionCommits=${transition.length} reactDurationMs=${transition.reduce((sum,commit)=>sum+commit.duration,0).toFixed(2)} elapsedIncludingPollMs=${elapsed.toFixed(2)} panelPreserved=true composerPreserved=true manualSubtreeReplaced=true`);
    document.documentElement.lang='en';
  });
});
