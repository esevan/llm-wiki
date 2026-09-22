export interface InputImage { name: string; mediaType: string; data: string }

export type TaskState = "task" | "in_progress" | "completed";
export type WorkbenchItem = CaptureCard | TaskCard | LegacyRefinementCard;

export interface CaptureCard {
  kind: "capture";
  hasImage?: boolean;
  id: string;
  text: string;
  category?: string;
  lastUserActivityAt?: string;
}
export type TaskContentVersions = Partial<Record<"ko" | "en", Partial<Record<"title" | "detail" | "outcome" | "scope" | "nonGoals" | "validationCriteria", string>>>>;

export interface TaskCard {
  contentVersions?: TaskContentVersions;
  completedAt?: string | null;
  originCaptureText?: string | null;
  kind: "task";
  id: string;
  taskRevision: number;
  refinedRevision?: number;
  parentTaskId?: string;
  state: TaskState;
  title: string;
  detail?: string;
  category?: string;
  lastUserActivityAt?: string;
  readiness?: ReadinessSummary;
}
export interface LegacyRefinementCard {
  kind: "refinement";
  id: string;
  problemId: string;
  problemRevision: number;
  title: string;
  category?: string;
  sourceKind: "legacy_problem";
  lastUserActivityAt?: string;
}
export interface ReadinessSummary {
  resolved: number;
  missing: number;
  notApplicable: number;
}
export interface WorkbenchSnapshot {
  revision: number;
  activeShortcuts: Array<{ kind: "task"; id: string; taskRevision: number }>;
  refiningShortcuts: Array<{
    kind: "capture" | "task";
    id: string;
    draftRevision: number;
  }>;
  categories: Array<{ id: string; label: string; items: WorkbenchItem[] }>;
}
export interface WorkLogEntry {
  bodyVersions?: Partial<Record<"ko" | "en", { body?: string }>>;
  translationJob?: { id: string; status: string; error?: string };
  imageSummary?: string;
  imageSummaryVersions?: Partial<Record<"ko" | "en", { image_summary: string }>>;
  imageSummaryJob?: { id: string; status: string; error?: string };
  id: string;
  body?: string;
  createdAt?: string;
  attachment?: {
    name?: string;
    mediaType?: string;
    data?: string;
    url?: string;
  };
  comments?: Array<{ id: string; body: string; createdAt?: string }>;
  execution?: TaskExecutionWorkLogLink;
}
export interface ChecklistItem {
  id: string;
  body: string;
  checked: boolean;
}
export interface ReadinessEntry {
  key: string;
  status: "resolved" | "missing" | "not_applicable";
  reason?: string;
  evidenceRefs?: string[];
  applicableReason?: string;
  provenance?: string;
}
export interface TaskAggregate extends TaskCard {
  originCapture?: { id: string; text: string; createdAt: string } | null;
  autoPublicationError?: string;
  hierarchy?: { parent?: TaskAggregate; siblings: TaskAggregate[]; children: TaskAggregate[] };
  detail?: string;
  outcome?: string;
  scope?: string;
  nonGoals?: string;
  validationCriteria?: string;
  readinessEntries?: ReadinessEntry[];
  problemLinks?: Array<{
    id: string;
    problemId: string;
    problemRevision: number;
    relationship?: string;
    note?: string;
  }>;
  relationships?: Array<{
    id: string;
    targetTaskId: string;
    kind: "prerequisite" | "split_from" | "related";
    note?: string;
  }>;
  workLog?: WorkLogEntry[];
  checklist?: ChecklistItem[];
  decisions?: Array<{
    id: string;
    body?: string;
    kind?: string;
    createdAt?: string;
  }>;
  reviews?: ConflictReview[];
  completion?: {
    id: string;
    evidence?: string;
    report?: string;
    createdAt?: string;
  };
  publishedKnowledge?: { draftRevision: number; bodyMarkdown: string };
  publication?: {
    state?: string;
    draftRevision?: number;
    contentHash?: string;
    sourceHash?: string;
    bodyMarkdown?: string;
    lineage?: { journey?: { sourceHash?: string; journey?: import("../features/workbench/TaskJourneyGraph").TaskJourney; modelStatus?: string; modelError?: string } };
  };
}
export interface RefinementSession {
  state?: "active" | "completed";
  id: string;
  taskId?: string;
  captureId?: string;
  inputDraft?: string;
  activeTab?: string;
  scrollAnchor?: string;
  draftRevision?: number;
  captureImage?: InputImage;
  captureImages?: InputImage[];
  messages?: Array<{ id: string; role: string; body: string; image?: InputImage; images?: InputImage[] }>;
  previewJobId?: string;
  previewStatus?: "queued" | "running" | "retryable" | "completed" | "failed" | "cancelled" | "stale";
  responseStatus?: "queued" | "running" | "completed" | "failed" | "cancelled";
}
export interface RefinementProposal {
  localizedFields?: TaskContentVersions;
  id: string;
  type: string;
  payload: Record<string, unknown>;
  draftRevision: number;
}
export interface ConflictReview {
  id: string;
  status:
    | "queued"
    | "running"
    | "clear"
    | "findings"
    | "insufficient_evidence"
    | "failed"
    | "cancelled"
    | "stale";
  current?: boolean;
  safeError?: string;
  findings?: Array<{
    id: string;
    sourceId?: string;
    path?: string;
    excerpt?: string;
  }>;
}
export interface ConflictReviewHistory {
  attempts: ConflictReview[];
  currentResult?: ConflictReview | null;
}
export interface LineageSnapshot {
  sourceHash?: string;
  journeySourceHash?: string;
  nodes?: Array<{ id: string; kind: string; title?: string }>;
  edges?: Array<{ from: string; to: string; kind: string }>;
  journey?: import("../features/workbench/TaskJourneyGraph").TaskJourney;
  recordedJourney?: import("../features/workbench/TaskJourneyGraph").TaskJourney;
  journeyStatus?: { jobId?: string; status?: string; error?: string } | null;
  modelStatus?: string;
  modelError?: string;
}
export type TaskWorkSessionAttachment = { name: string; mediaType: string; data: string };
export type TaskWorkSessionEntry = {
  id: string; author: "user" | "assistant" | "system";
  kind: "note" | "ai_output" | "execution_result"; body: string;
  attachment?: TaskWorkSessionAttachment; createdAt: string;
};
export type TaskExecutionStatus = "queued" | "running" | "awaiting_response" | "succeeded" | "failed" | "cancelled" | "interrupted" | "needs_attention";
export type FormalRequestStatus = "pending" | "submitting" | "answered" | "stale" | "error";
export type TaskExecutionEvidence = { id: string; kind: string; status: string; label: string; summary: string; command?: string; paths?: string[]; exitCode?: number; createdAt?: string };
export type TaskExecutionFormalRequest = {
  id: string; kind: "command_approval" | "file_change_approval" | "permissions_approval" | "user_input";
  status: FormalRequestStatus; isBlocking: boolean; title: string; prompt: string;
  choices: Array<{ value: string; label: string; description?: string }>;
  questions: Array<{ id: string; header: string; prompt: string; options: Array<{ value: string; label: string; description?: string }>; allowOther: boolean; isSecret: boolean }>;
  proposedResponse?: { decision?: string; answers?: Record<string, { answers: string[] }> };
  response?: { decision?: string; answers?: Record<string, { answers: string[] }> };
  error?: string;
};
export type TaskExecutionEffectiveConfig = {
  model: string; cwd: string; approvalPolicy: string; approvalsReviewer: "user" | "auto_review"; sandbox: string;
  provenance: "preflight" | "bound_thread"; settingsRevision: string; ready: boolean;
  capabilities: { structuredUserInput: boolean }; readinessError?: { code: string; message: string };
};
export type TaskExecutionRun = {
  id: string; taskId: string; sessionId: string; instruction: string; status: TaskExecutionStatus; stopRequested: boolean;
  provider: "codex"; model: string; workspacePath: string; threadId?: string; turnId?: string; startedAt?: string; finishedAt?: string;
  userEntryId?: string;
  finalReport?: string; error?: { code: string; message: string }; evidence: TaskExecutionEvidence[];
  liveStatus?: { kind: string; status: "running" | "completed" };
  formalRequests: TaskExecutionFormalRequest[]; workLogEntryId?: string; workLogSyncState: "pending" | "synced" | "failed";
  workLogSyncError?: string; retryOfRunId?: string; revision: number;
};
export type TaskExecutionSnapshot = { revision?: number; runs: TaskExecutionRun[]; activeRunId: string | null; selectedRun: TaskExecutionRun | null; effectiveConfig?: TaskExecutionEffectiveConfig };
export type TaskExecutionWorkLogLink = { runId: string; sessionId: string; status: TaskExecutionStatus; provider: "codex"; model: string; reportExcerpt?: string; evidence: TaskExecutionEvidence[]; artifacts: string[]; limitations: string[]; syncState: "pending" | "synced" | "failed" };
export type TaskWorkSession = {
  id: string; taskId: string; title: string; provider: "codex"; model: string;
  approvalMode: "ask" | "auto"; approvalsReviewer?: "user" | "auto_review"; workspacePath: string; createdAt: string; updatedAt: string;
};
export type TaskWorkSessionRecord = { session: TaskWorkSession; entries: TaskWorkSessionEntry[] };
