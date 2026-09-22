import type { WorkbenchScenarioHarness } from "./workbenchScenarios";
import { taskExecutionClient } from "../services/taskExecutionClient";

type Session = {
  id: string;
  title: string;
  provider: string;
  model: string;
  approvalMode: string;
  workspacePath: string;
};
type SessionRecord = {
  session: Session;
  entries: Array<{ body?: string; attachment?: { name?: string; mediaType?: string; data?: string } }>;
};
type TaskSnapshot = { id: string; title: string; state: string; taskRevision: number };
type RestartState = {
  taskA: TaskSnapshot;
  taskB: TaskSnapshot;
  sessionA1: string;
  sessionA2: string;
  sessionB: string;
  recordA1: string;
  recordA2: string;
  recordB: string;
  imageName: string;
  imageData: string;
};

async function waitSessionIdle(h: Pick<WorkbenchScenarioHarness, "waitFor">, label: string) {
  await h.waitFor(
    () => document.querySelector(".task-detail")?.getAttribute("aria-busy") !== "true",
    `idle Task detail before ${label}`,
  );
  await h.waitFor(() => {
    const space = document.querySelector(".task-session-space");
    const create = space?.querySelector<HTMLButtonElement>('[data-control="task-session-create"]');
    const message = space?.querySelector<HTMLTextAreaElement>('[data-control="task-session-message"]');
    return Boolean(space && create && !create.disabled && !space.textContent?.includes("Loading sessions") && !message?.disabled);
  }, `loaded and idle sessions before ${label}`);
}

async function openSessions(h: WorkbenchScenarioHarness, label: string) {
  const tab = document.querySelector<HTMLButtonElement>('[data-control="task-detail-tab-sessions"]');
  if (!tab) throw new Error("Missing Task work sessions tab");
  await h.prepareClick(tab, label);
  h.click(tab, label);
  await h.waitFor(() => tab.getAttribute("aria-selected") === "true", "selected work sessions tab");
  await waitSessionIdle(h, label);
}

async function closeDetail(h: WorkbenchScenarioHarness) {
  const close = document.querySelector<HTMLButtonElement>('[data-control="task-detail-close"]');
  if (!close) return;
  await h.prepareClick(close, "Close Task detail");
  h.click(close, "Close Task detail");
  await h.waitFor(() => !document.querySelector(".task-detail"), "closed Task detail");
}

async function createSession(h: WorkbenchScenarioHarness, label: string) {
  const previousId = sessionId();
  const button = document.querySelector<HTMLButtonElement>('[data-control="task-session-create"]');
  if (!button) throw new Error("Missing New session control");
  await h.prepareClick(button, label);
  h.click(button, label);
  await h.waitFor(() => Boolean(document.querySelector(".task-session-layout")) && sessionId() !== previousId, `opened ${label}`);
  await waitSessionIdle(h, label);
}

function sessionId() {
  return document.querySelector<HTMLSelectElement>('[data-control="task-session-select"]')?.value ?? "";
}

function settingsControl<T extends HTMLInputElement | HTMLSelectElement>(labelText: string): T {
  const label = [...document.querySelectorAll<HTMLLabelElement>(".task-session-settings label")]
    .find((candidate) => candidate.textContent?.includes(labelText));
  const control = label?.querySelector<T>("input,select");
  if (!control) throw new Error(`Missing session setting ${labelText}`);
  return control;
}

function setInput(h: Pick<WorkbenchScenarioHarness, "enter">, control: HTMLInputElement, value: string) {
  h.enter(control, value);
}

