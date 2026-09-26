export interface InputImage { name: string; mediaType: string; data: string }

export interface ExactReferenceBinding {
  documentId: string; documentVersion: string; section?: string;
  chunkIndex?: number; title: string; status?: string; informationType?: string;
  path?: string; excerpt?: string; aspect?: string; role?: string; claimIds?: string[];
}
export interface DocumentMention extends ExactReferenceBinding {
  mentionId: string; displayLabel: string; start?: number; end?: number;
}
export interface WorkPreviewFields {
  description: string; background: string; goal: string; scope: string;
  nonGoals: string; constraints: string[]; completionCriteria: string[];
  initialApproach: string[];
  assumptions: DraftAssumption[];
}
export interface DraftAssumption {
  id: string; text: string; status?: "open" | "confirmed" | "rejected" | "superseded";
  basis: "user_context" | "source" | "model_inference"; sourceClaimIds: string[];
}
export interface WorkPreviewVersion {
  previewId: string; version: number; current?: boolean;
  taskRevision?: number; contextRevision?: number;
  derivationKind: "generated" | "edited" | "restored" | "reconciled";
  derivedFromVersion?: number; locale?: "en" | "ko"; fields: WorkPreviewFields;
  assumptions?: DraftAssumption[]; references: ExactReferenceBinding[]; createdAt?: string; contentHash: string;
}
export type WorkPreviewSummary = Pick<WorkPreviewVersion, "version" | "derivationKind" | "createdAt" | "derivedFromVersion" | "current"> & { contentHash?: string };
export interface PreviewComparison {
  left: WorkPreviewVersion; right: WorkPreviewVersion;
  fields: Array<{ key: keyof WorkPreviewFields; state: "added" | "removed" | "changed" | "unchanged" }>;
}
export interface ReferenceViewState {
  query: string; filters: string[]; sort: string; orderEpoch: number;
  orderedReferenceIds: string[]; selectedReferenceId?: string;
  viewerHistory: ExactReferenceBinding[]; historyIndex: number;
}

export interface ReferenceUsageFact {
  documentId: string; documentVersion: string; section?: string;
  kind: "viewed" | "mentioned" | "used" | "adopted" | "excluded";
  contextRevision: number;
}
export interface ReferenceWorkspace {
  interactions?: ReferenceUsageFact[];
  generation?: { id?: string; status?: string; executionOutcome?: string; applicationDisposition?: string; safeError?: string };
  retrievalOutcome?: string;
  stageTimings?: Record<string, number>;
  preview?: { id: string; currentVersion: number; contextRevision: number; current?: WorkPreviewVersion; versions: WorkPreviewSummary[] } | null;
  references?: ExactReferenceBinding[];
  mentionDraft?: { text: string; mentions: DocumentMention[] };
  investigations?: Array<{ id: string; state: string; contextRevision: number; safeError?: string }>;
  findings?: Array<{ id: string; priority: "critical" | "supporting" | "ancillary"; summary: string; references: ExactReferenceBinding[] }>;
}

export interface CaptureDistillationSummary {
  captureId: string;
  sourceRevision: string;
  sourceNumber: number;
  currentRevision: number;
  contextRevision: number;
  eligible: boolean;
  source: { text: string; images: Array<{ index: number; name: string; mediaType: string; contentHash: string }> };
  display: {
    title: string; content: string; context: string; explicitRequests: string[];
    locale: "en" | "ko"; revision: number; placeholder: boolean; jobId?: string;
  };
  distillation?: {
    jobId: string; logicalOperationId: string; status: string;
    executionOutcome: string; applicationDisposition: string;
    safeError?: { code: string; message: string } | null;
    retryAllowed: boolean; attempt: number;
    prompt?: { id: string; version: number }; sourceRevision: string;
  } | null;
  proposal?: {
    id: string; title: string; content: string; context: string;
    explicitRequests: string[]; state: string; jobId: string;
  } | null;
}

export type TaskState = "task" | "in_progress" | "completed";
export type WorkbenchItem = CaptureCard | TaskCard | LegacyRefinementCard;

