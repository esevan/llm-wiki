import { Channel, invoke } from '@tauri-apps/api/core';
import type { ApplicationClient, ApplicationRequest, ApplicationResponse } from '../types/application';

interface NativeResponse {
  status: number;
  body: unknown;
}

interface NativeOperation {
  name: string;
  input: Record<string, unknown>;
}

type NativeCommand = 'system_command' | 'vault_command' | 'settings_command' | 'workflow_command' | 'jobs_command' | 'enqueue_ai_job';

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

const parseBody = (body?: string | null): Record<string, unknown> => {
  if (!body) return {};
  const value: unknown = JSON.parse(body);
  if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new Error('Expected an object body');
  return value as Record<string, unknown>;
};

const match = (path: string, pattern: RegExp) => pattern.exec(path)?.slice(1).map(decodeURIComponent);

const operationFor = (request: ApplicationRequest): NativeOperation => {
  const method = request.method ?? 'GET';
  const url = new URL(request.path, 'native://application');
  const path = url.pathname;
  const body = parseBody(request.body);
  const withIds = (name: string, ids: Record<string, string>): NativeOperation => ({ name, input: { ...body, ...ids } });
  let ids: string[] | undefined;

  if (method === 'GET' && path === '/health') return { name: 'health.get', input: {} };
  if (method === 'POST' && path === '/index') return { name: 'vault.index', input: {} };
  if (method === 'GET' && path === '/search') return { name: 'vault.search', input: { query: url.searchParams.get('q') ?? '', limit: Number(url.searchParams.get('limit') ?? 20), offset: Number(url.searchParams.get('offset') ?? 0), semantic: url.searchParams.get('semantic') === 'true' } };
  if (method === 'GET' && path === '/settings/locale') return { name: 'locale.get', input: { browserLocale: url.searchParams.get('browser_locale') ?? 'en' } };
  if (method === 'PUT' && path === '/settings/locale') return { name: 'locale.save', input: body };
  ids = match(path, /^\/i18n\/([^/]+)$/);
  if (ids && method === 'GET') return withIds('i18n.get', { locale: ids[0] });
  if (method === 'GET' && path === '/provider/config') return { name: 'provider.get', input: {} };
  if (method === 'PUT' && path === '/provider/config') return { name: 'provider.save', input: body };
  if (method === 'POST' && path === '/captures') return { name: 'capture.create', input: body };
  if (method === 'GET' && path === '/board') return { name: 'board.get', input: {} };
  if (method === 'GET' && path === '/dashboard') return { name: 'compass.dashboard', input: {} };
  if (method === 'POST' && path === '/goals') return { name: 'compass.goal.create', input: body };
  if (method === 'GET' && path === '/events') return { name: 'events.list', input: {} };
  if (method === 'GET' && path === '/transitions') return { name: 'transitions.list', input: {} };
  if (method === 'GET' && path === '/jobs') return { name: 'jobs.list', input: {} };
  if (method === 'GET' && path === '/jobs/events') return { name: 'jobs.events', input: {} };
  if (method === 'GET' && path === '/notifications') return { name: 'notifications.list', input: { unreadOnly: url.searchParams.get('unread_only') === 'true' } };
  if (method === 'GET' && path === '/workbench/recent-archive') return { name: 'workbench.recent', input: { limit: Number(url.searchParams.get('limit') ?? 5) } };
  if (method === 'GET' && path === '/workbench/completed-solutions') return { name: 'workbench.completed', input: { limit: Number(url.searchParams.get('limit') ?? 20) } };
  if (method === 'PUT' && path === '/workbench/category') return { name: 'workbench.category.save', input: body };
  if (method === 'PUT' && path === '/workbench/importance') return { name: 'workbench.importance.save', input: body };

  ids = match(path, /^\/transitions\/([^/]+)\/([^/]+)$/);
  if (ids && method === 'GET') return withIds('transitions.entity', { entityType: ids[0], entityId: ids[1] });
  if (ids && method === 'POST') return withIds('transitions.apply', { entityType: ids[0], entityId: ids[1] });

  ids = match(path, /^\/captures\/([^/]+)\/promote$/);
  if (ids && method === 'POST') return withIds('capture.promote', { captureId: ids[0] });
  ids = match(path, /^\/problems\/([^/]+)\/approve$/);
  if (ids && method === 'POST') return withIds('problem.approve', { problemId: ids[0] });
  ids = match(path, /^\/problems\/([^/]+)\/features$/);
  if (ids && method === 'POST') return withIds('solution.create', { problemId: ids[0] });
  ids = match(path, /^\/features\/([^/]+)\/conflict$/);
  if (ids && method === 'PUT') return withIds('solution.conflict.save', { solutionId: ids[0] });
  ids = match(path, /^\/features\/([^/]+)\/approve$/);
  if (ids && method === 'POST') return withIds('solution.approve', { solutionId: ids[0] });
  ids = match(path, /^\/features\/([^/]+)\/stage$/);
  if (ids && method === 'PUT') return withIds('solution.stage.save', { solutionId: ids[0] });
  ids = match(path, /^\/features\/([^/]+)\/progress$/);
  if (ids && method === 'GET') return withIds('solution.progress.get', { solutionId: ids[0] });
  if (ids && method === 'POST') return withIds('solution.progress.add', { solutionId: ids[0] });
  ids = match(path, /^\/progress\/([^/]+)\/comments$/);
  if (ids && method === 'POST') return withIds('solution.comment.add', { entryId: ids[0] });
  ids = match(path, /^\/features\/([^/]+)\/checklist$/);
  if (ids && method === 'POST') return withIds('solution.checklist.add', { solutionId: ids[0] });
  ids = match(path, /^\/features\/([^/]+)\/follow-up-problem$/);
  if (ids && method === 'POST') return withIds('solution.follow_up', { solutionId: ids[0] });
  ids = match(path, /^\/problems\/([^/]+)\/complete$/);
  if (ids && method === 'POST') return withIds('problem.complete', { problemId: ids[0] });
  ids = match(path, /^\/features\/([^/]+)\/lineage$/);
  if (ids && method === 'GET') return withIds('solution.lineage', { solutionId: ids[0] });
  ids = match(path, /^\/features\/([^/]+)\/handoff$/);
  if (ids && method === 'GET') return withIds('solution.handoff', { solutionId: ids[0] });
  ids = match(path, /^\/checklist\/([^/]+)$/);
  if (ids && method === 'PUT') return withIds('solution.checklist.update', { itemId: ids[0] });
  ids = match(path, /^\/items\/([^/]+)\/([^/]+)$/);
  if (ids && method === 'GET') return withIds('item.get', { entityType: ids[0], entityId: ids[1] });
  if (ids && method === 'PUT') return withIds('item.update', { entityType: ids[0], entityId: ids[1] });
  if (ids && method === 'DELETE') return withIds('item.delete', { entityType: ids[0], entityId: ids[1] });
  ids = match(path, /^\/items\/([^/]+)\/([^/]+)\/restore$/);
  if (ids && method === 'POST') return withIds('item.restore', { entityType: ids[0], entityId: ids[1] });
  ids = match(path, /^\/(captures|problems|features)\/([^/]+)\/refinement-context$/);
  if (ids && method === 'GET') return withIds('refinement.context', { entityType: ids[0], entityId: ids[1] });
  ids = match(path, /^\/(captures|problems|features)\/([^/]+)\/(chat|next-chat|completed-chat)$/);
  if (ids && method === 'POST') return { name: 'conversation.stream', input: { entityType: ids[0], entityId: ids[1], message: body.message } };
  ids = match(path, /^\/(captures|problems|features)\/([^/]+)\/(draft|refine)$/);
  if (ids && method === 'POST') return { name: 'jobs.enqueue', input: { taskKind: ids[2] === 'draft' ? 'workflow_draft' : 'workflow_refinement', entityType: ids[0], entityId: ids[1] } };
  ids = match(path, /^\/features\/([^/]+)\/(conflict-review|completion-review)$/);
  if (ids && method === 'POST') return { name: 'jobs.enqueue', input: { taskKind: ids[1] === 'conflict-review' ? 'conflict_review' : 'completion_review', entityType: 'features', entityId: ids[0] } };
  ids = match(path, /^\/jobs\/([^/]+)\/result$/);
  if (ids && method === 'GET') return withIds('jobs.result', { jobId: ids[0] });
  ids = match(path, /^\/jobs\/([^/]+)\/(cancel|retry)$/);
  if (ids && method === 'POST') return withIds(ids[1] === 'cancel' ? 'jobs.cancel' : 'jobs.retry', { jobId: ids[0] });
  ids = match(path, /^\/jobs\/([^/]+)$/);
  if (ids && method === 'GET') return withIds('jobs.get', { jobId: ids[0] });
  ids = match(path, /^\/notifications\/([^/]+)\/(read|dismiss)$/);
  if (ids && method === 'POST') return withIds(ids[1] === 'read' ? 'notifications.read' : 'notifications.dismiss', { notificationId: ids[0] });
  ids = match(path, /^\/knowledge$/);
  if (ids && method === 'GET') return { name: 'knowledge.read', input: { path: url.searchParams.get('path') ?? '' } };

  throw new Error(`Native operation is not mapped: ${method} ${path}`);
};

