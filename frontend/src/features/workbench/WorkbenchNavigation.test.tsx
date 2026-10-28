import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { WorkbenchView } from "./WorkbenchView";
const leaveGate=vi.hoisted(()=>({blocked:false,pending:undefined as (()=>void)|undefined}));
vi.mock("./TaskDetail",async()=>{const {useImperativeHandle}=await import('react');return {TaskDetail:({taskId,onClose,queueAIOrigin,ref}:{taskId:string;onClose:()=>void;queueAIOrigin?:unknown;ref:React.Ref<unknown>})=>{
 useImperativeHandle(ref,()=>({requestLeave:(next:()=>void)=>{if(leaveGate.blocked)leaveGate.pending=next;else next();}}));
 return <div data-testid="detail">{taskId}<span data-testid="origin">{JSON.stringify(queueAIOrigin)}</span><button onClick={onClose}>Close test detail</button></div>;
}}});
vi.mock("./RefinementPanel", () => ({ RefinementPanel: ({subjectId,kind,onClose}:{subjectId:string;kind:string;onClose:()=>void}) => <div data-testid="refinement">{kind}:{subjectId}<button onClick={onClose}>Close test refinement</button></div> }));
const task=(id:string,state='task',extra={}) => ({kind:'task',id,title:id,state,taskRevision:1,...extra});
const items=[task('active','in_progress'),task('ready'),task('refining-task'),task('held','on_hold'),task('returned','task',{executionBacklog:true}),{kind:'capture',id:'capture',text:'capture'},{kind:'capture',id:'inbox',text:'inbox'},{kind:'refinement',id:'legacy',problemId:'problem-id',problemRevision:2,title:'legacy',sourceKind:'legacy_problem'}];
const board={revision:1,activeShortcuts:[],refiningShortcuts:[{kind:'task',id:'refining-task'},{kind:'task',id:'returned'},{kind:'capture',id:'capture'}],categories:[{id:'General',label:'General',items}]};
function setup() { const request=vi.fn().mockResolvedValue({ok:true,status:200,json:async()=>board,text:async()=>'',body:null}); window.llmWikiApplication={request}; return request; }
describe('Workbench card navigation',()=>{
  it.each(['active','ready','refining-task','held','returned'])('opens canonical %s Task by title regardless of lane',async id=>{
    setup(); render(<WorkbenchView active />); await screen.findByText('ready');
    if(id==='held') fireEvent.click(document.querySelector('.workbench-held summary')!);
    fireEvent.click(document.querySelector(`article[data-entity-id="${id}"] .canonical-card-title`)!);
    expect(screen.getByTestId('detail')).toHaveTextContent(id); expect(screen.queryByTestId('refinement')).not.toBeInTheDocument();
  });
  it.each(['capture','inbox'])('opens %s refinement from background and restores title focus',async id=>{
    setup(); render(<WorkbenchView active />); await screen.findByText('ready');
    const card=document.querySelector<HTMLElement>(`article[data-entity-id="${id}"]`)!;
    fireEvent.click(card.querySelector('small')!); expect(screen.getByTestId('refinement')).toHaveTextContent(`capture:${id}`);
    fireEvent.click(screen.getByRole('button',{name:'Close test refinement'}));
    await waitFor(()=>expect(card.querySelector('.canonical-card-title')).toHaveFocus());
  });
  it('uses legacy Problem identity and ignores selection and nested menu interactions',async()=>{
    const openChat=vi.fn(); window.openChat=openChat; setup(); render(<WorkbenchView active />); await screen.findByText('ready');
    fireEvent.click(document.querySelector('article[data-entity-id="legacy"] .canonical-card-title')!);
    expect(openChat).toHaveBeenCalledWith('problems','problem-id',{problemRevision:2,sourceTitle:'legacy',workspaceDock:true});
    const card=document.querySelector('article[data-entity-id="ready"]')!;
    const selection=vi.spyOn(window,'getSelection').mockReturnValue({toString:()=> 'selected text'} as Selection);
    fireEvent.click(card.querySelector('small')!); expect(screen.queryByTestId('detail')).not.toBeInTheDocument(); selection.mockRestore();
    fireEvent.click(card.querySelector('[data-control="item-actions-menu"]')!); expect(screen.getByRole('menu')).toBeInTheDocument(); expect(screen.queryByTestId('detail')).not.toBeInTheDocument();
    delete window.openChat;
  });
  it('opens the parent hierarchy in detail instead of growing a fixed card',async()=>{
    const data={...board,refiningShortcuts:[],categories:[{id:'General',label:'General',items:[task('parent'),task('child','task',{parentTaskId:'parent'})]}]};
    window.llmWikiApplication={request:vi.fn().mockResolvedValue({ok:true,status:200,json:async()=>data,text:async()=>'',body:null})};
    render(<WorkbenchView active />); await screen.findByText('parent');
    const opener=document.querySelector<HTMLButtonElement>('[data-control="task-subtasks-expand"]')!;
    expect(opener).toHaveAttribute('aria-haspopup','dialog');
    fireEvent.click(opener);
    expect(screen.getByTestId('detail')).toHaveTextContent('parent');
    expect(document.querySelector('.subtask-tree')).not.toBeInTheDocument();
  });
  it('moves via the menu, preserves identity and reveals the shared held result',async()=>{
    let state='in_progress';
    const request=vi.fn().mockImplementation(({path,body}:{path:string;body?:string})=>{
      if(path.endsWith('/transitions')) { const to=JSON.parse(body!).to; state=to==='resume'?'in_progress':to==='backlog'?'task':to; }
      const data={...board,categories:[{id:'General',label:'General',items:[task('moving',state,{executionBacklog:state==='task'})]}]};
      return Promise.resolve({ok:true,status:200,json:async()=>data,text:async()=>'',body:null});
    }); window.llmWikiApplication={request}; render(<WorkbenchView active />); await screen.findByText('moving');
    fireEvent.click(document.querySelector('[data-control="item-actions-menu"]')!); fireEvent.click(screen.getByRole('menuitem',{name:'Put on hold'}));
    await waitFor(()=>expect(document.querySelector('.workbench-held')).toHaveTextContent('moving'));
    await waitFor(()=>expect(document.querySelector<HTMLDetailsElement>('.workbench-held')!.open).toBe(true));
    expect(document.querySelector('.workbench-active')).not.toHaveTextContent('moving');
    fireEvent.click(document.querySelector('.workbench-held [data-control="item-actions-menu"]')!); fireEvent.click(screen.getByRole('menuitem',{name:'Move to execution backlog'}));
    await waitFor(()=>expect(document.querySelector('[data-lane="tasks"]')).toHaveTextContent('moving'));
    expect(request.mock.calls.filter(([arg])=>arg.path.endsWith('/transitions')).map(([arg])=>JSON.parse(arg.body).to)).toEqual(['on_hold','backlog']);
  });
  it('keeps returned work in refined lane and holds in shared collapsed area',async()=>{
    setup(); render(<WorkbenchView active />); await screen.findByText('ready');
    expect(document.querySelector('[data-lane="tasks"]')).toHaveTextContent('returned'); expect(document.querySelector('[data-lane="refining"]')).not.toHaveTextContent('returned');
    const held=document.querySelector<HTMLDetailsElement>('.workbench-held')!; expect(held.open).toBe(false); expect(held).toHaveTextContent('On hold (1)'); expect(document.querySelector('[data-lane="tasks"]')).not.toHaveTextContent('held');
  });
 it('opens a tracked origin without executing work and honors the existing unsaved guard',async()=>{
  const request=setup();render(<WorkbenchView active/>);await screen.findByText('ready');
  fireEvent.click(document.querySelector('article[data-entity-id="active"] .canonical-card-title')!);
  leaveGate.blocked=true;
  act(()=>window.llmWikiOpenAIOrigin?.({taskId:'ready',sessionId:'saved-session',view:'sessions'}));
  expect(screen.getByTestId('detail')).toHaveTextContent('active');expect(leaveGate.pending).toBeTypeOf('function');
  leaveGate.blocked=false;act(()=>leaveGate.pending?.());
  expect(screen.getByTestId('detail')).toHaveTextContent('ready');expect(screen.getByTestId('origin')).toHaveTextContent('saved-session');
  fireEvent.click(screen.getByRole('button',{name:'Close test detail'}));
  fireEvent.click(document.querySelector('article[data-entity-id="ready"] .canonical-card-title')!);
  expect(screen.getByTestId('origin')).toBeEmptyDOMElement();
  expect(request.mock.calls.some(([arg])=>arg.method==='POST')).toBe(false);leaveGate.pending=undefined;
 });

});
