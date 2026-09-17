import {
  completeDesktopE2e,
  desktopE2eMode,
  reportDesktopE2eProgress,
  type DesktopE2eResult,
} from "../services/tauriApplicationClient";
import { invoke } from "@tauri-apps/api/core";
import {
  runLegacyChatTrackingScenario,
  runLegacyPreviewWarningRetryScenario,
  prepareRefinementRelaunchScenario,
  runRefinementClosePendingScenario,
  runRefinementCloseRetryScenario,
  runRefinementProviderRecoveryScenario,
  runTaskChatScenario,
} from "./chatScenarios";
import {
  runLegacyProblemRefinementScenario,
  runTaskControlMatrixScenario,
  runWorkbenchRetryScenario,
  type WorkbenchTask,
} from "./workbenchScenarios";
import { createInteractionCoverage } from "./productionInteractiveSources";
import {
  runCompassGoalScenario,
  runFirstRunIntroNavigationScenario,
  runFirstRunIntroRetryScenario,
  runMcpSettingsScenario,
  runMigrationRestoreScenario,
  runMigrationRetryScenario,
  runNoticeScenario,
  runProviderSettingsScenario,
  runQueueNotificationActionsScenario,
  runSearchScenario,
  runShellNavigationScenario,
  runVaultChooseScenario,
  runVaultRetryScenario,
} from "./globalScenarios";

type Task = {
  id: string;
  title: string;
  detail?: string;
  outcome?: string;
  scope?: string;
  nonGoals?: string;
  validationCriteria?: string;
  taskRevision: number;
  state: string;
  workLog?: Array<{
    id: string;
    body?: string;
    attachment?: { name?: string; mediaType?: string; data?: string };
    imageSummaryVersions?: { ko?: { image_summary: string }; en?: { image_summary: string } };
    imageSummaryJob?: { id: string; status: string };
    comments?: Array<{ body: string }>;
  }>;
  checklist?: Array<{ checked: boolean }>;
  decisions?: Array<{ body?: string }>;
  problemLinks?: Array<{ problemId: string; problemRevision: number }>;
  relationships?: Array<{ targetTaskId: string }>;
  completion?: unknown;
  publication?: { state?: string; draftRevision?: number };
};
type RefinementSnapshot = {
  inputDraft?: string;
  activeTab?: string;
  messages?: Array<{ id: string }>;
};
const pause = (ms: number) =>
  new Promise((resolve) => window.setTimeout(resolve, ms));
async function waitFor(check: () => boolean, label: string) {
  const deadline = performance.now() + 10_000;
  while (performance.now() < deadline) {
    if (check()) return;
    await pause(50);
  }
  throw new Error(`Timed out waiting for ${label}`);
}
async function waitForAsync(check: () => Promise<boolean>, label: string) {
  const deadline = performance.now() + 10_000;
  while (performance.now() < deadline) {
    if (await check()) return;
    await pause(50);
  }
  throw new Error(`Timed out waiting for ${label}`);
}
async function api<T>(
  path: string,
  method: "GET" | "POST" | "PUT" | "DELETE" = "GET",
  body?: Record<string, unknown>,
) {
  const response = await window.llmWikiApplication.request({
    path,
    method,
    headers: {
      "Content-Type": "application/json",
      "X-LLM-Wiki-Locale": document.documentElement.lang || "en",
    },
    body: body
      ? JSON.stringify({ operationId: crypto.randomUUID(), ...body })
      : undefined,
  });
  if (!response.ok)
    throw new Error(`${method} ${path}: ${await response.text()}`);
  return response.status === 204 ? (undefined as T) : response.json<T>();
}
function enter(element: HTMLInputElement | HTMLTextAreaElement, value: string) {
  element.focus();
  const setter = Object.getOwnPropertyDescriptor(
    Object.getPrototypeOf(element),
    "value",
  )?.set;
  if (!setter) throw new Error("Input value setter is unavailable");
  setter.call(element, value);
  element.dispatchEvent(new Event("input", { bubbles: true }));
  element.dispatchEvent(new Event("change", { bubbles: true }));
}
/** Webview automation has no native pointer adapter. Synthetic click remains accepted only after a visible centre-point hit test. */
function positionElement(el: HTMLElement) {
  const panel = el.closest<HTMLElement>(".task-detail");
  const header = panel?.querySelector<HTMLElement>(".task-detail-header");
  if (!header?.contains(el)) el.scrollIntoView({ block: "nearest" });
  if (panel && header && !header.contains(el)) {
    const rect = el.getBoundingClientRect();
    const visibleTop = header.getBoundingClientRect().bottom + 8;
    const centre = rect.top + rect.height / 2;
    if (centre < visibleTop) panel.scrollTop -= visibleTop - centre;
    const next = el.getBoundingClientRect();
    const visibleBottom = Math.min(window.innerHeight, panel.getBoundingClientRect().bottom) - 8;
    if (next.top + next.height / 2 > visibleBottom) panel.scrollTop += next.top + next.height / 2 - visibleBottom;
  }
}
async function prepareClick(el: HTMLElement, label: string) {
  positionElement(el);
  await waitFor(() => {
    const rect = el.getBoundingClientRect();
    const hit = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
    return rect.width > 0 && rect.height > 0 && Boolean(hit && (hit === el || el.contains(hit)));
  }, `visible click target for ${label}`);
}
function clickElement(el: HTMLElement, label: string) {
  if (el instanceof HTMLButtonElement && el.disabled) throw new Error(`${label} is disabled`);
  positionElement(el);
  const panel = el.closest<HTMLElement>(".task-detail");
  const header = panel?.querySelector<HTMLElement>(".task-detail-header");
  const rect = el.getBoundingClientRect();
  if (rect.width < 1 || rect.height < 1)
    throw new Error(`${label} has no hit area`);
  const hit = document.elementFromPoint(
    rect.left + rect.width / 2,
    rect.top + rect.height / 2,
  );
  if (!hit || (hit !== el && !el.contains(hit)))
    throw new Error(`${label} is covered by ${hit?.tagName}.${hit?.className}; target=${rect.x},${rect.y},${rect.width},${rect.height}; headerBottom=${header?.getBoundingClientRect().bottom}; panelScroll=${panel?.scrollTop}; panel=${JSON.stringify(panel?.getBoundingClientRect())}; viewport=${window.innerHeight}`);
  el.click();
}
function click(selector: string, label: string) {
  const el = document.querySelector<HTMLElement>(selector);
  if (!el) throw new Error(`Missing ${label}`);
  clickElement(el, label);
}
function taskTab(name: "Work" | "Details" | "Review") {
  const key = name.toLowerCase();
  const candidates = [
    ...document.querySelectorAll<HTMLButtonElement>(
      `.task-detail [role="tab"], .task-detail button[data-control*="tab"]`,
    ),
  ];
  const tab = candidates.find((candidate) => {
    const control = candidate.getAttribute("data-control")?.toLowerCase() ?? "";
    const label = (candidate.getAttribute("aria-label") ?? candidate.textContent ?? "").trim().toLowerCase();
    return control.includes(key) || label === key || label.includes(`${key} tab`);
  });
  if (!tab) throw new Error(`Missing visible Task ${name} tab`);
  return tab;
}
async function selectTaskTab(name: "Work" | "Details" | "Review", label: string) {
  const tab = taskTab(name);
  if (tab.getAttribute("aria-selected") === "true") return;
  await prepareClick(tab, label);
  clickElement(tab, label);
  await waitFor(
    () => tab.getAttribute("aria-selected") === "true" || tab.getAttribute("aria-pressed") === "true",
    `selected Task ${name} tab`,
  );
}
async function editTaskDefinition(label = "Edit Task details") {
  const edit = document.querySelector<HTMLButtonElement>('[data-control="task-definition-edit"]');
  if (!edit || edit.hidden) return;
  clickElement(edit, label);
  await waitFor(() => document.querySelector('[data-control="task-revision-detail"]')?.getAttribute("readonly") === null, "editable Task details");
}
async function revealTaskField(label: string) {
  const work = ["Work Log entry", "Checklist item", "Decision"];
  const review = ["Completion evidence", "Knowledge draft body"];
  await selectTaskTab(work.includes(label) ? "Work" : review.includes(label) ? "Review" : "Details", `Open ${label}`);
  const element = field(label);
  const parents: HTMLDetailsElement[] = [];
  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    if (parent instanceof HTMLDetailsElement && !parent.open) parents.unshift(parent);
  }
  for (const parent of parents) {
    const summary = parent.querySelector<HTMLElement>(":scope > summary");
    if (summary) { await prepareClick(summary, `Expand ${label}`); clickElement(summary, `Expand ${label}`); await waitFor(() => parent.open, `expanded ${label}`); }
  }
}

const report = (result: DesktopE2eResult) => completeDesktopE2e(result);
let e2eProviderUrl = "";

