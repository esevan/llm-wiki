import type { ApplicationResponse, HttpMethod } from "../types/application";
import type {
  ConflictReview,
  ConflictReviewHistory,
  RefinementProposal,
  RefinementSession,
  TaskAggregate,
  WorkbenchSnapshot,
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
  createCapture: (text: string) => request("/captures", "POST", { text }),
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
    request<TaskAggregate>(
      `/tasks/${encodeURIComponent(id)}/work-log`,
      "POST",
      { expectedTaskRevision, body, attachment },
    ),
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
  saveWorkspace: (id: string, data: Record<string, unknown>) =>
    request(`/refinement/${encodeURIComponent(id)}/workspace`, "PUT", data),
  proposals: (id: string) =>
    request<RefinementProposal[]>(
      `/refinement/${encodeURIComponent(id)}/proposals`,
    ),
  message: (id: string, message: string) =>
    request<RefinementSession>(
      `/refinement/${encodeURIComponent(id)}/messages`,
      "POST",
      { message },
    ),
  proposalDecision: (
    id: string,
    proposalId: string,
    draftRevision: number,
    decision: "accept" | "reject",
    editedPayload?: Record<string, unknown>,
  ) =>
    request(
      `/refinement/${encodeURIComponent(id)}/proposal-decisions`,
      "POST",
      { proposalId, draftRevision, decision, editedPayload },
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
  knowledgeDraft: (id: string, expectedTaskRevision: number) =>
    request<{
      draftRevision: number;
      bodyMarkdown: string;
      contentHash: string;
      sourceHash?: string;
      state: string;
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
};