export async function saveSettings(h: Pick<WorkbenchScenarioHarness, "enter" | "prepareClick" | "click" | "waitFor">, model: string, path: string) {
  const modelControl = settingsControl<HTMLSelectElement>("Model");
  modelControl.value = model;
  modelControl.dispatchEvent(new Event("change", { bubbles: true }));
  setInput(h, settingsControl<HTMLInputElement>("Project folder path"), path);
  const save = document.querySelector<HTMLButtonElement>('[data-control="task-session-settings-save"]');
  if (!save) throw new Error("Missing session settings save control");
  await h.prepareClick(save, "Save session settings");
  h.click(save, "Save session settings");
  // Saving closes the editor. The captured Save button is then detached and
  // retains its busy/disabled state; inspect the current UI instead.
  await h.waitFor(() => {
    const settings = document.querySelector(".task-session-settings");
    const prepare = settings?.querySelector<HTMLButtonElement>('[data-control="task-session-prepare"]');
    return Boolean(settings?.querySelector('[role="status"]') && prepare && !prepare.disabled
      && !settings.querySelector('[data-control="task-session-settings-save"]'));
  }, "saved session settings");
  await waitSessionIdle(h, "saved session settings");
}

async function saveRecord(h: WorkbenchScenarioHarness, body: string, file?: File) {
  const message = document.querySelector<HTMLTextAreaElement>('[data-control="task-session-message"]');
  if (!message) throw new Error("Missing session message composer");
  h.enter(message, body);
  if (file) {
    const input = document.querySelector<HTMLInputElement>('[data-control="task-session-attachment"]');
    if (!input) throw new Error("Missing session attachment control");
    const transfer = new DataTransfer();
    transfer.items.add(file);
    Object.defineProperty(input, "files", { configurable: true, value: transfer.files });
    input.dispatchEvent(new Event("change", { bubbles: true }));
    await h.waitFor(() => Boolean(document.querySelector(".task-session-pending")), "pending session attachment");
  }
  const send = document.querySelector<HTMLButtonElement>('[data-control="task-session-send"]');
  if (!send) throw new Error("Missing Save record control");
  // Synthetic input events can return before React commits the new draft.
  // File records already wait for FileReader; text-only records need this too.
  await h.waitFor(() => !send.disabled, "enabled Save record after draft input");
  await h.prepareClick(send, "Save session record");
  h.click(send, "Save session record");
  await h.waitFor(() => !document.querySelector<HTMLTextAreaElement>('[data-control="task-session-message"]')?.value, "cleared saved session draft");
  await waitSessionIdle(h, "saved session record");
}

export async function selectSession(h: Pick<WorkbenchScenarioHarness, "waitFor">, id: string, expectedBody: string, source: "record" | "run" = "record") {
  const select = document.querySelector<HTMLSelectElement>('[data-control="task-session-select"]');
  if (!select) throw new Error("Missing session picker");
  select.value = id;
  select.dispatchEvent(new Event("change", { bubbles: true }));
  await h.waitFor(() => sessionId() === id && Boolean(document.querySelector(".task-session-layout")), `opened session ${id}`);
  await h.waitFor(() => source === "run"
    ? [...document.querySelectorAll<HTMLElement>('[data-control="task-session-run-open"]')].some((entry) => entry.textContent?.includes(expectedBody))
    : [...document.querySelectorAll<HTMLElement>(".task-session-entries li")].some((entry) => entry.getAttribute("data-entry-author") === "user" && entry.textContent?.includes(expectedBody)), `rendered ${source} for session ${id}`);
  await waitSessionIdle(h, `opened session ${id}`);
}

async function listSessions(h: WorkbenchScenarioHarness, taskId: string) {
  return (await h.api<{ sessions: Session[] }>(`/tasks/${taskId}/work-sessions`)).sessions;
}