async function workbench() {
  await waitFor(
    () => document.documentElement.dataset.applicationReady === "true",
    "application initialization",
  );
  const locale = document.querySelector<HTMLSelectElement>("#locale-select");
  if (locale && document.documentElement.lang !== "en") {
    locale.value = "en";
    locale.dispatchEvent(new Event("change", { bubbles: true }));
    await waitFor(() => document.documentElement.lang === "en", "English setup");
  }
  click('[data-view="workbench"]', "Workbench navigation");
  await waitFor(
    () =>
      document.getElementById("workbench")?.classList.contains("active") ??
      false,
    "Task Workbench",
  );
}
async function taskDetailIdle(label: string) {
  await waitFor(
    () => {
      const detail = document.querySelector(".task-detail");
      return detail !== null && detail.hasAttribute("data-task-state") && detail.getAttribute("aria-busy") === "false";
    },
    `idle Task detail before ${label}`,
  );
}
async function create(title: string, kind: "capture" | "task" = "task") {
  await waitFor(() => !document.querySelector<HTMLElement>(".app")?.inert, "interactive Workbench entry");
  const mode = [
    ...document.querySelectorAll<HTMLInputElement>('input[name="entry-mode"]'),
  ].find((input) => input.value === kind);
  if (!mode) throw new Error(`Missing ${kind} mode`);
  clickElement(mode, `${kind} mode`);
  await pause(50);
  const field = document.querySelector<HTMLTextAreaElement>("#task-input-text");
  if (!field) throw new Error("Missing Workbench entry");
  enter(field, title);
  await waitFor(
    () => field.value === title && !document.querySelector<HTMLButtonElement>('#workbench form button[type="submit"]')?.disabled,
    `committed ${kind} entry`,
  );
  click('#workbench form button[type="submit"]', "Save entry");
  await waitForAsync(
    async () => {
      const snapshot = await api<{
        categories: Array<{
          items: Array<{ kind: string; text?: string; title?: string }>;
        }>;
      }>("/workbench");
      return snapshot.categories
        .flatMap((category) => category.items)
        .some(
          (item) =>
            item.kind === kind && (item.kind === "task" ? item.title : item.text) === title,
        );
    },
    `canonical ${kind}`,
  );
  await waitFor(
    () =>
      [...document.querySelectorAll<HTMLElement>(".canonical-card")].some((card) =>
        card.textContent?.includes(title),
      ),
    `rendered ${kind}`,
  );
}
async function task(title: string) {
  const snapshot = await api<{
    categories: Array<{
      items: Array<{ id: string; kind: string; title?: string }>;
    }>;
  }>("/workbench");
  const item = snapshot.categories
    .flatMap((group) => group.items)
    .find((item) => item.kind === "task" && item.title === title);
  if (!item) throw new Error(`Task ${title} missing from canonical projection`);
  return api<Task>(`/tasks/${item.id}`);
}
async function detail(title: string) {
  await waitFor(() => !document.querySelector(".task-detail") || document.querySelector(".task-detail h2")?.textContent === title, "previous Task detail closed or matching Task restored");
  if (document.querySelector(".task-detail")) {
    await taskDetailIdle("using restored Task detail");
    return;
  }
  const card = [
    ...document.querySelectorAll<HTMLElement>(".canonical-card"),
  ].find((card) => card.querySelector("h3")?.textContent?.trim() === title);
  const completed = card?.closest<HTMLDetailsElement>("details");
  if (completed && !completed.open) clickElement(completed.querySelector<HTMLElement>("summary")!, "Show completed Tasks");
  const button = card?.querySelector<HTMLButtonElement>("button") ?? [...document.querySelectorAll<HTMLButtonElement>('[data-control="workbench-completed-open"]')].find(button => button.textContent?.trim() === title);
  if (!button) throw new Error(`No detail control for ${title}`);
  clickElement(button, `Open ${title}`);
  await waitFor(
    () =>
      document.querySelector(".task-detail")?.textContent?.includes(title) ??
      false,
    "Task detail",
  );
}
function field(label: string) {
  const control = [
    ...document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>(
      ".task-detail input, .task-detail textarea",
    ),
  ].find((item) => item.getAttribute("aria-label") === label);
  if (!control) {
    const detailState = document.querySelector(".task-detail")?.innerHTML ?? "absent";
    const completion = document.getElementById("task-completion-evidence");
    throw new Error(
      `Missing ${label}; completion=${completion?.outerHTML ?? "absent"}; controls=${document.querySelectorAll(".task-detail input, .task-detail textarea").length}; Task detail DOM: ${detailState.slice(-1600)}`,
    );
  }
  return control;
}
async function clickAfter(label: string, description: string) {
  await taskDetailIdle(description);
  const control = field(label);
  const button =
    control.parentElement?.querySelector<HTMLElement>("button") ??
    (control.nextElementSibling as HTMLElement | null);
  if (!button) throw new Error(`Missing ${description}`);
  clickElement(button, description);
}

