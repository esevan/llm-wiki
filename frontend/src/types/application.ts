export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

export interface ApplicationRequest {
  path: string;
  method?: HttpMethod;
  headers?: Record<string, string>;
  body?: string;
  signal?: AbortSignal;
  cache?: RequestCache;
}

export interface ApplicationResponse {
  ok: boolean;
  status: number;
  json<T>(): Promise<T>;
  text(): Promise<string>;
  body: ReadableStream<Uint8Array> | null;
}

export interface ApplicationClient {
  request(request: ApplicationRequest): Promise<ApplicationResponse>;
}

export type WorkTrackingStage = 'capture' | 'task' | 'problem' | 'solution' | 'checkpoint' | 'conflict' | 'completion' | 'knowledge';
export type WorkTrackingProjection = 'pending_review' | 'queued' | 'claimed' | 'waiting_solution' | 'applied' | 'ignored_late' | 'conflict' | 'failed';

export interface WorkTrackingCardModel {
  id: string;
  stage: WorkTrackingStage;
  title: string;
  summary: string;
  revision: number;
  projectionStatus?: WorkTrackingProjection;
  publicationState?: 'not_requested' | 'offered' | 'deferred' | 'draft_review' | 'draft_saved' | 'published';
  draftMarkdown?: string;
  editable?: boolean;
}

export interface WorkTrackingSession {
  sessionId: string;
  headEventId: string;
  headRevision: number;
  state: 'active' | 'completion_proposed' | 'completed';
  publicationState: string;
  capture?: { id: string; summary: string } | null;
  linkedWorkflow?: { task?: { id: string; taskId?: string; taskRevision?: number; title: string; state: string; sourceEventId: string }; problem?: { id: string; title: string; state: string; sourceEventId: string } };
  taskCompletion?: { id: string; taskRevision: number; evidence: string; report: string } | null;
  recentEvents: Array<Record<string, unknown>>;
  nextActions: string[];
}

declare global {
  interface Window {
    llmWikiApplication: ApplicationClient;
    llmWikiFormatSystemTime: (
      value: string,
      locale: string,
      options?: Intl.DateTimeFormatOptions,
    ) => string;
    workbenchBoard?: {
      problems?: Array<{ id: string; state: string }>;
      features?: Array<{ id: string; state: string }>;
    };
    loadBoard: () => Promise<void>;
    openChat?: (
      type: "captures" | "problems" | "features" | "tasks",
      id: string,
      context?: { problemRevision?: number; sourceTitle?: string; workspaceDock?: boolean },
    ) => void;
    __TAURI_INTERNALS__?: object;
  }
}
