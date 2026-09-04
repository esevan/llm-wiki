import {
  completeDesktopE2e,
  desktopE2eMode,
  reportDesktopE2eProgress,
  type DesktopE2eResult,
} from '../services/tauriApplicationClient';
import { getVaultSetupStatus } from '../services/vaultSetupClient';

const waitFor = async (condition: () => boolean, description: string) => {
  if (condition()) return;
  await new Promise<void>((resolve, reject) => {
    const observer = new MutationObserver(() => {
      if (!condition()) return;
      window.clearTimeout(timeout);
      observer.disconnect();
      resolve();
    });
    const timeout = window.setTimeout(() => {
      observer.disconnect();
      reject(new Error(`Timed out waiting for ${description}`));
    }, 10_000);
    observer.observe(document.documentElement, {
      attributes: true,
      characterData: true,
      childList: true,
      subtree: true,
    });
  });
  if (!condition()) throw new Error(`Did not observe ${description}`);
};

interface CreatedRecord {
  id: string;
}

interface BoardRecord {
  id: string;
  text?: string;
  state?: string;
  localized_versions?: Record<string, { text?: string }>;
}

interface BoardResponse {
  captures: BoardRecord[];
  problems: BoardRecord[];
}

const waitForJobKind = async (taskKind: string, entityId: string) => {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    const result = await applicationJson<{ jobs: Array<JobResponse & { entity_id?: string }> }>('/jobs');
    const job = result.jobs.find((item) => item.task_kind === taskKind && item.entity_id === entityId);
    if (job) return job;
    await new Promise((resolve) => window.setTimeout(resolve, 50));
  }
  throw new Error(`${taskKind} was not queued for ${entityId}`);
};

interface CompletionResponse {
  path: string;
  closed: {
    solutions: string[];
    problem: string;
    capture: string | null;
  };
}

interface LineageResponse {
  lineage: { stages: Array<{ kind: string }> };
}

interface JobResponse {
  id: string;
  status: string;
  task_kind?: string;
}

const applicationJson = async <T>(
  path: string,
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' = 'GET',
  body?: object,
) => {
  const response = await window.llmWikiApplication.request({
    path,
    method,
    headers: { 'Content-Type': 'application/json', 'X-LLM-Wiki-Locale': 'en' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!response.ok) throw new Error(`${method} ${path} failed (${response.status}): ${await response.text()}`);
  return response.status === 204 ? (undefined as T) : response.json<T>();
};

const waitForJobResult = async <T>(jobId: string) => {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    const job = await applicationJson<JobResponse>(`/jobs/${jobId}`);
    if (['completed', 'awaiting_review'].includes(job.status)) {
      return applicationJson<{ result: T }>(`/jobs/${jobId}/result`).then((value) => value.result);
    }
    if (['failed', 'cancelled', 'stale'].includes(job.status)) {
      throw new Error(`Desktop job ${jobId} ended as ${job.status}`);
    }
    await new Promise((resolve) => window.setTimeout(resolve, 50));
  }
  throw new Error(`Desktop job ${jobId} did not finish`);
};

const waitForIdleJobs = async () => {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    const result = await applicationJson<{ jobs: JobResponse[] }>('/jobs');
    const failedJob = result.jobs.find((job) => job.status === 'failed');
    if (failedJob) throw new Error(`${failedJob.task_kind ?? 'Background job'} ${failedJob.id} failed`);
    if (!result.jobs.some((job) => ['queued', 'running', 'retryable', 'cancelling'].includes(job.status))) return;
    await new Promise((resolve) => window.setTimeout(resolve, 50));
  }
  throw new Error('Desktop background jobs did not quiesce');
};

const waitForSemanticStartupIndex = async () => {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    const search = await applicationJson<{
      results: Array<{ semantic_score: number | null }>;
      semantic_available: boolean;
    }>('/search?q=startup&semantic=true');
    if (search.semantic_available && typeof search.results[0]?.semantic_score === 'number') return;
    await new Promise((resolve) => window.setTimeout(resolve, 50));
  }
  throw new Error('Startup Vault indexing did not prepare bundled semantic Search');
};