async function capture(step: Step) {
  const c = `capture ${Date.now()}`,
    t = `direct task ${Date.now()}`;
  await create(c, "capture");
  await create(t);
  const saved = await task(t);
  if (saved.state !== "task")
    throw new Error("Direct Task state was not persisted");
  await step(
    "Rendered Capture and direct-Task controls created canonical records; the Task was independently read back.",
  );
}
async function log(step: Step, interactionCoverage: ReturnType<typeof createInteractionCoverage>) {
  const title = `work log ${Date.now()}`;
  await create(title);
  await detail(title);
  await selectTaskTab("Details", "Open Task Details for Work Log scenario");
  await editTaskDefinition();
  await selectTaskTab("Work", "Open Task Work tab");
  await revealTaskField("Work Log entry");
  enter(field("Work Log entry"), "visible work evidence");
  await clickAfter("Work Log entry", "Add Work Log");
  await waitFor(
    () =>
      document
        .querySelector(".log-entry")
        ?.textContent?.includes("visible work evidence") ?? false,
    "Work Log",
  );
  const comment = document.querySelector<HTMLInputElement>(".log-entry input")!;
  enter(comment, "reviewed");
  clickElement(
    comment.parentElement!.querySelector<HTMLButtonElement>("button")!,
    "Add Work Log comment",
  );
  await waitForAsync(async () =>
    (await task(title)).workLog?.some((item) => item.comments?.some((savedComment) => savedComment.body === "reviewed")) ?? false,
  "persisted Work Log comment");
  await taskDetailIdle("adding an attachment");
  const file = document.querySelector<HTMLInputElement>(
    '[aria-label="Attach image or file"]',
  )!;
  const files = new DataTransfer();
  files.items.add(
    new File(["desktop evidence"], "evidence.txt", { type: "text/plain" }),
  );
  Object.defineProperty(file, "files", { value: files.files });
  file.dispatchEvent(new Event("change", { bubbles: true }));
  await revealTaskField("Work Log entry");
  enter(field("Work Log entry"), "attached evidence");
  await clickAfter("Work Log entry", "Add attachment");
  await waitForAsync(async () => (await task(title)).workLog?.some((item) => item.attachment?.name === "evidence.txt") ?? false, "persisted attachment");
  await taskDetailIdle("pasting a screenshot");
  await revealTaskField("Work Log entry");
  const screenshotData = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=";
  const screenshot = new DataTransfer();
  screenshot.items.add(new File([Uint8Array.from(atob(screenshotData), character => character.charCodeAt(0))], "screenshot.png", { type: "image/png" }));
  field("Work Log entry").dispatchEvent(new ClipboardEvent("paste", { bubbles: true, cancelable: true, clipboardData: screenshot }));
  await waitFor(() => !document.querySelector<HTMLButtonElement>('[data-control="task-worklog-add"]')?.disabled, "screenshot-only entry enabled");
  await clickAfter("Work Log entry", "Save pasted screenshot");
  await waitForAsync(async () => (await task(title)).workLog?.some(item => item.body === "" && item.attachment?.name === "screenshot.png" && item.attachment?.mediaType === "image/png" && item.attachment?.data === screenshotData) ?? false, "pasted screenshot bytes persisted");
  await waitFor(() => {
    const preview = document.querySelector<HTMLImageElement>('.work-log-image[alt="screenshot.png"]');
    return !!preview && preview.complete && preview.naturalWidth > 0;
  }, "saved screenshot rendered inline");
  await waitForAsync(async () => (await task(title)).workLog?.find(item => item.attachment?.name === "screenshot.png")?.imageSummaryJob?.status === "failed", "automatic image summary reports missing provider in the Queue");
  await api("/provider/config", "PUT", { base_url: e2eProviderUrl, model: "deterministic", api_key: "desktop-e2e-key" });
  await waitFor(() => !!document.querySelector('[data-control="task-image-summary"]'), "saved-image summary retry available");
  interactionCoverage.observe(document, "task-worklog", "saved image retry");
  interactionCoverage.interact("task-image-summary", () => click('[data-control="task-image-summary"]', "Generate both image summary languages"));
  await waitForAsync(async () => {
    const saved = (await task(title)).workLog?.find(item => item.attachment?.name === "screenshot.png");
    return saved?.imageSummaryJob?.status === "completed"
      && saved.imageSummaryVersions?.ko?.image_summary === "결정론적 이미지 요약"
      && saved.imageSummaryVersions?.en?.image_summary === "Deterministic image summary";
  }, "bilingual image summary retry completed through the AI queue");
  await waitFor(() => document.querySelector(".work-log-image-summary")?.textContent?.includes("Deterministic image summary") ?? false, "completed image summary rendered");
  interactionCoverage.assertEffect("task-image-summary", () => document.querySelector(".work-log-image-summary")?.textContent?.includes("Deterministic image summary") === true);
  await step("Pasted an image through the Work Log clipboard handler, persisted exact screenshot bytes without requiring text, and rendered the saved image inline.");
  await revealTaskField("Checklist item");
  enter(field("Checklist item"), "verify result");
  await clickAfter("Checklist item", "Add checklist");
  await waitFor(
    () => !!document.querySelector('.task-detail input[type="checkbox"]'),
    "checklist",
  );
  clickElement(
    document.querySelector<HTMLInputElement>(
      '.task-detail input[type="checkbox"]',
    )!,
    "Check checklist item",
  );
  await waitForAsync(
    async () => (await task(title)).checklist?.some((item) => item.checked) ?? false,
    "checked checklist item",
  );
  await revealTaskField("Decision");
  enter(field("Decision"), "ship deliberately");
  await clickAfter("Decision", "Add decision");
  await waitForAsync(
    async () =>
      (await task(title)).decisions?.some(
        (item) => item.body === "ship deliberately",
      ) ?? false,
    "saved decision",
  );
  const saved = await task(title);
  if (
    !saved.workLog?.[0]?.comments?.some((item) => item.body === "reviewed") ||
    !saved.workLog?.some((item) => item.attachment?.name === "evidence.txt") ||
    !saved.checklist?.some((item) => item.checked) ||
    !saved.decisions?.some((item) => item.body === "ship deliberately")
  )
    throw new Error(
      "Work Log, attachment, comment, checklist, or decision did not persist",
    );
  await step(
    "Rendered Work Log, file attachment, comment, checklist, and decision controls persisted their evidence.",
  );
}
async function refinement(step: Step) {
  await api("/provider/config", "PUT", { base_url: e2eProviderUrl, model: "deterministic", api_key: "desktop-e2e-key" });
  const a = `refine A ${Date.now()}`,
    b = `refine B ${Date.now()}`;
  await create(a);
  await create(b);
  await detail(a);
  click('[data-control="task-detail-refine"]', "Refine A");
  await waitFor(
    () => !!document.querySelector(".refinement-panel"),
    "A refinement",
  );
  enter(
    document.querySelector<HTMLTextAreaElement>(
      '[aria-label="Saved refinement note"]',
    )!,
    "A unsent note",
  );
  const messages = document.querySelector<HTMLDivElement>(
    ".refinement-messages",
  )!;
  messages.scrollTop = 0;
  messages.dispatchEvent(new Event("scroll", { bubbles: true }));
  await pause(650);
  click(".refinement-panel header button", "Close A refinement");
  await waitFor(() => !document.querySelector(".refinement-panel"), "closed A refinement");
  click('[aria-label="Close Task detail"]', "Close A detail");
  await detail(b);
  click('[data-control="task-detail-refine"]', "Refine B");
  await waitFor(
    () => !!document.querySelector(".refinement-panel"),
    "B refinement",
  );
  click(".refinement-panel header button", "Close B refinement");
  await waitFor(() => !document.querySelector(".refinement-panel"), "closed B refinement");
  click('[aria-label="Close Task detail"]', "Close B detail");
  await detail(a);
  click('[data-control="task-detail-refine"]', "Resume A refinement");
  await waitFor(
    () => Boolean(document.querySelector(".refinement-panel .refinement-messages")),
    "A refinement conversation restore",
  );
  await waitFor(
    () =>
      document.querySelector<HTMLTextAreaElement>(
        '[aria-label="Saved refinement note"]',
      )?.value === "A unsent note",
    "A workspace restore",
  );
  const png = Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII="), char => char.charCodeAt(0));
  const files = new DataTransfer();
  for (const name of ["first.png", "second.png"]) files.items.add(new File([png], name, { type: "image/png" }));
  const picker = document.querySelector<HTMLInputElement>('.refinement-panel [data-control="input-image-file"]')!;
  if (!picker.multiple) throw new Error("Refinement image picker must allow multiple files");
  Object.defineProperty(picker, "files", { configurable: true, value: files.files });
  picker.dispatchEvent(new Event("change", { bubbles: true }));
  await waitFor(() => document.querySelectorAll(".refinement-composer .input-image-preview").length === 2, "two unsent image previews");
  const chat = document.querySelector<HTMLTextAreaElement>('[data-control="refinement-message"]')!;
  enter(chat, Array.from({ length: 35 }, (_, index) => `Image context line ${index}`).join("\n"));
  click('[data-control="refinement-send"]', "Send multiple images");
  await waitFor(() => document.querySelectorAll(".refinement-messages .input-image-preview").length === 2
    && document.querySelector(".refinement-panel")?.getAttribute("data-refinement-polling") === "false", "saved multiple images and completed response");
  await waitFor(() => {
    const conversation = document.querySelector<HTMLElement>(".refinement-messages")!;
    return conversation.scrollHeight > conversation.clientHeight && conversation.scrollHeight - conversation.scrollTop - conversation.clientHeight < 5;
  }, "new messages scroll overflowing conversation to bottom");
  click(".refinement-panel header button", "Close multi-image conversation");
  await waitFor(() => !document.querySelector(".refinement-panel"), "closed multi-image conversation");
  click('[data-control="task-detail-refine"]', "Reopen multi-image conversation");
  await waitFor(() => document.querySelectorAll(".refinement-messages .input-image-preview").length === 2, "both saved images restored after reopening");
  await step("Multiple image attachments persisted through send and reopen, and new overflowing messages scrolled to the bottom.");
  await step(
    "A→B→A refinement restored the unsent note, active tab, and scroll workspace after UI close/reopen.",
  );
}
async function relationships(step: Step) {
  const a = `relationship A ${Date.now()}`,
    b = `relationship B ${Date.now()}`;
  await create(a);
  await create(b);
  const target = await task(b);
  await detail(a);
  await selectTaskTab("Details", "Open Task Details tab");
  clickElement(
    document.querySelector<HTMLDetailsElement>(".connection-details > summary")!,
    "Open connection details",
  );
  await revealTaskField("New Problem statement");
  enter(
    field("New Problem statement"),
    "Known Problem with a preserved solution",
  );
  await clickAfter("New Problem statement", "Create and link Problem");
  await waitFor(
    () =>
      !!document
        .querySelector(".task-detail")
        ?.textContent?.includes("revision 1"),
    "linked Problem",
  );
  await revealTaskField("Problem revision statement");
  enter(field("Problem revision statement"), "Known Problem revised");
  await clickAfter("Problem revision statement", "Revise Problem");
  await waitFor(
    () =>
      document.querySelector<HTMLInputElement>(
        '[aria-label="Problem revision"]',
      )?.value === "2",
    "Problem revision 2",
  );
  await clickAfter("Problem ID", "Link revised Problem");
  await waitForAsync(async () => (await task(a)).problemLinks?.some((link) => link.problemRevision === 2) ?? false, "persisted revised Problem link");
  await taskDetailIdle("linking prerequisite");
  await revealTaskField("Related Task ID");
  enter(field("Related Task ID"), target.id);
  const kind = document.querySelector<HTMLSelectElement>(
    '[aria-label="Relationship kind"]',
  )!;
  kind.value = "prerequisite";
  kind.dispatchEvent(new Event("change", { bubbles: true }));
  clickElement(
    kind.parentElement!.querySelector<HTMLButtonElement>("button")!,
    "Link prerequisite",
  );
  await waitForAsync(async () => {
    const saved = await task(a);
    return Boolean(
      saved.problemLinks?.some((link) => link.problemRevision === 1) &&
        saved.problemLinks?.some((link) => link.problemRevision === 2) &&
        saved.relationships?.some((link) => link.targetTaskId === target.id),
    );
  }, "exact Problem revisions and prerequisite");
  await taskDetailIdle("starting Task");
  click('[data-control="task-transition-start"]', "Start Task");
  await waitFor(
    () =>
      document
        .querySelector(".task-detail")
        ?.getAttribute("data-task-state") === "in_progress",
    "Task start",
  );
  await step(
    "Rendered Problem r1/r2 links, prerequisite, and start controls persisted exact identities without a readiness gate.",
  );
}
async function review(step: Step) {
  const title = `Startup indexing review ${Date.now()}`;
  await api("/index", "POST", {});
  await api("/provider/config", "PUT", {
    base_url: e2eProviderUrl,
    model: "deterministic-review-pending",
    api_key: "desktop-e2e-key",
  });
  await create(title);
  await detail(title);
  await selectTaskTab("Review", "Open Task Review tab");
  click('[data-control="conflict-review-run"]', "Run review");
  await waitFor(
    () =>
      !!document.querySelector(
        '[data-control="conflict-review-run"] ~ * [data-review-status="queued"], .review-panel [data-review-status="queued"], .review-panel [data-review-status="running"]',
      ),
    "queued review",
  );
  await revealTaskField("Work Log entry");
  enter(field("Work Log entry"), "continued during review");
  await clickAfter("Work Log entry", "Concurrent Work Log");
  await waitForAsync(
    async () => (await task(title)).workLog?.some((item) => item.body === "continued during review") ?? false,
    "concurrent Work Log readback",
  );
  await taskDetailIdle("cancelling review");
  await selectTaskTab("Review", "Return to review after recording work");
  click('[data-control="conflict-review-cancel"]', "Cancel review");
  await waitFor(
    () =>
      !!document.querySelector(
        '.review-panel [data-review-status="cancelled"]',
      ),
    "cancelled review",
  );
  await api("/provider/config", "PUT", {
    base_url: e2eProviderUrl,
    model: "deterministic-review-stale",
    api_key: "desktop-e2e-key",
  });
  click('[data-control="conflict-review-retry"]', "Retry delayed review");
  await waitFor(
    () =>
      !!document.querySelector(
        '.review-panel [data-review-status="queued"], .review-panel [data-review-status="running"]',
      ),
    "second queued review",
  );
  await selectTaskTab("Details", "Open Task Details for revision");
  await editTaskDefinition();
  const titleField = document.querySelector<HTMLInputElement>('[data-control="task-revision-title"]');
  if (!titleField) throw new Error("Missing Task title editor");
  enter(titleField, `${title} revised`);
  const save = document.querySelector<HTMLButtonElement>('[data-control="task-revision-save"]');
  if (!save) throw new Error("Missing Task revision action");
  await waitFor(() => !save.disabled, "committed Task revision draft");
  clickElement(save, "Save Task revision");
  await waitFor(
    () =>
      !!document.querySelector('.review-panel [data-review-status="stale"]'),
    "stale review after Task revision",
  );
  await api("/provider/config", "PUT", {
    base_url: e2eProviderUrl,
    model: "deterministic-test-model",
    api_key: "desktop-e2e-key",
  });
  const retriedAt = performance.now();
  await selectTaskTab("Review", "Return to Task Review");
  click('[data-control="conflict-review-retry"]', "Retry cited review");
  await waitFor(
    () =>
      !!document.querySelector(
        '.review-panel [role="status"] [data-review-status="findings"]',
      ),
    "retried review result",
  );
  if (performance.now() - retriedAt > 10_000)
    throw new Error(
      "Cited review exceeded the deterministic 10s latency budget",
    );
  if (
    !document.querySelector(
      '.review-panel [role="status"] [data-review-status="findings"]',
    )
  )
    throw new Error("Seeded evidence did not produce findings");
  if (!document.querySelector(".review-panel .citation[data-citation-path]"))
    throw new Error("Finding omitted its Vault citation");
  await step(
    "Delayed review was cancelled, stale after a rendered Task revision, retried, and remained nonblocking with cited findings when evidence existed.",
  );
}

