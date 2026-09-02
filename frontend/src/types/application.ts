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

declare global {
  interface Window {
    llmWikiApplication: ApplicationClient;
    __TAURI_INTERNALS__?: object;
  }
}
