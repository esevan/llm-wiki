import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { TauriApplicationClient } from '../../services/tauriApplicationClient';
import { App } from '../../app/App';
import { fixtureWorkspace, fixtureVersion } from '../workbench/referencePreviewFixtures';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
// jsdom does not implement browser focus/inert layout; modal behavior is covered separately.
vi.mock('../workbench/useModalInteraction', () => ({ useModalInteraction: () => {} }));
// Keep the publication route real; unrelated inactive surfaces are outside this fixture.
vi.mock('../../features/settings/SettingsView', () => ({ SettingsView: () => null }));
vi.mock('../../features/search/SearchView', () => ({ SearchView: () => null }));
const source = { type: "task_distillation", id: "claim-1", revision: "2", locator: "claims/claim-1", quote: "The verified result is available offline." };
const version = (revision = 2, bodyMarkdown = "# Offline use\n\nThe verified result is available offline.") => ({
  revision,
  parentRevision: revision - 1 || undefined,
  derivationKind: revision === 1 ? "generated" : "edited",
  articleType: "guide",
  title: "Offline use",
  bodyMarkdown,
  contentHash: `content-${revision}`,
  generationSnapshotHash: "snapshot-current",
  freshness: "current",
  storedFreshness: "current",
  qualityState: "valid",
  modelStatus: "enhanced",
  modelError: "",
  createdAt: `2026-09-2${revision}T12:00:00Z`,
  applicability: {
    summary: "Use this when work must remain local.",
    representativeQuestions: ["Can this run offline?"],
    helpsWith: ["Local review"],
    conditions: ["The local index is current"],
    exclusions: ["It does not prove remote availability"],
  },
  result: {
    article: {
      type: "guide",
      title: "Offline use",
      finalOutcomes: [{ topicKey: "storage", statement: "Keep final Knowledge local.", claimIds: ["claim-1"] }],
      bodyMarkdown,
      applicability: { summary: "Use this when work must remain local.", representativeQuestions: ["Can this run offline?"], helpsWith: ["Local review"], conditions: ["The local index is current"], exclusions: ["It does not prove remote availability"], sourceRefs: [source] },
      claimBindings: [{ claimId: "claim-1", statement: "The verified result is available offline.", epistemicState: "verified", sourceRefs: [source] }],
      assumptionBindings: [],
    },
    ideas: [],
    qualityFindings: [{ kind: "source_note", severity: "info", locator: "claims/claim-1", message: "Exact source retained", sourceRefs: [source] }],
  },
});
const idea = {
  id: "idea-reminder", revision: 1, knowledgeRevision: 2, title: "Reminder automation",
  bodyMarkdown: "Try a reminder automation later.", disposition: "unverified", reconsiderationConditions: ["A scheduler is connected"],
  sourceRefs: [source], relatedTopicKeys: ["storage"], contentHash: "idea-hash", publicationState: "private",
};
const projection = (versions = [version(1), version(2)]) => ({
  taskId: "task-1",
  pointers: { currentPrivateRevision: versions.at(-1)?.revision, publishedRevision: 1, publicationDocumentId: "knowledge-1", publicationPath: "knowledge/offline.md", publicationContentHash: "published-hash" },
  versions,
  ideas: [idea],
});