export async function prepareTaskWorkSessionRestartScenario(h: WorkbenchScenarioHarness) {
  const titleA = `Task sessions A ${Date.now()}`;
  const titleB = `Task sessions B ${Date.now()}`;
  await h.create(titleA);
  await h.create(titleB);
  const taskA = await h.task(titleA) as TaskSnapshot;
  const taskB = await h.task(titleB) as TaskSnapshot;
  const beforeA = await h.api<TaskSnapshot>(`/tasks/${taskA.id}`);
  const beforeB = await h.api<TaskSnapshot>(`/tasks/${taskB.id}`);

  await h.detail(titleA);
  await openSessions(h, "Open Task A work sessions");
  const emptyA = await listSessions(h, taskA.id);
  if (emptyA.length !== 0) throw new Error("Opening an empty Task created a work session");
  await h.step("Task A opened with zero sessions until the explicit New session action.");

  await createSession(h, "Create Task A session one");
  const sessionA1 = sessionId();
  if (!sessionA1) throw new Error("Task A session one did not become active");
  await saveSettings(h, "gpt-5.6-luna", "/opaque/project-a-one");
  const pngData = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
  const imageName = "task-a-image.png";
  await saveRecord(h, "Task A session one record", new File([Uint8Array.from(atob(pngData), (char) => char.charCodeAt(0))], imageName, { type: "image/png" }));
  const recordA1 = (await h.api<SessionRecord>(`/tasks/${taskA.id}/work-sessions/${sessionA1}`)).entries[0]?.body ?? "";
  if (recordA1 !== "Task A session one record") throw new Error("Task A session one record body was not saved exactly");

  await createSession(h, "Create Task A session two");
  const sessionA2 = sessionId();
  if (!sessionA2 || sessionA2 === sessionA1) throw new Error("Task A sessions were not distinct");
  await saveSettings(h, "gpt-5.5", "/opaque/project-a-two");
  await saveRecord(h, "Task A session two record");
  const recordA2 = (await h.api<SessionRecord>(`/tasks/${taskA.id}/work-sessions/${sessionA2}`)).entries[0]?.body ?? "";
  if (recordA2 !== "Task A session two record") throw new Error("Task A session two record body was not saved exactly");

  await closeDetail(h);
  await h.detail(titleB);
  await openSessions(h, "Open Task B work sessions");
  if ((await listSessions(h, taskB.id)).length !== 0) throw new Error("Task A sessions leaked into Task B");
  await createSession(h, "Create Task B session");
  const sessionB = sessionId();
  if (!sessionB) throw new Error("Task B session did not become active");
  await saveSettings(h, "gpt-5.6-terra", "/opaque/project-b");
  await saveRecord(h, "Task B isolated record");
  const recordB = (await h.api<SessionRecord>(`/tasks/${taskB.id}/work-sessions/${sessionB}`)).entries[0]?.body ?? "";
  if (recordB !== "Task B isolated record") throw new Error("Task B record was not saved");
  if ((await listSessions(h, taskB.id)).some((item) => item.id === sessionA1 || item.id === sessionA2)) throw new Error("Task A session identifiers were visible under Task B");

  const afterA = await h.api<TaskSnapshot>(`/tasks/${taskA.id}`);
  const afterB = await h.api<TaskSnapshot>(`/tasks/${taskB.id}`);
  if (afterA.state !== beforeA.state || afterA.taskRevision !== beforeA.taskRevision || afterB.state !== beforeB.state || afterB.taskRevision !== beforeB.taskRevision) throw new Error("Task state changed while using work sessions before relaunch");
  const state: RestartState = { taskA: beforeA, taskB: beforeB, sessionA1, sessionA2, sessionB, recordA1, recordA2, recordB, imageName, imageData: pngData };
  await h.step("Created two isolated Task A sessions and one Task B session with distinct records, settings, and an image; both Task states were captured before relaunch.");
  return state;
}

