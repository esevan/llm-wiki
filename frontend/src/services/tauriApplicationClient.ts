import { Channel, invoke } from '@tauri-apps/api/core';
import type { ApplicationClient, ApplicationRequest, ApplicationResponse } from '../types/application';

interface CommandResponse {
  status: number;
  contentType: string;
  body: string;
}

export interface DesktopE2eResult {
  status: 'passed' | 'failed' | 'relaunch' | 'progress';
  steps: string[];
  error: string | null;
  capture?: string | null;
}

export interface DesktopE2eState {
  providerUrl: string;
  restoreCapture: string | null;
  restoreSteps: string[];
}

export const desktopE2eMode = () => invoke<DesktopE2eState | null>('desktop_e2e_mode');
export const reportDesktopE2eProgress = (steps: string[]) =>
  completeDesktopE2e({ status: 'progress', steps, error: null });
export const completeDesktopE2e = (result: DesktopE2eResult) => invoke('desktop_e2e_complete', { result });

type StreamEvent =
  | { kind: 'chunk'; data: number[] }
  | { kind: 'complete' }
  | { kind: 'cancelled' }
  | { kind: 'error'; message: string };

let nextRequestId = 0;

const requestId = () => `ui-${Date.now()}-${++nextRequestId}`;

const responseFromCommand = (value: CommandResponse): ApplicationResponse => {
  const bytes = new TextEncoder().encode(value.body);
  return {
    ok: value.status >= 200 && value.status < 300,
    status: value.status,
    json: async <T>() => JSON.parse(value.body) as T,
    text: async () => value.body,
    body: new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(bytes);
        controller.close();
      },
    }),
  };
};

const needsStream = (request: ApplicationRequest) =>
  Boolean(request.signal) || /\/(?:chat|next-chat|completed-chat|events)(?:\?|$)/.test(request.path);

const responseFromStream = (value: CommandResponse, body: ReadableStream<Uint8Array>): ApplicationResponse => {
  const readText = async () => {
    const reader = body.getReader();
    const decoder = new TextDecoder();
    let text = '';
    for (;;) {
      const next = await reader.read();
      if (next.done) return text + decoder.decode();
      text += decoder.decode(next.value, { stream: true });
    }
  };
  return {
    ok: value.status >= 200 && value.status < 300,
    status: value.status,
    json: async <T>() => JSON.parse(await readText()) as T,
    text: readText,
    body,
  };
};

export class TauriApplicationClient implements ApplicationClient {
  async request(request: ApplicationRequest): Promise<ApplicationResponse> {
    if (request.signal?.aborted) throw new DOMException('The operation was aborted', 'AbortError');
    const commandRequest = {
      path: request.path,
      method: request.method ?? 'GET',
      headers: request.headers ?? {},
      body: request.body ?? null,
    };
    if (!needsStream(request)) {
      return responseFromCommand(await invoke<CommandResponse>('application_request', { request: commandRequest }));
    }
    const id = requestId();
    let streamController: ReadableStreamDefaultController<Uint8Array> | undefined;
    let finished = false;
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        streamController = controller;
      },
      cancel() {
        if (!finished) void invoke('cancel_application_request', { requestId: id });
      },
    });
    const channel = new Channel<StreamEvent>();
    const cleanup = () => request.signal?.removeEventListener('abort', abort);
    channel.onmessage = (event) => {
      if (finished) return;
      if (event.kind === 'chunk') streamController?.enqueue(Uint8Array.from(event.data));
      if (event.kind === 'complete') {
        finished = true;
        cleanup();
        streamController?.close();
      }
      if (event.kind === 'cancelled') {
        finished = true;
        cleanup();
        streamController?.error(new DOMException('The operation was aborted', 'AbortError'));
      }
      if (event.kind === 'error') {
        finished = true;
        cleanup();
        streamController?.error(new Error(event.message));
      }
    };
    const abort = () => {
      if (!finished) void invoke('cancel_application_request', { requestId: id });
    };
    request.signal?.addEventListener('abort', abort, { once: true });
    try {
      const response = await invoke<CommandResponse>('application_stream', {
        requestId: id,
        request: commandRequest,
        onEvent: channel,
      });
      return responseFromStream(response, body);
    } catch (error) {
      finished = true;
      cleanup();
      streamController?.error(error);
      throw error;
    }
  }
}
