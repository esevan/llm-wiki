import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { KnowledgeRevisionChat } from "./KnowledgeRevisionChat";
import { TaskDetail } from "./TaskDetail";
import { KnowledgeReviewPanel } from "./KnowledgeReviewPanel";
import type { KnowledgeVersion, KnowledgeRevisionConversation, KnowledgeRevisionReply } from "../../types/taskWorkbench";
const response = (body: unknown, ok = true) => ({ ok, status: ok ? 200 : 409, json: async <T,>() => body as T, text: async () => "", body: null });
const source = { type: "task_revision", id: "task", revision: "1", locator: "definition", quote: "Approval remains required" };
export const chatVersion = (): KnowledgeVersion => ({
  revision: 1, derivationKind: "generated", articleType: "concept", title: "Approval guide", bodyMarkdown: "# Approval guide\n\nApproval remains required", contentHash: "original-hash", generationSnapshotHash: "source-hash", freshness: "current", qualityState: "blocked_unsupported", modelStatus: "enhanced", modelError: "Rejected citation", createdAt: "2026-10-21",
  applicability: { summary: "Approval remains required", representativeQuestions: ["What was recorded?"], helpsWith: ["Approval remains required"], conditions: [], exclusions: [] },
  result: { article: { type: "concept", title: "Approval guide", bodyMarkdown: "# Approval guide\n\nApproval remains required", finalOutcomes: [], applicability: { summary: "Approval remains required", representativeQuestions: ["What was recorded?"], helpsWith: ["Approval remains required"], conditions: [], exclusions: [], sourceRefs: [source] }, claimBindings: [{ claimId: "reported", statement: source.quote, epistemicState: "reported", sourceRefs: [source] }], assumptionBindings: [] }, ideas: [], qualityFindings: [] },
});
const diagnostic = { kind: "missing_reference", locator: "article.claimBindings[0]", statement: "Approval remains required", reason: "Quote belongs to another source", direction: "Replace with exact Task definition quote", evidence: [source] };
const proposal = (): KnowledgeRevisionReply => ({ id: "turn-1", baseRevision: 1, message: "Replace the rejected citation and preserve approval.", missingEvidence: [], candidate: { ...chatVersion().result, article: { ...chatVersion().result.article!, bodyMarkdown: "# Corrected guide\n\nApproval remains required" } }, qualityState: "valid", diagnostics: [] });
function fixture(fail = false) {
  const conversation: KnowledgeRevisionConversation = { baseRevision: 1, diagnostics: [diagnostic], turns: [] };
  const requests: Array<{path: string; body?: string}> = [];
  let applied = false;
  window.llmWikiApplication = { request: vi.fn(async ({ path, body }) => {
    requests.push({ path, body });
    if (path.endsWith("/apply")) { applied = true; return response({ draftRevision: 2 }); }
    if (path.endsWith("/cancel")) { conversation.turns[0].state = "cancelled"; return response({ state: "cancelled" }); }
    if (body) {
      if (fail) return response({ detail: "fixture provider unavailable" }, false);
      conversation.turns.push({ id: "turn-1", userMessage: JSON.parse(body).message, reply: proposal(), state: "proposed", createdAt: "now" });
      return response(proposal());
    }
    return response(conversation);
  }) };
  return { requests, conversation, applied: () => applied };
}
describe("Knowledge correction chat", () => {
  it("shows an actionable sentence/source/reason then compares before explicit apply", async () => {
    const f = fixture(); const onApplied = vi.fn().mockResolvedValue(undefined);
    render(<KnowledgeRevisionChat taskId="task" version={chatVersion()} disabled={false} onStateChange={vi.fn()} onApplied={onApplied} />);
    await screen.findByText("Quote belongs to another source");
    fireEvent.click(document.querySelector("[data-control=knowledge-chat-diagnostics] summary")!);
    expect(screen.getByText("Quote belongs to another source")).toBeVisible();
    expect(screen.getByText(source.quote, { selector: "blockquote p" })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Help fix this issue" }));
    expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toContain("Replace with exact Task");
    fireEvent.click(screen.getByRole("button", { name: "Propose revision" }));
    expect(await screen.findByText("Proposed change")).toBeVisible();
    expect(f.applied()).toBe(false); expect(onApplied).not.toHaveBeenCalled();
    expect(screen.getByText("# Corrected guide", { selector: "ins" })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Apply and recheck private draft" }));
    await waitFor(() => expect(onApplied).toHaveBeenCalledWith(2));
    expect(f.requests.some(r => r.path.endsWith("/apply"))).toBe(true);
  });
  it("preserves failed input and allows explicit discard", async () => {
    fixture(true); const state = vi.fn();
    render(<KnowledgeRevisionChat taskId="task" version={chatVersion()} disabled={false} onStateChange={state} onApplied={vi.fn()} />);
    await screen.findByText("Quote belongs to another source");
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Keep my corrected sentence" } });
    fireEvent.click(screen.getByRole("button", { name: "Propose revision" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("input and reviewed version are preserved");
    expect(screen.getByRole("textbox")).toHaveValue("Keep my corrected sentence");
    fireEvent.click(screen.getByRole("button", { name: "Discard unsent message" }));
    expect(screen.getByRole("textbox")).toHaveValue("");
  });
  it("cancels a proposal without applying and keeps evidence requests distinct", async () => {
    const f = fixture(); f.conversation.turns.push({ id: "turn-1", userMessage: "Fix quote", reply: proposal(), state: "proposed", createdAt: "now" });
    render(<KnowledgeRevisionChat taskId="task" version={chatVersion()} disabled={false} onStateChange={vi.fn()} onApplied={vi.fn()} />);
    fireEvent.click(await screen.findByRole("button", { name: "Cancel proposal" }));
    expect(await screen.findByText("Proposal cancelled.")).toBeVisible(); expect(f.applied()).toBe(false);
  });
  it("keeps a late cancelled response from erasing a newer unsent request", async () => {
    let finish: ((v: ReturnType<typeof response>) => void) | undefined;
    window.llmWikiApplication = { request: vi.fn(async ({ body, path }) => {
      if (path.endsWith("/cancel")) return response({ state: "cancelled" });
      if (body) return new Promise<ReturnType<typeof response>>(resolve => { finish = resolve; });
      return response({ turns: [], diagnostics: [] });
    }) };
    render(<KnowledgeRevisionChat taskId="task" version={chatVersion()} disabled={false} onStateChange={vi.fn()} onApplied={vi.fn()} />);
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Old request" } });
    fireEvent.click(screen.getByRole("button", { name: "Propose revision" }));
    fireEvent.click(await screen.findByRole("button", { name: "Cancel pending request" }));
    await waitFor(() => expect(screen.getByRole("textbox")).toBeEnabled());
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Keep this newer request" } });
    await act(async () => finish?.(response(proposal())));
    expect(screen.getByRole("textbox")).toHaveValue("Keep this newer request");
  });
  it("offers the published version as a distinct correction base and protects unsent input", async () => {
    fixture(); const select = vi.fn();
    render(<KnowledgeRevisionChat taskId="task" version={{...chatVersion(), revision:2}} publishedRevision={1} onSelectPublished={select} disabled={false} onStateChange={vi.fn()} onApplied={vi.fn()} />);
    fireEvent.click(await screen.findByRole("button", {name: "Revise published Knowledge · v1"}));
    expect(select).toHaveBeenCalledOnce();
    fireEvent.change(screen.getByRole("textbox"), {target:{value:"Unsaved correction"}});
    expect(screen.getByRole("button", {name:"Revise published Knowledge · v1"})).toBeDisabled();
  });
  it("selects an already published version as the correction base without publishing", async () => {
    const v = chatVersion(); const correction = proposal(); let applied = false; let published = false;
    let turns: KnowledgeRevisionConversation["turns"] = [];
    window.llmWikiApplication = { request: vi.fn(async ({ path, body }) => {
      if (path.includes("revision-chat")) {
        if (path.endsWith("/apply")) { applied = true; return response({ draftRevision: 2 }); }
        if (body) { turns = [{ id: "turn-1", userMessage: JSON.parse(body).message, reply: correction, state: "proposed", createdAt: "now" }]; return response(correction); }
        return response({ turns, diagnostics: [diagnostic] });
      }
      if (path.includes("publish")) published = true;
      const versions = applied ? [v, { ...v, revision: 2, contentHash: "new-hash", qualityState: "valid", bodyMarkdown: correction.candidate!.article!.bodyMarkdown }] : [v];
      return response({ taskId: "task", pointers: { currentPrivateRevision: applied ? 2 : 1, publishedRevision: 1 }, versions, ideas: [] });
    }) };
    render(<KnowledgeReviewPanel taskId="task" completed onGenerate={vi.fn()} />);
    await screen.findByText("Quote belongs to another source");
    fireEvent.change(screen.getByRole("textbox", { name: "What should change?" }), { target: { value: "Repair the published article's citation" } });
    fireEvent.click(screen.getByRole("button", { name: "Propose revision" }));
    fireEvent.click(await screen.findByRole("button", { name: "Apply and recheck private draft" }));
    await waitFor(() => expect(document.querySelector('.knowledge-review')).toHaveAttribute('data-content-hash', 'new-hash'));
    expect(published).toBe(false);
  });
});

describe("Knowledge chat modal sessions", () => {
  it("sends Enter, keeps Shift+Enter and IME Enter for composition", async () => {
    const { requests } = fixture();
    render(<KnowledgeRevisionChat taskId="keyboard" version={chatVersion()} disabled={false} onStateChange={vi.fn()} onApplied={vi.fn()} />);
    const input = screen.getByRole("textbox");
    fireEvent.change(input, {target:{value:"My experience"}});
    fireEvent.keyDown(input, {key:"Enter", shiftKey:true});
    fireEvent.keyDown(input, {key:"Enter", isComposing:true});
    fireEvent.keyDown(input, {key:"Enter", keyCode:229});
    expect(requests.filter(r => r.body)).toHaveLength(0);
    fireEvent.keyDown(input, {key:"Enter"});
    await waitFor(() => expect(requests.filter(r => r.body)).toHaveLength(1));
  });
  it("reconnects a running request after close without sending again or affecting another Task", async () => {
    let finish: ((value: ReturnType<typeof response>) => void) | undefined;
    let sends = 0; let done = false;
    window.llmWikiApplication = {request:vi.fn(async ({body}) => {
      if (body) { sends++; return new Promise<ReturnType<typeof response>>(resolve => { finish = resolve; }); }
      return response({diagnostics:[],turns:done ? [{id:"running",userMessage:"My story",reply:proposal(),state:"proposed"}] : []});
    })};
    const props = {version:chatVersion(),disabled:false,onStateChange:vi.fn(),onApplied:vi.fn()};
    const first=render(<KnowledgeRevisionChat taskId="session-a" {...props}/>);
    fireEvent.change(screen.getByRole("textbox"),{target:{value:"My story"}});
    fireEvent.keyDown(screen.getByRole("textbox"),{key:"Enter"});
    first.unmount();
    const other=render(<KnowledgeRevisionChat taskId="session-b" {...props}/>);
    expect(screen.getByRole("textbox")).toHaveValue(""); other.unmount();
    const reopened=render(<KnowledgeRevisionChat taskId="session-a" {...props}/>);
    expect(screen.getByRole("textbox")).toHaveValue("My story");
    expect(screen.getByRole("textbox")).toBeDisabled();
    expect(sends).toBe(1);
    await act(async()=>{done=true;finish?.(response(proposal()));});
    await waitFor(()=>expect(screen.getByRole("textbox")).toBeEnabled());
    expect(screen.getByRole("textbox")).toHaveValue("");
    expect(await screen.findByText("My story")).toBeInTheDocument();
    reopened.unmount();
  });
});

it.each(["button", "escape"])("closes the Task modal during chat via %s and restores the same request", async (method) => {
  let finish: ((value: ReturnType<typeof response>) => void) | undefined;
  let sent = 0;
  const task={id:"modal-chat",kind:"task",taskRevision:1,state:"completed",title:"Completed story",workLog:[],checklist:[],decisions:[],readinessEntries:[]};
  window.llmWikiApplication={request:vi.fn(async({path,body})=>{
    if(path.includes("revision-chat")) {
      if(body){ sent++; return new Promise<ReturnType<typeof response>>(resolve=>{finish=resolve;}); }
      return response({turns:[],diagnostics:[]});
    }
    if(path.endsWith("/knowledge"))return response({taskId:task.id,pointers:{currentPrivateRevision:1},versions:[chatVersion()],ideas:[]});
    if(path.includes("lineage"))return response({nodes:[],edges:[],unresolved:[]});
    return response(task);
  })};
  const close=vi.fn();
  const first=render(<TaskDetail taskId={task.id} onClose={close} onChanged={vi.fn()}/>);
  fireEvent.click(await screen.findByRole("tab",{name:/Review/}));
  const input=await screen.findByRole("textbox",{name:"What should change?"});
  fireEvent.change(input,{target:{value:"Keep my experience"}});fireEvent.keyDown(input,{key:"Enter"});
  await waitFor(()=>expect(sent).toBe(1));
  if(method==="escape")fireEvent.keyDown(document,{key:"Escape"});else fireEvent.click(document.querySelector('[data-control="task-detail-close"]')!);
  await waitFor(()=>expect(close).toHaveBeenCalledOnce());first.unmount();
  render(<TaskDetail taskId={task.id} onClose={close} onChanged={vi.fn()}/>);
  fireEvent.click(await screen.findByRole("tab",{name:/Review/}));
  expect(await screen.findByRole("textbox",{name:"What should change?"})).toHaveValue("Keep my experience");
  expect(screen.getByRole("textbox",{name:"What should change?"})).toBeDisabled();
  await act(async()=>finish?.(response(proposal())));
  await waitFor(()=>expect(screen.getByRole("textbox",{name:"What should change?"})).toBeEnabled());
  expect(sent).toBe(1);
});

describe("readable diagnostics and main review",()=>{
  it.each([0,1,6])("renders exactly %i expanded diagnostics without a nested height-limited region",async(count)=>{
    window.llmWikiApplication={request:vi.fn(async()=>response({diagnostics:Array.from({length:count},(_,i)=>({...diagnostic,statement:`Sentence ${i}`,reason:`Long reason ${i} `.repeat(30)})),turns:[]}))};
    render(<KnowledgeRevisionChat taskId="diagnostics" version={chatVersion()} disabled={false} onStateChange={vi.fn()} onApplied={vi.fn()}/>);
    await waitFor(()=>expect(document.querySelector('[data-control="knowledge-chat-diagnostics"] summary')).toHaveTextContent(`What needs fixing · ${count}`));
    fireEvent.click(document.querySelector('[data-control="knowledge-chat-diagnostics"] summary')!);
    expect(document.querySelectorAll('.knowledge-revision-diagnostics .knowledge-revision-issues > li')).toHaveLength(count);
    expect(document.querySelector('.knowledge-revision-transcript > .knowledge-revision-diagnostics')).not.toBeNull();
    if(count)expect(screen.getByText('Sentence 0')).toBeVisible();
  });
  it("renders a meaningful legacy note instead of an empty row and keeps the count consistent",async()=>{
    window.llmWikiApplication={request:vi.fn(async()=>response({diagnostics:[{},null,"invalid"],turns:[]}))};
    render(<KnowledgeRevisionChat taskId="legacy-note" version={chatVersion()} disabled={false} onStateChange={vi.fn()} onApplied={vi.fn()}/>);
    await waitFor(()=>expect(document.querySelector('[data-control="knowledge-chat-diagnostics"] summary')).toHaveTextContent('· 1'));
    fireEvent.click(document.querySelector('[data-control="knowledge-chat-diagnostics"] summary')!);
    expect(screen.getByText('Saved review note')).toBeVisible();expect(screen.getByText(/A detailed reason was not saved/)).toBeVisible();
  });
  it("renders comparison and Apply in the main document slot and keeps non-applicable proposals reviewable",async()=>{
    const slot=document.createElement('div');slot.className='knowledge-revision-review-slot';document.body.append(slot);
    const reply={...proposal(),applicable:false,preparationError:'Article body is incomplete'};
    window.llmWikiApplication={request:vi.fn(async()=>response({diagnostics:[],turns:[{id:'bad',state:'failed',userMessage:'Keep my edit',reply}]}))};
    const mounted=render(<KnowledgeRevisionChat taskId="main-slot" version={chatVersion()} reviewContainerRef={{current:slot}} disabled={false} onStateChange={vi.fn()} onApplied={vi.fn()}/>);
    await screen.findByText(/Article body is incomplete/);
    expect(slot.querySelector('[data-control="knowledge-chat-apply"]')).toBeDisabled();
    expect(screen.getByText(/This proposal needs a structural correction/)).toBeVisible();
    expect(screen.queryByText(/You can apply it without/)).toBeNull();
    expect(document.querySelector('.knowledge-revision-transcript .knowledge-revision-proposal')).toBeNull();
    expect(slot.querySelector('[data-control="knowledge-chat-full-compare"]')).toHaveAttribute('open');
    fireEvent.click(screen.getByRole('button',{name:'Request updated proposal'}));
    await waitFor(()=>expect(window.llmWikiApplication!.request).toHaveBeenCalledWith(expect.objectContaining({method:'POST',body:expect.stringContaining('Adapt this saved proposal')})));
    mounted.unmount();slot.remove();
  });
  it("keeps ordinary review notes applicable and separates stale proposals from structural failures", async () => {
    const reply = {...proposal(), applicable:true, qualityState:'blocked_unsupported'};
    window.llmWikiApplication={request:vi.fn(async()=>response({diagnostics:[],turns:[{id:'warning',state:'proposed',userMessage:'Retain warning',reply}]}))};
    const mounted=render(<KnowledgeRevisionChat taskId="warning-apply" version={chatVersion()} disabled={false} onStateChange={vi.fn()} onApplied={vi.fn()}/>);
    await screen.findByText(/You can apply it without/);
    expect(screen.getByRole('button',{name:'Apply and recheck private draft'})).toBeEnabled();
    mounted.unmount();
    window.llmWikiApplication={request:vi.fn(async()=>response({diagnostics:[],turns:[{id:'stale',state:'proposed',userMessage:'Old proposal',reply:{...reply, applicable:false, applicationState:'stale'}}]}))};
    render(<KnowledgeRevisionChat taskId="stale-apply" version={chatVersion()} disabled={false} onStateChange={vi.fn()} onApplied={vi.fn()}/>);
    await screen.findByText(/The document or its sources changed/);
    expect(screen.getByRole('button',{name:'Apply and recheck private draft'})).toBeDisabled();
    expect(screen.queryByText(/This proposal needs a structural correction/)).toBeNull();
    expect(screen.queryByText(/You can apply it without/)).toBeNull();
  });

  it("regenerates a saved failed v1 proposal on current v2 in one click and applies only the new proposal", async () => {
    const version={...chatVersion(),revision:2,contentHash:'v2-hash',bodyMarkdown:'Preserve newer v2 edits'};
    const bad={...proposal(),applicable:false,preparationError:'invalid_knowledge: idea disposition is invalid'};
    const turns: KnowledgeRevisionConversation['turns']=[{id:'old-bad',state:'failed',userMessage:'Original edit',reply:bad,createdAt:'old'}];
    const requests: Array<{path:string;body?:string}>=[];
    let release!:()=>void;
    const waiting=new Promise<void>(resolve=>{release=resolve;});
    window.llmWikiApplication={request:vi.fn(async ({path,body})=>{
      requests.push({path,body});
      if(path.endsWith('/apply')) return response({draftRevision:3});
      if(body){
        const input=JSON.parse(body); await waiting;
        turns.push({id:input.operationId,state:'proposed',userMessage:input.message,reply:{...proposal(),id:input.operationId,baseRevision:2,applicable:true},createdAt:'new'});
        return response(turns.at(-1)!.reply);
      }
      return response({diagnostics:[],turns});
    })};
    const onApplied=vi.fn().mockResolvedValue(undefined);
    const props={taskId:'recover-current-v2',version,versions:[chatVersion(),version],disabled:false,onStateChange:vi.fn(),onApplied};
    const mounted=render(<KnowledgeRevisionChat {...props}/>);
    await screen.findByText(/idea disposition is invalid/);
    fireEvent.change(screen.getByRole('textbox'),{target:{value:'Keep my unsent detail'}});
    const update=screen.getByRole('button',{name:'Request updated proposal'});
    fireEvent.click(update);fireEvent.click(update);
    const sent=requests.filter(r=>r.body);
    expect(sent).toHaveLength(1);
    expect(sent[0].path).toContain('/versions/2/revision-chat');
    expect(JSON.parse(sent[0].body!)).toMatchObject({previousTurnId:'old-bad',expectedContentHash:'v2-hash'});
    expect(JSON.parse(sent[0].body!).message).toContain('Keep my unsent detail');
    expect(screen.getByRole('button',{name:'Apply and recheck private draft'})).toBeDisabled();
    mounted.unmount();render(<KnowledgeRevisionChat {...props}/>);
    expect((screen.getByRole('textbox') as HTMLTextAreaElement).value).toContain('Keep my unsent detail');
    await act(async()=>{release();await waiting;});
    await waitFor(()=>expect(screen.getByRole('button',{name:'Apply and recheck private draft'})).toBeEnabled());
    fireEvent.click(screen.getByRole('button',{name:'Apply and recheck private draft'}));
    await waitFor(()=>expect(onApplied).toHaveBeenCalledWith(3));
    expect(requests.find(r=>r.path.endsWith('/apply'))!.path).not.toContain('old-bad');
  });

  it("shows the latest clarification instead of automatically reopening an old failed proposal",async()=>{
    const turns: KnowledgeRevisionConversation['turns']=[{id:'old-failed',state:'failed',userMessage:'Original request',reply:{...proposal(),applicable:false,preparationError:'invalid_knowledge: idea disposition is invalid'},createdAt:'old'}];
    window.llmWikiApplication={request:vi.fn(async({body})=>{
      if(body){const input=JSON.parse(body);turns.push({id:input.operationId,state:'needs_evidence',userMessage:input.message,reply:{...proposal(),candidate:null,message:'Which classification matches your original idea?',applicable:false},createdAt:'new'});}
      return response({turns,diagnostics:[]});
    })};
    render(<KnowledgeRevisionChat taskId="clarify-recovery" version={chatVersion()} disabled={false} onStateChange={vi.fn()} onApplied={vi.fn()}/>);
    await screen.findByText(/idea disposition is invalid/);
    fireEvent.click(screen.getByRole('button',{name:'Request updated proposal'}));
    await screen.findByText('Which classification matches your original idea?');
    await waitFor(()=>expect(screen.queryByText(/idea disposition is invalid/)).toBeNull());
    expect(screen.queryByRole('button',{name:'Apply and recheck private draft'})).toBeNull();
    fireEvent.click(screen.getByRole('button',{name:/Review proposal in document/}));
    expect(screen.getByText(/idea disposition is invalid/)).toBeVisible();
  });

});
