import type { ApplicationResponse, HttpMethod } from "../types/application";
import type {
  InputImage,
  ConflictReview,
  ConflictReviewHistory,
  KnowledgeReviewProjection,
  KnowledgeArchiveProposal,
  RefinementProposal,
  RefinementSession,
  TaskAggregate,
  TaskWorkSession,
  TaskWorkSessionAttachment,
  TaskWorkSessionRecord,
  WorkbenchSnapshot,
  ReferenceWorkspace,
  WorkPreviewVersion,
  PreviewComparison,
  ExactReferenceBinding,
  DocumentMention,
  WorkPreviewFields,
} from "../types/taskWorkbench";

const locale = () =>
  document.documentElement.lang.startsWith("ko") ? "ko" : "en";
const operationId = () =>
  globalThis.crypto?.randomUUID?.() ??
  `operation-${Date.now()}-${Math.random().toString(16).slice(2)}`;
const request = async <T>(
  path: string,
  method: HttpMethod = "GET",
  body?: Record<string, unknown>,
): Promise<T> => {
  const response: ApplicationResponse = await window.llmWikiApplication.request(
    {
      path,
      method,
      headers: {
        "X-LLM-Wiki-Locale": locale(),
        ...(body ? { "Content-Type": "application/json" } : {}),
      },
      body: body
        ? JSON.stringify({ operationId: operationId(), ...body })
        : undefined,
    },
  );
  if (!response.ok) {
    const payload = await response
      .json<{ detail?: string; error?: string }>()
      .catch(() => undefined);
    throw new Error(
      payload?.detail ?? payload?.error ?? `Request failed (${response.status})`,
    );
  }
  return response.status === 204 ? (undefined as T) : response.json<T>();
};