export interface CaptureCard {
  captureDistillation?: CaptureDistillationSummary;
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
  distillation?: DistillationProjection;
  originalAvailable?: boolean;
}
export type DistillationFreshness = "current" | "pending" | "stale" | "retryable_failure" | "repair_required" | "unavailable";
export type DistillationSource = { type: string; id: string; revision: string; locator: string; quote?: string };
export type DistillationClaim = { id: string; kind: string; statement: string; actor: "user" | "ai" | "tool" | "system" | "unknown"; epistemicState: "suggested" | "decided" | "attempted" | "performed" | "observed" | "verified" | "unresolved"; status: "current" | "historical" | "contradicted" | "unknown"; topicKey?: string; sources: DistillationSource[]; contradictedBy?: DistillationSource[] };
export type DistillationProjection = { projectionRevision: number; sourceSetHash?: string; freshness: DistillationFreshness; jobId?: string; prompt?: { id: string; version: number }; rulesVersion?: string; resultSchemaVersion?: string; locale?: string; createdAt?: string; result: { claims: DistillationClaim[]; workLogView: { sections: Array<{ kind: string; claimIds: string[] }> }; warnings: string[] } };
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
  captureDistillation?: CaptureDistillationSummary;
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
  distillation?: (DistillationProjection & {
    nodes: Array<{ id: string; revision: number; kind: string; topicKey?: string; status: string; claimIds: string[]; detail: { before?: string; after?: string; reason?: string; evidenceClaimIds?: string[]; result?: string; currentStatus: string; links?: DistillationSource[] }; active: boolean }>;
    relationships: Array<{ id: string; kind: string; from: string; to: string; reason?: string; sources: DistillationSource[]; active: boolean }>;
    topicStates: Array<{ topicKey: string; status: string; currentNodeId?: string; replacementNodeId?: string; sources: DistillationSource[] }>;
    completionSnapshots: Array<{ id: string; status: string; claimIds: string[] }>;
  }) | null;
}
export interface KnowledgeSourceRef {
  type: string;
  id: string;
  revision: string;
  locator: string;
  quote: string;
}
export interface KnowledgeClaimBinding {
  claimId: string;
  statement: string;
  epistemicState: string;
  sourceRefs: KnowledgeSourceRef[];
}
export interface KnowledgeApplicability {
  summary: string;
  representativeQuestions: string[];
  helpsWith: string[];
  conditions: string[];
  exclusions: string[];
  sourceRefs?: KnowledgeSourceRef[];
}
export interface KnowledgeVersion {
  revision: number;
  parentRevision?: number;
  derivedFromRevision?: number;
  derivationKind: "generated" | "edited" | "restored";
  articleType: string;
  title: string;
  bodyMarkdown: string;
  contentHash: string;
  generationSnapshotHash?: string;
  freshness: "current" | "stale";
  storedFreshness?: string;
  qualityState: string;
  modelStatus: string;
  modelError?: string;
  createdAt: string;
  selected?: boolean;
  applicability: KnowledgeApplicability;
  result: {
    article?: {
      type: string;
      title: string;
      finalOutcomes: Array<{ topicKey: string; statement: string; claimIds: string[] }>;
      bodyMarkdown: string;
      applicability: KnowledgeApplicability;
      claimBindings: KnowledgeClaimBinding[];
      assumptionBindings: Array<{ assumptionId: string; bodyLocator: string; sourceRefs: KnowledgeSourceRef[] }>;
    };
    ideas?: KnowledgeIdea[];
    qualityFindings?: KnowledgeQualityFinding[];
  };
}
export interface KnowledgeIdea {
  id: string;
  revision: number;
  knowledgeRevision: number;
  title: string;
  bodyMarkdown: string;
  disposition: "unverified" | "deferred" | "rejected" | "out_of_scope";
  reconsiderationConditions: string[];
  sourceRefs: KnowledgeSourceRef[];
  relatedTopicKeys: string[];
  contentHash: string;
  publicationState: string;
}
export interface KnowledgeQualityFinding {
  kind: string;
  severity: string;
  locator: string;
  message: string;
  sourceRefs: KnowledgeSourceRef[];
}
export interface KnowledgeReviewProjection {
  taskId: string;
  pointers: {
    currentPrivateRevision?: number;
    publishedRevision?: number;
    publicationDocumentId?: string;
    publicationPath?: string;
    publicationContentHash?: string;
  };
  versions: KnowledgeVersion[];
  ideas: KnowledgeIdea[];
  archive?: {
    proposals: KnowledgeArchiveProposal[];
    operations: Array<{
      operationId: string;
      state: "writing" | "index_pending" | "index_failed" | "complete" | "conflict" | "repair_required" | "compensated";
      error?: string;
      proposalId?: string;
      proposalVersion?: number;
    }>;
  };
}
export interface KnowledgeArchiveArtifact {
  documentId: string;
  path: string | null;
  bytes: string | null;
  sha256: string | null;
  expectedHash: string | null;
  kind: string;
}
export interface KnowledgeArchiveMocPatch { path: string; before: string; after: string }
export interface KnowledgeArchiveReferenceLink {
  documentId: string;
  documentVersion: string;
  section?: string;
  path: string;
  role: string;
  rationale: string;
}
export interface KnowledgeArchiveProposal {
  operationId?: string;
  proposalId: string;
  proposalVersion: number;
  proposalHash: string;
  state: string;
  outcome: string;
  rationale: string;
  artifacts: KnowledgeArchiveArtifact[];
  mocPatches: KnowledgeArchiveMocPatch[];
  referenceLinks: KnowledgeArchiveReferenceLink[];
  input?: unknown;
  target?: { documentId: string; path: string | null };
  unresolvedConflicts?: Array<{ path: string; reason: string }>;
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
  liveStatus?: { kind: string; status: "running" | "completed"; itemId?: string; command?: string; text?: string; output?: string };
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
export type CodexThreadSummary = {
  id: string; title: string; preview: string; cwd?: string; model?: string; source: string; status: string;
  createdAt: string; updatedAt: string; linkedTaskId?: string; linkedSessionId?: string;
};
export type CodexThreadActivity = {
  id: string; kind: string; label: string; status: string; command?: string; output?: string; exitCode?: number; order?: number; createdAt?: string;
};
export type CodexThreadMessage = { id: string; role: "user" | "assistant"; body: string; order?: number; createdAt?: string };
export type CodexThreadItem = ({ type: "message" } & CodexThreadMessage) | ({ type: "activity" } & CodexThreadActivity);
export type CodexThreadTurn = {
  id: string; status: string; createdAt?: string; completedAt?: string;
  messages: CodexThreadMessage[];
  activity: CodexThreadActivity[];
  items?: CodexThreadItem[];
};
export type CodexThreadTranscript = { thread: CodexThreadSummary; turns: CodexThreadTurn[]; nextCursor?: string | null };
export type CodexThreadList = { threads: CodexThreadSummary[]; nextCursor?: string | null };
export type CodexThreadLinkResult = CodexThreadTranscript & { taskId: string; sessionId: string; threadId: string; linked: boolean };


export type TaskApplicationEvent = {
  taskId: string;
  revision: number;
  cause: "preview_adopted";
};
