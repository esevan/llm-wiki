import { afterEach, describe, expect, it, vi } from 'vitest';
import { HttpApplicationClient } from './httpApplicationClient';

describe('HTTP application adapter', () => {
  afterEach(() => vi.restoreAllMocks());

  it('Given a typed command, when requested, then transport details are serialized outside feature code', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{}', { status: 201 }));
    const client = new HttpApplicationClient();

    const response = await client.request({
      path: '/captures',
      method: 'POST',
      headers: { 'X-LLM-Wiki-Locale': 'en' },
      body: JSON.stringify({ text: 'Keep this thought' }),
    });

    expect(response.status).toBe(201);
    expect(fetchMock).toHaveBeenCalledWith('/api/captures', expect.objectContaining({ method: 'POST' }));
  });

  it('Given cancellation, when requested, then the AbortSignal reaches the transport', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{}'));
    const client = new HttpApplicationClient();
    const controller = new AbortController();

    await client.request({ path: '/board', signal: controller.signal });

    expect(fetchMock).toHaveBeenCalledWith('/api/board', expect.objectContaining({ signal: controller.signal }));
  });
});
