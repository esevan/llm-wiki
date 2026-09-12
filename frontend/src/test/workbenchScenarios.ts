import { invoke } from "@tauri-apps/api/core";
import type { InteractionCoverage } from "./interactionCoverage";

export type WorkbenchTask = {
  id: string;
  taskRevision: number;
  state: string;
  title: string;
  detail?: string;
  outcome?: string;
  scope?: string;
  nonGoals?: string;
  validationCriteria?: string;
  workLog?: Array<{ id: string; body?: string; comments?: Array<{ body: string }> }>;
  checklist?: Array<{ id: string; body: string; checked: boolean }>;
  decisions?: Array<{ body?: string }>;
  readinessEntries?: Array<{ key: string; status: string; reason?: string }>;
  problemLinks?: Array<{ id: string; problemId: string; problemRevision: number }>;
  relationships?: Array<{ id: string; targetTaskId: string; kind: string }>;
  completion?: { evidence?: string };
  publication?: { state?: string; draftRevision?: number };
};

export type WorkbenchScenarioHarness = {
  coverage: InteractionCoverage;
  create: (title: string, kind?: "capture" | "task") => Promise<void>;
  detail: (title: string) => Promise<void>;
  task: (title: string) => Promise<WorkbenchTask>;
  api: <T>(path: string, method?: "GET" | "POST" | "PUT" | "DELETE", body?: Record<string, unknown>) => Promise<T>;
  waitFor: (check: () => boolean, label: string) => Promise<void>;
  waitForAsync: (check: () => Promise<boolean>, label: string) => Promise<void>;
  click: (element: HTMLElement, label: string) => void;
  enter: (element: HTMLInputElement | HTMLTextAreaElement, value: string) => void;
  step: (message: string) => Promise<void>;
};

function observe(h: WorkbenchScenarioHarness, state: string) {
  h.coverage.observe(document, "task-controls", state);
}

async function clickControl(h: WorkbenchScenarioHarness, name: string, label: string, recordId?: string) {
  await h.waitFor(
    () => document.querySelector(".task-detail")?.getAttribute("aria-busy") !== "true",
    `idle Task detail before ${label}`,
  );
  h.coverage.interact(name, () => h.click(control(name, recordId), label));
}

function enterControl(h: WorkbenchScenarioHarness, name: string, value: string, recordId?: string) {
  const element = control<HTMLInputElement | HTMLTextAreaElement>(name, recordId);
  h.coverage.interact(name, () => h.enter(element, value));
  h.coverage.assertEffect(name, () => element.value === value);
}

function effect(h: WorkbenchScenarioHarness, ids: string[], assertion: () => boolean) {
  for (const id of ids) h.coverage.assertEffect(id, assertion);
}

function control<T extends HTMLElement>(name: string, recordId?: string): T {
  const suffix = recordId ? `[data-record-id="${CSS.escape(recordId)}"]` : "";
  const element = document.querySelector<T>(`[data-control="${name}"]${suffix}`);
  if (!element) throw new Error(`Missing rendered control ${name}${recordId ? ` for ${recordId}` : ""}`);
  return element;
}

function assertDisabled(name: string, expected: boolean) {
  const element = control<HTMLButtonElement>(name);
  if (element.disabled !== expected)
    throw new Error(`${name} expected disabled=${expected}, received ${element.disabled}`);
}

async function waitForTaskRevision(h: WorkbenchScenarioHarness, title: string, previous: number) {
  await h.waitForAsync(async () => {
    try {
      return (await h.task(title)).taskRevision > previous;
    } catch {
      return false;
    }
  }, `${title} revision after UI save`);
}

