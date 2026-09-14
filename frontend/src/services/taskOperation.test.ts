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
  it('does not swallow unrelated routes or unsupported verbs', () => {
    expect(taskOperation('GET', '/provider/config', {}, 'en')).toBeUndefined();
    expect(taskOperation('DELETE', '/tasks/t1/completions', {}, 'en')).toBeUndefined();
  });
});