export async function restoreTaskWorkSessionRestartScenario(h: WorkbenchScenarioHarness, token: string, steps: string[]) {
  const state = JSON.parse(token) as RestartState;
  await h.detail(state.taskA.title);
  await openSessions(h, "Reopen Task A work sessions after relaunch");
  const sessionsA = await listSessions(h, state.taskA.id);
  if (sessionsA.length !== 2 || !sessionsA.some((item) => item.id === state.sessionA1) || !sessionsA.some((item) => item.id === state.sessionA2)) throw new Error("Task A sessions did not restore after relaunch");
  await selectSession(h, state.sessionA1, state.recordA1);
  const restoredA1 = await h.api<SessionRecord>(`/tasks/${state.taskA.id}/work-sessions/${state.sessionA1}`);
  if (restoredA1.entries.length !== 1 || restoredA1.entries[0]?.body !== state.recordA1 || restoredA1.entries[0]?.attachment?.name !== state.imageName || restoredA1.entries[0]?.attachment?.data !== state.imageData || restoredA1.session.provider !== "codex" || restoredA1.session.model !== "gpt-5.6-luna" || restoredA1.session.approvalMode !== "ask" || restoredA1.session.workspacePath !== "/opaque/project-a-one") throw new Error("Task A session one record or settings did not restore");
  if (!document.querySelector(".task-session-entries")?.textContent?.includes(state.imageName)) throw new Error("Task A session one image was not visible after relaunch");
  if ([...document.querySelectorAll<HTMLElement>(".task-session-entries li")].filter((entry) => entry.getAttribute("data-entry-author") === "user").map((entry) => entry.textContent ?? "").join(" ").includes(state.recordA2)) throw new Error("Task A session two text leaked into session one view");
  await selectSession(h, state.sessionA2, state.recordA2);
  const restoredA2 = await h.api<SessionRecord>(`/tasks/${state.taskA.id}/work-sessions/${state.sessionA2}`);
  if (restoredA2.entries.length !== 1 || restoredA2.entries[0]?.body !== state.recordA2 || restoredA2.session.provider !== "codex" || restoredA2.session.model !== "gpt-5.5" || restoredA2.session.approvalMode !== "ask" || restoredA2.session.workspacePath !== "/opaque/project-a-two") throw new Error("Task A session two record or settings did not restore");
  if ([...document.querySelectorAll<HTMLElement>(".task-session-entries li")].some((entry) => (entry.textContent ?? "").includes(state.recordA1))) throw new Error("Task A session one text leaked into session two view");
  await closeDetail(h);
  await h.detail(state.taskB.title);
  await openSessions(h, "Reopen Task B work sessions after relaunch");
  const sessionsB = await listSessions(h, state.taskB.id);
  if (sessionsB.length !== 1 || sessionsB[0]?.id !== state.sessionB) throw new Error("Task B session did not restore in isolation");
  await selectSession(h, state.sessionB, state.recordB);
  const restoredB = await h.api<SessionRecord>(`/tasks/${state.taskB.id}/work-sessions/${state.sessionB}`);
  if (restoredB.entries.length !== 1 || restoredB.entries[0]?.body !== state.recordB || restoredB.session.provider !== "codex" || restoredB.session.model !== "gpt-5.6-terra" || restoredB.session.approvalMode !== "ask" || restoredB.session.workspacePath !== "/opaque/project-b") throw new Error("Task B record or settings did not restore");
  if ([...document.querySelectorAll<HTMLElement>(".task-session-entries li")].some((entry) => (entry.textContent ?? "").includes(state.recordA1) || (entry.textContent ?? "").includes(state.recordA2))) throw new Error("Task A record text leaked into Task B view");
  const afterA = await h.api<TaskSnapshot>(`/tasks/${state.taskA.id}`);
  const afterB = await h.api<TaskSnapshot>(`/tasks/${state.taskB.id}`);
  if (afterA.state !== state.taskA.state || afterA.taskRevision !== state.taskA.taskRevision || afterB.state !== state.taskB.state || afterB.taskRevision !== state.taskB.taskRevision) throw new Error("Task state changed while using work sessions");
  await h.step("Packaged relaunch restored all three sessions, their records, settings, and image bytes; Task A and Task B remained isolated and unchanged.");
  await h.api("/workbench");
  return steps;
}

export function interactWithObservedControl(h: Pick<WorkbenchScenarioHarness, "coverage">, id: string, action: () => void) {
  // Requests appear asynchronously; observe the current DOM before every action,
  // including newly rendered approval and question controls.
  h.coverage.observe(document, "task-codex-execution", id);
  h.coverage.interact(id, action);
}

