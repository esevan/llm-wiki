import { Channel, invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TauriApplicationClient } from './tauriApplicationClient';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  Channel: class MockChannel<T> {
    onmessage: (message: T) => void = () => undefined;
  },
}));

describe('Tauri application adapter', () => {
  beforeEach(() => vi.mocked(invoke).mockReset());

  it('Given a feature query, when the desktop adapter runs, then it invokes the thin command boundary', async () => {
    vi.mocked(invoke).mockResolvedValue({ status: 200, contentType: 'application/json', body: '{"captures":[]}' });

    const response = await new TauriApplicationClient().request({ path: '/board' });

    expect(await response.json()).toEqual({ captures: [] });
    expect(invoke).toHaveBeenCalledWith(
      'application_request',
      { request: { path: '/board', method: 'GET', headers: {}, body: null } },
    );
  });

  it('Given a live event query, when it starts, then chunks are delivered before completion', async () => {
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command !== 'application_stream') return false;
      const channel = (args as { onEvent: Channel<{ kind: string; data?: number[] }> }).onEvent;
      queueMicrotask(() => {
        channel.onmessage({ kind: 'chunk', data: Array.from(new TextEncoder().encode('event: jobs\n\n')) });
        channel.onmessage({ kind: 'complete' });
      });
      return { status: 200, contentType: 'text/event-stream', body: '' };
    });

    const response = await new TauriApplicationClient().request({ path: '/jobs/events' });

    expect(await response.text()).toBe('event: jobs\n\n');
    expect(invoke).toHaveBeenCalledWith(
      'application_stream',
      expect.objectContaining({ requestId: expect.stringMatching(/^ui-/), onEvent: expect.anything() }),
    );
  });

  it('Given an already-cancelled request, when invoked, then no native command starts', async () => {
    const controller = new AbortController();
    controller.abort();

    await expect(new TauriApplicationClient().request({ path: '/board', signal: controller.signal })).rejects.toMatchObject({
      name: 'AbortError',
    });
    expect(invoke).not.toHaveBeenCalled();
  });

  it('Given a running stream, when it is aborted, then the native request is cancelled', async () => {
    const controller = new AbortController();
    let channel!: Channel<{ kind: string }>;
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command === 'application_stream') {
        channel = (args as { onEvent: Channel<{ kind: string }> }).onEvent;
        return { status: 200, contentType: 'text/event-stream', body: '' };
      }
      channel.onmessage({ kind: 'cancelled' });
      return true;
    });

    const response = await new TauriApplicationClient().request({ path: '/features/1/chat', signal: controller.signal });
    controller.abort();

    await expect(response.text()).rejects.toMatchObject({ name: 'AbortError' });
    expect(invoke).toHaveBeenLastCalledWith('cancel_application_request', {
      requestId: expect.stringMatching(/^ui-/),
    });
  });
});
