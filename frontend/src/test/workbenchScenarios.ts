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
  prepareClick: (element: HTMLElement, label: string) => Promise<void>;
  enter: (element: HTMLInputElement | HTMLTextAreaElement, value: string) => void;
  step: (message: string) => Promise<void>;
  executionCwd?: string;
};

function observe(h: WorkbenchScenarioHarness, state: string) {
  h.coverage.observe(document, "task-controls", state);
}

async function clickControl(h: WorkbenchScenarioHarness, name: string, label: string, recordId?: string) {
  const suffix = recordId ? `[data-record-id="${CSS.escape(recordId)}"]` : "";
  await h.waitFor(() => Boolean(document.querySelector(`[data-control="${name}"]${suffix}`)), `rendered ${label}`);
  await h.waitFor(
    () => document.querySelector(".task-detail")?.getAttribute("aria-busy") !== "true",
    `idle Task detail before ${label}`,
  );
  await prepareTaskControl(h, name);
  await h.waitFor(() => !control(name, recordId).matches(":disabled"), `enabled ${label}`);
  await h.prepareClick(control(name, recordId), label);
  h.coverage.interact(name, () => h.click(control(name, recordId), label));
}

async function enterControl(h: WorkbenchScenarioHarness, name: string, value: string, recordId?: string) {
  await h.waitFor(
    () => document.querySelector(".task-detail")?.getAttribute("aria-busy") !== "true",
    `idle Task detail before editing ${name}`,
  );
  await prepareTaskControl(h, name);
  const element = control<HTMLInputElement | HTMLTextAreaElement>(name, recordId);
  const edit = () => h.coverage.interact(name, () => h.enter(element, value));
  if (name.startsWith("task-revision-") && name !== "task-revision-save") {
    h.coverage.interact("task-revision-dynamic-control", edit);
    h.coverage.assertEffect("task-revision-dynamic-control", () => element.value === value);
  } else edit();
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

async function selectTaskTab(h: WorkbenchScenarioHarness, name: "Work" | "Details" | "Review", label: string) {
  const key = name.toLowerCase();
  const tabs = [...document.querySelectorAll<HTMLButtonElement>('.task-detail [role="tab"], .task-detail button[data-control*="tab"]')];
  const tab = tabs.find((candidate) => {
    const controlName = candidate.getAttribute("data-control")?.toLowerCase() ?? "";
    const text = (candidate.getAttribute("aria-label") ?? candidate.textContent ?? "").trim().toLowerCase();
    return controlName.includes(key) || text === key || text.includes(`${key} tab`);
  });
  if (!tab) throw new Error(`Missing visible Task ${name} tab`);
  observe(h, "Task detail tabs");
  await h.prepareClick(tab, label);
  h.coverage.interact(`task-detail-tab-${key}`, () => h.click(tab, label));
  await h.waitFor(() => tab.getAttribute("aria-selected") === "true" || tab.getAttribute("aria-pressed") === "true", `selected Task ${name} tab`);
  h.coverage.assertEffect(`task-detail-tab-${key}`, () => tab.getAttribute("aria-selected") === "true");
}
async function ensureTaskEditing(h: WorkbenchScenarioHarness, label: string) {
  const edit = document.querySelector<HTMLButtonElement>('[data-control="task-definition-edit"]');
  if (!edit || edit.hidden) return;
  observe(h, "read-first Task details");
  await h.prepareClick(edit, label);
  h.coverage.interact("task-definition-edit", () => h.click(edit, label));
  await h.waitFor(() => Boolean(document.querySelector('[data-control="task-revision-detail"]')), "editable Task details");
  observe(h, "Task definition fields");
  effect(h, ["task-definition-edit"], () => Boolean(document.querySelector('[data-control="task-revision-detail"]')));
}

async function prepareTaskControl(h: WorkbenchScenarioHarness, name: string) {
  if (name.startsWith("task-revision-") && name !== "task-revision-save") {
    await selectTaskTab(h, "Details", "Open definition editor");
    await ensureTaskEditing(h, "Edit Task definition");
  }
  const element = document.querySelector<HTMLElement>(`[data-control="${name}"]`);
  const panel = element?.closest<HTMLElement>(".task-tab-panel");
  if (panel?.hidden) {
    await selectTaskTab(h, panel.dataset.taskTab === "work" ? "Work" : panel.dataset.taskTab === "review" ? "Review" : "Details", `Open section for ${name}`);
  }
  const parents: HTMLDetailsElement[] = [];
  for (let parent = element?.parentElement; parent; parent = parent.parentElement) {
    if (parent instanceof HTMLDetailsElement && !parent.open) parents.unshift(parent);
  }
  for (const parent of parents) {
    const summary = parent.querySelector<HTMLElement>(":scope > summary");
    if (!summary) continue;
    const id = parent.dataset.control;
    observe(h, "collapsed Task section");
    await h.prepareClick(summary, `Expand section for ${name}`);
    if (id) h.coverage.interact(id, () => h.click(summary, `Expand ${id}`));
    else h.click(summary, `Expand section for ${name}`);
    await h.waitFor(() => parent.open, "expanded Task section");
    observe(h, "expanded Task section");
    if (id) effect(h, [id], () => parent.open);
  }
  observe(h, "prepared Task controls");
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
  const imageFile = control<HTMLInputElement>("input-image-file");
  let pickerRequested = false;
  // The real chooser delegates to this input; replace only the OS file selection.
  imageFile.addEventListener("click", event => { pickerRequested = true; event.preventDefault(); }, { once: true });
  await clickControl(h, "input-image-choose", "Choose a Capture image");
  effect(h, ["input-image-choose"], () => pickerRequested);
  const imageFiles = new DataTransfer();
  const png = Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII="), char => char.charCodeAt(0));
  imageFiles.items.add(new File([png], "capture-evidence.png", { type: "image/png" }));
  h.coverage.interact("input-image-file", () => {
    Object.defineProperty(imageFile, "files", { configurable: true, value: imageFiles.files });
    imageFile.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await h.waitFor(() => {
    const preview = document.querySelector<HTMLImageElement>('.input-image-preview img[alt="capture-evidence.png"]');
    return Boolean(preview?.complete && preview.naturalWidth > 0) && !control<HTMLButtonElement>("task-entry-save").disabled;
  }, "selected Capture image preview and image-only save");
  effect(h, ["input-image-file"], () => Boolean(document.querySelector('.input-image-preview img[alt="capture-evidence.png"]')) && !control<HTMLButtonElement>("task-entry-save").disabled);
  observe(h, "selected Capture image");
  await clickControl(h, "input-image-remove", "Remove unsent Capture image");
  await h.waitFor(() => !document.querySelector(".input-image-preview") && control<HTMLButtonElement>("task-entry-save").disabled, "removed Capture image");
  effect(h, ["input-image-remove"], () => !document.querySelector(".input-image-preview") && control<HTMLButtonElement>("task-entry-save").disabled);

  await clickControl(h, "task-entry-task-mode", "Select Task entry mode");
  effect(h, ["task-entry-task-mode"], () => control<HTMLInputElement>("task-entry-task-mode").checked);
  await enterControl(h, "task-entry-text", title);
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
  await selectTaskTab(h, "Details", "Open Task Details tab");
  await ensureTaskEditing(h, "Edit Task details");
  observe(h, "definition editing");
  await clickControl(h, "task-definition-cancel", "Cancel definition editing");
  effect(h, ["task-definition-cancel"], () => !document.querySelector('[data-control="task-revision-detail"]'));
  await ensureTaskEditing(h, "Re-enter definition editing");
  await enterControl(h, "task-revision-detail", "Keep this draft while comparing another Task");
  await selectTaskTab(h, "Work", "Open Task Work tab");
  await enterControl(h, "task-worklog-text", "Independent Work Log while Task draft is dirty");
  await clickControl(h, "task-worklog-add", "Add Work Log without losing Task draft");
  await h.waitForAsync(async () => Boolean((await h.task(title)).workLog?.some(item => item.body === "Independent Work Log while Task draft is dirty")), "independent Work Log readback beside dirty Task draft");
  // Native persistence can finish before the component's queued aggregate GET.
  // Switching while that GET is pending is deliberately ignored by the guard.
  await h.waitFor(() => document.querySelector(".task-detail")?.getAttribute("aria-busy") === "false" &&
    Boolean(document.querySelector(".task-detail .log-entry")?.textContent?.includes("Independent Work Log while Task draft is dirty")),
  "rendered independent Work Log and idle Task detail");
  effect(h, ["task-worklog-add"], () => document.querySelector(".task-detail")?.getAttribute("aria-busy") === "false" &&
    document.querySelector(".task-detail .log-entry")?.textContent?.includes("Independent Work Log while Task draft is dirty") === true);
  await selectTaskTab(h, "Details", "Return to Task Details tab");
  if (control<HTMLTextAreaElement>("task-revision-detail").value !== "Keep this draft while comparing another Task")
    throw new Error("Work Log refresh lost the dirty Task definition");
  await clickControl(h, "task-detail-close", "Request close while the Task draft is dirty");
  await h.waitFor(() => Boolean(document.querySelector(".task-draft-guard")), "Task close leave guard");
  observe(h, "Task close leave guard");
  await clickControl(h, "task-draft-guard-keep-editing", "Keep editing the current Task draft");
  effect(h, ["task-draft-guard-keep-editing"], () => document.querySelector(".task-detail")?.textContent?.includes(title) === true && !document.querySelector(".task-draft-guard"));
  await clickControl(h, "task-detail-close", "Request close for dirty Task");
  await h.waitFor(() => Boolean(document.querySelector(".task-draft-guard")), "Task close leave guard");
  observe(h, "Task close leave guard");
  await clickControl(h, "task-draft-guard-discard", "Discard draft and close Task");
  await h.waitFor(() => !document.querySelector(".task-detail"), "discarded Task draft close");
  effect(h, ["task-draft-guard-discard", "task-detail-close"], () => !document.querySelector(".task-detail"));
  h.coverage.interact("task-card-open", () => h.click(taskOpen, `Reopen ${title} after discard`));
  await h.waitFor(() => document.querySelector(".task-detail")?.textContent?.includes(title) ?? false, "reopened Task after discard");
  h.coverage.assertEffect("task-card-open", () => document.querySelector(".task-detail")?.textContent?.includes(title) ?? false);
  await ensureTaskEditing(h, "Edit Task details before close");
  await enterControl(h, "task-revision-detail", "Save this draft while leaving");
  await clickControl(h, "task-detail-close", "Request save close for dirty Task");
  await h.waitFor(() => Boolean(document.querySelector(".task-draft-guard")), "Task save leave guard");
  observe(h, "Task save leave guard");
  await clickControl(h, "task-draft-guard-save", "Save draft and close Task");
  await h.waitFor(() => !document.querySelector(".task-detail"), "saved Task draft close");
  await h.waitForAsync(async () => (await h.task(title)).detail === "Save this draft while leaving", "saved leave draft readback");
  effect(h, ["task-draft-guard-save"], () => !document.querySelector(".task-detail"));
  h.coverage.interact("task-card-open", () => h.click(taskOpen, `Reopen ${title} after guarded save`));
  await h.waitFor(() => document.querySelector(".task-detail")?.textContent?.includes(title) ?? false, "reopened Task after guarded save");
  h.coverage.assertEffect("task-card-open", () => document.querySelector(".task-detail")?.textContent?.includes(title) ?? false);
  await ensureTaskEditing(h, "Edit Task details for conflict");
  await enterControl(h, "task-revision-detail", "Keep this local conflict edit");
  const firstConflictBase = await h.task(title);
  await h.api(`/tasks/${encodeURIComponent(firstConflictBase.id)}/revisions`, "POST", {
    expectedTaskRevision: firstConflictBase.taskRevision,
    patch: { detail: "External latest detail" },
  });
  await clickControl(h, "task-revision-save", "Save stale local Task edit");
  await h.waitFor(() => Boolean(document.querySelector(".task-draft-conflict")), "Task conflict comparison");
  observe(h, "Task conflict comparison");
  if (!document.querySelector(".task-draft-conflict")?.textContent?.includes("External latest detail") || !document.querySelector(".task-draft-conflict")?.textContent?.includes("Keep this local conflict edit"))
    throw new Error("Task conflict comparison omitted the latest or edited value");
  await clickControl(h, "task-draft-keep-mine", "Keep local Task edit after conflict");
  effect(h, ["task-draft-keep-mine"], () => control<HTMLTextAreaElement>("task-revision-detail").value === "Keep this local conflict edit");
  await clickControl(h, "task-revision-save", "Save rebased local Task edit");
  await h.waitForAsync(async () => (await h.task(title)).detail === "Keep this local conflict edit", "rebased Task edit readback");
  await ensureTaskEditing(h, "Edit Task details for latest-value conflict");
  await enterControl(h, "task-revision-detail", "Edit that will use the latest value");
  const secondConflictBase = await h.task(title);
  await h.api(`/tasks/${encodeURIComponent(secondConflictBase.id)}/revisions`, "POST", {
    expectedTaskRevision: secondConflictBase.taskRevision,
    patch: { detail: "Second external latest detail" },
  });
  await clickControl(h, "task-revision-save", "Create latest-value Task conflict");
  await h.waitFor(() => Boolean(document.querySelector(".task-draft-conflict")), "latest-value Task conflict");
  observe(h, "Task latest-value conflict");
  await clickControl(h, "task-draft-use-latest", "Use latest Task value after conflict");
  effect(h, ["task-draft-use-latest"], () => control<HTMLTextAreaElement>("task-revision-detail").value === "Second external latest detail" && !document.querySelector(".task-draft-conflict"));
  await selectTaskTab(h, "Review", "Open Task Review tab");
  const before = await h.task(title);
  const initialReadiness = before.readinessEntries?.find(item => item.status === "missing");
  if (!initialReadiness) throw new Error("New Task did not expose a missing readiness field");
  await enterControl(h, "task-readiness-reason", "Not needed for this bounded Task", initialReadiness.key);
  await clickControl(h, "task-readiness-not-applicable", "Mark readiness field not applicable", initialReadiness.key);
  await h.waitForAsync(async () => (await h.task(title)).readinessEntries?.find(item => item.key === initialReadiness.key)?.status === "not_applicable", "initial readiness decision readback");
  const readinessDecision = (await h.task(title)).readinessEntries?.find(item => item.key === initialReadiness.key);
  effect(h, ["task-readiness-not-applicable"], () => readinessDecision?.status === "not_applicable" && readinessDecision.reason === "Not needed for this bounded Task");
  await selectTaskTab(h, "Details", "Open Task Details for definition revision");
  await ensureTaskEditing(h, "Edit Task details for revision");
  const revisions: Array<[string, string]> = [
    ["task-revision-title", `${title} revised`],
    ["task-revision-detail", "Detailed current behavior"],
    ["task-revision-outcome", "Every control has an effect"],
    ["task-revision-scope", "Task Workbench controls"],
    ["task-revision-non-goals", "Architecture refactor"],
    ["task-revision-criteria", "Native readback matches"],
  ];
  for (const [name, value] of revisions) await enterControl(h, name, value);
  await clickControl(h, "task-revision-save", "Save every Task revision field");
  await waitForTaskRevision(h, `${title} revised`, before.taskRevision);
  const revisedTitle = `${title} revised`;
  const revised = await h.task(revisedTitle);
  if (revised.detail !== revisions[1][1] || revised.validationCriteria !== revisions[5][1])
    throw new Error("Task revision fields did not persist together");
  effect(h, ["task-revision-save"], () => revised.detail === revisions[1][1] && revised.validationCriteria === revisions[5][1]);

  await selectTaskTab(h, "Review", "Open Task Review tab");
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

  await selectTaskTab(h, "Work", "Return to Task Work tab");
  assertDisabled("task-worklog-add", true);
  await enterControl(h, "task-worklog-text", "Matrix work evidence");
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
  await enterControl(h, "task-comment-text", "Matrix comment", log.id);
  await clickControl(h, "task-comment-add", "Add Work Log comment", log.id);
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).workLog?.find(item => item.id === log.id)?.comments?.some(item => item.body === "Matrix comment")), "comment readback");
  await h.waitFor(() => document.body.textContent?.includes("Matrix comment") === true, "rendered Work Log comment");
  effect(h, ["task-comment-add"], () => document.body.textContent?.includes("Matrix comment") === true);

  assertDisabled("task-checklist-add", true);
  await enterControl(h, "task-checklist-text", "Matrix checklist");
  await clickControl(h, "task-checklist-add", "Add checklist item");
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).checklist?.some(item => item.body === "Matrix checklist")), "checklist readback");
  const checklist = (await h.task(revisedTitle)).checklist?.find(item => item.body === "Matrix checklist");
  if (!checklist) throw new Error("Checklist item missing");
  effect(h, ["task-checklist-add"], () => Boolean(checklist));
  observe(h, "checklist row");
  await clickControl(h, "task-checklist-toggle", "Toggle checklist item", checklist.id);
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).checklist?.find(item => item.id === checklist.id)?.checked), "checked checklist readback");
  await clickControl(h, "task-checklist-completed-toggle", "Show completed checklist item");
  effect(h, ["task-checklist-completed-toggle"], () => control<HTMLInputElement>("task-checklist-toggle", checklist.id).checked);
  await h.waitFor(() => control<HTMLInputElement>("task-checklist-toggle", checklist.id).checked, "rendered checked checklist state");
  effect(h, ["task-checklist-toggle"], () => control<HTMLInputElement>("task-checklist-toggle", checklist.id).checked);

  assertDisabled("task-decision-add", true);
  await enterControl(h, "task-decision-text", "Matrix decision");
  await clickControl(h, "task-decision-add", "Add decision");
  await h.waitForAsync(async () => Boolean((await h.task(revisedTitle)).decisions?.some(item => item.body === "Matrix decision")), "decision readback");
  await h.waitFor(() => document.body.textContent?.includes("Matrix decision") === true, "rendered decision row");
  effect(h, ["task-decision-add"], () => document.body.textContent?.includes("Matrix decision") === true);

  const readiness = (await h.task(revisedTitle)).readinessEntries?.find(item => item.status === "missing");
  if (readiness) {
    observe(h, "missing readiness field");
    await enterControl(h, "task-readiness-reason", "Not needed for this bounded Task", readiness.key);
    await clickControl(h, "task-readiness-not-applicable", "Mark readiness field not applicable", readiness.key);
    await h.waitForAsync(async () => (await h.task(revisedTitle)).readinessEntries?.find(item => item.key === readiness.key)?.status === "not_applicable", "readiness decision readback");
    await h.waitFor(() => document.body.textContent?.includes("Not needed for this bounded Task") === true, "rendered readiness decision");
    effect(h, ["task-readiness-not-applicable"], () => document.body.textContent?.includes("Not needed for this bounded Task") === true);
  }

  await selectTaskTab(h, "Details", "Open Task Details connections");
  const connections = control<HTMLDetailsElement>("task-connection-details");
  const connectionSummary = connections.querySelector<HTMLElement>("summary");
  if (!connectionSummary) throw new Error("Task connection details summary missing");
  await h.prepareClick(connectionSummary, "Open connection details");
  h.coverage.interact("task-connection-details", () => h.click(connectionSummary, "Open connection details"));
  await h.waitFor(() => connections.open, "open Task connection details");
  effect(h, ["task-connection-details"], () => connections.open);
  const target = await h.task(targetTitle);
  observe(h, "Task search and connection controls");
  await enterControl(h, "task-relationship-search", targetTitle);
  await h.waitFor(() => Boolean(document.querySelector(`[data-control="task-relationship-select"][data-record-id="${CSS.escape(target.id)}"]`)), "Task search result");
  await h.click(control("task-relationship-select", target.id), "Select related Task from Workbench");
  effect(h, ["task-relationship-select"], () => control<HTMLButtonElement>("task-relationship-select", target.id).getAttribute("aria-pressed") === "true");
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

  await selectTaskTab(h, "Review", "Return to Task Review transitions");
  await clickControl(h, "task-transition-start", "Start Task");
  await h.waitFor(() => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress", "in-progress UI");
  effect(h, ["task-transition-start"], () => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress");
  await h.step("Task controls: definitions, evidence, connections, and start verified");
  await clickControl(h, "task-detail-close", "Close active Task detail");
  await h.waitFor(() => !document.querySelector(".task-detail"), "closed active Task detail");
  effect(h, ["task-detail-close"], () => !document.querySelector(".task-detail"));
  await h.waitFor(() => Boolean(document.querySelector('[data-control="task-shortcut-open"]')), "rendered active Task shortcut");
  observe(h, "active Task shortcut");
  await h.step("Task controls: closed active Task detail");
  await clickControl(h, "task-shortcut-open", "Resume active Task");
  await h.waitFor(() => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress", "resumed active Task detail");
  effect(h, ["task-shortcut-open"], () => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress");
  observe(h, "in-progress Task");
  await h.step("Task controls: resumed active Task");
  await clickControl(h, "task-transition-complete-focus", "Focus completion evidence");
  await h.waitFor(() => document.activeElement === control("task-completion-evidence"), "focused completion evidence");
  if (document.activeElement !== control("task-completion-evidence")) throw new Error("Complete shortcut did not focus evidence");
  effect(h, ["task-transition-complete-focus"], () => document.activeElement === control("task-completion-evidence"));
  await h.step("Task controls: completion evidence focused");
  assertDisabled("task-completion-complete", true);
  await enterControl(h, "task-completion-evidence", "Matrix completion evidence");
  assertDisabled("task-completion-complete", false);
  await h.step("Task controls: authored completion evidence");
  await clickControl(h, "task-completion-complete", "Complete Task with evidence");
  await h.waitForAsync(async () => (await h.task(revisedTitle)).state === "completed", "Task completion readback");
  await h.waitFor(() => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "completed", "rendered completed Task");
  effect(h, ["task-completion-complete"], () => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "completed");

  await clickControl(h, "task-detail-close", "Close completed Task");
  await h.waitFor(() => !document.querySelector(".task-detail"), "closed completed detail");
  await clickControl(h, "sidebar-view-compass", "Open Compass");
  await invoke("desktop_e2e_arm_one_shot_failure", { operation: "workbench.get" });
  await clickControl(h, "compass-completed-tab", "Open completed list");
  await h.waitFor(() => Boolean(document.querySelector('[data-control="compass-completed-retry"]')), "completed list failure");
  observe(h, "completed list recovery");
  await clickControl(h, "compass-completed-retry", "Retry completed list");
  await h.waitFor(() => Boolean(document.querySelector('[data-control="compass-completed-open"]')), "completed list loaded");
  effect(h, ["compass-completed-retry", "compass-completed-tab"], () => Boolean(document.querySelector('[data-control="compass-completed-open"]')) && !document.querySelector('[data-control="compass-completed-retry"]'));
  observe(h, "Compass completed tasks");
  await clickControl(h, "compass-completed-open", "Open completed Task in Compass");
  await h.waitFor(() => Boolean(document.querySelector('.task-detail')), "Compass Task details");
  effect(h, ["compass-completed-open"], () => Boolean(document.querySelector('.task-detail')));
  await clickControl(h, "task-detail-close", "Close Compass Task details");
  await h.waitFor(() => !document.querySelector('.task-detail'), "Compass detail closed");
  await clickControl(h, "sidebar-view-workbench", "Return to Workbench");
  await h.waitFor(() => Boolean(document.querySelector('[data-control="workbench-completed-open"]')), "recent completed Tasks");
  observe(h, "recent completed Tasks");
  await clickControl(h, "workbench-completed-open", "Open completed Task");
  effect(h, ["workbench-completed-open"], () => Boolean(document.querySelector(".task-detail")));
  await h.waitFor(() => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "completed", "reopened completed detail");
  await h.step("Task controls: completed Task rendered");
  observe(h, "completed Task");
  await clickControl(h, "task-lineage-open", "Open Task lineage");
  await h.waitFor(() => Boolean(document.querySelector(".task-journey, .task-journey-empty")), "lineage queue state");
  effect(h, ["task-lineage-open"], () => Boolean(document.querySelector(".task-journey, .task-journey-empty")));
  await h.step("Task controls: lineage opened");
  await invoke("desktop_e2e_arm_one_shot_failure", { operation: "jobs.enqueue" });
  await clickControl(h, "task-knowledge-draft", "Create Knowledge draft");
  await h.waitFor(() => Boolean(document.querySelector('[data-control="task-knowledge-draft-retry"]')), "Knowledge enqueue failure");
  observe(h, "failed Knowledge draft enqueue");
  await clickControl(h, "task-knowledge-draft-retry", "Retry Knowledge enqueue");
  await h.waitFor(() => document.querySelector<HTMLButtonElement>('[data-control="task-knowledge-draft"]')?.disabled === true, "Knowledge draft queued");
  const knowledgeTask = await h.task(revisedTitle);
  let knowledgeJobId = "";
  await h.waitForAsync(async () => {
    const { jobs } = await h.api<{ jobs: Array<{ id: string; task_kind: string; entity_id: string; status: string }> }>("/jobs");
    const job = jobs.find(job => job.task_kind === "knowledge_draft" && job.entity_id === knowledgeTask.id);
    knowledgeJobId = job?.id ?? "";
    if (job?.status === "failed") throw new Error("Knowledge Queue generation failed");
    return job?.status === "completed";
  }, "completed exact Knowledge Queue job");
  await h.step("Task controls: Knowledge Queue job completed");
  await clickControl(h, "task-detail-close", "Close Task before opening Queue result");
  await h.waitFor(() => !document.querySelector(".task-detail") && !document.querySelector<HTMLElement>(".app")?.inert, "interactive shell before Queue result");
  h.click(document.querySelector<HTMLButtonElement>("#queue-toggle")!, "Open Knowledge Queue");
  const resultSelector = `#queue-list [data-job-id="${knowledgeJobId}"] [data-job-action="result"]`;
  await h.waitFor(() => { const button = document.querySelector<HTMLButtonElement>(resultSelector); return Boolean(button && !button.disabled); }, "enabled completed Knowledge Queue result button");
  h.click(document.querySelector<HTMLButtonElement>(resultSelector)!, "Open exact Knowledge Queue result");
  await h.waitFor(() => Boolean(document.querySelector(".knowledge-draft")), "exact Knowledge draft preview");
  effect(h, ["task-knowledge-draft", "task-knowledge-draft-retry"], () => Boolean(document.querySelector(".knowledge-draft-preview")));
  await h.waitFor(() => document.activeElement?.classList.contains("knowledge-draft-preview") === true
    && document.querySelector<HTMLElement>("#queue-panel")?.hidden === true, "Queue result focuses visible draft preview");
  await h.step("Task controls: Queue result opened exact Knowledge preview");
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
  await h.waitFor(() => !regeneratedPublish.disabled, "regenerated Knowledge revision loaded");
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
  await h.waitFor(() => !document.querySelector(".refinement-panel") && Boolean(document.querySelector(".task-detail")), "restored Task detail");
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
  await h.waitFor(() => !document.querySelector(".refinement-panel"), "closed refinement before deletion");
  const savedCaptureCard = [...document.querySelectorAll<HTMLElement>(".canonical-card")].find(card => card.textContent?.includes(shortcutCapture));
  const captureDelete = savedCaptureCard?.querySelector<HTMLElement>('[data-control="task-card-delete"]');
  if (!captureDelete) throw new Error("Capture delete control missing");
  observe(h, "Workbench deletion controls");
  h.coverage.interact("task-card-delete", () => h.click(captureDelete, "Ask to delete Capture"));
  await h.waitFor(() => Boolean(document.querySelector(".workbench-delete-dialog[open]")), "Capture delete confirmation");
  effect(h, ["task-card-delete"], () => Boolean(document.querySelector(".workbench-delete-dialog[open]")));
  observe(h, "delete confirmation");
  await clickControl(h, "task-delete-cancel", "Keep Capture");
  effect(h, ["task-delete-cancel"], () => !document.querySelector(".workbench-delete-dialog") && captureDelete.isConnected);
  h.click(captureDelete, "Ask again to delete Capture");
  await h.waitFor(() => Boolean(document.querySelector(".workbench-delete-dialog[open]")), "reopened delete confirmation");
  await clickControl(h, "task-delete-confirm", "Delete Capture");
  await h.waitFor(() => !document.querySelector(".workbench-delete-dialog") && !captureDelete.isConnected, "deleted Capture removed");
  effect(h, ["task-delete-confirm"], () => !document.querySelector(".canonical-work")?.textContent?.includes(shortcutCapture));
  await h.detail(revisedTitle);
  observe(h, "Task detail delete");
  await clickControl(h, "task-detail-delete", "Ask to delete Task from detail");
  await h.waitFor(() => Boolean(document.querySelector(".workbench-delete-dialog[open]")), "Task delete confirmation");
  effect(h, ["task-detail-delete"], () => document.querySelector(".workbench-delete-dialog")?.textContent?.includes(revisedTitle) === true);
  await clickControl(h, "task-delete-confirm", "Delete Task from detail");
  await h.waitFor(() => !document.querySelector(".task-detail") && !document.querySelector(".workbench-delete-dialog"), "deleted Task detail closes");
  const afterDeletion = await h.api<{ categories: Array<{ items: Array<{ title?: string; text?: string }> }> }>("/workbench");
  if (afterDeletion.categories.some(group => group.items.some(item => item.title === revisedTitle || item.text === shortcutCapture))) throw new Error("Deleted item remains in native Workbench readback");
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
  const title = `Task detail retry ${Date.now()}`;
  await h.create(title);
  await invoke("desktop_e2e_arm_one_shot_failure", { operation: "task.get" });
  const card = [...document.querySelectorAll<HTMLElement>(".canonical-card")].find(item => item.querySelector("h3")?.textContent === title);
  const open = card?.querySelector<HTMLElement>('[data-control="task-card-open"]');
  if (!open) throw new Error("Missing retry Task card");
  h.click(open, "Open Task with failed read");
  await h.waitFor(() => Boolean(document.querySelector('[data-control="task-detail-retry"]')), "failed Task detail");
  observe(h, "failed Task detail");
  await clickControl(h, "task-detail-retry", "Retry Task detail");
  await h.waitFor(() => document.querySelector(".task-detail h2")?.textContent === title, "retried Task detail");
  effect(h, ["task-detail-retry"], () => document.querySelector(".task-detail h2")?.textContent === title);
  await clickControl(h, "task-detail-close", "Close recovered Task detail");
  await h.waitFor(() => !document.querySelector(".task-detail"), "closed recovered Task detail");
}
