import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { TauriApplicationClient } from '../../services/tauriApplicationClient';
import { CompassView } from './CompassView';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
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


describe('Compass publication review through the native adapter', () => {
  it('prepares exact files, waits for approval and completes publication', async () => {
    const task = { id: 'task-1', kind: 'task', title: 'Completed offline work', state: 'completed', taskRevision: 2, workLog: [], checklist: [], decisions: [], readinessEntries: [] };
    let published = false;
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      expect(command).toBe('workflow_command');
      const { operation } = args as { operation: { name: string; input: Record<string, unknown> } };
      let body: unknown = {};
      switch (operation.name) {
        case 'workbench.get': body = { categories: [{ items: [task] }] }; break;
        case 'task.get': body = task; break;
        case 'task.lineage': body = { nodes: [], edges: [], unresolved: [] }; break;
        case 'task-knowledge.get': body = projection(); break;
        case 'task-knowledge.archive-prepare':
          expect(operation.input).toMatchObject({ taskId: 'task-1', knowledgeRevision: 2, expectedKnowledgeContentHash: 'content-2', expectedGenerationSnapshotHash: 'snapshot-current', selectedIdeaRevisionIds: [] });
          body = { proposalId: 'proposal-1', proposalVersion: 1, proposalHash: 'exact-hash', state: 'review', outcome: 'ready', rationale: 'Reviewed local file', artifacts: [{ documentId: 'knowledge-1', path: 'knowledge/offline.md', bytes: '# Exact offline article', sha256: 'file-hash', kind: 'knowledge' }], mocPatches: [], referenceLinks: [] }; break;
        case 'task-knowledge.archive-publish':
          expect(operation.input).toMatchObject({ proposalId: 'proposal-1', proposalVersion: 1, proposalHash: 'exact-hash' });
          published = true; body = { operationId: 'publication-1', state: 'complete' }; break;
      }
      return { status: 200, body };
    });
    window.llmWikiApplication = new TauriApplicationClient();
    render(<CompassView active />);
    fireEvent.click(screen.getByRole('button', { name: 'Completed tasks' }));
    fireEvent.click(await screen.findByRole('button', { name: /Completed offline work/ }));
    fireEvent.click(await screen.findByRole('tab', { name: 'Review' }));
    const prepare = await screen.findByRole('button', { name: 'Publish Knowledge' });
    expect(prepare).toBeEnabled();
    fireEvent.click(prepare);
    expect(await screen.findByRole('button', { name: 'knowledge/offline.md' })).toBeVisible();
    expect(published).toBe(false);
    fireEvent.click(screen.getByRole('button', { name: 'knowledge/offline.md' }));
    expect(await screen.findByRole('dialog', { name: 'knowledge/offline.md' })).toHaveTextContent('Exact offline article');
    fireEvent.click(screen.getByRole('button', { name: 'Close reference' }));
    fireEvent.click(screen.getByRole('button', { name: 'Publish these exact files' }));
    await waitFor(() => expect(published).toBe(true));
    await waitFor(() => expect(vi.mocked(invoke).mock.calls.some(([, args]) => (args as { operation?: { name: string } })?.operation?.name === 'task-knowledge.archive-publish')).toBe(true));
  });
});
