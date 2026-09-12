import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import runtime from '../../../public/runtime/work-tracking.js?raw';
import { ChatTrackingSurface } from './ChatTrackingSurface';

const listeners: Array<[string, EventListener]> = [];
afterEach(() => { for (const [name, handler] of listeners) window.removeEventListener(name, handler); listeners.length=0; vi.restoreAllMocks(); });

function setup(targetOverride={type:'captures',id:'capture',mode:'chat'}, sessionOverride?: Record<string, unknown>) {
  document.documentElement.lang='en';
  const fixture=document.createElement('div');
  fixture.innerHTML='<dialog id="runtime-chat"><button id="chat-track-toggle"></button><h2 id="chat-title">Preserved idea</h2><textarea id="chat-message"></textarea></dialog><dialog id="runtime-manual"></dialog>';
  document.body.append(fixture);
  const session={sessionId:'session',headRevision:1,state:'active',capture:{id:'capture',summary:'Preserved idea'},linkedWorkflow:{} as Record<string,{id:string;title:string;state:string;sourceEventId:string}>,recentEvents:[],recentDecisions:[],nextActions:['create_task'],publicationState:'not_requested',...sessionOverride};
  let creates=0;
  const api=vi.fn(async(path: string, options?: {body?: string})=>{
    const body=JSON.parse(options?.body||'{}');
    if(path==='/work-tracking/open')return {decisionRequired:true,reviewState:'review-'+api.mock.calls.length,stage:body.mode==='continue_task'?'task_continuation':'capture',preview:body.mode==='continue_task'?{task:{id:'task-1',taskId:'task-1',title:'Bound Task',state:'task',taskRevision:2},problem:{title:'Bound Problem'},relationships:[{type:'blocks',id:'related-task'}]}:body.capture};
    if(path==='/work-tracking/open/review'){if(body.decision==='accept'){creates++;return session;}return {decision:body.decision};}
    if(path==='/work-tracking/sessions/session')return structuredClone(session);
    if(path==='/work-tracking/append'){session.headRevision++;return {eventId:'checkpoint',headRevision:session.headRevision};}
    if(path==='/work-tracking/advance/preview')return {reviewState:'governed-'+api.mock.calls.length,decisionRequired:true};
    if(path==='/work-tracking/advance/review'){
      if(body.decision==='accept'){
        if(body.proposal.expectedHeadRevision!==session.headRevision)throw Error('head_conflict');
        session.headRevision++;
        session.linkedWorkflow.task={id:'task',title:'Preserved idea',state:'task',sourceEventId:'checkpoint'};
        session.nextActions=['transition_task'];
      }
      return {decision:body.decision};
    }
    throw Error('Unexpected request '+path);
  });
  const add=window.addEventListener.bind(window);
  vi.spyOn(window,'addEventListener').mockImplementation((name,handler,options)=>{if(typeof handler==='function')listeners.push([name,handler as EventListener]);add(name,handler,options);});
  const target=targetOverride;
  const controller=new Function('api','$','chatModal','manualModal','chatTarget','openChat','showNotice',runtime+'\nreturn {checkpoint:window.offerTrackedCheckpoint,start:startTrackedChat};')(api,(selector:string)=>document.querySelector(selector),fixture.querySelector('#runtime-chat'),fixture.querySelector('#runtime-manual'),target,vi.fn(),vi.fn());
  render(<ChatTrackingSurface />);
  return {api,controller,target,session,created:()=>creates,dispose:()=>fixture.remove()};
}

