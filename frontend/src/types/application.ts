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

export type WorkTrackingStage = 'capture' | 'problem' | 'solution' | 'checkpoint' | 'conflict' | 'completion' | 'knowledge';
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
    __TAURI_INTERNALS__?: object;
  }
}
