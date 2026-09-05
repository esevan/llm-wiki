import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import runtime from '../../../public/runtime/work-tracking.js?raw';
import { ChatTrackingSurface } from './ChatTrackingSurface';

const listeners: Array<[string, EventListener]> = [];
afterEach(() => { for (const [name, handler] of listeners) window.removeEventListener(name, handler); listeners.length=0; vi.restoreAllMocks(); });

function setup() {
  document.documentElement.lang='en';
  const fixture=document.createElement('div');
  fixture.innerHTML='<dialog id="runtime-chat"><button id="chat-track-toggle"></button><h2 id="chat-title">Preserved idea</h2><textarea id="chat-message"></textarea></dialog><dialog id="runtime-manual"></dialog>';
  document.body.append(fixture);
  const session={sessionId:'session',headRevision:1,state:'active',capture:{id:'capture',summary:'Preserved idea'},linkedWorkflow:{} as Record<string,{id:string;title:string;state:string;sourceEventId:string}>,recentEvents:[],recentDecisions:[],nextActions:['adopt_problem'],publicationState:'not_requested'};
  let creates=0;
  const api=vi.fn(async(path: string, options?: {body?: string})=>{
    const body=JSON.parse(options?.body||'{}');
    if(path==='/work-tracking/open')return {decisionRequired:true,reviewState:'review-'+api.mock.calls.length,preview:body.capture};
    if(path==='/work-tracking/open/review'){if(body.decision==='accept'){creates++;return session;}return {decision:body.decision};}
    if(path==='/work-tracking/sessions/session')return structuredClone(session);
    if(path==='/work-tracking/append'){session.headRevision++;return {eventId:'checkpoint',headRevision:session.headRevision};}
    if(path==='/work-tracking/advance/preview')return {reviewState:'governed-'+api.mock.calls.length,decisionRequired:true};
    if(path==='/work-tracking/advance/review'){
      if(body.decision==='accept'){
        if(body.proposal.expectedHeadRevision!==session.headRevision)throw Error('head_conflict');
        session.headRevision++;
        session.linkedWorkflow.problem={id:'problem',title:'Preserved idea',state:'proposed',sourceEventId:'checkpoint'};
        session.nextActions=['approve_problem'];
      }
      return {decision:body.decision};
    }
    throw Error('Unexpected request '+path);
  });
  const add=window.addEventListener.bind(window);
  vi.spyOn(window,'addEventListener').mockImplementation((name,handler,options)=>{if(typeof handler==='function')listeners.push([name,handler as EventListener]);add(name,handler,options);});
  const target={type:'captures',id:'capture',mode:'chat'};
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
    fireEvent.click(screen.getByRole('button',{name:'Review Problem proposal'}));
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
});
