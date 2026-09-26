import { describe, expect, it } from 'vitest';
import { taskOperation } from './taskOperation';

describe('Task contract routing', () => {
  it('binds the URL identity instead of accepting a forged body identity', () => {
    expect(taskOperation('POST', '/tasks/actual/completions', { taskId: 'other', operationId: 'op', expectedTaskRevision: 3 }, 'ko'))
      .toEqual({ name: 'task.completion.create', input: { taskId: 'actual', operationId: 'op', expectedTaskRevision: 3, locale: 'ko' } });
  });
  it('keeps Capture refinement separate from Task persistence', () => {
    expect(taskOperation('POST', '/captures/c1/refinement', { operationId: 'op' }, 'en'))
      .toEqual({ name: 'task-refinement.open', input: { operationId: 'op', captureId: 'c1', locale: 'en' } });
  });
  it('adds an idempotency identity when the legacy runtime revises a current Problem', () => {
    const operation = taskOperation('POST', '/problems/p1/revisions', { statement: 'Revised', expectedProblemRevision: 2 }, 'en');
    expect(operation).toMatchObject({ name: 'problem.revision', input: { problemId: 'p1', statement: 'Revised', expectedProblemRevision: 2 } });
    expect(operation?.input.operationId).toEqual(expect.any(String));
  });
  it('routes exact draft publication and URL-decodes identifiers', () => {
    expect(taskOperation('POST', '/tasks/t%201/knowledge/drafts/2/publish', { expectedContentHash: 'hash' }, 'en'))
      .toEqual({ name: 'task-knowledge.publish', input: { taskId: 't 1', draftRevision: 2, expectedContentHash: 'hash', locale: 'en' } });
  });
  it('queues Knowledge draft generation with the URL task identity and expected revision', () => {
    expect(taskOperation('POST', '/tasks/t%201/knowledge/drafts', { expectedTaskRevision: 4 }, 'en'))
      .toEqual({ name: 'jobs.enqueue', input: expect.objectContaining({ taskKind: 'knowledge_draft', entityType: 'tasks', entityId: 't 1', taskId: 't 1', expectedTaskRevision: 4, locale: 'en' }) });
  });
  it('routes read-only Knowledge review and restore-as-new separately', () => {
    expect(taskOperation('GET', '/tasks/task%201/knowledge', {}, 'en'))
      .toEqual({ name: 'task-knowledge.get', input: { taskId: 'task 1', locale: 'en' } });
    expect(taskOperation('POST', '/tasks/task%201/knowledge/versions/7/restore', { expectedCurrentPrivateRevision: 9 }, 'ko'))
      .toEqual({ name: 'task-knowledge.restore', input: { taskId: 'task 1', revision: 7, expectedCurrentPrivateRevision: 9, locale: 'ko' } });
  });
  it('routes archive review, organization, publication, status, retry, and recovery commands', () => {
    expect(taskOperation('POST', '/knowledge/archive/prepare', { operationId: 'prepare-1', taskId: 'task-1' }, 'en'))
      .toEqual({ name: 'task-knowledge.archive-prepare', input: { operationId: 'prepare-1', taskId: 'task-1', locale: 'en' } });
    expect(taskOperation('POST', '/knowledge/archive/organize', { operationId: 'organize-1', documentId: 'doc-1', intent: 'rename' }, 'en'))
      .toEqual({ name: 'task-knowledge.archive-organize', input: { operationId: 'organize-1', documentId: 'doc-1', intent: 'rename', locale: 'en' } });
    expect(taskOperation('POST', '/knowledge/archive/publish', { operationId: 'publish-1', proposalId: 'proposal-1' }, 'ko'))
      .toEqual({ name: 'task-knowledge.archive-publish', input: { operationId: 'publish-1', proposalId: 'proposal-1', locale: 'ko' } });
    expect(taskOperation('GET', '/knowledge/archive/operations/publish%201', { operationId: 'forged' }, 'en'))
      .toEqual({ name: 'task-knowledge.archive-status', input: { operationId: 'publish 1', locale: 'en' } });
    expect(taskOperation('POST', '/knowledge/archive/operations/publish%201/retry', { operationId: 'forged' }, 'en'))
      .toEqual({ name: 'task-knowledge.archive-retry', input: { operationId: 'publish 1', locale: 'en' } });
    expect(taskOperation('POST', '/knowledge/archive/operations/publish%201/recover', { operationId: 'forged', choice: 'finish' }, 'ko'))
      .toEqual({ name: 'task-knowledge.archive-recover', input: { operationId: 'publish 1', choice: 'finish', locale: 'ko' } });
  });
  it('queues image summaries for the URL Work Log entry without body overrides', () => {
    expect(taskOperation('POST', '/work-log/entry%201/image-summary', { entityId: 'forged', entityType: 'features', taskKind: 'other' }, 'ko'))
      .toEqual({ name: 'jobs.enqueue', input: { taskKind: 'image_summary', entityType: 'task_work_log_entries', entityId: 'entry 1', locale: 'ko' } });
  });
  it('binds work-session reads and writes to both URL identities', () => {
    expect(taskOperation('GET','/tasks/task%20a/work-sessions/session%201',{ taskId:'forged',sessionId:'wrong' },'en'))
      .toEqual({name:'task.work-session.get',input:{taskId:'task a',sessionId:'session 1',locale:'en'}});
    expect(taskOperation('POST','/tasks/task%20a/work-sessions/session%201/entries',{operationId:'entry',taskId:'forged',sessionId:'wrong',body:'note'},'ko'))
      .toEqual({name:'task.work-session.entry.create',input:{operationId:'entry',taskId:'task a',sessionId:'session 1',body:'note',locale:'ko'}});
  });
  it('routes execution reads and controls to the exact Task, session, Run, and request', () => {
    expect(taskOperation('GET', '/tasks/task%20a/work-sessions/session%201/runs', { taskId: 'forged' }, 'en'))
      .toEqual({ name: 'task.execution.list', input: { taskId: 'task a', sessionId: 'session 1', locale: 'en' } });
    expect(taskOperation('POST', '/tasks/task%20a/work-sessions/session%201/runs', { sessionId: 'wrong', instruction: 'run' }, 'en'))
      .toEqual({ name: 'task.execution.start', input: { taskId: 'task a', sessionId: 'session 1', instruction: 'run', locale: 'en' } });
    expect(taskOperation('POST', '/tasks/task%20a/work-sessions/session%201/runs/run%201/interrupt', { runId: 'wrong' }, 'ko'))
      .toEqual({ name: 'task.execution.interrupt', input: { taskId: 'task a', sessionId: 'session 1', runId: 'run 1', locale: 'ko' } });
    expect(taskOperation('POST', '/tasks/task%20a/work-sessions/session%201/runs/run%201/formal-requests/request%201/response', { requestId: 'wrong', response: { decision: 'approve' } }, 'en'))
      .toEqual({ name: 'task.execution.formal-response', input: { taskId: 'task a', sessionId: 'session 1', runId: 'run 1', requestId: 'request 1', response: { decision: 'approve' }, locale: 'en' } });
  });
  it('does not swallow unrelated routes or unsupported verbs', () => {
    expect(taskOperation('GET', '/provider/config', {}, 'en')).toBeUndefined();
    expect(taskOperation('DELETE', '/tasks/t1/completions', {}, 'en')).toBeUndefined();
  });
});