export async function interactWithCommittedAnswer(
  h: Pick<WorkbenchScenarioHarness, "coverage" | "waitFor">,
  id: string,
  action: () => void,
  committed: () => boolean,
  label: string,
) {
  interactWithObservedControl(h, id, action);
  // Native events can share one React batch. Wait for React-owned DOM state,
  // not just the input.value assigned by the harness, before the next answer
  // reads its render's state or Submit validates the complete response.
  await h.waitFor(committed, label);
}

/**
 * Controlled packaged acceptance for feature 014.  The fixture exercises the
 * native lifecycle and formal-request UI; it never claims real Codex evidence.
 * Keep this scenario deliberately small because the live two-turn acceptance
 * is a separate gate.
 */
export async function runTaskCodexExecutionScenario(h: WorkbenchScenarioHarness) {
  const titleA = `Codex execution A ${Date.now()}`;
  const titleB = `Codex execution B ${Date.now()}`;
  await h.create(titleA);
  await h.create(titleB);
  const taskA = await h.task(titleA);
  const taskB = await h.task(titleB);
  const taskAStateBefore = await h.api<{ state: string; taskRevision: number }>(`/tasks/${taskA.id}`);

  await h.detail(titleA);
  await openSessions(h, "Open Task A execution sessions");
  await createSession(h, "Create Task A execution session");
  const sessionA = sessionId();
  if (!sessionA) throw new Error("Task A execution session did not become active");
  const executionCwd = h.executionCwd;
  if (!executionCwd) throw new Error("Controlled Codex fixture did not provide an isolated execution folder");
  await saveSettings(h, "gpt-5.6-luna", executionCwd);

  const prepare = document.querySelector<HTMLButtonElement>('[data-control="task-session-prepare"]');
  if (!prepare) throw new Error("Missing Prepare conversation control");
  await h.prepareClick(prepare, "Prepare Task A Codex conversation");
  interactWithObservedControl(h, "task-session-prepare", () => h.click(prepare, "Prepare Task A Codex conversation"));
  await h.waitFor(() => !prepare.disabled && Boolean(document.querySelector(".task-execution-effective")), "prepared Task A Codex conversation");
  h.coverage.assertEffect("task-session-prepare", () => Boolean(document.querySelector(".task-execution-effective")));

  const prepared = await taskExecutionClient.prepare({ taskId: taskA.id, sessionId: sessionA });
  if (!prepared.effectiveConfig?.ready || !prepared.effectiveConfig.settingsRevision) throw new Error("Prepare did not return a ready effective configuration");
  const settingsRevision = prepared.effectiveConfig.settingsRevision;
  const first = await executeInstruction(h, "Run Task A harmless instruction", "Report the controlled fixture result", taskA.id, sessionA, settingsRevision);
  if (first.status !== "succeeded" || !first.finalReport || !first.workLogEntryId) throw new Error("Task A first execution did not produce a report and Work Log link");
  const threadId = first.threadId;
  if (!threadId) throw new Error("Task A first execution did not bind a thread");
  await h.step("Task A prepared one controlled Codex conversation and produced one completed Run with a report and Work Log link.");

  await executeInstruction(h, "Run Task A follow-up", "Confirm this is a follow-up turn on the same conversation", taskA.id, sessionA, settingsRevision);
  const followup = await taskExecutionClient.subscribe({ taskId: taskA.id, sessionId: sessionA });
  const runs = followup.runs.filter((run) => run.sessionId === sessionA);
  if (runs.length < 2 || runs.some((run) => run.status !== "succeeded") || runs[0]?.threadId !== threadId || runs[1]?.threadId !== threadId || !runs[0]?.turnId || !runs[1]?.turnId || runs[0].turnId === runs[1].turnId) throw new Error("Follow-up did not create a distinct completed turn on the exact Task A thread");
  const taskAAfterRuns = await h.api<{ state: string; taskRevision: number }>(`/tasks/${taskA.id}`);
  if (taskAAfterRuns.state !== taskAStateBefore.state || taskAAfterRuns.taskRevision !== taskAStateBefore.taskRevision) throw new Error("Codex execution changed Task A state");
  await h.step("Task A follow-up completed on the exact bound thread and created a separate Run.");

  await closeDetail(h);
  await h.detail(titleB);
  await openSessions(h, "Open isolated Task B execution sessions");
  await createSession(h, "Create Task B execution session");
  const sessionB = sessionId();
  if (!sessionB) throw new Error("Task B execution session did not become active");
  await saveSettings(h, "gpt-5.6-luna", executionCwd);
  const taskBExecution = await taskExecutionClient.subscribe({ taskId: taskB.id, sessionId: sessionB });
  if (taskBExecution.runs.length !== 0) throw new Error("Task A execution state leaked into Task B");
  await h.step("Task B has an isolated session with no Task A Run or thread state.");

  await closeDetail(h);
  await h.detail(titleA);
  await openSessions(h, "Reopen Task A execution session");
  await selectSession(h, sessionA, "Report the controlled fixture result", "run");
  const reopened = await taskExecutionClient.subscribe({ taskId: taskA.id, sessionId: sessionA });
  if (reopened.runs.length < 2 || reopened.runs.some((run) => run.status !== "succeeded")) throw new Error("Task A execution did not restore after detach and reopen");
  const workLog = await h.api<{ workLog?: Array<{ execution?: { runId?: string } }> }>(`/tasks/${taskA.id}`);
  const linkedRuns = workLog.workLog?.map((entry) => entry.execution?.runId).filter(Boolean) ?? [];
  if (linkedRuns.length !== 2 || !linkedRuns.includes(first.id) || !linkedRuns.includes(runs[0]?.id)) throw new Error("Completed executions were not projected exactly once into the existing Work Log");
  await h.step("Detaching and reopening restored Task A’s completed Runs and the existing Work Log execution link.");
}