/** Drives each current Workbench/Task-detail control through a semantic readback. */
export async function runTaskControlMatrixScenario(h: WorkbenchScenarioHarness) {
  const title = `control matrix ${Date.now()}`;
  observe(h, "blank Task entry");
  assertDisabled("task-entry-save", true);
  await clickControl(h, "task-entry-capture-mode", "Select Capture entry mode");
  effect(h, ["task-entry-capture-mode"], () => control<HTMLInputElement>("task-entry-capture-mode").checked);
  await clickControl(h, "task-entry-task-mode", "Select Task entry mode");
  effect(h, ["task-entry-task-mode"], () => control<HTMLInputElement>("task-entry-task-mode").checked);
  enterControl(h, "task-entry-text", title);
  assertDisabled("task-entry-save", false);
  const form = control("task-entry-save").closest("form");
  if (!form) throw new Error("Task entry form missing");
  h.coverage.interact("task-entry-save", () => form.requestSubmit());
  await h.waitForAsync(async () => {
    try {
      return (await h.task(title)).title === title;
    } catch {
      return false;
    }
  }, "keyboard-submitted Task");
  await h.waitFor(
    () => [...document.querySelectorAll(".canonical-card")].some(card => card.textContent?.includes(title)),
    "submitted Task in canonical projection",
  );
  effect(h, ["task-entry-save"], () => document.body.textContent?.includes(title) === true);

  const targetTitle = `${title} target`;
  await h.create(targetTitle);
  observe(h, "created Task cards");
  const taskCard = [...document.querySelectorAll<HTMLElement>(".canonical-card")].find(
    card => card.querySelector("h3")?.textContent === title,
  );
  const taskOpen = taskCard?.querySelector<HTMLElement>('[data-control="task-card-open"]');
  if (!taskOpen) throw new Error("Created Task open control missing");
  h.coverage.interact("task-card-open", () => h.click(taskOpen, `Open ${title}`));
  await h.waitFor(() => document.querySelector(".task-detail")?.textContent?.includes(title) ?? false, "Task detail");
  effect(h, ["task-card-open"], () => document.querySelector(".task-detail")?.textContent?.includes(title) ?? false);
  observe(h, "Task detail");
  const before = await h.task(title);
  const initialReadiness = before.readinessEntries?.find(item => item.status === "missing");
  if (!initialReadiness) throw new Error("New Task did not expose a missing readiness field");
  enterControl(h, "task-readiness-reason", "Not needed for this bounded Task", initialReadiness.key);
  await clickControl(h, "task-readiness-not-applicable", "Mark readiness field not applicable", initialReadiness.key);
  await h.waitForAsync(async () => (await h.task(title)).readinessEntries?.find(item => item.key === initialReadiness.key)?.status === "not_applicable", "initial readiness decision readback");
  const readinessDecision = (await h.task(title)).readinessEntries?.find(item => item.key === initialReadiness.key);
  effect(h, ["task-readiness-not-applicable"], () => readinessDecision?.status === "not_applicable" && readinessDecision.reason === "Not needed for this bounded Task");
  const revisions: Array<[string, string]> = [
    ["task-revision-title", `${title} revised`],
    ["task-revision-detail", "Detailed current behavior"],
    ["task-revision-outcome", "Every control has an effect"],
    ["task-revision-scope", "Task Workbench controls"],
    ["task-revision-non-goals", "Architecture refactor"],
    ["task-revision-criteria", "Native readback matches"],
  ];
  for (const [name, value] of revisions) enterControl(h, name, value);
  await clickControl(h, "task-revision-save", "Save every Task revision field");
  await waitForTaskRevision(h, `${title} revised`, before.taskRevision);
  const revisedTitle = `${title} revised`;
  const revised = await h.task(revisedTitle);
  if (revised.detail !== revisions[1][1] || revised.validationCriteria !== revisions[5][1])
    throw new Error("Task revision fields did not persist together");
  effect(h, ["task-revision-save"], () => revised.detail === revisions[1][1] && revised.validationCriteria === revisions[5][1]);

  await clickControl(h, "conflict-review-run", "Run Conflict Review");
  await h.waitFor(() => Boolean(document.querySelector('[data-control="conflict-review-cancel"]')), "running Conflict Review");
  effect(h, ["conflict-review-run"], () => document.querySelector(".review-panel")?.getAttribute("aria-busy") === "true");
  observe(h, "running Conflict Review");
  await clickControl(h, "conflict-review-cancel", "Cancel Conflict Review");
  await h.waitFor(() => Boolean(document.querySelector('[data-control="conflict-review-retry"]')), "cancelled Conflict Review");
  effect(h, ["conflict-review-cancel"], () => document.body.textContent?.toLowerCase().includes("cancel") === true);
  observe(h, "cancelled Conflict Review");
  await clickControl(h, "conflict-review-retry", "Retry Conflict Review");
  await h.waitFor(() => Boolean(document.querySelector('[data-review-status="clear"],[data-review-status="findings"],[data-review-status="insufficient_evidence"]')), "retried Conflict Review result");
  effect(h, ["conflict-review-retry"], () => Boolean(document.querySelector(".review-current")));

  assertDisabled("task-worklog-add", true);
  enterControl(h, "task-worklog-text", "Matrix work evidence");
  const file = control<HTMLInputElement>("task-worklog-file");
  const transfer = new DataTransfer();
  transfer.items.add(new File(["matrix"], "matrix.txt", { type: "text/plain" }));
  Object.defineProperty(file, "files", { configurable: true, value: transfer.files });
  h.coverage.interact("task-worklog-file", () => file.dispatchEvent(new Event("change", { bubbles: true })));
  h.coverage.assertEffect("task-worklog-file", () => file.files?.[0]?.name === "matrix.txt");
  await clickControl(h, "task-worklog-add", "Add Work Log text and file");
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).workLog?.some(item => item.body === "Matrix work evidence")), "Work Log readback");
  const withLog = await h.task(revisedTitle);
  const log = withLog.workLog?.find(item => item.body === "Matrix work evidence");
  if (!log) throw new Error("Work Log entry missing");
  effect(h, ["task-worklog-add"], () => Boolean(log));
  observe(h, "Work Log row");
  enterControl(h, "task-comment-text", "Matrix comment", log.id);
  await clickControl(h, "task-comment-add", "Add Work Log comment", log.id);
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).workLog?.find(item => item.id === log.id)?.comments?.some(item => item.body === "Matrix comment")), "comment readback");
  await h.waitFor(() => document.body.textContent?.includes("Matrix comment") === true, "rendered Work Log comment");
  effect(h, ["task-comment-add"], () => document.body.textContent?.includes("Matrix comment") === true);

  assertDisabled("task-checklist-add", true);
  enterControl(h, "task-checklist-text", "Matrix checklist");
  await clickControl(h, "task-checklist-add", "Add checklist item");
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).checklist?.some(item => item.body === "Matrix checklist")), "checklist readback");
  const checklist = (await h.task(revisedTitle)).checklist?.find(item => item.body === "Matrix checklist");
  if (!checklist) throw new Error("Checklist item missing");
  effect(h, ["task-checklist-add"], () => Boolean(checklist));
  observe(h, "checklist row");
  await clickControl(h, "task-checklist-toggle", "Toggle checklist item", checklist.id);
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).checklist?.find(item => item.id === checklist.id)?.checked), "checked checklist readback");
  await h.waitFor(() => control<HTMLInputElement>("task-checklist-toggle", checklist.id).checked, "rendered checked checklist state");
  effect(h, ["task-checklist-toggle"], () => control<HTMLInputElement>("task-checklist-toggle", checklist.id).checked);

  assertDisabled("task-decision-add", true);
  enterControl(h, "task-decision-text", "Matrix decision");
  await clickControl(h, "task-decision-add", "Add decision");
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).decisions?.some(item => item.body === "Matrix decision")), "decision readback");
  await h.waitFor(() => document.body.textContent?.includes("Matrix decision") === true, "rendered decision row");
  effect(h, ["task-decision-add"], () => document.body.textContent?.includes("Matrix decision") === true);

  const readiness = (await h.task(revisedTitle)).readinessEntries?.find(item => item.status === "missing");
  if (readiness) {
    observe(h, "missing readiness field");
    enterControl(h, "task-readiness-reason", "Not needed for this bounded Task", readiness.key);
    await clickControl(h, "task-readiness-not-applicable", "Mark readiness field not applicable", readiness.key);
    await h.waitForAsync(async () => (await h.task(revisedTitle)).readinessEntries?.find(item => item.key === readiness.key)?.status === "not_applicable", "readiness decision readback");
    await h.waitFor(() => document.body.textContent?.includes("Not needed for this bounded Task") === true, "rendered readiness decision");
    effect(h, ["task-readiness-not-applicable"], () => document.body.textContent?.includes("Not needed for this bounded Task") === true);
  }

  const connections = control<HTMLDetailsElement>("task-connection-details");
  const connectionSummary = connections.querySelector<HTMLElement>("summary");
  if (!connectionSummary) throw new Error("Task connection details summary missing");
  h.coverage.interact("task-connection-details", () => h.click(connectionSummary, "Open connection details"));
  await h.waitFor(() => connections.open, "open Task connection details");
  effect(h, ["task-connection-details"], () => connections.open);
  observe(h, "connection editors");
  enterControl(h, "task-problem-create-text", "Matrix linked Problem");
  await clickControl(h, "task-problem-create", "Create and link Problem");
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).problemLinks?.length), "Problem link readback");
  const problemOne = (await h.task(revisedTitle)).problemLinks?.[0];
  if (!problemOne) throw new Error("Created Problem link missing");
  effect(h, ["task-problem-create"], () => Boolean(problemOne));
  observe(h, "linked Problem row");
  enterControl(h, "task-problem-revision-text", "Matrix linked Problem revision two");
  await clickControl(h, "task-problem-revise", "Revise linked Problem");
  await h.waitFor(() => control<HTMLInputElement>("task-problem-link-revision").value === "2", "Problem revision editor update");
  effect(h, ["task-problem-revise"], () => control<HTMLInputElement>("task-problem-link-revision").value === "2");
  enterControl(h, "task-problem-link-id", problemOne.problemId);
  enterControl(h, "task-problem-link-revision", "2");
  await clickControl(h, "task-problem-link", "Link exact Problem revision two");
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).problemLinks?.some(link => link.problemRevision === 2)), "Problem r2 link readback");
  await h.waitFor(() => document.body.textContent?.includes("revision 2") === true, "rendered Problem r2 link");
  effect(h, ["task-problem-link"], () => document.body.textContent?.includes("revision 2") === true);

  const target = await h.task(targetTitle);
  enterControl(h, "task-relationship-target", target.id);
  const kind = control<HTMLSelectElement>("task-relationship-kind");
  for (const relationship of ["related", "prerequisite", "split_from"]) {
    if (relationship === "related") {
      h.coverage.interact("task-relationship-kind", () => {
        kind.value = relationship;
        kind.dispatchEvent(new Event("change", { bubbles: true }));
      });
      h.coverage.assertEffect("task-relationship-kind", () => kind.value === relationship);
    } else {
      kind.value = relationship;
      kind.dispatchEvent(new Event("change", { bubbles: true }));
    }
    if (relationship === "related") await clickControl(h, "task-relationship-link", `Link ${relationship} Task`);
    else {
      await h.waitFor(() => document.querySelector(".task-detail")?.getAttribute("aria-busy") !== "true", `idle Task detail before linking ${relationship}`);
      h.click(control("task-relationship-link"), `Link ${relationship} Task`);
    }
    await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).relationships?.some(link => link.kind === relationship && link.targetTaskId === target.id)), `${relationship} relationship readback`);
  }
  await h.waitFor(() => document.body.textContent?.includes(targetTitle) === true, "rendered Task relationship");
  effect(h, ["task-relationship-link"], () => document.body.textContent?.includes(targetTitle) === true);
  const relation = (await h.task(revisedTitle)).relationships?.find(link => link.kind === "related");
  if (!relation) throw new Error("Related relationship missing");
  observe(h, "Task relationship rows");
  await clickControl(h, "task-relationship-unlink", "Unlink related Task", relation.id);
  await h.waitForAsync(async () => !(await h.task(revisedTitle)).relationships?.some(link => link.id === relation.id), "relationship unlink readback");
  await h.waitFor(() => !document.querySelector(`[data-record-id="${CSS.escape(relation.id)}"]`), "removed Task relationship row");
  effect(h, ["task-relationship-unlink"], () => !document.querySelector(`[data-record-id="${CSS.escape(relation.id)}"]`));

  const resolvableLink = (await h.task(revisedTitle)).problemLinks?.find(link => link.problemId === problemOne.problemId && link.problemRevision === 2);
  if (!resolvableLink) throw new Error("Current Problem revision link missing before resolution");
  await clickControl(h, "task-problem-resolve", "Resolve exact linked Problem", resolvableLink.id);
  let resolvedProblem = false;
  await h.waitForAsync(async () => {
    const record = await h.api<{ state?: string }>(`/problems/${encodeURIComponent(problemOne.problemId)}/record`);
    resolvedProblem = record.state === "resolved";
    return resolvedProblem;
  }, "Problem resolution readback");
  effect(h, ["task-problem-resolve"], () => resolvedProblem);
  const secondLink = (await h.task(revisedTitle)).problemLinks?.find(link => link.problemRevision === 2);
  if (secondLink) {
    await clickControl(h, "task-problem-unlink", "Unlink Problem revision two", secondLink.id);
    await h.waitForAsync(async () => !(await h.task(revisedTitle)).problemLinks?.some(link => link.id === secondLink.id), "Problem unlink readback");
    await h.waitFor(() => !document.querySelector(`[data-record-id="${CSS.escape(secondLink.id)}"]`), "removed Problem link row");
    effect(h, ["task-problem-unlink"], () => !document.querySelector(`[data-record-id="${CSS.escape(secondLink.id)}"]`));
  }

  await clickControl(h, "task-transition-start", "Start Task");
  await h.waitFor(() => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress", "in-progress UI");
  effect(h, ["task-transition-start"], () => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress");
  await clickControl(h, "task-detail-close", "Close active Task detail");
  await h.waitFor(() => !document.querySelector(".task-detail"), "closed active Task detail");
  effect(h, ["task-detail-close"], () => !document.querySelector(".task-detail"));
  observe(h, "active Task shortcut");
  await clickControl(h, "task-shortcut-open", "Resume active Task");
  await h.waitFor(() => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress", "resumed active Task detail");
  effect(h, ["task-shortcut-open"], () => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress");
  observe(h, "in-progress Task");
  await clickControl(h, "task-transition-complete-focus", "Focus completion evidence");
  if (document.activeElement !== control("task-completion-evidence")) throw new Error("Complete shortcut did not focus evidence");
  effect(h, ["task-transition-complete-focus"], () => document.activeElement === control("task-completion-evidence"));
  assertDisabled("task-completion-complete", true);
  enterControl(h, "task-completion-evidence", "Matrix completion evidence");
  assertDisabled("task-completion-complete", false);
  await clickControl(h, "task-completion-complete", "Complete Task with evidence");
  await h.waitForAsync(async () => (await h.task(revisedTitle)).state === "completed", "Task completion readback");
  await h.waitFor(() => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "completed", "rendered completed Task");
  effect(h, ["task-completion-complete"], () => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "completed");

  observe(h, "completed Task");
  await clickControl(h, "task-lineage-load", "Load Task lineage");
  await h.waitFor(() => Boolean(document.querySelector(".lineage-flow li")), "lineage nodes");
  effect(h, ["task-lineage-load"], () => Boolean(document.querySelector(".lineage-flow li")));
  await clickControl(h, "task-knowledge-draft", "Create Knowledge draft");
  await h.waitFor(() => Boolean(document.querySelector(".knowledge-draft")), "Knowledge draft");
  effect(h, ["task-knowledge-draft"], () => Boolean(document.querySelector(".knowledge-draft")));
  observe(h, "Knowledge draft");
  const body = control<HTMLTextAreaElement>("task-knowledge-draft-body");
  const initialHash = document.querySelector(".knowledge-draft")?.getAttribute("data-content-hash");
  const correctedBody = `${body.value}\n\nMatrix correction.`;
  h.coverage.interact("task-knowledge-draft-body", () => h.enter(body, correctedBody));
  h.coverage.assertEffect("task-knowledge-draft-body", () => body.value === correctedBody);
  await clickControl(h, "task-knowledge-correct", "Correct Knowledge draft");
  await h.waitFor(() => !control<HTMLButtonElement>("task-knowledge-correct").disabled && document.querySelector(".knowledge-draft")?.getAttribute("data-content-hash") !== initialHash, "Knowledge correction readback");
  effect(h, ["task-knowledge-correct"], () => document.querySelector(".knowledge-draft")?.getAttribute("data-content-hash") !== initialHash);
  await clickControl(h, "task-knowledge-publish", "Publish Knowledge draft");
  await h.waitFor(() => document.querySelector(".publication-controls")?.getAttribute("data-publication-state") === "published", "published Knowledge");
  effect(h, ["task-knowledge-publish"], () => document.querySelector(".publication-controls")?.getAttribute("data-publication-state") === "published");
  observe(h, "published Knowledge");
  await clickControl(h, "task-knowledge-regenerate", "Regenerate Knowledge draft");
  await h.waitFor(() => Boolean(document.querySelector(".knowledge-draft")), "regenerated Knowledge draft");
  effect(h, ["task-knowledge-regenerate"], () => Boolean(document.querySelector(".knowledge-draft")));
  observe(h, "regenerated Knowledge draft");
  const regeneratedPublish = document.querySelector<HTMLButtonElement>('.knowledge-draft [data-control="task-knowledge-publish"]');
  if (!regeneratedPublish) throw new Error("Regenerated Knowledge publish control missing");
  h.click(regeneratedPublish, "Publish regenerated Knowledge");
  await h.waitFor(() => !document.querySelector(".knowledge-draft"), "republished Knowledge");
  await clickControl(h, "task-knowledge-withdraw", "Withdraw Knowledge");
  await h.waitForAsync(async () => (await h.task(revisedTitle)).publication?.state === "withdrawn", "withdrawn Knowledge readback");
  await h.waitFor(() => document.querySelector(".publication-controls")?.getAttribute("data-publication-state") === "withdrawn", "rendered withdrawn Knowledge");
  effect(h, ["task-knowledge-withdraw"], () => document.querySelector(".publication-controls")?.getAttribute("data-publication-state") === "withdrawn");
  await clickControl(h, "task-transition-reopen", "Reopen completed Task");
  await h.waitForAsync(async () => (await h.task(revisedTitle)).state === "in_progress", "reopened Task readback");
  await h.waitFor(() => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress", "rendered reopened Task");
  effect(h, ["task-transition-reopen"], () => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress");
  await clickControl(h, "task-detail-refine", "Open Task refinement from detail");
  await h.waitFor(() => Boolean(document.querySelector(".refinement-panel")), "Task refinement panel");
  effect(h, ["task-detail-refine"], () => Boolean(document.querySelector(".refinement-panel")));
  h.click(control("refinement-close"), "Close Task refinement");
  await h.waitFor(() => Boolean(document.querySelector(".task-detail")), "restored Task detail");
  observe(h, "restored Task detail");
  await clickControl(h, "task-detail-close", "Close Task detail");
  await h.waitFor(() => !document.querySelector(".task-detail"), "closed Task detail");
  effect(h, ["task-detail-close"], () => !document.querySelector(".task-detail"));
  // Active Tasks intentionally have only the active shortcut. A separate
  // Capture creates the saved refinement shortcut at its valid precondition.
  const shortcutCapture = `${title} saved refinement Capture`;
  await h.create(shortcutCapture, "capture");
  const captureCard = [...document.querySelectorAll<HTMLElement>(".canonical-card")].find(card => card.textContent?.includes(shortcutCapture));
  const captureRefine = captureCard?.querySelector<HTMLElement>('[data-control="task-card-refine"]');
  if (!captureRefine) throw new Error("Saved shortcut Capture refinement control missing");
  h.click(captureRefine, "Create saved Capture refinement");
  await h.waitFor(() => Boolean(document.querySelector('.refinement-panel[data-refinement-session]:not([data-refinement-session=""])')), "loaded saved Capture refinement session");
  h.click(control("refinement-close"), "Save Capture refinement shortcut");
  await h.waitFor(() => !document.querySelector(".refinement-panel") && Boolean(document.querySelector('[data-control="task-shortcut-refine"]')), "rendered saved refinement shortcut");
  observe(h, "saved refinement shortcut");
  await clickControl(h, "task-shortcut-refine", "Resume saved Task refinement");
  await h.waitFor(() => Boolean(document.querySelector(".refinement-panel")), "resumed Task refinement");
  effect(h, ["task-shortcut-refine"], () => Boolean(document.querySelector(".refinement-panel")));
  h.click(control("refinement-close"), "Close resumed Task refinement");
  await h.step("F03–F30 current Workbench controls were enabled only at valid preconditions and produced native readback or an independent visible effect.");
}

/** Proves a migrated Problem-only bridge is visible and enters the wired legacy refinement surface without inventing a Task. */
export async function runLegacyProblemRefinementScenario(h: WorkbenchScenarioHarness) {
  const statement = `Migrated Problem-only ${Date.now()}`;
  const seeded = await invoke<{ problemId: string; itemId: string }>(
    "desktop_e2e_seed_legacy_refinement",
    { statement },
  );
  if (!seeded) throw new Error("Legacy refinement fixture was unavailable outside isolated desktop E2E mode");
  window.dispatchEvent(new CustomEvent("llm-wiki:task-workbench-refresh"));
  await h.waitFor(() => [...document.querySelectorAll(".canonical-card")].some(card => card.textContent?.includes(statement)), "migrated Problem-only refinement card");
  const snapshot = await h.api<{ categories: Array<{ items: Array<{ kind: string; problemId?: string }> }> }>("/workbench");
  const item = snapshot.categories.flatMap(group => group.items).find(candidate => candidate.kind === "refinement" && candidate.problemId === seeded.problemId);
  if (!item) throw new Error("Migrated Problem-only item was not discoverable");
  if (snapshot.categories.flatMap(group => group.items).some(candidate => candidate.kind === "task" && (candidate as { problemId?: string }).problemId === seeded.problemId))
    throw new Error("Problem-only migration invented a Task");
  const card = [...document.querySelectorAll<HTMLElement>(".canonical-card")].find(candidate => candidate.textContent?.includes(statement));
  const refine = card?.querySelector<HTMLElement>('[data-control="task-card-refine"]');
  if (!refine) throw new Error("Migrated Problem refinement control missing");
  observe(h, "migrated Problem card");
  h.coverage.interact("task-card-refine", () => h.click(refine, "Refine migrated Problem-only item"));
  await h.waitFor(() => (document.getElementById("chat-modal") as HTMLDialogElement | null)?.open === true, "Problem refinement dialog");
  if (!document.getElementById("chat-title")?.textContent?.includes("Problem")) throw new Error("Problem refinement opened the wrong target");
  effect(h, ["task-card-refine"], () => (document.getElementById("chat-modal") as HTMLDialogElement | null)?.open === true);
  await h.step("Migrated Problem-only history stayed discoverable as a refinement item, opened the wired Problem context, and created no Task.");
}

/** A real one-shot native read failure must leave the rendered Workbench retry usable. */
export async function runWorkbenchRetryScenario(h: WorkbenchScenarioHarness) {
  await invoke("desktop_e2e_arm_one_shot_failure", { operation: "workbench.get" });
  window.dispatchEvent(new CustomEvent("llm-wiki:task-workbench-refresh"));
  await h.waitFor(() => Boolean(document.querySelector('[data-control="task-workbench-retry"]')), "Workbench retry after native failure");
  observe(h, "failed Workbench projection");
  await clickControl(h, "task-workbench-retry", "Retry Workbench projection");
  await h.waitFor(() => !document.querySelector('[data-control="task-workbench-retry"]') && Boolean(document.querySelector('[data-task-workbench="true"]')), "recovered Workbench projection");
  effect(h, ["task-workbench-retry"], () => !document.querySelector('[data-control="task-workbench-retry"]'));
  await h.step("A one-shot native Workbench read failure rendered a retry action, and that action restored the canonical projection.");
}
