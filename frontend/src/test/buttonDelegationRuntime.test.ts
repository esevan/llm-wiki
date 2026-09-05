import archive from '../../public/runtime/archive.js?raw';
import conflicts from '../../public/runtime/conflicts.js?raw';
import explore from '../../public/runtime/explore.js?raw';
import foundation from '../../public/runtime/foundation.js?raw';
import jobs from '../../public/runtime/jobs.js?raw';
import { beforeEach, expect, it, vi } from 'vitest';

const line = (source: string, prefix: string) => source.split('\n').find((value) => value.includes(prefix))!;
const $ = (selector: string) => document.querySelector(selector) as HTMLElement;

beforeEach(() => { document.body.innerHTML = ''; vi.restoreAllMocks(); });

it('CB-039 binds refinement cancellation to the rendered Close action, never Track this chat', () => {
  document.body.innerHTML = '<dialog id="chat-modal"><section class="modal"><form id="chat-form"><footer><button id="chat-track-toggle" type="button">Track this chat</button><button aria-label="Close" type="button">×</button></footer></form></section></dialog>';
  const chatModal = document.querySelector<HTMLDialogElement>('#chat-modal')!, close = vi.fn();
  const binding = foundation.match(/chatModal\.querySelector\('#chat-form \[aria-label="Close"\]'\)\.onclick=closeRefinementModal;/)?.[0];
  new Function('chatModal', 'closeRefinementModal', binding!)(chatModal, close);
  document.querySelector<HTMLButtonElement>('#chat-track-toggle')!.click();
  expect(close).not.toHaveBeenCalled();
  document.querySelector<HTMLButtonElement>('[aria-label="Close"]')!.click();
  expect(close).toHaveBeenCalledOnce();
});

it('CB-033 routes reactive archive regenerate and delete buttons through the rendered delegated handler', () => {
  document.body.innerHTML = '<section class="workbench-context"><button id="regenerate" data-archive-action="regenerate" data-problem-id="p"></button><button id="delete" data-archive-action="delete" data-problem-id="p"></button></section>';
  const regenerate = vi.fn(), remove = vi.fn();
  new Function('$', 'regenerateCompletionPlaybook', 'deleteCompletionPlaybook', line(archive, "$('.workbench-context').addEventListener"))($, regenerate, remove);
  document.querySelector<HTMLButtonElement>('#regenerate')!.click();
  document.querySelector<HTMLButtonElement>('#delete')!.click();
  expect(regenerate).toHaveBeenCalledWith('p', expect.any(HTMLButtonElement));
  expect(remove).toHaveBeenCalledWith('p', expect.any(HTMLButtonElement));
});

it('CB-034 routes queue cancel/retry and result controls through their delegated handler', async () => {
  document.body.innerHTML = '<section id="queue-list"><article data-job-id="one"><button id="cancel" data-job-action="cancel"></button><button id="result" data-job-action="result"></button></article></section>';
  const api = vi.fn().mockResolvedValue({}), refresh = vi.fn().mockResolvedValue(undefined), open = vi.fn().mockResolvedValue(undefined);
  const handler = line(jobs, "$('#queue-list').onclick").match(/\$\('#queue-list'\)\.onclick=async event=>\{.*?\};/)?.[0];
  new Function('$', 'api', 'refreshQueue', 'openJobResult', handler!)($, api, refresh, open);
  document.querySelector<HTMLButtonElement>('#cancel')!.click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(api).toHaveBeenCalledWith('/jobs/one/cancel', { method: 'POST' });
  expect(refresh).toHaveBeenCalledOnce();
  document.querySelector<HTMLButtonElement>('#result')!.click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(open).toHaveBeenCalledWith('one');
});

it('CB-037 routes rendered notification open and dismiss controls through their delegated handler', async () => {
  document.body.innerHTML = '<section id="alert-list"><article data-notification-id="note"><button id="open" data-notification-action="open" data-job-id="job"></button><button id="dismiss" data-notification-action="dismiss"></button></article></section>';
  const api = vi.fn().mockResolvedValue({}), refresh = vi.fn().mockResolvedValue(undefined), open = vi.fn().mockResolvedValue(undefined);
  const handler = line(jobs, "$('#alert-list').onclick").match(/\$\('#alert-list'\)\.onclick=async event=>\{.*?\};/)?.[0];
  new Function('$', 'api', 'refreshNotifications', 'openJobResult', handler!)($, api, refresh, open);
  document.querySelector<HTMLButtonElement>('#open')!.click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(api).toHaveBeenCalledWith('/notifications/note/read', { method: 'POST' });
  expect(open).toHaveBeenCalledWith('job');
  document.querySelector<HTMLButtonElement>('#dismiss')!.click();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(api).toHaveBeenCalledWith('/notifications/note/dismiss', { method: 'POST' });
  expect(refresh).toHaveBeenCalledTimes(2);
});

it('CB-035 turns an Explore quick prompt click into a normal Chat form submission', () => {
  document.body.innerHTML = '<section id="chat-log"><button data-chat-prompt="Define outcome"><span>nested</span></button></section><textarea id="chat-message"></textarea><form id="chat-form"></form>';
  const form = document.querySelector<HTMLFormElement>('#chat-form')!;
  const submit = vi.fn();
  form.requestSubmit = submit;
  new Function('$', line(explore, "$('#chat-log').onclick"))($);
  document.querySelector<HTMLElement>('#chat-log span')!.click();
  expect(document.querySelector<HTMLTextAreaElement>('#chat-message')!.value).toBe('Define outcome');
  expect(submit).toHaveBeenCalledOnce();
});

it('CB-036 routes every board button action from nested targets to its production handler', () => {
  document.body.innerHTML = '<section id="board"><button data-approve-problem="p"><span id="approve-icon"></span></button><button data-next-chat-id="p" data-next-chat-type="problems"><span id="next-icon"></span></button><button data-solution-action="conflict" data-solution-id="s"><span id="conflict-icon"></span></button><button data-solution-action="review" data-solution-id="s"><span id="review-icon"></span></button><button data-solution-action="stage" data-solution-id="s" data-solution-state="approved"><span id="stage-icon"></span></button><button data-delete-type="problems" data-delete-id="p"><span id="delete-icon"></span></button></section>';
  const approve = vi.fn(), next = vi.fn(), conflict = vi.fn(), review = vi.fn(), move = vi.fn(), remove = vi.fn();
  new Function('$', 'approveProblem', 'openNextChat', 'runConflictReview', 'reviewCompletion', 'moveSolution', 'deleteItem', 'openItemDetail', line(conflicts, "$('#board').onclick"))($, approve, next, conflict, review, move, remove, vi.fn());
  document.querySelector<HTMLElement>('#approve-icon')!.click();
  document.querySelector<HTMLElement>('#next-icon')!.click();
  document.querySelector<HTMLElement>('#conflict-icon')!.click();
  document.querySelector<HTMLElement>('#review-icon')!.click();
  document.querySelector<HTMLElement>('#stage-icon')!.click();
  document.querySelector<HTMLElement>('#delete-icon')!.click();
  expect(approve).toHaveBeenCalledWith('p', expect.any(HTMLButtonElement));
  expect(next).toHaveBeenCalledWith('problems', 'p');
  expect(conflict).toHaveBeenCalledWith('s', expect.any(HTMLButtonElement), false);
  expect(review).toHaveBeenCalledWith('s', expect.any(HTMLButtonElement));
  expect(move).toHaveBeenCalledWith('s', 'approved', expect.any(HTMLButtonElement));
  expect(remove).toHaveBeenCalledWith('problems', 'p');
});
