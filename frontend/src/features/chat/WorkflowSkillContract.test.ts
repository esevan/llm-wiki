import { describe, expect, it } from 'vitest';
import skill from '../../../../.agents/skills/llm-wiki-workflow/SKILL.md?raw';
import conflict from '../../../../.agents/skills/llm-wiki-workflow/references/conflict-review.md?raw';
import overview from '../../../../.agents/skills/llm-wiki-workflow/references/workbench-overview.md?raw';

describe('representative chat Skill contract cases',()=>{
  it('orients a resume request without fetching the whole Workbench',()=>{
    expect(skill).toContain('current tracked session by default');
    expect(skill).toContain('llm-wiki://workbench/current');
    expect(skill).toContain('llm-wiki://work-session/{sessionId}');
  });
  it('requires Capture consent and fresh review after an edit',()=>{
    expect(skill).toContain('Cancel/reject creates no Capture or session');
    expect(skill).toContain('new operation ID and a fresh preview');
  });
  it('reviews completion, private draft and publication independently',()=>{
    expect(skill).toContain('selectedEvidence');
    expect(skill).toContain('Knowledge로 발행할까요?');
    expect(skill).toContain('call `knowledge_draft_save`');
    expect(skill).toContain('call `knowledge_publish`');
    expect(skill).toContain('knowledge_publication_withdraw');
  });
  it('does conflict synthesis in this Chat with scoped dual retrieval',()=>{
    for(const tool of ['vault_search_lexical','vault_search_semantic','vault_evidence_read'])expect(conflict).toContain(tool);
    expect(conflict).toContain('explicit membership');
    expect(conflict).toContain('`coverage` to `insufficient`');
    expect(conflict).toContain('not a\nSolution-draft ID');
  });
  it('covers every overview page and never merges stale snapshots',()=>{
    expect(overview).toContain('until `nextCursor` is absent');
    expect(overview).toContain('do not combine pages from different revisions');
    expect(overview).toContain('Every visible non-archived item');
  });
});
