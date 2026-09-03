import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TauriApplicationClient } from './tauriApplicationClient';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

describe('Tauri domain command adapter', () => {
  beforeEach(() => vi.mocked(invoke).mockReset());

  it('Given a board query, when invoked, then only the workflow command receives a typed operation', async () => {
    vi.mocked(invoke).mockResolvedValue({ status: 200, body: { captures: [] } });
    const response = await new TauriApplicationClient().request({ path: '/board' });
    expect(await response.json()).toEqual({ captures: [] });
    expect(invoke).toHaveBeenCalledWith('workflow_command', {
      operation: { name: 'board.get', input: {} },
    });
  });

  it('Given vault and settings queries, then they cannot cross domain command boundaries', async () => {
    vi.mocked(invoke).mockResolvedValue({ status: 200, body: {} });
    const client = new TauriApplicationClient();
    await client.request({ path: '/search?q=native' });
    await client.request({ path: '/provider/config' });
    expect(invoke).toHaveBeenNthCalledWith(1, 'vault_command', {
      operation: { name: 'vault.search', input: { query: 'native', limit: 20, offset: 0, semantic: false } },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, 'settings_command', {
      operation: { name: 'provider.get', input: {} },
    });
  });

  it('Given a durable AI request, then the native job command receives domain identifiers instead of HTTP data', async () => {
    vi.mocked(invoke).mockResolvedValue({ status: 202, body: { id: 'job-1', status: 'queued' } });
    await new TauriApplicationClient().request({ path: '/captures/capture-1/draft', method: 'POST' });
    expect(invoke).toHaveBeenCalledWith('enqueue_ai_job', {
      operation: { name: 'jobs.enqueue', input: { taskKind: 'workflow_draft', entityType: 'captures', entityId: 'capture-1' } },
    });
  });

  it('Given an already-cancelled request, when invoked, then no native command starts', async () => {
    const controller = new AbortController();
    controller.abort();
    await expect(new TauriApplicationClient().request({ path: '/board', signal: controller.signal })).rejects.toMatchObject({ name: 'AbortError' });
    expect(invoke).not.toHaveBeenCalled();
  });
});