describe('actual Chat runtime and React decision integration',()=>{
  it('does not create a Capture on Track or edit, then accepts the exact replacement and checkpoints only on consent',async()=>{
    const context=setup();
    await act(async()=>context.controller.start());
    expect(context.created()).toBe(0);
    fireEvent.click(screen.getByRole('button',{name:'Edit'}));
    fireEvent.change(screen.getByRole('textbox',{name:'Edit review content'}),{target:{value:JSON.stringify({title:'Edited title',summary:'Edited exact summary'})}});
    fireEvent.click(screen.getByRole('button',{name:'New preview'}));
    await waitFor(()=>expect(screen.getByRole('heading',{name:'Edited title'})).toBeVisible());
    expect(context.created()).toBe(0);
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(context.created()).toBe(1));
    await waitFor(()=>expect(screen.queryByRole('article')).toBeNull());
    act(()=>context.controller.checkpoint('Meaningful result','Tests passed',context.target));
    expect(context.api.mock.calls.some(([path])=>path==='/work-tracking/append')).toBe(false);
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(context.api.mock.calls.filter(([path])=>path==='/work-tracking/append')).toHaveLength(1));
    await waitFor(()=>expect(screen.queryByRole('article')).toBeNull());
    context.dispose();
  });
  it('rejecting the server preview creates neither Capture nor session',async()=>{
    const context=setup();await act(async()=>context.controller.start());
    fireEvent.click(screen.getByRole('button',{name:'Reject'}));
    await waitFor(()=>expect(screen.queryByRole('article')).toBeNull());
    expect(context.created()).toBe(0);context.dispose();
  });
  it('refreshes a stale governed card without accepting until the user reviews again',async()=>{
    const context=setup();await act(async()=>context.controller.start());
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(screen.queryByRole('article')).toBeNull());
    fireEvent.click(screen.getByRole('button',{name:'Review Task proposal'}));
    await waitFor(()=>expect(screen.getByRole('article')).toBeVisible());
    context.session.headRevision++;
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(screen.getByRole('alert')).toHaveTextContent('Review the refreshed state'));
    expect(context.api.mock.calls.filter(([path])=>path==='/work-tracking/advance/review')).toHaveLength(0);
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(screen.queryByRole('article')).toBeNull());
    expect(context.api.mock.calls.filter(([path])=>path==='/work-tracking/advance/review')).toHaveLength(1);
    context.dispose();
  });
  it('ignores a late accepted Capture response after the Chat target changes',async()=>{
    const context=setup();await act(async()=>context.controller.start());
    let complete!: (value:typeof context.session)=>void;
    context.api.mockImplementationOnce(()=>new Promise(resolve=>{complete=resolve;}));
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    act(()=>window.dispatchEvent(new Event('llm-wiki:chat-target-changing')));
    await act(async()=>complete(context.session));
    expect(screen.queryByRole('article')).toBeNull();
    expect(screen.queryByRole('navigation')).toBeNull();
    context.dispose();
  });
  it('continues an existing Task without a fabricated Capture and keeps the exact continuation review non-editable',async()=>{
    const context=setup({type:'tasks',id:'task-1',mode:'chat'}, {capture:null,linkedWorkflow:{task:{id:'task-1',taskId:'task-1',title:'Existing Task',state:'task',taskRevision:2,sourceEventId:'task-event'}},nextActions:['continue_task']});
    await act(async()=>context.controller.start());
    const openCall=context.api.mock.calls.find(([path])=>path==='/work-tracking/open');
    expect(openCall?.[1]).toEqual(expect.objectContaining({body:expect.stringContaining('"mode":"continue_task"')}));
    expect(JSON.parse((openCall?.[1] as {body:string}).body)).not.toHaveProperty('capture');
    expect(screen.queryByRole('button',{name:'Edit'})).not.toBeInTheDocument();
    expect(screen.getByRole('heading',{name:'Bound Task'})).toBeInTheDocument();
    expect(screen.getByText('State: task · Revision 2 · Problem: Bound Problem · 1 relationship')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(screen.queryByRole('article')).toBeNull());
    expect(context.created()).toBe(1);
    context.dispose();
  });
  it('rejects a direct Task continuation without creating a Capture',async()=>{
    const context=setup({type:'tasks',id:'task-3',mode:'chat'}, {capture:null,linkedWorkflow:{task:{id:'task-3',taskId:'task-3',title:'Rejected Task',state:'task',taskRevision:4,sourceEventId:'task-event'}},nextActions:['continue_task']});
    await act(async()=>context.controller.start());
    const openCall=context.api.mock.calls.find(([path])=>path==='/work-tracking/open');
    expect(JSON.parse((openCall?.[1] as {body:string}).body)).not.toHaveProperty('capture');
    expect(screen.queryByRole('button',{name:'Edit'})).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button',{name:'Reject'}));
    await waitFor(()=>expect(screen.queryByRole('article')).toBeNull());
    expect(context.created()).toBe(0);
    context.dispose();
  });
  it('reviews starting the exact bound Task before changing its state',async()=>{
    const context=setup({type:'tasks',id:'task-1',mode:'chat'}, {capture:null,linkedWorkflow:{task:{id:'task-1',taskId:'task-1',title:'Existing Task',state:'task',taskRevision:2,sourceEventId:'task-event'}},nextActions:['transition_task']});
    await act(async()=>context.controller.start());
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(screen.getByRole('button',{name:'Review Task state change'})).toBeEnabled());
    fireEvent.click(screen.getByRole('button',{name:'Review Task state change'}));
    await waitFor(()=>expect(screen.getByRole('article')).toBeInTheDocument());
    expect(context.api.mock.calls.some(([path])=>path==='/work-tracking/advance/review')).toBe(false);
    const append=context.api.mock.calls.find(([path])=>path==='/work-tracking/append');
    expect(JSON.parse(append?.[1]?.body||'{}').event).toEqual(expect.objectContaining({kind:'task_transition_proposed',taskId:'task-1',expectedTaskRevision:2,to:'in_progress'}));
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(context.api.mock.calls.some(([path])=>path==='/work-tracking/advance/review')).toBe(true));
    const reviewed=context.api.mock.calls.find(([path])=>path==='/work-tracking/advance/review');
    expect(JSON.parse(reviewed?.[1]?.body||'{}')).toEqual(expect.objectContaining({decision:'accept',proposal:expect.objectContaining({action:'transition_task',proposedPayload:expect.objectContaining({taskId:'task-1',expectedTaskRevision:2,to:'in_progress'})})}));
    context.dispose();
  });
  it('uses the bound Task title when a completed captureless session offers Knowledge',async()=>{
    const context=setup({type:'tasks',id:'task-2',mode:'chat'}, {capture:null,linkedWorkflow:{task:{id:'task-2',taskId:'task-2',title:'Completed Task',state:'completed',taskRevision:3,sourceEventId:'task-event'}},publicationState:'offered',nextActions:['offer_knowledge_publication']});
    await act(async()=>context.controller.start());
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(screen.getByRole('heading',{name:'Completed Task'})).toBeInTheDocument());
    expect(context.created()).toBe(1);
    context.dispose();
  });
  it('reviews and publishes the exact canonical saved draft with its source hash',async()=>{
    const context=setup({type:'tasks',id:'task-2',mode:'chat'}, {capture:null,state:'completed',linkedWorkflow:{task:{id:'task-2',taskId:'task-2',title:'Completed Task',state:'completed',taskRevision:3,sourceEventId:'task-event'}},latestDraft:{taskId:'task-2',draftRevision:2,contentHash:'body-hash',sourceHash:'source-hash',state:'draft',canonical:true},nextActions:['review_knowledge_draft']});
    const original=context.api.getMockImplementation()!;
    context.api.mockImplementation(async(path,options)=>{
      if(path==='/work-tracking/knowledge/publish/preview')return {decisionRequired:true,reviewState:'publish-review',stage:'task_knowledge_publish',preview:{request:JSON.parse(options?.body||'{}'),target:{task:{title:'Completed Task'}},prepared:{title:'Exact Knowledge',bodyMarkdown:'# Exact approved body',sourceHash:'source-hash'}}};
      if(path==='/work-tracking/knowledge/publish/review')return {decision:'accept',publicationStatus:'queued'};
      return original(path,options);
    });
    await act(async()=>context.controller.start());
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(screen.getByRole('button',{name:'Review saved Knowledge draft'})).toBeVisible());
    fireEvent.click(screen.getByRole('button',{name:'Review saved Knowledge draft'}));
    await waitFor(()=>expect(screen.getByRole('heading',{name:'Exact Knowledge'})).toBeVisible());
    expect(screen.getByText('# Exact approved body',{selector:'pre'})).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button',{name:'Publish exact draft'}));
    await waitFor(()=>expect(context.api.mock.calls.some(([path])=>path==='/work-tracking/knowledge/publish/review')).toBe(true));
    const reviewed=context.api.mock.calls.find(([path])=>path==='/work-tracking/knowledge/publish/review');
    expect(JSON.parse(reviewed?.[1]?.body||'{}').proposal).toEqual(expect.objectContaining({taskId:'task-2',draftRevision:2,expectedContentHash:'body-hash',expectedSourceHash:'source-hash'}));
    context.dispose();
  });

  it('prepares Knowledge from the exact desktop completion without a fabricated Chat event',async()=>{
    const context=setup({type:'tasks',id:'task-2',mode:'chat'}, {capture:null,linkedWorkflow:{task:{id:'task-2',taskId:'task-2',title:'Desktop-completed Task',state:'completed',taskRevision:3,sourceEventId:'binding'}},taskCompletion:{id:'canonical-completion',taskRevision:3,evidence:'Desktop verification',report:'Completed independently'},publicationState:'offered',nextActions:['offer_knowledge_publication','reopen_task']});
    const original=context.api.getMockImplementation()!;
    context.api.mockImplementation(async(path,options)=>path==='/work-tracking/knowledge/draft'?{decisionRequired:true,reviewState:'canonical-draft'}:original(path,options));
    await act(async()=>context.controller.start());
    fireEvent.click(screen.getByRole('button',{name:'Accept'}));
    await waitFor(()=>expect(screen.getByRole('button',{name:'Review Knowledge draft'})).toBeVisible());
    fireEvent.click(screen.getByRole('button',{name:'Review Knowledge draft'}));
    await waitFor(()=>expect(context.api.mock.calls.some(([path])=>path==='/work-tracking/knowledge/draft')).toBe(true));
    const request=context.api.mock.calls.find(([path])=>path==='/work-tracking/knowledge/draft');
    const payload=JSON.parse(request?.[1]?.body||'{}');
    expect(payload).toEqual(expect.objectContaining({taskId:'task-2',expectedTaskRevision:3,completionId:'canonical-completion'}));
    expect(payload).not.toHaveProperty('completionEventId');
    expect(payload.bodyMarkdown).toContain('Desktop verification');
    context.dispose();
  });

});