export const taskClient = {
  workbench: () => request<WorkbenchSnapshot>("/workbench"),
  createCapture: (text: string, images?: InputImage[]) => request("/captures", "POST", { text, images }),
  createTask: (inputText: string) =>
    request<TaskAggregate>("/tasks", "POST", { inputText, title: inputText }),
  deleteItem: (entityType: "captures" | "problems" | "features" | "tasks", id: string) =>
    entityType === "tasks"
      ? request(`/tasks/${encodeURIComponent(id)}`, "DELETE")
      : request(`/items/${entityType}/${encodeURIComponent(id)}`, "DELETE"),
  createProblem: (statement: string, detail = "") =>
    request<{ id: string; problemRevision: number }>("/problems", "POST", {
      statement,
      detail,
    }),
  reviseProblem: (id: string, statement: string, detail = "") =>
    request<{ id: string; problemRevision: number }>(
      `/problems/${encodeURIComponent(id)}/revisions`,
      "POST",
      { statement, detail },
    ),
  task: (id: string) =>
    request<TaskAggregate>(`/tasks/${encodeURIComponent(id)}`),
  prepareKnowledgeArchive: (input: { operationId: string; taskId: string; knowledgeRevision: number; expectedKnowledgeContentHash: string; expectedGenerationSnapshotHash: string; selectedIdeaRevisionIds: Array<{ id: string; revision: number }>; locale: "en" | "ko" }) =>
    request<KnowledgeArchiveProposal>("/knowledge/archive/prepare", "POST", input),
  organizeKnowledgeArchive: (input: { operationId: string; documentId: string; expectedRevision: string; intent: "move" | "rename" | "withdraw" | "repair"; requestedPath?: string }) =>
    request<KnowledgeArchiveProposal>("/knowledge/archive/organize", "POST", input),
  publishKnowledgeArchive: (input: { operationId: string; proposalId: string; proposalVersion: number; proposalHash: string }) =>
    request<{ operationId?: string; state: "writing" | "index_pending" | "index_failed" | "complete" | "conflict" | "repair_required" | "compensated"; error?: string }>("/knowledge/archive/publish", "POST", input),
  knowledgeArchiveStatus: (archiveOperationId: string) =>
    request<{ operationId?: string; state: "writing" | "index_pending" | "index_failed" | "complete" | "conflict" | "repair_required" | "compensated"; error?: string }>(`/knowledge/archive/operations/${encodeURIComponent(archiveOperationId)}`),
  retryKnowledgeArchive: (archiveOperationId: string) =>
    request<{ operationId?: string; state: "writing" | "index_pending" | "index_failed" | "complete" | "conflict" | "repair_required" | "compensated"; error?: string }>(`/knowledge/archive/operations/${encodeURIComponent(archiveOperationId)}/retry`, "POST"),
  recoverKnowledgeArchive: (input: { operationId: string; choice: "finish" | "compensate" }) =>
    request<{ operationId?: string; state: "writing" | "index_pending" | "index_failed" | "complete" | "conflict" | "repair_required" | "compensated"; error?: string }>(`/knowledge/archive/operations/${encodeURIComponent(input.operationId)}/recover`, "POST", input),
  workSessions: (taskId: string) => request<{ sessions: TaskWorkSession[] }>(`/tasks/${encodeURIComponent(taskId)}/work-sessions`),
  createWorkSession: (taskId: string, title: string) => request<TaskWorkSession>(`/tasks/${encodeURIComponent(taskId)}/work-sessions`, "POST", { title }),
  workSession: (taskId: string, sessionId: string) => request<TaskWorkSessionRecord>(`/tasks/${encodeURIComponent(taskId)}/work-sessions/${encodeURIComponent(sessionId)}`),
  saveWorkSession: (taskId: string, session: TaskWorkSession) => request<TaskWorkSession>(`/tasks/${encodeURIComponent(taskId)}/work-sessions/${encodeURIComponent(session.id)}`, "PUT", { title: session.title, provider: session.provider, model: session.model, approvalMode: session.approvalMode, approvalsReviewer: session.approvalsReviewer, workspacePath: session.workspacePath }),
  appendWorkSessionEntry: (taskId: string, sessionId: string, body: string, attachment: TaskWorkSessionAttachment | undefined, entryOperationId: string) => request(`/${"tasks"}/${encodeURIComponent(taskId)}/work-sessions/${encodeURIComponent(sessionId)}/entries`, "POST", { operationId: entryOperationId, author: "user", kind: "note", body, attachment }),
  revise: (
    id: string,
    expectedTaskRevision: number,
    patch: Record<string, string>,
  ) =>
    request<Required<Pick<TaskAggregate, "id" | "taskRevision" | "title" | "detail" | "outcome" | "scope" | "nonGoals" | "validationCriteria">>>(
      `/tasks/${encodeURIComponent(id)}/revisions`,
      "POST",
      { expectedTaskRevision, patch },
    ),
  transition: (
    id: string,
    expectedTaskRevision: number,
    to: "in_progress" | "completed" | "reopen",
    reason?: string,
    continueDecision?: boolean,
  ) =>
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(id)}/transitions`,
      "POST",
      { expectedTaskRevision, to, reason, continueDecision },
    ),
  workLog: (
    id: string,
    expectedTaskRevision: number,
    body: string,
    attachment?: Record<string, string>,
  ) =>
    request<TaskAggregate & { imageSummaryQueueError?: string; translationQueueError?: string }>(
      `/tasks/${encodeURIComponent(id)}/work-log`,
      "POST",
      { expectedTaskRevision, body, attachment },
    ),
  summarizeImage: (entryId: string) =>
    request(`/work-log/${encodeURIComponent(entryId)}/image-summary`, "POST", {}),
  comment: (entryId: string, body: string) =>
    request(`/work-log/${encodeURIComponent(entryId)}/comments`, "POST", {
      body,
    }),
  checklist: (id: string, expectedTaskRevision: number, body: string) =>
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(id)}/checklist`,
      "POST",
      { expectedTaskRevision, body },
    ),
  updateChecklist: (
    taskId: string,
    expectedTaskRevision: number,
    itemId: string,
    checked: boolean,
    body: string,
  ) =>
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(taskId)}/checklist/${encodeURIComponent(itemId)}`,
      "PUT",
      { expectedTaskRevision, checked, body },
    ),
  decision: (id: string, expectedTaskRevision: number, body: string) =>
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(id)}/decisions`,
      "POST",
      { expectedTaskRevision, kind: "user", payload: { body } },
    ),
  complete: (id: string, expectedTaskRevision: number, evidence: string) =>
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(id)}/completions`,
      "POST",
      { expectedTaskRevision, evidence },
    ),
  readiness: (id: string) =>
    request<TaskAggregate>(`/tasks/${encodeURIComponent(id)}/readiness`),
  readinessDecision: (
    id: string,
    expectedTaskRevision: number,
    key: string,
    reason: string,
  ) =>
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(id)}/readiness-decisions`,
      "POST",
      { expectedTaskRevision, key, status: "not_applicable", reason },
    ),
  problemLink: (
    id: string,
    expectedTaskRevision: number,
    problemId: string,
    problemRevision: number,
    relationship = "context",
    note = "",
  ) =>
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(id)}/problem-links`,
      "POST",
      { expectedTaskRevision, problemId, problemRevision, relationship, note },
    ),
  unlinkProblem: (id: string, linkId: string, expectedTaskRevision: number) =>
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(id)}/problem-links/${encodeURIComponent(linkId)}`,
      "DELETE",
      { expectedTaskRevision },
    ),
  relationship: (
    id: string,
    expectedTaskRevision: number,
    targetTaskId: string,
    kind: "prerequisite" | "split_from" | "related",
    note = "",
  ) =>
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(id)}/relationships`,
      "POST",
      { expectedTaskRevision, targetTaskId, kind, note },
    ),
  unlinkRelationship: (
    id: string,
    relationshipId: string,
    expectedTaskRevision: number,
  ) =>
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(id)}/relationships/${encodeURIComponent(relationshipId)}`,
      "DELETE",
      { expectedTaskRevision },
    ),
  resolveProblem: (
    problemId: string,
    problemRevision: number,
    rationale: string,
  ) =>
    request(`/problems/${encodeURIComponent(problemId)}/resolutions`, "POST", {
      expectedProblemRevision: problemRevision,
      rationale,
      evidenceRefs: [],
    }),
  refinement: (kind: "capture" | "task", id: string) =>
    request<RefinementSession>(
      `/${kind === "capture" ? "captures" : "tasks"}/${encodeURIComponent(id)}/refinement`,
      "POST",
      {},
    ),
  refinementStatus: (kind: "capture" | "task", id: string) =>
    request<RefinementSession>(
      `/${kind === "capture" ? "captures" : "tasks"}/${encodeURIComponent(id)}/refinement`,
    ),
  saveWorkspace: (id: string, data: Record<string, unknown>) =>
    request(`/refinement/${encodeURIComponent(id)}/workspace`, "PUT", data),
  proposals: (id: string) =>
    request<RefinementProposal[]>(
      `/refinement/${encodeURIComponent(id)}/proposals`,
    ),
  message: (id: string, message: string, images?: InputImage[], mentions?: DocumentMention[]) =>
    request<RefinementSession>(
      `/refinement/${encodeURIComponent(id)}/messages`,
      "POST",
      { message, images, mentions },
    ),
  referenceWorkspace: (id: string) =>
    request<ReferenceWorkspace>(`/refinement/${encodeURIComponent(id)}/reference-workspace`),
  generatePreview: (id: string, expectedContextRevision?: string, expectedTaskRevision?: number) =>
    request<{ jobId: string; status: string }>(`/refinement/${encodeURIComponent(id)}/preview-generations`, "POST", {
      expectedContextRevision, expectedTaskRevision, locale: locale(),
    }),
  investigate: (id: string, input: Record<string, unknown> = {}) =>
    request<{ status: string }>(`/refinement/${encodeURIComponent(id)}/investigations`, "POST", input),
  references: (id: string, query = "") =>
    request<{ items: ExactReferenceBinding[]; orderEpoch?: number }>(`/refinement/${encodeURIComponent(id)}/references${query ? `?query=${encodeURIComponent(query)}` : ""}`),
  openReference: (id: string, binding: ExactReferenceBinding, version?: number) =>
    request<{ reference: ExactReferenceBinding; source: { markdown?: string; body?: string } }>(`/refinement/${encodeURIComponent(id)}/references/open`, "POST", { ...binding, version }),
  referenceUsage: (id: string, binding: ExactReferenceBinding, kind: "used" | "excluded", reason = "Explicit user reference choice") =>
    request(`/refinement/${encodeURIComponent(id)}/references/${encodeURIComponent(binding.documentId)}/usage`, "POST", { ...binding, kind, reason, useScope: "preview" }),
  saveMentionDraft: (id: string, mentions: DocumentMention[], text: string, expectedDraftRevision?: number) =>
    request(`/refinement/${encodeURIComponent(id)}/mention-draft`, "PUT", { mentions, text, expectedDraftRevision }),
  previewVersion: (id: string, version: number) =>
    request<WorkPreviewVersion>(`/refinement/${encodeURIComponent(id)}/preview-versions/${version}`),
  comparePreview: (id: string, left: number, right: number) =>
    request<{ left: WorkPreviewVersion; right: WorkPreviewVersion; comparison: PreviewComparison }>(`/refinement/${encodeURIComponent(id)}/preview-versions/${left}/compare/${right}`),
  editPreview: (id: string, version: number, fields: WorkPreviewFields, expectedCurrentPreviewVersion: number, expectedContentHash: string) =>
    request(`/refinement/${encodeURIComponent(id)}/preview-versions/${version}/edits`, "POST", { version, fields, expectedCurrentPreviewVersion, expectedContentHash }),
  restorePreview: (id: string, version: number, expectedCurrentPreviewVersion: number) =>
    request(`/refinement/${encodeURIComponent(id)}/preview-versions/${version}/restore`, "POST", { version, expectedCurrentPreviewVersion }),
  applyPreview: (id: string, version: number, expectedCurrentPreviewVersion: number, expectedContentHash: string, expectedTaskRevision?: number, editedFieldHashes?: Record<string, string>) =>
    request<{ task: { id: string; taskRevision: number }; transitionCause: "preview_adopted"; version: number }>(`/refinement/${encodeURIComponent(id)}/preview-versions/${version}/apply`, "POST", { version, expectedCurrentPreviewVersion, expectedContentHash, expectedTaskRevision, editedFieldHashes }),
  proposalDecision: (
    id: string,
    proposalId: string,
    draftRevision: number,
    decision: "accept" | "reject",
    editedPayload?: Record<string, unknown>,
    intent?: "split",
  ) =>
    request(
      `/refinement/${encodeURIComponent(id)}/proposal-decisions`,
      "POST",
      { proposalId, draftRevision, decision, editedPayload, ...(intent ? { intent } : {}) },
    ),
  review: (subject: Record<string, unknown>) =>
    request<ConflictReview>("/conflict-reviews", "POST", {
      subject,
      triggerKind: "explicit",
    }),
  reviewStatus: (id: string) =>
    request<ConflictReview>(`/conflict-reviews/${encodeURIComponent(id)}`),
  reviewHistory: (subject: Record<string, unknown>) =>
    request<ConflictReviewHistory>("/conflict-reviews", "GET", { subject }),
  cancelReview: (id: string) =>
    request(`/conflict-reviews/${encodeURIComponent(id)}/cancel`, "POST"),
  knowledge: (id: string) =>
    request<KnowledgeReviewProjection>(`/tasks/${encodeURIComponent(id)}/knowledge`),
  knowledgeDraft: (id: string, expectedTaskRevision: number) =>
    request<{
      id: string;
      status: "queued" | "running" | "completed" | "failed" | "cancelled";
    }>(`/tasks/${encodeURIComponent(id)}/knowledge/drafts`, "POST", {
      expectedTaskRevision,
    }),
  publish: (
    id: string,
    draftRevision: number,
    expectedContentHash: string,
    expectedSourceHash: string,
  ) =>
    request(
      `/tasks/${encodeURIComponent(id)}/knowledge/drafts/${draftRevision}/publish`,
      "POST",
      { expectedContentHash, expectedSourceHash },
    ),
  correctKnowledge: (
    id: string,
    draftRevision: number,
    expectedContentHash: string,
    expectedSourceHash: string,
    bodyMarkdown: string,
  ) =>
    request<{
      draftRevision: number;
      bodyMarkdown: string;
      contentHash: string;
      state: string;
    }>(
      `/tasks/${encodeURIComponent(id)}/knowledge/drafts/${draftRevision}/correction`,
      "POST",
      { expectedContentHash, expectedSourceHash, bodyMarkdown },
    ),
  restoreKnowledge: (
    id: string,
    revision: number,
    expectedCurrentPrivateRevision: number,
  ) => request<{
    taskId: string;
    draftRevision: number;
    derivedFromRevision: number;
    contentHash: string;
    bodyMarkdown: string;
    state: string;
  }>(
    `/tasks/${encodeURIComponent(id)}/knowledge/versions/${revision}/restore`,
    "POST",
    { expectedCurrentPrivateRevision },
  ),
  regenerateKnowledge: (
    id: string,
    draftRevision: number,
    expectedTaskRevision: number,
  ) =>
    request(
      `/tasks/${encodeURIComponent(id)}/knowledge/drafts/${draftRevision}/regenerate`,
      "POST",
      { expectedTaskRevision },
    ),
  withdrawKnowledge: (
    id: string,
    draftRevision: number,
    expectedContentHash: string,
    expectedSourceHash: string,
  ) =>
    request(
      `/tasks/${encodeURIComponent(id)}/knowledge/drafts/${draftRevision}/withdraw`,
      "POST",
      { expectedContentHash, expectedSourceHash },
    ),
  lineage: (id: string) =>
    request<import("../types/taskWorkbench").LineageSnapshot>(
      `/tasks/${encodeURIComponent(id)}/lineage`,
    ),
  retryJob: (id: string) => request(`/jobs/${encodeURIComponent(id)}/retry`, "POST"),
  repairDistillation: (id: string, reason = "explicit repair", runId?: string) => request(`/tasks/${encodeURIComponent(id)}/distillation-repair`, "POST", { reason, runId }),
};