const applicationText = async (path: string, body: object) => {
  const response = await window.llmWikiApplication.request({
    path,
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-LLM-Wiki-Locale': 'en' },
    body: JSON.stringify(body),
  });
  const text = await response.text();
  if (!response.ok) throw new Error(`POST ${path} failed (${response.status}): ${text}`);
  return text;
};

const report = async (result: DesktopE2eResult) => completeDesktopE2e(result);

const run = async (providerUrl: string) => {
  const steps: string[] = [];
  const step = async (message: string) => {
    steps.push(message);
    await reportDesktopE2eProgress(steps);
  };
  try {
    await waitFor(() => document.documentElement.dataset.applicationReady === 'true', 'application initialization');
    await step('application launched and initialized through the native command path');
    const vaultSetup = await getVaultSetupStatus();
    if (vaultSetup.required || !vaultSetup.path) throw new Error('Configured Vault was not restored');
    await step('the configured Vault was restored without reopening first-run setup');
    await waitForSemanticStartupIndex();
    await step('startup indexing prepared the Vault with the bundled embedding model');
    await applicationJson('/provider/config', 'PUT', {
      base_url: providerUrl,
      model: 'deterministic-test-model',
      async_worker_count: 1,
    });

    const searchNavigation = document.querySelector<HTMLButtonElement>('[data-view="search"]');
    searchNavigation?.click();
    await waitFor(
      () => document.getElementById('search')?.classList.contains('active') ?? false,
      'React Search navigation',
    );
    await step('primary navigation changed the visible React screen');

    document.querySelector<HTMLButtonElement>('[data-view="workbench"]')?.click();
    await waitFor(
      () => document.getElementById('workbench')?.classList.contains('active') ?? false,
      'React Workbench navigation',
    );
    const value = `Native desktop capture ${Date.now()}`;
    const capture = document.getElementById('capture-text') as HTMLInputElement | null;
    if (!capture) throw new Error('Capture input was not rendered');
    capture.value = value;
    capture.dispatchEvent(new Event('input', { bubbles: true }));
    (document.getElementById('capture') as HTMLFormElement | null)?.requestSubmit();
    await waitFor(() => document.getElementById('board')?.textContent?.includes(value) ?? false, 'persisted Capture');
    await step('Capture was created and rendered through React, Tauri, and the real application runtime');

    const workflowCapture = await applicationJson<CreatedRecord>('/captures', 'POST', {
      text: 'Native desktop workflow Capture',
    });
    const chat = await applicationText(`/captures/${workflowCapture.id}/chat`, {
      message: 'Verify native desktop streaming',
    });
    if (!chat.includes('Deterministic') || !chat.includes('event: done')) {
      throw new Error('Native streamed Chat did not complete');
    }
    const draftJob = await applicationJson<JobResponse>(`/captures/${workflowCapture.id}/draft`, 'POST');
    const draft = await waitForJobResult<{ title: string }>(draftJob.id);
    if (draft.title !== 'Clear problem') throw new Error('Native deterministic draft was not returned');
    await step('streamed Chat and durable AI draft passed through Tauri and deterministic provider boundaries');

    const problem = await applicationJson<CreatedRecord>(`/captures/${workflowCapture.id}/promote`, 'POST', {
      statement: 'Native desktop workflow Problem',
      detail: 'Exercise the final Tauri application boundary.',
    });
    const refinementJob = await applicationJson<JobResponse>(`/problems/${problem.id}/refine`, 'POST');
    const refinement = await waitForJobResult<{ title: string }>(refinementJob.id);
    if (refinement.title !== 'Refined problem') throw new Error('Native refinement result was not returned');
    await window.loadBoard();
    await waitFor(() => Boolean(document.querySelector(`[data-approve-problem="${problem.id}"]`)), 'Problem approval action');
    document.querySelector<HTMLButtonElement>(`[data-approve-problem="${problem.id}"]`)?.click();
    await waitFor(
      () => window.workbenchBoard?.problems?.some((item: { id: string; state: string }) => item.id === problem.id && item.state === 'approved') ?? false,
      'Problem approval from its Workbench action',
    );
    await step('Problem approval responded to the packaged Workbench click and refreshed its state');
    document.querySelector<HTMLButtonElement>(`[data-next-chat-id="${problem.id}"]`)?.click();
    await waitFor(
      () => document.querySelector<HTMLDialogElement>('#chat-modal')?.open ?? false,
      'next Solution exploration from its Workbench action',
    );
    document.querySelector<HTMLDialogElement>('#chat-modal')?.close();
    const solution = await applicationJson<CreatedRecord>(`/problems/${problem.id}/features`, 'POST', {
      title: 'Native desktop Solution',
      outcome: 'The packaged command path preserves workflow behavior.',
      non_goals: 'No external provider call.',
      validation_criteria: '- [ ] Native command state persists',
    });
    await window.loadBoard();
    document.querySelector<HTMLButtonElement>(`[data-solution-action="conflict"][data-solution-id="${solution.id}"]`)?.click();
    const conflictJob = await waitForJobKind('conflict_review', solution.id);
    const conflictReview = await waitForJobResult<{ conflicts: unknown[] }>(conflictJob.id);
    if (conflictReview.conflicts.length) throw new Error('Deterministic desktop conflict review was not clear');
    await applicationJson<void>(`/features/${solution.id}/conflict`, 'PUT', {
      state: 'clear',
      citation: 'Native desktop deterministic review',
    });
    await applicationJson<void>(`/features/${solution.id}/approve`, 'POST');
    await window.loadBoard();
    document.querySelector<HTMLButtonElement>(`[data-solution-action="stage"][data-solution-state="proposed"][data-solution-id="${solution.id}"]`)?.click();
    await waitFor(
      () => window.workbenchBoard?.features?.some((item) => item.id === solution.id && item.state === 'proposed') ?? false,
      'move to proposed from its Workbench action',
    );
    await applicationJson<void>(`/features/${solution.id}/approve`, 'POST');
    const progress = await applicationJson<CreatedRecord>(`/features/${solution.id}/progress`, 'POST', {
      body: 'Native command evidence',
    });
    await applicationJson<CreatedRecord>(`/progress/${progress.id}/comments`, 'POST', {
      body: 'Native comment persisted',
    });
    const checklist = await applicationJson<CreatedRecord>(`/features/${solution.id}/checklist`, 'POST', {
      body: 'Native command state persists',
    });
    await applicationJson<void>(`/checklist/${checklist.id}`, 'PUT', {
      body: 'Native command state persists',
      checked: true,
    });
    const restoredProgress = await applicationJson<{ checklist: Array<{ checked: number }> }>(
      `/features/${solution.id}/progress`,
    );
    if (!restoredProgress.checklist.some((item) => Boolean(item.checked))) {
      throw new Error('Native checklist persistence was not restored');
    }
    await waitForIdleJobs();
    await step('refinement, conflict review, workflow, Work Log, comments, and checklist passed');

    await window.loadBoard();
    document.querySelector<HTMLButtonElement>(`[data-solution-action="review"][data-solution-id="${solution.id}"]`)?.click();
    const reviewJob = await waitForJobKind('completion_review', solution.id);
    const review = await waitForJobResult<{ report: { resolution: string } }>(reviewJob.id);
    if (review.report.resolution !== 'complete') throw new Error('Native completion review was not deterministic');
    const notifications = await applicationJson<{ unread_count: number }>('/notifications?unread_only=true');
    if (notifications.unread_count < 1) throw new Error('Completion review notification was not published');
    await step('completion review and persisted notification passed through the real desktop worker');

    const completed = await applicationJson<CompletionResponse>(`/problems/${problem.id}/complete`, 'POST', {
      reason: 'Native desktop E2E verification',
    });
    if (!completed.path.endsWith('.md')) throw new Error('Completion did not produce a Knowledge document');
    if (
      completed.closed.problem !== problem.id ||
      completed.closed.capture !== workflowCapture.id ||
      !completed.closed.solutions.includes(solution.id)
    ) {
      throw new Error('Completion did not close the full Capture, Problem, and Solution chain');
    }
    const lineage = await applicationJson<LineageResponse>(`/features/${solution.id}/lineage`);
    if (lineage.lineage.stages.map((stage) => stage.kind).join(',') !== 'capture,problem,solution,complete') {
      throw new Error('Completed Lineage did not preserve all workflow stages');
    }
    const followUp = await applicationJson<CreatedRecord>(`/features/${solution.id}/follow-up-problem`, 'POST');
    await applicationJson<void>(`/items/problems/${followUp.id}`, 'DELETE');
    let board = await applicationJson<BoardResponse>('/board');
    if (board.problems.some((item) => item.id === followUp.id)) throw new Error('Soft delete did not hide follow-up');
    await applicationJson<void>(`/items/problems/${followUp.id}/restore`, 'POST');
    board = await applicationJson<BoardResponse>('/board');
    if (!board.problems.some((item) => item.id === followUp.id)) throw new Error('Restore did not recover follow-up');
    await step('completion, filesystem projection, Lineage, delete, restore, and follow-up behavior passed');

    await applicationJson<object>('/index', 'POST', {});
    const search = await applicationJson<{
      results: Array<{ path: string; semantic_score: number | null }>;
      semantic_available: boolean;
    }>('/search?q=Native&limit=20&semantic=true');
    if (!search.results.length) throw new Error('Filesystem-backed Search returned no completed-work evidence');
    if (!search.semantic_available || typeof search.results[0].semantic_score !== 'number') {
      throw new Error('Bundled embedding model did not serve native semantic Search');
    }
    const locale = await applicationJson<{ locale: string }>('/settings/locale?browser_locale=en-US');
    await applicationJson('/settings/locale', 'PUT', { locale: locale.locale === 'ko' ? 'en' : 'ko' });
    await applicationJson('/settings/locale', 'PUT', { locale: locale.locale });
    const provider = await applicationJson<Record<string, unknown>>('/provider/config');
    if ('api_key' in provider) throw new Error('Provider configuration exposed the API key');
    await step('bundled offline embeddings, Search, locale restoration, and secret-safe configuration passed through Tauri');

    const persistedBoard = await applicationJson<BoardResponse>('/board');
    const persistedCapture = persistedBoard.captures.find(
      (item) =>
        item.text === value ||
        Object.values(item.localized_versions ?? {}).some((localized) => localized.text === value),
    );
    if (!persistedCapture?.text) throw new Error('Authored Capture was not preserved across locale changes');
    await report({ status: 'relaunch', steps, error: null, capture: persistedCapture.text });
  } catch (error) {
    await report({ status: 'failed', steps, error: error instanceof Error ? error.message : String(error) });
  }
};

const verifyRestored = async (value: string, steps: string[]) => {
  try {
    steps.push('desktop process relaunched and restoration scenario started');
    await reportDesktopE2eProgress(steps);
    await waitFor(() => document.documentElement.dataset.applicationReady === 'true', 'application restoration');
    steps.push('relaunched application initialized');
    await reportDesktopE2eProgress(steps);
    await waitFor(() => document.getElementById('board')?.textContent?.includes(value) ?? false, 'restored Capture');
    steps.push('persisted state was restored after a full desktop process relaunch');
    await report({ status: 'passed', steps, error: null });
  } catch (error) {
    await report({ status: 'failed', steps, error: error instanceof Error ? error.message : String(error) });
  }
};

export function installDesktopScenario() {
  if (!window.__TAURI_INTERNALS__) return;
  void desktopE2eMode().then((state) => {
    if (!state) return;
    if (state.restoreCapture) void verifyRestored(state.restoreCapture, state.restoreSteps);
    else void run(state.providerUrl);
  });
}