const applicationResponse = (response: NativeResponse): ApplicationResponse => {
  const text = response.status === 204 ? '' : typeof response.body === 'string' ? response.body : JSON.stringify(response.body);
  const bytes = new TextEncoder().encode(text);
  return {
    ok: response.status >= 200 && response.status < 300,
    status: response.status,
    json: async <T>() => response.body as T,
    text: async () => text,
    body: new ReadableStream<Uint8Array>({ start(controller) { controller.enqueue(bytes); controller.close(); } }),
  };
};

const commandFor = (operation: NativeOperation): NativeCommand => {
  const root = operation.name.split('.')[0];
  if (operation.name === 'jobs.enqueue') return 'enqueue_ai_job';
  if (root === 'health' || root === 'events') return 'system_command';
  if (root === 'vault' || root === 'knowledge') return 'vault_command';
  if (root === 'locale' || root === 'provider' || root === 'i18n') return 'settings_command';
  if (root === 'jobs' || root === 'notifications') return 'jobs_command';
  return 'workflow_command';
};

export class TauriApplicationClient implements ApplicationClient {
  async request(request: ApplicationRequest): Promise<ApplicationResponse> {
    if (request.signal?.aborted) throw new DOMException('The operation was aborted', 'AbortError');
    const operation = operationFor(request);
    if (operation.name === 'conversation.stream') return this.streamConversation(operation, request.signal);
    const response = await invoke<NativeResponse>(commandFor(operation), { operation });
    if (request.signal?.aborted) throw new DOMException('The operation was aborted', 'AbortError');
    if (operation.name === 'jobs.events') {
      const bytes = new TextEncoder().encode(String(response.body));
      let streamController!: ReadableStreamDefaultController<Uint8Array>;
      const body = new ReadableStream<Uint8Array>({ start(value) { streamController = value; value.enqueue(bytes); } });
      request.signal?.addEventListener('abort', () => streamController.close(), { once: true });
      return { ...applicationResponse(response), body };
    }
    return applicationResponse(response);
  }