async function taskMcpContinuation(step: Step) {
  const title = `desktop MCP Task ${Date.now()}`;
  const relatedTitle = `desktop MCP related Task ${Date.now()}`;
  await create(title);
  await create(relatedTitle);
  const related = await task(relatedTitle);
  await detail(title);
  await selectTaskTab("Details", "Open Task Details for MCP evidence");
  clickElement(
    document.querySelector<HTMLElement>(".connection-details > summary")!,
    "Open Task connections for MCP evidence",
  );
  await revealTaskField("New Problem statement");
  enter(field("New Problem statement"), "Exact desktop Problem for MCP continuation");
  await clickAfter("New Problem statement", "Create exact Problem link");
  await waitForAsync(async () => Boolean((await task(title)).problemLinks?.length), "desktop Problem link before MCP discovery");
  await taskDetailIdle("linking related Task for MCP");
  await revealTaskField("Related Task ID");
  enter(field("Related Task ID"), related.id);
  const relationship = document.querySelector<HTMLSelectElement>('[aria-label="Relationship kind"]');
  if (!relationship) throw new Error("Missing relationship kind for MCP scenario");
  relationship.value = "related";
  relationship.dispatchEvent(new Event("change", { bubbles: true }));
  clickElement(relationship.parentElement!.querySelector<HTMLButtonElement>("button")!, "Link Task before MCP discovery");
  await waitForAsync(async () => Boolean((await task(title)).relationships?.length), "desktop Task relationship before MCP discovery");
  const saved = await task(title);
  const connection = await api<{ id: string }>("/work-tracking/connections", "POST", {
    name: "Packaged Task continuation",
    scopes: ["session:read", "session:write", "workbench:current:read", "workbench:overview:read"],
    topicIds: [],
    checkpointPolicy: "confirm_each",
  });
  const probe = await invoke<{ captureless: boolean; taskId: string; problemLinksVerified: boolean; relationshipsVerified: boolean; cancelledWithoutSession: boolean; changedDetail: string; expectedTaskRevision: number }>("desktop_e2e_mcp_probe", {
    connectionId: connection.id,
    revoked: false,
    taskId: saved.id,
  });
  if (!probe.captureless || probe.taskId !== saved.id || !probe.problemLinksVerified || !probe.relationshipsVerified || !probe.cancelledWithoutSession) {
    throw new Error("Real packaged MCP did not preserve direct Task identity, exact links, or cancelled continuation");
  }
  await waitForAsync(async () => {
    const changed = await task(title);
    return changed.taskRevision === probe.expectedTaskRevision && changed.detail === probe.changedDetail;
  }, "canonical Task revision committed through real MCP");
  click('[aria-label="Close Task detail"]', "Close Task detail before MCP refresh");
  await waitFor(() => !document.querySelector(".task-detail"), "Task detail closed before refresh");
  await detail(title);
  await waitFor(() => document.querySelector(".task-detail")?.textContent?.includes(probe.changedDetail) ?? false, "MCP edit rendered in desktop Task detail");
  await step("Desktop-created Task and exact Problem/Task relationships were read through a real stdio MCP child; rejected continuation created no session and accepted continuation created no Capture.");
  await step("Exact reviewed MCP Task revision was committed through the GUI owner and rendered after reopening desktop detail.");
  await api(`/work-tracking/connections/${connection.id}`, "DELETE");
  const revoked = await invoke<{ revoked: boolean }>("desktop_e2e_mcp_probe", { connectionId: connection.id, revoked: true, taskId: saved.id });
  if (!revoked.revoked) throw new Error("Revoked Task connection retained packaged MCP access");
  await step("Revoked connection failed closed in the packaged stdio/GUI IPC boundary.");
}
async function publication(step: Step) {
  const activate = async (selector: string, label: string) => {
    await waitFor(() => {
      const button = document.querySelector<HTMLElement>(selector);
      return Boolean(button && !button.matches(":disabled") && !button.closest("[inert]")
        && document.querySelector(".task-detail")?.getAttribute("aria-busy") !== "true"
        && !document.querySelector('[data-control="task-knowledge-draft"][aria-busy="true"]'));
    }, `ready ${label}`);
    const button = document.querySelector<HTMLElement>(selector)!;
    await prepareClick(button, label);
    clickElement(button, label);
  };
  const title = `publish ${Date.now()}`;
  await create(title);
  await detail(title);
  await selectTaskTab("Details", "Open Task Details for publication fields");
  await editTaskDefinition("Edit Task details for publication");
  const authoredTaskFields = {
    detail: "Context: verify the signed desktop package through its rendered controls.",
    outcome: "A reviewable Knowledge record preserves the packaged acceptance evidence.",
    scope: "Task Workbench publication and exact provenance readback.",
    nonGoals: "Replacing the installed app or changing the deferred architecture.",
    validationCriteria: "The final Markdown contains these authored Task fields and cited evidence.",
  };
  for (const [controlName, value] of Object.entries({
    "task-revision-detail": authoredTaskFields.detail,
    "task-revision-outcome": authoredTaskFields.outcome,
    "task-revision-scope": authoredTaskFields.scope,
    "task-revision-non-goals": authoredTaskFields.nonGoals,
    "task-revision-criteria": authoredTaskFields.validationCriteria,
  })) {
    enter(document.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[data-control="${controlName}"]`)!, value);
    await pause(25);
  }
  click('[data-control="task-revision-save"]', "Save authored publication fields");
  await waitForAsync(async () => {
    const revised = await task(title);
    return revised.detail === authoredTaskFields.detail && revised.outcome === authoredTaskFields.outcome &&
      revised.scope === authoredTaskFields.scope && revised.nonGoals === authoredTaskFields.nonGoals &&
      revised.validationCriteria === authoredTaskFields.validationCriteria;
  }, "authored publication fields");
  await waitFor(
    () => document.querySelector<HTMLElement>(".task-detail")?.dataset.taskRevision === "2",
    "rendered authored publication revision",
  );
  await taskDetailIdle("authored publication revision settled");
  await selectTaskTab("Work", "Open Task Work for publication evidence");
  const publicationWorkLogText = document.querySelector<HTMLTextAreaElement>('[data-control="task-worklog-text"]');
  if (!publicationWorkLogText) throw new Error("Missing publication Work Log field");
  enter(publicationWorkLogText, "Validated the signed desktop release");
  await waitFor(
    () => !document.querySelector<HTMLButtonElement>('[data-control="task-worklog-add"]')?.disabled,
    "enabled publication Work Log action",
  );
  await taskDetailIdle("adding publication Work Log");
  click('[data-control="task-worklog-add"]', "Add publication Work Log");
  await waitForAsync(async () =>
    Boolean((await task(title)).workLog?.some((item) => item.body === "Validated the signed desktop release")),
  "persisted publication Work Log");
  await waitFor(
    () =>
      document
        .querySelector(".log-entry")
        ?.textContent?.includes("Validated the signed desktop release") ?? false,
    "publication Work Log",
  );
  await taskDetailIdle("publication Work Log settled");
  await revealTaskField("Checklist item");
  enter(field("Checklist item"), "Verify packaged acceptance scenarios");
  await clickAfter("Checklist item", "Add publication checklist");
  await waitFor(
    () => !!document.querySelector('.task-detail input[type="checkbox"]'),
    "publication checklist",
  );
  await activate('.task-detail input[type="checkbox"]', "Check publication evidence");
  await revealTaskField("Decision");
  enter(field("Decision"), "Publish only the reviewed revision");
  await clickAfter("Decision", "Add publication decision");
  await waitForAsync(async () => (await task(title)).decisions?.some((item) => item.body === "Publish only the reviewed revision") ?? false, "publication decision readback");
  await taskDetailIdle("opening publication connections");
  await selectTaskTab("Details", "Open Task Details for publication connections");
  clickElement(
    document.querySelector<HTMLDetailsElement>(".connection-details > summary")!,
    "Open publication connection details",
  );
  await revealTaskField("New Problem statement");
  enter(
    field("New Problem statement"),
    "Release evidence must remain traceable",
  );
  await clickAfter("New Problem statement", "Create publication Problem");
  await waitFor(
    () =>
      document
        .querySelector(".task-detail")
        ?.textContent?.includes("revision 1") ?? false,
    "publication Problem revision",
  );
  await activate('[data-control="task-transition-start"]', "Start Task");
  await waitFor(
    () =>
      document
        .querySelector(".task-detail")
        ?.getAttribute("data-task-state") === "in_progress",
    "Task start",
  );
  await taskDetailIdle("publication Task start settled");
  await selectTaskTab("Review", "Open Task Review for publication");
  await revealTaskField("Completion evidence");
  enter(field("Completion evidence"), "verified in packaged E2E");
  await activate("#task-completion-evidence + button", "Complete Task");
  await waitFor(
    () =>
      document
        .querySelector(".task-detail")
        ?.getAttribute("data-task-state") === "completed",
    "completion",
  );
  const buttons = [
    ...document.querySelectorAll<HTMLButtonElement>(".task-detail button"),
  ];
  const draft = buttons.find((button) =>
    /create draft/i.test(button.textContent ?? ""),
  );
  if (!draft) throw new Error("Missing Knowledge draft action");
  await activate('[data-control="task-knowledge-draft"]', "Create Knowledge draft");
  const completedTask = await task(title);
  let knowledgeJobId = "";
  await waitForAsync(async () => {
    const { jobs } = await api<{ jobs: Array<{ id: string; task_kind: string; entity_id: string; status: string }> }>("/jobs");
    const job = jobs.find(job => job.task_kind === "knowledge_draft" && job.entity_id === completedTask.id);
    knowledgeJobId = job?.id ?? "";
    return job?.status === "completed";
  }, "completed Knowledge Queue job");
  click('[data-control="task-detail-close"]', "Close Task detail before opening Queue");
  await waitFor(() => !document.querySelector(".task-detail"), "Task detail closed for Queue navigation");
  clickElement(document.querySelector<HTMLButtonElement>("#queue-toggle")!, "Open AI Queue");
  const resultSelector = `#queue-list [data-job-id="${knowledgeJobId}"] [data-job-action="result"]`;
  await waitFor(() => { const button = document.querySelector<HTMLButtonElement>(resultSelector); return Boolean(button && !button.disabled); }, "enabled exact Knowledge Queue result");
  await activate(resultSelector, "Open exact Knowledge Queue result");
  await waitFor(
    () => !!document.querySelector(".knowledge-draft"),
    "Knowledge preview",
  );
  await waitFor(() => {
    const body = document.querySelector<HTMLTextAreaElement>('[data-control="task-knowledge-draft-body"]');
    const save = document.querySelector<HTMLButtonElement>('[data-control="task-knowledge-correct"]');
    return Boolean(body && !body.readOnly && save && !save.disabled);
  }, "editable current Knowledge draft");
  await revealTaskField("Knowledge draft body");
  const correction = field("Knowledge draft body") as HTMLTextAreaElement;
  for (const expected of [
    "Validated the signed desktop release",
    "Verify packaged acceptance scenarios",
    "Publish only the reviewed revision",
    "Release evidence must remain traceable",
    "source `",
  ]) {
    if (!correction.value.includes(expected))
      throw new Error(`Knowledge draft omitted rich evidence: ${expected}`);
  }
  const initialHash = document
    .querySelector(".knowledge-draft")
    ?.getAttribute("data-content-hash");
  if (!initialHash) throw new Error("Knowledge draft omitted its exact hash");
  const correctedBody = `${correction.value}\n\nCorrected by packaged desktop E2E.`;
  enter(correction, correctedBody);
  const correctionButton = document.querySelector<HTMLButtonElement>(
    ".knowledge-draft button",
  )!;
  await waitFor(() => !correctionButton.disabled, "committed Knowledge draft correction");
  await activate('[data-control="task-knowledge-correct"]', "Save Knowledge draft correction");
  await waitFor(
    () =>
      document.querySelector<HTMLButtonElement>('[data-control="task-knowledge-correct"]')?.disabled === false &&
      Boolean(document.querySelector(".knowledge-draft")?.getAttribute("data-content-hash")) &&
      document
        .querySelector(".knowledge-draft")
        ?.getAttribute("data-content-hash") !== initialHash &&
      document.querySelector<HTMLTextAreaElement>(
        "[aria-label='Knowledge draft body']",
      )?.value === correctedBody,
    "persisted Knowledge correction",
  );
  const publishDraft = [
    ...document.querySelectorAll<HTMLButtonElement>(".knowledge-draft button"),
  ].find((button) => /publish/i.test(button.textContent ?? ""));
  if (!publishDraft)
    throw new Error("Missing corrected Knowledge publish action");
  await activate('.knowledge-draft [data-control="task-knowledge-publish"]', "Publish corrected Knowledge draft");
  await waitFor(
    () =>
      document
        .querySelector(".publication-controls")
        ?.getAttribute("data-publication-state") === "published",
    "published Knowledge controls",
  );
  const regenerate = [
    ...document.querySelectorAll<HTMLButtonElement>(".task-detail button"),
  ].find((button) => /regenerate draft/i.test(button.textContent ?? ""));
  if (!regenerate) throw new Error("Missing regenerate action");
  await activate('[data-control="task-knowledge-regenerate"]', "Regenerate Knowledge draft");
  await waitFor(
    () => !!document.querySelector(".knowledge-draft"),
    "regenerated Knowledge preview",
  );
  const regeneratedRevision = document
    .querySelector(".knowledge-draft h4")
    ?.textContent?.match(/r(\d+)/)?.[1];
  if (!regeneratedRevision)
    throw new Error("Regenerated Knowledge omitted its draft revision");
  const publish = [
    ...document.querySelectorAll<HTMLButtonElement>(".knowledge-draft button"),
  ].find((button) => /publish/i.test(button.textContent ?? ""));
  if (!publish) throw new Error("Missing regenerated publish action");
  await activate('.knowledge-draft [data-control="task-knowledge-publish"]', "Publish regenerated Knowledge draft");
  await waitFor(
    () =>
      !document.querySelector(".knowledge-draft") &&
      document
        .querySelector(".publication-controls")
        ?.getAttribute("data-publication-revision") === regeneratedRevision &&
      document
        .querySelector(".publication-controls")
        ?.getAttribute("data-publication-state") === "published",
    "republished Knowledge",
  );
  const withdraw = [
    ...document.querySelectorAll<HTMLButtonElement>(".task-detail button"),
  ].find((button) => /withdraw knowledge/i.test(button.textContent ?? ""));
  if (!withdraw) throw new Error("Missing withdraw action");
  await activate('[data-control="task-knowledge-withdraw"]', "Withdraw Knowledge");
  await waitFor(
    () =>
      document
        .querySelector(".publication-controls")
        ?.getAttribute("data-publication-state") === "withdrawn",
    "withdrawn Knowledge",
  );
  const saved = await task(title);
  if (!saved.completion || saved.publication?.state !== "withdrawn" || saved.publication.draftRevision !== Number(regeneratedRevision))
    throw new Error(
      "Completion, regenerate, publish, and withdraw did not persist separately",
    );
  await step(
    "Rendered completion, exact draft correction, explicit publish, regenerate, and withdraw actions persisted as separate decisions.",
  );
}
async function problemResolution(step: Step) {
  const title = `problem resolution ${Date.now()}`;
  await create(title);
  await detail(title);
  await selectTaskTab("Details", "Open Task Details for Problem resolution");
  clickElement(
    document.querySelector<HTMLDetailsElement>(".connection-details > summary")!,
    "Open Problem connection details",
  );
  await revealTaskField("New Problem statement");
  enter(field("New Problem statement"), "Resolution must remain explicit");
  await clickAfter("New Problem statement", "Create resolution Problem");
  await waitForAsync(
    async () => Boolean((await task(title)).problemLinks?.some(candidate => candidate.problemRevision === 1)),
    "exact first Problem revision link",
  );
  const linked = await task(title);
  const link = linked.problemLinks?.[0];
  if (!link || link.problemRevision !== 1)
    throw new Error("Problem link was not persisted at its exact first revision");
  click('[data-control="task-transition-start"]', "Start Task");
  await waitFor(
    () => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress",
    "Task start before completion",
  );
  await selectTaskTab("Review", "Open Task Review before completion");
  await revealTaskField("Completion evidence");
  enter(field("Completion evidence"), "Task evidence does not resolve its Problem");
  click("#task-completion-evidence + button", "Complete Task");
  await waitFor(
    () => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "completed",
    "Task completion without Problem resolution",
  );
  if ((await task(title)).state !== "completed")
    throw new Error("Task completion was not persisted independently");
  const resolve = [...document.querySelectorAll<HTMLButtonElement>(".task-detail button")].find(
    (button) => button.textContent === "Resolve Problem",
  );
  if (!resolve) throw new Error("Missing explicit Problem resolution action");
  await selectTaskTab("Details", "Open explicit Problem resolution");
  clickElement(resolve, "Resolve Problem at exact revision");
  await pause(200);
  if (document.querySelector(".task-detail [role='alert']"))
    throw new Error("Exact explicit Problem resolution was rejected");
  const revised = await api<{ problemRevision: number }>(
    `/problems/${link.problemId}/revisions`,
    "POST",
    { statement: "Resolution must remain explicit, revised" },
  );
  if (revised.problemRevision !== 2)
    throw new Error("Problem revision did not preserve the explicit resolution history");
  const stale = await window.llmWikiApplication.request({
    path: `/problems/${link.problemId}/resolutions`,
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      operationId: crypto.randomUUID(),
      expectedProblemRevision: 1,
      rationale: "stale evidence must be rejected",
      evidenceRefs: ["Task completion evidence"],
    }),
  });
  if (stale.ok || stale.status !== 409)
    throw new Error("Stale Problem resolution was accepted");
  const stalePayload = await stale.json<{ detail?: string }>();
  if (!stalePayload.detail?.includes("head_conflict"))
    throw new Error("Stale Problem resolution omitted its conflict detail");
  const exact = await api<{ state: string; problemRevision: number }>(
    `/problems/${link.problemId}/resolutions`,
    "POST",
    {
      expectedProblemRevision: 2,
      rationale: "Evidence remains attached to the explicit decision",
      evidenceRefs: ["Task completion evidence"],
    },
  );
  if (exact.state !== "resolved" || exact.problemRevision !== 2)
    throw new Error("Exact Problem resolution did not preserve its expected revision");
  await step(
    "Task completion left its Problem separate; the rendered exact-resolution control succeeded, explicit evidence resolved revision 2, and stale revision 1 was rejected.",
  );
}
async function persistence() {
  const title = `persist ${Date.now()}`;
  await create(title);
  const saved = await task(title);
  await report({
    status: "relaunch",
    steps: [`Created Task ${saved.id} before relaunch.`],
    error: null,
    capture: saved.id,
  });
}
async function restored(id: string, steps: string[]) {
  try {
    await workbench();
    if (!(await api<Task>(`/tasks/${id}`)).title.startsWith("persist "))
      throw new Error("Task identity did not survive relaunch");
    await report({
      status: "passed",
      steps: [...steps, "Task persisted across a full packaged-app relaunch."],
      error: null,
    });
  } catch (error) {
    await report({ status: "failed", steps, error: String(error) });
  }
}
async function restoredRefinement(token: string, steps: string[]) {
  try {
    await workbench();
    const restored = JSON.parse(token) as { captureId: string; noteText: string; messageIds: string[] };
    if (!restored.captureId || !restored.noteText || restored.messageIds.length !== 2)
      throw new Error("Invalid refinement relaunch state");
    const snapshot = await api<RefinementSnapshot>(`/captures/${encodeURIComponent(restored.captureId)}/refinement`);
    if (snapshot.inputDraft !== restored.noteText)
      throw new Error("Refinement workspace did not survive the packaged-app relaunch");
    if (JSON.stringify(snapshot.messages?.map(message => message.id) ?? []) !== JSON.stringify(restored.messageIds))
      throw new Error("Refinement message identities changed across the packaged-app relaunch");
    click(`[data-entity-id="${CSS.escape(restored.captureId)}"]:is([data-control="task-card-refine"],[data-control="task-shortcut-refine"])`, "Reopen persisted refinement after relaunch");
    await waitFor(() => Boolean(document.querySelector(".refinement-panel")), "restored refinement panel");
    await waitFor(() => Boolean(document.querySelector(".refinement-panel .refinement-messages")), "rendered relaunch conversation");
    await waitFor(() => document.querySelector<HTMLTextAreaElement>('[data-control="refinement-note"]')?.value === snapshot.inputDraft, "rendered relaunch note");
    await report({ status: "passed", steps: [...steps, "The note, conversation, and proposal preview restored in a second packaged-app process."], error: null });
  } catch (error) {
    await report({ status: "failed", steps, error: String(error) });
  }
}
async function localization(step: Step) {
  const locale = document.querySelector<HTMLSelectElement>("#locale-select");
  if (!locale) throw new Error("Missing locale selector");
  const longTitle = `A long localized Workbench title that must preserve the shortcut action at every supported desktop size https://example.test/${"a".repeat(120)}-${Date.now()}`;
  await create("Pending localization capture", "capture");
  await create(longTitle, "task");
  const created = [...document.querySelectorAll<HTMLElement>(".canonical-card")].find((card) => card.textContent?.includes(longTitle));
  const open = created?.querySelector<HTMLElement>('[data-control="task-card-open"]');
  if (!open) throw new Error("Missing long-title Task card");
  clickElement(open, "Open long-title Task");
  await taskDetailIdle("starting long-title Task");
  click('[data-control="task-transition-start"]', "Start long-title Task");
  await waitFor(() => document.querySelector(".task-detail")?.getAttribute("data-task-state") === "in_progress", "started long-title Task");
  await selectTaskTab("Details", "Open long-title details");
  await editTaskDefinition();
  const titleInput = document.querySelector<HTMLTextAreaElement>('[data-control="task-revision-title"]');
  const detailPanel = document.querySelector<HTMLElement>(".task-detail");
  if (!titleInput || !detailPanel || titleInput.getBoundingClientRect().width < detailPanel.getBoundingClientRect().width * 0.7)
    throw new Error("Task title input did not use the available detail width");
  const detailRect = detailPanel.getBoundingClientRect();
  const detailHeading = detailPanel.querySelector<HTMLElement>("h2")?.getBoundingClientRect();
  const detailClose = detailPanel.querySelector<HTMLElement>('[data-control="task-detail-close"]')?.getBoundingClientRect();
  if (!detailHeading || !detailClose || detailHeading.left < detailRect.left || detailHeading.right > detailRect.right || detailClose.left < detailRect.left || detailClose.right > detailRect.right || document.documentElement.scrollWidth > document.documentElement.clientWidth + 1)
    throw new Error("Long unbroken Task title overflowed the detail heading or displaced its close control");
  click('[data-control="task-detail-close"]', "Close long-title Task detail");
  await waitFor(() => !document.querySelector(".task-detail"), "closed long-title Task detail");
  const assertGeometry = async (width: number, height: number) => {
    const nativeSize = await invoke<{ windowWidth: number; windowHeight: number; requestedWidth: number; requestedHeight: number }>("desktop_e2e_resize_window", { width, height });
    let measured = { innerWidth: window.innerWidth, innerHeight: window.innerHeight, outerWidth: window.outerWidth, outerHeight: window.outerHeight, clientWidth: document.documentElement.clientWidth, clientHeight: document.documentElement.clientHeight };
    let previous = "";
    let stableReads = 0;
    for (let attempt = 0; attempt < 200; attempt += 1) {
      measured = { innerWidth: window.innerWidth, innerHeight: window.innerHeight, outerWidth: window.outerWidth, outerHeight: window.outerHeight, clientWidth: document.documentElement.clientWidth, clientHeight: document.documentElement.clientHeight };
      const fingerprint = Object.values(measured).join("×");
      stableReads = fingerprint === previous ? stableReads + 1 : 0;
      previous = fingerprint;
      if (Math.abs(measured.innerWidth - nativeSize.windowWidth) <= 2 && Math.abs(measured.clientWidth - measured.innerWidth) <= 2 && Math.abs(measured.clientHeight - measured.innerHeight) <= 2 && measured.innerHeight > 0 && measured.innerHeight <= nativeSize.windowHeight + 2 && stableReads >= 1) break;
      await pause(50);
    }
    if (Math.abs(measured.innerWidth - nativeSize.windowWidth) > 2 || Math.abs(measured.clientWidth - measured.innerWidth) > 2 || Math.abs(measured.clientHeight - measured.innerHeight) > 2 || measured.innerHeight <= 0 || measured.innerHeight > nativeSize.windowHeight + 2 || stableReads < 1)
      throw new Error(`Browser metrics did not settle after native window resize: requested native window ${nativeSize.requestedWidth}×${nativeSize.requestedHeight}, native window ${nativeSize.windowWidth}×${nativeSize.windowHeight}, WebKit inner ${measured.innerWidth}×${measured.innerHeight}, outer ${measured.outerWidth}×${measured.outerHeight}, document client ${measured.clientWidth}×${measured.clientHeight}`);
    const card = [...document.querySelectorAll<HTMLElement>(".workbench-active .canonical-card")].find((item) => item.textContent?.includes(longTitle));
    const board = document.querySelector<HTMLElement>(".workbench-board");
    const action = card?.querySelector<HTMLElement>('[data-control="task-card-open"]');
    const title = card?.querySelector<HTMLElement>("h3");
    if (!card || !board || !action || !title) throw new Error(`Missing active Task geometry at ${width}×${height}`);
    const cardRect = card.getBoundingClientRect(), actionRect = action.getBoundingClientRect(), titleRect = title.getBoundingClientRect();
    if (titleRect.left < cardRect.left || titleRect.right > cardRect.right || actionRect.right > cardRect.right || cardRect.bottom > board.getBoundingClientRect().top || document.documentElement.scrollWidth > document.documentElement.clientWidth + 1)
      throw new Error(`Active Task overflowed its card or workflow lanes at ${width}×${height}`);
    const lanes = [...board.querySelectorAll<HTMLElement>("[data-lane]")];
    if (lanes.map(lane => lane.dataset.lane).join(",") !== "inbox,refining,tasks") throw new Error("Workflow lane order changed");
    if (board.textContent?.includes(longTitle)) throw new Error("Active Task was duplicated in the workflow lanes");
    const rects = lanes.map(lane => lane.getBoundingClientRect());
    const wide = document.querySelector(".workbench-main")!.getBoundingClientRect().width > 700;
    if (wide ? !(rects[0].right <= rects[1].left && rects[1].right <= rects[2].left) : !(rects[0].bottom <= rects[1].top && rects[1].bottom <= rects[2].top))
      throw new Error(`Workflow lanes overlap at ${width}×${height}`);
    return { nativeSize, measured };
  };
  const englishCompact = await assertGeometry(900, 640);
  await step(`Geometry requested native window ${englishCompact.nativeSize.windowWidth}×${englishCompact.nativeSize.windowHeight}; WebKit viewport ${englishCompact.measured.innerWidth}×${englishCompact.measured.innerHeight}, browser outer ${englishCompact.measured.outerWidth}×${englishCompact.measured.outerHeight}.`);
  locale.value = "ko";
  locale.dispatchEvent(new Event("change", { bubbles: true }));
  await waitFor(() => document.documentElement.lang === "ko", "Korean");
  const koreanCompact = await assertGeometry(904, 768);
  await step(`Geometry requested native window ${koreanCompact.nativeSize.windowWidth}×${koreanCompact.nativeSize.windowHeight}; WebKit viewport ${koreanCompact.measured.innerWidth}×${koreanCompact.measured.innerHeight}, browser outer ${koreanCompact.measured.outerWidth}×${koreanCompact.measured.outerHeight}.`);
  const settings = document.querySelector<HTMLElement>('[data-view="ai-setup"]');
  if (!settings) throw new Error("Missing AI setup navigation");
  clickElement(settings, "Open Korean AI setup");
  await waitFor(() => document.body.textContent?.includes("API 키는 이 기기의 로컬 설정 파일에만 저장됩니다. Vault나 앱 데이터베이스에는 저장되지 않습니다.") === true, "Korean privacy copy");
  locale.value = "en";
  locale.dispatchEvent(new Event("change", { bubbles: true }));
  await waitFor(() => document.documentElement.lang === "en", "English");
  click('[data-view="workbench"]', "Return to Workbench");
  const englishWide = await assertGeometry(1280, 820);
  await step(`Geometry requested native window ${englishWide.nativeSize.windowWidth}×${englishWide.nativeSize.windowHeight}; WebKit viewport ${englishWide.measured.innerWidth}×${englishWide.measured.innerHeight}, browser outer ${englishWide.measured.outerWidth}×${englishWide.measured.outerHeight}.`);
  await detail(longTitle);
  for (const width of [1200, 900]) {
    const size = await invoke<{ windowWidth: number }>("desktop_e2e_resize_window", { width, height: 820 });
    await waitFor(() => Math.abs(window.innerWidth - size.windowWidth) < 3, "settled Task modal resize");
    await waitFor(() => {
      const layer = document.querySelector<HTMLElement>(".task-detail-modal-layer");
      const panel = document.querySelector<HTMLElement>(".task-detail[role='dialog']");
      const app = document.querySelector<HTMLElement>(".app");
      if (!layer || !panel || !app?.inert) return false;
      const rect = panel.getBoundingClientRect();
      return getComputedStyle(layer).position === "fixed" && panel.getAttribute("aria-modal") === "true"
        && rect.width > 0 && rect.height > 0 && rect.left >= -1 && rect.top >= -1
        && rect.right <= window.innerWidth + 1 && rect.bottom <= window.innerHeight + 1
        && getComputedStyle(panel).overflowY === "auto";
    }, "bounded Task detail modal with inert background and independent scrolling");
  }
  await invoke("desktop_e2e_resize_window", { width: 1280, height: 820 });
  await taskDetailIdle("opening Task refinement dialog");
  const retainedTask = document.querySelector(".task-detail");
  const refineTask = document.querySelector<HTMLElement>('[data-control="task-detail-refine"]');
  if (!refineTask) throw new Error("Missing Task refinement action");
  await prepareClick(refineTask, "Open Task refinement dialog");
  clickElement(refineTask, "Open Task refinement dialog");
  await waitFor(() => Boolean(document.querySelector(".refinement-panel[data-refinement-session]:not([data-refinement-session=''])")), "loaded refinement dialog");
  for (const width of [1200, 900]) {
    const size = await invoke<{ windowWidth: number }>("desktop_e2e_resize_window", { width, height: 820 });
    await waitFor(() => Math.abs(window.innerWidth - size.windowWidth) < 3, "refinement window resized");
    await waitFor(() => {
      const layer = document.querySelector<HTMLElement>(".refinement-modal-layer");
      const backdrop = document.querySelector<HTMLElement>(".refinement-modal-backdrop");
      const refinement = document.querySelector<HTMLElement>(".refinement-panel[role='dialog']");
      const preview = document.querySelector<HTMLElement>(".refinement-preview");
      const conversation = document.querySelector<HTMLElement>(".refinement-conversation");
      const messages = document.querySelector<HTMLElement>(".refinement-messages");
      const composer = document.querySelector<HTMLElement>(".refinement-composer");
      const app = document.querySelector<HTMLElement>(".app");
      if (!layer || !backdrop || !refinement || !preview || !conversation || !messages || !composer || !app || !app.inert) return false;
      const panel = refinement.getBoundingClientRect(), left = preview.getBoundingClientRect(), right = conversation.getBoundingClientRect(), compose = composer.getBoundingClientRect();
      const withinViewport = panel.left >= -1 && panel.top >= -1 && panel.right <= window.innerWidth + 1 && panel.bottom <= window.innerHeight + 1;
      const columns = left.width > 0 && right.width > 0 && left.right <= right.left && compose.width > 0 && compose.height > 0 && Math.abs(compose.bottom - right.bottom) <= 1;
      const scrollable = getComputedStyle(preview).overflowY !== "visible" && getComputedStyle(messages).overflowY !== "visible" && preview !== messages;
      return getComputedStyle(layer).position === "fixed" && getComputedStyle(backdrop).position === "absolute" && withinViewport && columns && scrollable;
    }, "bounded modal with side-by-side preview, visible composer, and independent scrolling panes");
  }
  const closeRefinement = document.querySelector<HTMLElement>('[data-control="refinement-close"]');
  if (!closeRefinement) throw new Error("Missing refinement return action");
  await waitFor(() => closeRefinement.getAttribute("aria-label") === "Close refinement" && document.querySelector(".refinement-panel")?.getAttribute("data-refinement-polling") === "false", "ready to return from refinement");
  await prepareClick(closeRefinement, "Return from refinement");
  clickElement(closeRefinement, "Return from refinement");
  await waitFor(() => !document.querySelector(".refinement-panel"), "returned from refinement");
  if (document.querySelector(".task-detail") !== retainedTask) throw new Error("Refinement remounted Task context");
  await step("Task refinement opened as a bounded modal with an inert Workbench, side-by-side preview and chat panes, a visible composer, and independent scroll containers at 1200px and 900px; it retained the original Task context on return.");
  click('[data-control="task-detail-close"]', "Close responsive Task detail");
  await waitFor(() => !document.querySelector(".task-detail"), "returned to responsive Workbench");
  await step("Task detail remained a bounded, independently scrollable modal with an inert Workbench at 1200px and 900px.");
  await step(
    "Korean and English long-title active Tasks kept their action inside the card and above the responsive workflow lanes at the recorded native-window and WebKit viewport sizes; the Task title editor used the panel width and Korean AI privacy copy was exact.",
  );
}

