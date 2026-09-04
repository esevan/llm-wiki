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
      operation: { name: 'board.get', input: { locale: 'en' } },
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
      operation: { name: 'jobs.enqueue', input: { taskKind: 'workflow_draft', entityType: 'captures', entityId: 'capture-1', locale: 'en' } },
    });
  });

  it('Given a Korean Capture, when it is saved, then the authored locale reaches the translation scheduler', async () => {
    vi.mocked(invoke).mockResolvedValue({ status: 201, body: { id: 'capture-1', text: '기록' } });
    await new TauriApplicationClient().request({
      path: '/captures',
      method: 'POST',
      headers: { 'X-LLM-Wiki-Locale': 'ko' },
      body: JSON.stringify({ text: '기록' }),
    });
    expect(invoke).toHaveBeenCalledWith('workflow_command', {
      operation: { name: 'capture.create', input: { text: '기록', locale: 'ko' } },
    });
  });

  it('Given Korean content and a native-only feature, then locale and domain data survive without an HTTP route crossing IPC', async () => {
    vi.mocked(invoke).mockResolvedValue({ status: 204, body: null });
    await new TauriApplicationClient().request({
      path: '/items/problems/problem-1/localizations',
      method: 'PUT',
      headers: { 'X-LLM-Wiki-Locale': 'ko' },
      body: JSON.stringify({ locale: 'en', fields: { statement: 'English problem' } }),
    });
    expect(invoke).toHaveBeenCalledWith('workflow_command', {
      operation: {
        name: 'item.localization.save',
        input: { entityType: 'problems', entityId: 'problem-1', locale: 'en', fields: { statement: 'English problem' } },
      },
    });
  });

  it('Given provider verification, then the dedicated async provider command receives no URL-shaped payload', async () => {
    vi.mocked(invoke).mockResolvedValue({ status: 200, body: { models: ['local-model'] } });
    await new TauriApplicationClient().request({ path: '/provider/test', method: 'POST' });
    expect(invoke).toHaveBeenCalledWith('provider_request', {
      operation: { name: 'provider.test', input: {} },
    });
  });

  it('Given an already-cancelled request, when invoked, then no native command starts', async () => {
    const controller = new AbortController();
    controller.abort();
    await expect(new TauriApplicationClient().request({ path: '/board', signal: controller.signal })).rejects.toMatchObject({ name: 'AbortError' });
    expect(invoke).not.toHaveBeenCalled();
  });
});