  private async streamConversation(operation: NativeOperation, signal?: AbortSignal): Promise<ApplicationResponse> {
    const requestId = `conversation-${crypto.randomUUID()}`;
    let controller!: ReadableStreamDefaultController<Uint8Array>;
    const stream = new ReadableStream<Uint8Array>({ start(value) { controller = value; } });
    const onEvent = new Channel<{ kind: 'chunk' | 'complete' | 'cancelled' | 'error'; data?: number[]; message?: string }>();
    onEvent.onmessage = (event) => {
      if (event.kind === 'chunk' && event.data) controller.enqueue(new Uint8Array(event.data));
      else if (event.kind === 'complete') controller.close();
      else if (event.kind === 'cancelled') controller.error(new DOMException('The operation was aborted', 'AbortError'));
      else if (event.kind === 'error') controller.error(new Error(event.message ?? 'Conversation failed'));
    };
    const abort = () => { void invoke('cancel_conversation', { requestId }); };
    signal?.addEventListener('abort', abort, { once: true });
    try {
      const response = await invoke<NativeResponse>('conversation_stream', {
        requestId,
        entityType: operation.input.entityType,
        entityId: operation.input.entityId,
        message: operation.input.message,
        onEvent,
      });
      return { ...applicationResponse(response), body: stream, text: async () => new Response(stream).text() };
    } catch (error) {
      controller.error(error);
      throw error;
    }
  }
}
