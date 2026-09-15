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
export interface TaskCard {
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
  publication?: {
    state?: string;
    draftRevision?: number;
    contentHash?: string;
    sourceHash?: string;
    bodyMarkdown?: string;
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
  messages?: Array<{ id: string; role: string; body: string; image?: InputImage }>;
  previewJobId?: string;
  previewStatus?: "queued" | "running" | "retryable" | "completed" | "failed" | "cancelled" | "stale";
  responseStatus?: "queued" | "running" | "completed" | "failed" | "cancelled";
}
export interface RefinementProposal {
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
  nodes?: Array<{ id: string; kind: string; title?: string }>;
  edges?: Array<{ from: string; to: string; kind: string }>;
}
