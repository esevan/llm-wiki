import { describe, expect, it } from 'vitest';
import skill from '../../../../.agents/skills/llm-wiki-workflow/SKILL.md?raw';
import conflict from '../../../../.agents/skills/llm-wiki-workflow/references/conflict-review.md?raw';
import overview from '../../../../.agents/skills/llm-wiki-workflow/references/workbench-overview.md?raw';
import mcp from '../../../../src-tauri/src/mcp.rs?raw';

describe('representative chat Skill contract cases',()=>{
  it('orients a resume request without fetching the whole Workbench',()=>{
    expect(skill).toContain('current tracked session by default');
    expect(skill).toContain('llm-wiki://workbench/current');
    expect(skill).toContain('llm-wiki://work-session/{sessionId}');
  });
  it('requires exact review for Capture and direct Task continuation',()=>{
    expect(skill).toContain('mode:"continue_task"');
    expect(skill).toContain('taskId');
    expect(skill).toContain('inbound_work_open');
    expect(skill).toContain('"mode":"continue_task"');
    expect(skill).toContain('"taskId":"t1"');
    expect(skill).toContain('captureless session');
    expect(skill).toContain('Cancel/reject creates no session');
    expect(skill).toContain('new operation ID and a fresh preview');
    expect(mcp).toContain('name = "inbound_work_open"');
    expect(mcp).toContain('enum OpenMode');
    for(const mode of ['Create','Resume','ContinueTask']) expect(mcp).toContain(mode);
    for(const field of ['operation_id: String','lineage_key: String','mode: OpenMode','task_id: Option<String>']) expect(mcp).toContain(field);
  });
  it('reviews completion, private draft and publication independently',()=>{
    expect(skill).toContain('selectedEvidence');
    expect(skill).toContain('Knowledge로 발행할까요?');
    expect(skill).toContain('call `task_knowledge_draft`');
    expect(skill).toContain('task_knowledge_draft');
    expect(skill).toContain('completion_proposal');
    expect(skill).not.toContain('verify_and_complete');
    expect(skill).toContain('action":"complete_task"');
    expect(skill).toContain('expectedTaskRevision');
    expect(skill).toContain('sourceEventId');
    expect(skill).toContain('Saving the proposal event alone never completes work');
    for(const event of ['TaskCreated','TaskRevisionProposed','TaskTransitionProposed','ProblemResolutionProposed','WorkLogCheckpoint','CompletionProposal']) expect(mcp).toContain(event);
    for(const tool of ['task_refinement_open','task_refinement_get','task_refinement_message','task_refinement_workspace','task_refinement_proposals','task_refinement_decision','task_advisory_create','task_advisory_complete','task_advisory_get','task_advisory_history','task_advisory_cancel','task_advisory_decision','task_lineage_read','task_knowledge_draft','task_knowledge_correction','task_knowledge_regenerate','task_knowledge_publish','task_knowledge_withdraw']) expect(mcp).toContain(`name = "${tool}"`);
    expect(skill).toContain('task_knowledge_withdraw');
  });
  it('does conflict synthesis in this Chat with scoped dual retrieval',()=>{
    for(const tool of ['vault_search_lexical','vault_search_semantic','vault_evidence_read'])expect(conflict).toContain(tool);
    expect(conflict).toContain('explicit membership');
    expect(conflict).toContain('`coverage` to `insufficient`');
    expect(conflict).toContain('exact finding ID');
  });
  it('covers every overview page and never merges stale snapshots',()=>{
    expect(overview).toContain('until `nextCursor` is absent');
    expect(overview).toContain('do not combine pages from different revisions');
    expect(overview).toContain('Every visible non-archived item');
  });
});
