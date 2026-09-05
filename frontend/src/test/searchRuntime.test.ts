import source from '../../public/runtime/search-settings.js?raw';
import { beforeEach, expect, it, vi } from 'vitest';

const searchRuntime = source.slice(source.indexOf('let searchRequest='), source.indexOf("$('#results').addEventListener"));

function setup(api: (path: string) => Promise<{ results: Array<{ path: string; title: string; snippet: string }> }>) {
  document.body.innerHTML = '<form id="search-form"><input id="query"><input id="semantic" type="checkbox"></form><section id="results" class="results"></section>';
  new Function('api', '$', 'esc', 't', searchRuntime)(api, (selector: string) => document.querySelector(selector), (value: unknown) => String(value), (key: string) => ({
    'search.loading_title': 'Searching your Vault…',
    'search.loading_body': 'Looking through local notes.',
    'search.empty_title': 'No matching notes yet.',
    'search.empty_body': 'Try another query.',
    'search.error_title': 'Search could not finish',
  }[key] || key));
  return document.querySelector<HTMLFormElement>('#search-form')!;
}

beforeEach(() => { vi.restoreAllMocks(); });

it('shows loading first and then an error instead of leaving the loading state visible', async () => {
  let fail!: (reason: Error) => void;
  const form = setup(() => new Promise((_resolve, reject) => { fail = reject; }));
  const pending = form.onsubmit?.({ preventDefault: vi.fn() } as unknown as SubmitEvent);
  expect(document.querySelector('#results')).toHaveTextContent('Searching your Vault…');
  fail(new Error('Vault unavailable'));
  await pending;
  expect(document.querySelector('#results')).toHaveClass('results');
  expect(document.querySelector('#results')).toHaveTextContent('Search could not finish');
  expect(document.querySelector('#results')).toHaveTextContent('Vault unavailable');
});

it('replaces loading with a clear empty result state', async () => {
  const form = setup(async () => ({ results: [] }));
  await form.onsubmit?.({ preventDefault: vi.fn() } as unknown as SubmitEvent);
  expect(document.querySelector('#results')).toHaveTextContent('No matching notes yet.');
});

it('does not let an older response replace a newer search result', async () => {
  let first!: (value: { results: Array<{ path: string; title: string; snippet: string }> }) => void;
  let second!: (value: { results: Array<{ path: string; title: string; snippet: string }> }) => void;
  const api = vi.fn().mockImplementationOnce(() => new Promise(resolve => { first = resolve; })).mockImplementationOnce(() => new Promise(resolve => { second = resolve; }));
  const form = setup(api);
  const query = document.querySelector<HTMLInputElement>('#query')!;
  query.value = 'older';
  const older = form.onsubmit?.({ preventDefault: vi.fn() } as unknown as SubmitEvent);
  query.value = 'newer';
  const newer = form.onsubmit?.({ preventDefault: vi.fn() } as unknown as SubmitEvent);
  second({ results: [{ path: 'new.md', title: 'New result', snippet: 'current' }] });
  await newer;
  first({ results: [{ path: 'old.md', title: 'Old result', snippet: 'stale' }] });
  await older;
  expect(document.querySelector('#results')).toHaveTextContent('New result');
  expect(document.querySelector('#results')).not.toHaveTextContent('Old result');
});

it('does not append a stale pagination page after a new search begins', async () => {
  let resolveMore!: (value: { results: Array<{ path: string; title: string; snippet: string }> }) => void;
  const page = Array.from({ length: 20 }, (_, index) => ({ path: `${index}.md`, title: `Initial ${index}`, snippet: 'first page' }));
  const api = vi.fn()
    .mockResolvedValueOnce({ results: page })
    .mockImplementationOnce(() => new Promise(resolve => { resolveMore = resolve; }))
    .mockResolvedValueOnce({ results: [{ path: 'new.md', title: 'New query', snippet: 'current' }] });
  const form = setup(api);
  const query = document.querySelector<HTMLInputElement>('#query')!;
  await form.onsubmit?.({ preventDefault: vi.fn() } as unknown as SubmitEvent);
  const more = document.querySelector<HTMLButtonElement>('#more-results')!;
  more.click();
  expect(more).toBeDisabled();
  query.value = 'new query';
  await form.onsubmit?.({ preventDefault: vi.fn() } as unknown as SubmitEvent);
  resolveMore({ results: [{ path: 'old-more.md', title: 'Old pagination', snippet: 'stale' }] });
  await Promise.resolve();
  expect(document.querySelector('#results')).toHaveTextContent('New query');
  expect(document.querySelector('#results')).not.toHaveTextContent('Old pagination');
});