describe('App Workbench publication review through the native adapter', () => {
  it.each([false, true, "blocked", "chat"])('preserves review wiring after Workbench completion or recent reopen (already completed: %s)', async (alreadyCompleted) => {
    const task = { id: 'task-1', kind: 'task', title: 'Completed offline work', state: alreadyCompleted ? 'completed' : 'in_progress', taskRevision: 2, workLog: [], checklist: [], decisions: [], readinessEntries: [] };
    let published = false;
    let draftVersion = version(2);
    const notes: Array<{id:string;knowledgeRevision:number;contentHash:string;body:string;createdAt:string}> = [];
    const revisionTurns: Array<Record<string, unknown>> = [];
    if (alreadyCompleted === "blocked" || alreadyCompleted === "chat") {
      draftVersion.qualityState = "blocked_unsupported";
      draftVersion.modelError = "Rejected citation task_journey:5b722bbc-5fd8-4418-8f7d-ed77703ecbc6 quote absent";
      draftVersion.result.qualityFindings = [{ kind:"missing_reference",severity:"blocking",locator:"sourceRefs",message:draftVersion.modelError,sourceRefs:[] }];
    }
    vi.mocked(invoke).mockImplementation(async (_command, args) => {
      // Other mounted App surfaces may read settings through their own domains.
      const { operation } = args as { operation: { name: string; input: Record<string, unknown> } };
      let body: unknown = { connections: [] };
      switch (operation.name) {
        case 'workbench.get': body = { revision: 1, activeShortcuts: [], refiningShortcuts: [], categories: [{ id: 'general', label: 'General', items: [task] }] }; break;
        case 'task.get': body = task; break;
        case 'task.completion.create': task.state = 'completed'; task.taskRevision++; body = task; break;
        case 'task.lineage': body = { nodes: [], edges: [], unresolved: [] }; break;
        case 'task-knowledge.get': body = { ...projection([draftVersion]), reviewNotes:notes }; break;
        case 'task-knowledge.revision-get': body = {diagnostics:alreadyCompleted==='chat'?[{kind:'missing_reference',locator:'article.applicability',statement:'Review offline work',reason:'Quote does not match the selected source',direction:'Replace it with the exact saved quote',evidence:[source]}]:[],turns:revisionTurns};break;
        case 'task-knowledge.revision-send': {
          expect(operation.input).toMatchObject({taskId:'task-1',baseRevision:2,expectedContentHash:'content-2',message:'Replace the rejected citation and retain offline scope'});
          const reply={id:operation.input.operationId,baseRevision:2,message:'Corrected citation proposal',missingEvidence:[],candidate:version(3).result,qualityState:'valid',diagnostics:[]};
          revisionTurns.push({id:operation.input.operationId,userMessage:operation.input.message,reply,state:'proposed',createdAt:'now'});body=reply;break;
        }
        case 'task-knowledge.revision-apply': draftVersion=version(3);body={draftRevision:3};break;
        case 'task-knowledge.review-note': notes.push({id:'note-1',knowledgeRevision:draftVersion.revision,contentHash:draftVersion.contentHash,body:operation.input.body as string,createdAt:'2026-10-21'}); body={}; break;
        case 'jobs.enqueue': draftVersion=version(3);body={jobId:'regenerated-job',status:'queued'};break;
        case 'task-knowledge.archive-prepare':
          expect(operation.input).toMatchObject({ taskId: 'task-1', knowledgeRevision: draftVersion.revision, expectedKnowledgeContentHash: draftVersion.contentHash, expectedGenerationSnapshotHash: 'snapshot-current', selectedIdeaRevisionIds: [] });
          body = { proposalId: 'proposal-1', proposalVersion: 1, proposalHash: 'exact-hash', state: 'review', outcome: 'ready', rationale: 'Reviewed local file', artifacts: [{ documentId: 'knowledge-1', path: 'knowledge/offline.md', bytes: '# Exact offline article', sha256: 'file-hash', kind: 'knowledge' }], mocPatches: [], referenceLinks: [] }; break;
        case 'task-knowledge.archive-publish':
          expect(operation.input).toMatchObject({ proposalId: 'proposal-1', proposalVersion: 1, proposalHash: 'exact-hash' });
          published = true; body = { operationId: 'publication-1', state: 'complete' }; break;
      }
      return { status: 200, body };
    });
    window.llmWikiApplication = new TauriApplicationClient();
    render(<App />);
    await waitFor(() => expect(document.querySelector('.completed-task-list button, .canonical-card-title')).not.toBeNull());
    const open = document.querySelector<HTMLButtonElement>(alreadyCompleted ? '[data-control="workbench-completed-open"]' : '[data-control="task-shortcut-open"]');
    expect(open).not.toBeNull();
    fireEvent.click(open!);
    await waitFor(() => expect(document.querySelector('[data-control=task-detail-tab-review]')).not.toBeNull());
    fireEvent.click(document.querySelector('[data-control=task-detail-tab-review]')!);
    if (!alreadyCompleted) {
      fireEvent.change(document.querySelector('[data-control=task-completion-evidence]')!, { target: { value: 'Verified completion fixture' } });
      fireEvent.click(document.querySelector('[data-control=task-completion-complete]')!);
      await waitFor(() => expect(task.state).toBe('completed'));
      await waitFor(() => expect(document.querySelector('[data-control="workbench-completed-open"]')).toBeInTheDocument());
    }
    expect(document.querySelector('#workbench')).toHaveClass('active');
    expect(document.querySelector('#compass')).not.toHaveClass('active');
    expect(screen.queryByText(/Publication review is unavailable in this view/)).toBeNull();
    await waitFor(() => expect(document.querySelector('[data-control=knowledge-archive-prepare]')).not.toBeNull());
    expect(document.querySelector(".knowledge-reading .knowledge-article")).not.toBeNull();
    expect(document.querySelector(".knowledge-controls [data-control=knowledge-archive-prepare]")).not.toBeNull();
    const prepare = document.querySelector('[data-control=knowledge-archive-prepare]')!;
    expect(prepare).toBeEnabled();
    fireEvent.change(document.querySelector('[data-control=knowledge-review-note-body]')!, {target:{value:'Review recorded privately.'}});
    fireEvent.click(document.querySelector('[data-control=knowledge-review-note-save]')!);
    await screen.findByText('Review recorded privately.');
    fireEvent.click(prepare);
    if (alreadyCompleted === 'chat') {
      expect(await screen.findByRole('alert')).toHaveTextContent('Some statements or citations');
      fireEvent.change(screen.getByRole('textbox',{name:'What should change?'}),{target:{value:'Replace the rejected citation and retain offline scope'}});
      fireEvent.click(screen.getByRole('button',{name:'Propose revision'}));
      expect(await screen.findByText('Corrected citation proposal')).toBeVisible();expect(published).toBe(false);
      fireEvent.click(screen.getByRole('button',{name:'Apply and recheck private draft'}));
      await waitFor(()=>expect(document.querySelector('.knowledge-review')).toHaveAttribute('data-content-hash','content-3'));
      expect(published).toBe(false);
      fireEvent.click(document.querySelector('[data-control=knowledge-archive-prepare]')!);
    }
    if (alreadyCompleted === 'blocked') {
      expect(await screen.findByRole('alert')).toHaveTextContent('Some statements or citations');
      expect(document.querySelector('.knowledge-review')!.textContent).not.toContain('5b722bbc');
      expect(published).toBe(false);
      fireEvent.click(document.querySelector('[data-control=knowledge-publication-regenerate]')!);
      await waitFor(()=>expect(draftVersion.revision).toBe(3));
      await act(async()=>{window.llmWikiOpenKnowledgeDraft?.({taskId:task.id,draftRevision:3,bodyMarkdown:draftVersion.bodyMarkdown,contentHash:draftVersion.contentHash,state:"draft"});});
      await waitFor(()=>expect(document.querySelector('.knowledge-review')).toHaveAttribute('data-content-hash','content-3'));
      fireEvent.click(document.querySelector('[data-control=knowledge-archive-prepare]')!);
    }
    await waitFor(() => expect(document.querySelector('[data-control=knowledge-archive-artifact-open]')).toBeVisible());
    expect(published).toBe(false);
    fireEvent.click(document.querySelector('[data-control=knowledge-archive-artifact-open]')!);
    expect(await screen.findByRole('dialog', { name: 'knowledge/offline.md' })).toHaveTextContent('Exact offline article');
    fireEvent.click(screen.getByRole('button', { name: 'Close reference' }));
    fireEvent.click(document.querySelector('[data-control=knowledge-archive-publish]')!);
    await waitFor(() => expect(published).toBe(true));
    await waitFor(() => expect(vi.mocked(invoke).mock.calls.some(([, args]) => (args as { operation?: { name: string } })?.operation?.name === 'task-knowledge.archive-publish')).toBe(true));
  });
  it('keeps preview details editable and maps applied completion criteria to checkable Task items', async () => {
    const ws=fixtureWorkspace(2);
    const task={id:'preview-task',kind:'task',title:'Original work',state:'in_progress',taskRevision:1,workLog:[],checklist:[{id:'manual',body:'Manual check',checked:true}],decisions:[],readinessEntries:[]};
    vi.mocked(invoke).mockImplementation(async (_command,args)=>{
      const {operation}=args as {operation:{name:string;input:Record<string,unknown>}};
      let body:unknown={connections:[]};
      switch(operation.name){
        case 'workbench.get':body={revision:1,activeShortcuts:[],refiningShortcuts:[],categories:[{id:'general',label:'General',items:[task]}]};break;
        case 'task.get':body=task;break;
        case 'task.lineage':body={nodes:[],edges:[],unresolved:[]};break;
        case 'task-refinement.open':case 'task-refinement.get':body={id:'preview-session',draftRevision:1,messages:[]};break;
        case 'task-refinement.reference-workspace':body=ws;break;
        case 'task-refinement.proposals':body=[];break;
        case 'task-refinement.mention-draft':body={mentions:[]};break;
        case 'task-refinement.reference-compare':body={left:fixtureVersion(1),right:ws.preview!.current,comparison:{left:fixtureVersion(1),right:ws.preview!.current!,fields:[{key:'goal',state:'changed'}]}};break;
        case 'task-refinement.reference-edit':{
          const fields=operation.input.fields as NonNullable<NonNullable<typeof ws.preview>["current"]>["fields"];
          expect(fields.scope).toBe('Preserved edited scope');
          expect(fields.completionCriteria).toEqual(['Reviewed contract']);
          ws.preview!.current={...ws.preview!.current!,version:3,contentHash:'edited-3',fields};ws.preview!.currentVersion=3;
          body={previewId:'preview',version:3,derivedFromVersion:2,contentHash:'edited-3'};break;
        }
        case 'task-refinement.reference-apply':
          task.checklist.push({id:'criterion',body:'Reviewed contract',checked:false});task.taskRevision++;body={task:{id:task.id,taskRevision:task.taskRevision},version:3,transitionCause:'preview_adopted'};break;
        case 'task.checklist.update':task.checklist[1].checked=true;body=task;break;
      }
      return {status:200,body};
    });
    window.llmWikiApplication=new TauriApplicationClient();render(<App/>);
    fireEvent.click(await screen.findByRole('button',{name:'Original work'}));
    await waitFor(()=>expect(document.querySelector('[data-control="task-detail-tab-details"]')).not.toBeNull());
    fireEvent.click(document.querySelector('[data-control="task-detail-tab-details"]')!);
    await screen.findByText('Contract preview 2');
    expect(task.checklist).toHaveLength(1);
    expect(document.querySelector(".refinement-support-rail .refinement-conversation")).not.toBeNull();
    expect(document.querySelector(".refinement-evidence-slot .preview-evidence-card")).not.toBeNull();
    expect(document.querySelector(".reference-aware-preview .reference-aware-reference-list")).toBeNull();
    expect(document.querySelector('[data-control="reference-preview-more"]')).not.toHaveAttribute('open');
    fireEvent.click(screen.getByRole('button',{name:'Edit preview'}));
    fireEvent.change(screen.getByLabelText('Edit preview: Scope'),{target:{value:'Preserved edited scope'}});
    fireEvent.click(screen.getByRole('button',{name:'Save preview'}));
    await screen.findByRole('button',{name:'Edit preview'});
    fireEvent.click(screen.getByRole('button',{name:'Apply to Task'}));
    await waitFor(()=>expect(task.checklist).toHaveLength(2));
    await waitFor(()=>expect(document.querySelector('[data-control="task-detail-tab-work"]')).not.toBeNull());
    fireEvent.click(document.querySelector('[data-control="task-detail-tab-work"]')!);
    const item=await screen.findByRole('checkbox',{name:'Reviewed contract'});
    expect(item).not.toBeChecked();fireEvent.click(item);
    await waitFor(()=>expect(task.checklist[1].checked).toBe(true));
    expect(task.checklist[0].checked).toBe(true);
  });

});