async function executeInstruction(h: WorkbenchScenarioHarness, label: string, instruction: string, taskId: string, sessionIdValue: string, settingsRevision: string) {
  if (!settingsRevision) throw new Error(`Cannot ${label} without the prepared settings revision`);
  const message = document.querySelector<HTMLTextAreaElement>('[data-control="task-session-message"]');
  if (!message) throw new Error("Missing Codex instruction composer");
  h.enter(message, instruction);
  const run = document.querySelector<HTMLButtonElement>('[data-control="task-session-run"]');
  if (!run) throw new Error("Missing Run with Codex control");
  await h.waitFor(() => !run.disabled, `enabled ${label}`);
  await h.prepareClick(run, label);
  interactWithObservedControl(h, "task-session-run", () => h.click(run, label));
  let latest: Awaited<ReturnType<typeof taskExecutionClient.subscribe>> | undefined;
  const handled = new Set<string>();
  let approvals = 0;
  let questions = 0;
  const responseEffects: Array<{ control: string; requestId: string; questionId?: string; answer?: string; decision?: string }> = [];
  let callbackError: Error | undefined;
  const onSnapshot = (snapshot: Awaited<ReturnType<typeof taskExecutionClient.subscribe>>) => {
    latest = snapshot;
    const active = snapshot.runs.find((value) => value.id === snapshot.activeRunId) ?? snapshot.selectedRun;
    if (!active) return;
    for (const request of active.formalRequests) {
      if (handled.has(request.id) || !["pending", "error"].includes(request.status)) continue;
      handled.add(request.id);
      void (async () => {
        if (request.choices.length) {
          await h.waitFor(() => Boolean(document.querySelector('[data-control="task-session-approval-choice"]')), "rendered provider approval decision");
          const choice = document.querySelector<HTMLButtonElement>('[data-control="task-session-approval-choice"]');
          if (!choice) throw new Error("Missing provider approval decision button");
          await h.prepareClick(choice, "Answer provider approval request");
          interactWithObservedControl(h, "task-session-approval-choice", () => h.click(choice, "Answer provider approval request"));
          responseEffects.push({ control: "task-session-approval-choice", requestId: request.id, decision: request.choices[0].value });
          approvals += 1;
          return;
        }
        await h.waitFor(() => document.querySelectorAll(".task-execution-request fieldset").length >= request.questions.length, "rendered provider questions");
        const renderedFields = [...document.querySelectorAll<HTMLElement>(".task-execution-request fieldset")];
        if (renderedFields.length < request.questions.length) throw new Error("Not every provider question rendered");
        for (const question of request.questions) {
          if (question.isSecret) throw new Error("Controlled fixture unexpectedly requested a secret");
          const text = document.querySelector<HTMLTextAreaElement>(`[data-control="task-session-question-text"][data-question-id="${CSS.escape(question.id)}"]`);
          const option = renderedFields.shift()?.querySelector<HTMLButtonElement>('[data-control="task-session-question-option"]');
          if (option) {
            await h.prepareClick(option, "Choose provider question option");
            await interactWithCommittedAnswer(h, "task-session-question-option", () => h.click(option, "Choose provider question option"),
              () => option.getAttribute("aria-pressed") === "true", "committed provider question option");
            responseEffects.push({ control: "task-session-question-option", requestId: request.id, questionId: question.id, answer: question.options[0].value });
          } else if (text) {
            // React mirrors its committed controlled value to defaultValue;
            // the native input setter used by h.enter changes only value.
            await interactWithCommittedAnswer(h, "task-session-question-text", () => h.enter(text, "controlled fixture answer"),
              () => text.value === "controlled fixture answer" && text.defaultValue === "controlled fixture answer", "committed provider free-text answer");
            responseEffects.push({ control: "task-session-question-text", requestId: request.id, questionId: question.id, answer: "controlled fixture answer" });
          } else throw new Error(`Missing provider question input ${question.id}`);
        }
        const submit = document.querySelector<HTMLButtonElement>('[data-control="task-session-request-submit"]');
        if (!submit) throw new Error("Missing provider question submit button");
        await h.waitFor(() => !submit.disabled, "enabled provider question submit");
        await h.prepareClick(submit, "Submit provider question response");
        interactWithObservedControl(h, "task-session-request-submit", () => h.click(submit, "Submit provider question response"));
        responseEffects.push({ control: "task-session-request-submit", requestId: request.id });
        questions += 1;
      })().catch((error) => { callbackError = error instanceof Error ? error : new Error(String(error)); });
    }
  };
  const initial = await taskExecutionClient.subscribe({ taskId, sessionId: sessionIdValue }, onSnapshot);
  // A request can already be pending when subscription returns; the channel
  // only delivers later changes, so process this snapshot through the same path.
  onSnapshot(initial);
  await h.waitFor(() => {
    if (callbackError) throw callbackError;
    return Boolean(latest?.runs.some((value) => value.instruction === instruction && value.status === "succeeded"));
  }, `completed ${label}`);
  const snapshot = latest!;
  const result = [...snapshot.runs].reverse().find((value) => value.instruction === instruction);
  if (!result) throw new Error(`${label} did not create a Run`);
  if (instruction.includes("Report the controlled") && (approvals !== 1 || questions !== 1)) throw new Error(`Fixture formal request coverage incomplete: approvals=${approvals}, questions=${questions}`);
  for (const control of new Set(responseEffects.map((effect) => effect.control))) {
    h.coverage.assertEffect(control, () => responseEffects.filter((effect) => effect.control === control).every((effect) => {
      const request = result.formalRequests.find((request) => request.id === effect.requestId);
      return request?.status === "answered" && (effect.decision === undefined || request.response?.decision === effect.decision)
        && (effect.questionId === undefined || request.response?.answers?.[effect.questionId]?.answers.includes(effect.answer!));
    }));
  }
  h.coverage.assertEffect("task-session-run", () => Boolean(result.id));
  return result;
}