type Step = (message: string) => Promise<void>;
export function installDesktopScenario() {
  if (!window.__TAURI_INTERNALS__) return;
  void desktopE2eMode().then(async (state) => {
    if (!state) return;
    const introSurface = new URLSearchParams(window.location.search).get("surface") === "first-run-intro";
    const introScenario = state.scenario.startsWith("global-intro-");
    if (introSurface !== introScenario) return;
    e2eProviderUrl = state.providerUrl;
    if (state.restoreCapture)
      return state.scenario === "task-refinement-relaunch"
        ? restoredRefinement(state.restoreCapture, state.restoreSteps)
        : restored(state.restoreCapture, state.restoreSteps);
    const steps: string[] = [];
    const step: Step = async (message) => {
      steps.push(message);
      await reportDesktopE2eProgress(steps);
    };
    const coverage = createInteractionCoverage();
    try {
      const startupScenario = introScenario || state.scenario.startsWith("global-vault-") || state.scenario.startsWith("global-migration-");
      if (!startupScenario) await workbench();
      const scenarioHarness = {
        coverage,
        create,
        detail,
        task: async (title: string) => (await task(title)) as WorkbenchTask,
        api,
        waitFor,
        waitForAsync,
        click: clickElement,
        prepareClick,
        enter,
        step,
        request: api,
      };
      const activateView = async (view: string) => {
        click(`[data-control="sidebar-view-${view}"]`, `Open ${view}`);
        await waitFor(() => document.getElementById(view)?.classList.contains("active") ?? false, `${view} view`);
      };
      const closeLegacyChat = async (label: string) => {
        coverage.observe(document.getElementById("chat-modal")!, state.scenario);
        const close = document.getElementById("chat-close")!;
        coverage.interact("chat-close", () => clickElement(close, label));
        await waitFor(() => !(document.getElementById("chat-modal") as HTMLDialogElement).open, "closed legacy chat");
        coverage.assertEffect("chat-close", () => !(document.getElementById("chat-modal") as HTMLDialogElement).open);
      };
      const cases: Record<string, (step: Step) => Promise<void>> = {
        "task-capture": capture,
        "task-worklog": step => log(step, coverage),
        "task-refinement": refinement,
        "task-relationships": relationships,
        "task-review": review,
        "task-publication": publication,
        "task-problem-resolution": problemResolution,
        "task-persistence": persistence,
        "task-localization": localization,
        "task-controls": () => runTaskControlMatrixScenario(scenarioHarness),
        "task-workbench-retry": () => runWorkbenchRetryScenario(scenarioHarness),
        "task-legacy-refinement": async () => {
          await runLegacyProblemRefinementScenario(scenarioHarness);
          await closeLegacyChat("Close migrated Problem refinement");
        },
        "task-legacy-chat-controls": async () => {
          await runLegacyProblemRefinementScenario(scenarioHarness);
          await runLegacyChatTrackingScenario({ ...scenarioHarness, providerUrl: e2eProviderUrl });
          await closeLegacyChat("Close migrated Problem chat");
        },
        "task-legacy-preview-retry": async () => {
          await invoke("desktop_e2e_arm_one_shot_failure", { operation: "refinement.context" });
          await runLegacyProblemRefinementScenario(scenarioHarness);
          await runLegacyPreviewWarningRetryScenario(scenarioHarness);
          await closeLegacyChat("Close recovered migrated Problem refinement");
        },
        "task-chat-controls": () =>
          runTaskChatScenario({ ...scenarioHarness, providerUrl: e2eProviderUrl }),
        "task-refinement-provider-recovery": () =>
          runRefinementProviderRecoveryScenario({ ...scenarioHarness, providerUrl: e2eProviderUrl }),
        "task-refinement-close-retry": () =>
          runRefinementCloseRetryScenario({ ...scenarioHarness, providerUrl: e2eProviderUrl }),
        "task-refinement-close-pending": () =>
          runRefinementClosePendingScenario({ ...scenarioHarness, providerUrl: e2eProviderUrl }),
        "task-refinement-relaunch": async () => {
          const captureId = await prepareRefinementRelaunchScenario({ ...scenarioHarness, providerUrl: e2eProviderUrl });
          await report({ status: "relaunch", steps, error: null, capture: captureId, coverage: coverage.report() });
        },
        "task-mcp-continuation": taskMcpContinuation,
        "global-shell": () => runShellNavigationScenario(scenarioHarness),
        "global-search": async () => {
          await activateView("search");
          await invoke("desktop_e2e_arm_one_shot_failure", { operation: "knowledge.read" });
          await runSearchScenario(scenarioHarness, "Startup indexing");
        },
        "global-compass": async () => {
          await activateView("compass");
          await runCompassGoalScenario(scenarioHarness, `Coverage goal ${Date.now()}`);
        },
        "global-provider": async () => {
          await activateView("ai-setup");
          await runProviderSettingsScenario(scenarioHarness, {
            url: e2eProviderUrl,
            model: "deterministic-test-model",
            advancedModel: "deterministic-test-model",
            key: "desktop-e2e-key",
          });
        },
        "global-mcp": async () => {
          await activateView("ai-setup");
          await runMcpSettingsScenario(scenarioHarness, `coverage-${Date.now()}`, "coverage-topic");
        },
        "global-queue-notifications": () =>
          runQueueNotificationActionsScenario(scenarioHarness, () =>
            invoke("desktop_e2e_seed_queue_notifications"), e2eProviderUrl,
          ),
        "global-notice": () => runNoticeScenario(scenarioHarness),
        "global-intro-navigation": () => runFirstRunIntroNavigationScenario(scenarioHarness),
        "global-intro-retry": () => runFirstRunIntroRetryScenario(scenarioHarness),
        "global-vault-choose": () => runVaultChooseScenario(scenarioHarness),
        "global-vault-retry": () => runVaultRetryScenario(scenarioHarness),
        "global-migration-restore": () => runMigrationRestoreScenario(scenarioHarness),
        "global-migration-retry": () => runMigrationRetryScenario(scenarioHarness),
      };
      const run = cases[state.scenario];
      if (!run)
        throw new Error(`Unknown Task desktop scenario ${state.scenario}`);
      await run(step);
      if (!["task-persistence", "task-refinement-relaunch"].includes(state.scenario))
        await report({ status: "passed", steps, error: null, coverage: coverage.report() });
    } catch (error) {
      await report({
        status: "failed",
        steps,
        error: error instanceof Error ? error.message : String(error),
        coverage: coverage.report(),
      });
    }
  });
}
