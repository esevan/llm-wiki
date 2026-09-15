import type { InteractionCoverage } from "./interactionCoverage";
import { invoke } from "@tauri-apps/api/core";

type RefinementMessage = { id: string; role: string; body: string };
type RefinementSnapshot = { id: string; previewStatus?: string; previewJobId?: string; inputDraft?: string; messages?: RefinementMessage[]; responseStatus?: "queued" | "running" | "completed" | "failed" | "cancelled" };
type ProviderRequest = { messages: Array<{ role: string; content: string }> };

/** Desktop primitives supplied by the central scenario registry. */
export type ChatScenarioHarness = {
  create: (title: string, kind?: "capture" | "task") => Promise<void>;
  detail: (title: string) => Promise<void>;
  api: <T>(path: string, method?: "GET" | "POST" | "PUT" | "DELETE", body?: Record<string, unknown>) => Promise<T>;
  waitFor: (check: () => boolean, label: string) => Promise<void>;
  waitForAsync: (check: () => Promise<boolean>, label: string) => Promise<void>;
  click: (element: HTMLElement, label: string) => void;
  prepareClick: (element: HTMLElement, label: string) => Promise<void>;
  enter: (element: HTMLInputElement | HTMLTextAreaElement, value: string) => void;
  step: (message: string) => Promise<void>;
  providerUrl: string;
  coverage?: InteractionCoverage;
};

/** Kept beside the packaged flows so each exercised control has an observable effect. */
export const CHAT_CONTROL_EFFECTS = [
  ["F31.refinement-close", "[data-control=refinement-close]", "workspace PUT succeeds before the panel closes"],
  ["F33.refinement-send", "[data-control=refinement-send]", "one persisted user turn produces a terminal provider response"],
  ["F34.refinement-note", "[data-control=refinement-note]", "draft survives close and reopen"],
  ["F35.refinement-proposal-edit/accept/reject", ".proposal [data-control]", "each explicit decision removes only its reviewed proposal; accept creates a Task"],
  ["F44.chat-track-toggle", "[data-control=chat-track-toggle]", "tracking attaches to the rendered legacy Problem chat"],
  ["F45.chat-propose", "[data-control=chat-propose]", "Task proposal card appears from the selected current Problem"],
  ["F46.tracking-accept/reject/edit", "[data-control^=tracking-]", "accept persists Task/provenance; reject removes the reviewed card; edit preserves preview"],
  ["F47.tracking-json-edit/cancel/preview", "[data-control^=tracking-json]", "invalid JSON alerts, cancel discards, valid JSON previews"],
  ["F48.legacy feature/draft/manual forms", "#feature-modal, #draft-modal, #manual-modal", "not reached from the migrated Problem-only Task bridge; retained as an explicit unsupported legacy route"],
  ["F49.chat Ask/Enter/Close", "#chat-form, #chat-close", "real legacy stream creates checkpoint cards and close ends the surface"],
] as const;

function control<T extends HTMLElement>(selector: string, label: string) {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`Missing rendered ${label}`);
  return element;
}

function observeCoverage(harness: Pick<ChatScenarioHarness, "coverage">, root: ParentNode, scenario: string) {
  harness.coverage?.observe(root, scenario);
}

function coverInteract(harness: Pick<ChatScenarioHarness, "coverage">, id: string, action: () => void) {
  // Tracking cards and proposal editors mount between asynchronous responses.
  // Audit the actual current DOM at each action rather than a prior snapshot.
  harness.coverage?.observe(document, "chat-interaction", id);
  if (harness.coverage) harness.coverage.interact(id, action);
  else action();
}

function coverEffect(harness: Pick<ChatScenarioHarness, "coverage">, id: string, effect: () => boolean) {
  harness.coverage?.assertEffect(id, effect);
}

async function providerRequests(providerUrl: string): Promise<ProviderRequest[]> {
  const evidence = await invoke<{ requests: ProviderRequest[] }>("desktop_e2e_provider_requests", { baseUrl: providerUrl });
  return evidence.requests;
}

async function openCaptureRefinement(harness: ChatScenarioHarness, captureText: string) {
  const snapshot = await harness.api<{ categories: Array<{ items: Array<{ kind: string; id: string; text?: string }> }> }>("/workbench");
  const capture = snapshot.categories.flatMap(category => category.items).find(item => item.kind === "capture" && item.text === captureText);
  if (!capture) throw new Error("Saved Capture was not present before opening refinement");
  const button = control<HTMLButtonElement>(`[data-entity-id="${CSS.escape(capture.id)}"][data-control="task-card-refine"]`, "Capture refinement entry");
  harness.click(button, "Open Capture refinement");
  await harness.waitFor(() => Boolean(document.querySelector('.refinement-panel[data-refinement-session]:not([data-refinement-session=""])')), "loaded Capture refinement panel");
  return capture.id;
}

async function waitForTerminal(harness: ChatScenarioHarness, sessionPath: string, expected: "completed" | "failed") {
  await harness.waitForAsync(async () => (await harness.api<RefinementSnapshot>(sessionPath)).responseStatus === expected, `refinement ${expected}`);
}

const refinementPath = (captureId: string) => `/captures/${encodeURIComponent(captureId)}/refinement`;

async function closeRefinement(harness: ChatScenarioHarness, label: string) {
  const close = control<HTMLButtonElement>('[data-control="refinement-close"]', "refinement close");
  coverInteract(harness, "refinement-close", () => harness.click(close, label));
}

async function send(harness: ChatScenarioHarness, message: string, keyboard = false) {
  const input = control<HTMLTextAreaElement>('[data-control="refinement-message"]', "refinement message");
  coverInteract(harness, "refinement-message", () => harness.enter(input, message));
  const sendButton = control<HTMLButtonElement>('[data-control="refinement-send"]', "refinement Send");
  await harness.waitFor(() =>
    !sendButton.disabled &&
    control<HTMLElement>(".refinement-panel", "refinement panel").dataset.refinementMessageLength === String(message.length),
  "committed refinement message before send");
  if (keyboard) coverInteract(harness, "refinement-send", () => {
    input.focus();
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", code: "Enter", ctrlKey: true, bubbles: true, cancelable: true }));
  });
  else coverInteract(harness, "refinement-send", () => harness.click(sendButton, "Send refinement message"));
}

/**
 * Packaged F31–F35 exercise. All state-changing operations use rendered UI;
 * native calls only inspect persistence and configure the local fake provider.
 */
export async function runTaskChatScenario(harness: ChatScenarioHarness): Promise<void> {
  const captureText = `Existing Solution preserved ${Date.now()}`;
  await harness.api("/provider/config", "PUT", { base_url: harness.providerUrl, model: "deterministic-slow-preview", api_key: "desktop-e2e-key" });
  await harness.create(captureText, "capture");
  const captureId = await openCaptureRefinement(harness, captureText);
  const refinementPanel = control<HTMLElement>(".refinement-panel", "Capture refinement panel");
  observeCoverage(harness, refinementPanel, "task-chat-controls");
  const sessionPath = `/captures/${encodeURIComponent(captureId)}/refinement`;
  await harness.waitFor(() => Boolean(refinementPanel.dataset.refinementSession), "opened native refinement session");
  const turns = ["Keep the existing Solution and identify only missing evidence.", "Correction: preserve the validation criteria exactly.", "Offer a reviewable Task proposal only; do not apply it."];
  for (let index = 0; index < turns.length; index += 1) {
    await send(harness, turns[index], true);
    if (index === 0) control<HTMLTextAreaElement>('[data-control="refinement-message"]', "refinement message").dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", ctrlKey: true, bubbles: true, cancelable: true }));
    await harness.waitForAsync(async () => {
      const snapshot = await harness.api<RefinementSnapshot>(sessionPath);
      return (snapshot.messages?.filter(message => message.role === "user").length ?? 0) === index + 1;
    }, `persisted refinement user turn ${index + 1}`);
    await harness.waitFor(() => refinementPanel.dataset.refinementPolling === "true", `rendered polling refinement turn ${index + 1}`);
    await waitForTerminal(harness, sessionPath, "completed");
    await harness.waitForAsync(async () => {
      const snapshot = await harness.api<RefinementSnapshot>(sessionPath);
      return (snapshot.messages?.filter(message => message.role === "assistant").length ?? 0) === index + 1;
    }, `persisted refinement assistant turn ${index + 1}`);
    await harness.waitFor(() => refinementPanel.dataset.refinementSending === "false", `refinement sender settled turn ${index + 1}`);
    if (index < turns.length - 1) {
      await harness.waitFor(() => document.querySelectorAll(".refinement-messages .message-assistant").length === index + 1, "chat visible before preview completion");
      const snapshot = await harness.api<RefinementSnapshot>(sessionPath);
      if (!["queued", "running"].includes(snapshot.previewStatus ?? "")) throw new Error("Slow preview must remain independently queued while the next chat is available");
    } else {
      await harness.waitFor(() => refinementPanel.dataset.refinementPolling === "false", `rendered terminal refinement turn ${index + 1}`);
    }
  }
  await harness.waitFor(() => document.querySelectorAll(".refinement-messages .message-assistant").length >= 3, "three rendered assistant turns");
  coverEffect(harness, "refinement-message", () => document.querySelectorAll(".refinement-messages .message-user").length === 3);
  coverEffect(harness, "refinement-send", () => document.querySelectorAll(".refinement-messages .message-assistant").length >= 3);
  const completed = await harness.api<RefinementSnapshot>(sessionPath);
  const userTurns = completed.messages?.filter(message => message.role === "user").map(message => message.body) ?? [];
  if (JSON.stringify(userTurns) !== JSON.stringify(turns)) throw new Error("Native refinement session did not retain all ordered user turns");
  const finalPrompt = (await providerRequests(harness.providerUrl)).at(-1)?.messages?.[0]?.content ?? "";
  for (const retained of [captureText, ...turns, "I will use those details to update the preview in the background."]) if (!finalPrompt.includes(retained)) throw new Error(`Provider request omitted prior conversation context: ${retained}`);

  const noteDetails = control<HTMLDetailsElement>('[data-control="refinement-note-details"]', "saved refinement note disclosure");
  await harness.prepareClick(noteDetails.querySelector("summary")!, "Open saved refinement note");
  coverInteract(harness, "refinement-note-details", () => harness.click(noteDetails.querySelector("summary")!, "Open saved refinement note"));
  await harness.waitFor(() => noteDetails.open, "saved refinement note disclosure opened");
  coverEffect(harness, "refinement-note-details", () => noteDetails.open);
  const note = control<HTMLTextAreaElement>('[data-control="refinement-note"]', "saved refinement note");
  coverInteract(harness, "refinement-note", () => harness.enter(note, "Unsent note survives close and reopen"));
  await harness.waitFor(() => Boolean(document.querySelector(".refinement-preview .proposal")), "reviewable proposal");
  observeCoverage(harness, refinementPanel, "task-chat-controls-proposal-ready");
  const proposal = control<HTMLElement>(".proposal", "proposal card");
  const editProposal = control<HTMLButtonElement>("[data-control=refinement-proposal-edit]", "proposal edit");
  await harness.prepareClick(editProposal, "Edit proposal");
  coverInteract(harness, "refinement-proposal-edit", () => harness.click(editProposal, "Edit proposal"));
  await harness.waitFor(() => Boolean(document.querySelector("[aria-label^='Edit ']")), "rendered proposal editor");
  observeCoverage(harness, refinementPanel, "task-chat-controls-proposal-editor");
  const proposalEditor = control<HTMLTextAreaElement>("[aria-label='Edit Title']", "proposal title editor");
  coverInteract(harness, "refinement-proposal-editor", () => harness.enter(proposalEditor, "Edited deterministic Task"));
  coverEffect(harness, "refinement-proposal-edit", () => Boolean(document.querySelector("[aria-label^='Edit ']")));
  coverEffect(harness, "refinement-proposal-editor", () => proposalEditor.value === "Edited deterministic Task");
  const acceptProposal = control<HTMLButtonElement>("[data-control=refinement-proposal-accept]", "proposal accept");
  await harness.prepareClick(acceptProposal, "Accept edited proposal");
  coverInteract(harness, "refinement-proposal-accept", () => harness.click(acceptProposal, "Accept edited proposal"));
  await harness.waitFor(() => !proposal.isConnected, "accepted proposal removed from pending review");
  coverEffect(harness, "refinement-proposal-accept", () => !proposal.isConnected);
  const accepted = await harness.api<{ categories: Array<{ items: Array<{ kind: string; title?: string }> }> }>("/workbench");
  if (!accepted.categories.flatMap(category => category.items).some(item => item.kind === "task" && item.title === "Edited deterministic Task")) throw new Error("Accepted refinement proposal did not create its edited Task");

  coverInteract(harness, "refinement-close", () => harness.click(control<HTMLButtonElement>("[data-control=refinement-close]", "refinement close"), "Close refinement after saving workspace"));
  await harness.waitFor(() => !document.querySelector(".refinement-panel"), "closed refinement panel");
  coverEffect(harness, "refinement-close", () => !document.querySelector(".refinement-panel"));
  await openCaptureRefinement(harness, captureText);
  observeCoverage(harness, control<HTMLElement>(".refinement-panel", "reopened Capture refinement panel"), "task-chat-controls");
  const reopenedNoteDetails = control<HTMLDetailsElement>('[data-control="refinement-note-details"]', "restored saved refinement note disclosure");
  await harness.prepareClick(reopenedNoteDetails.querySelector("summary")!, "Open restored saved refinement note");
  coverInteract(harness, "refinement-note-details", () => harness.click(reopenedNoteDetails.querySelector("summary")!, "Open restored saved refinement note"));
  await harness.waitFor(() => reopenedNoteDetails.open, "restored saved refinement note disclosure opened");
  await harness.waitFor(() => document.querySelector<HTMLTextAreaElement>("[data-control=refinement-note]")?.value === "Unsent note survives close and reopen", "restored saved note");
  coverEffect(harness, "refinement-note", () => control<HTMLTextAreaElement>("[data-control=refinement-note]", "restored refinement note").value === "Unsent note survives close and reopen");

  await harness.api("/provider/config", "PUT", { base_url: harness.providerUrl, model: "deterministic-failure-once", api_key: "desktop-e2e-key" });
  await send(harness, "Retry after a deterministic provider failure.");
  await waitForTerminal(harness, sessionPath, "failed");
  await harness.waitFor(() => Boolean(document.querySelector(".refinement-panel [role=alert]")), "visible refinement failure");
  await send(harness, "Retry after a deterministic provider failure.");
  await waitForTerminal(harness, sessionPath, "completed");
  await harness.waitFor(() => Boolean(document.querySelector(".proposal")), "retry proposal");
  const rejected = control<HTMLElement>(".proposal", "proposal to reject");
  const rejectProposal = control<HTMLButtonElement>("[data-control=refinement-proposal-reject]", "proposal reject");
  await harness.prepareClick(rejectProposal, "Reject proposal");
  coverInteract(harness, "refinement-proposal-reject", () => harness.click(rejectProposal, "Reject proposal"));
  await harness.waitFor(() => !rejected.isConnected, "rejected proposal removed without application");
  coverEffect(harness, "refinement-proposal-reject", () => !rejected.isConnected);
  await harness.step("Packaged Capture refinement retained three turns through the native provider, preserved an existing Solution, accepted one edited proposal, rejected a later proposal, restored an unsent note after close/reopen, and retried a provider failure through rendered controls.");
}

/** Provider absence is a recoverable terminal refinement failure, surfaced in the panel. */
export async function runRefinementProviderRecoveryScenario(harness: ChatScenarioHarness): Promise<void> {
  const captureText = `Provider recovery ${Date.now()}`;
  await harness.create(captureText, "capture");
  const captureId = await openCaptureRefinement(harness, captureText);
  const sessionPath = refinementPath(captureId);
  const panel = control<HTMLElement>(".refinement-panel", "Capture refinement panel");
  observeCoverage(harness, panel, "task-refinement-provider-recovery");
  await send(harness, "Show the provider-absence failure without losing this turn.");
  await waitForTerminal(harness, sessionPath, "failed");
  await harness.waitFor(() => Boolean(panel.querySelector('[role="alert"]')), "visible provider-absence failure");
  const failed = await harness.api<RefinementSnapshot>(sessionPath);
  if (!failed.messages?.some(item => item.role === "user" && item.body.includes("provider-absence")))
    throw new Error("Provider-absence failure did not retain its user turn");
  coverEffect(harness, "refinement-send", () => Boolean(panel.querySelector('[role="alert"]')));

  await harness.api("/provider/config", "PUT", { base_url: harness.providerUrl, model: "deterministic-test-model", api_key: "desktop-e2e-key" });
  await send(harness, "Recover with the configured provider.");
  await waitForTerminal(harness, sessionPath, "completed");
  await harness.waitFor(() => panel.querySelectorAll(".message-assistant").length === 1, "rendered recovered assistant response");
  const recovered = await harness.api<RefinementSnapshot>(sessionPath);
  if (recovered.messages?.filter(item => item.role === "user").length !== 2 || recovered.messages?.filter(item => item.role === "assistant").length !== 1)
    throw new Error("Recovered refinement did not preserve the failed turn and successful response");
  await harness.step("A missing provider produced a visible terminal failure; configuring the isolated provider and sending again preserved the failed turn and rendered the recovered response.");
}

/** The close action remains open on a failed workspace PUT, then retries the same draft durably. */
export async function runRefinementCloseRetryScenario(harness: ChatScenarioHarness): Promise<void> {
  const captureText = `Close retry ${Date.now()}`;
  const noteText = `Workspace survives retry ${Date.now()}`;
  await harness.create(captureText, "capture");
  const captureId = await openCaptureRefinement(harness, captureText);
  const panel = control<HTMLElement>(".refinement-panel", "Capture refinement panel");
  observeCoverage(harness, panel, "task-refinement-close-retry");
  await invoke("desktop_e2e_arm_one_shot_failure", { operation: "task-refinement.workspace" });
  coverInteract(harness, "refinement-note", () => harness.enter(control<HTMLTextAreaElement>('[data-control="refinement-note"]', "refinement note"), noteText));
  await closeRefinement(harness, "Close refinement with one-shot workspace failure");
  await harness.waitFor(() => panel.isConnected && Boolean(panel.querySelector('[role="alert"]')) && control<HTMLTextAreaElement>('[data-control="refinement-note"]', "retained refinement note").value === noteText, "visible close-save failure with panel and draft retained");
  coverEffect(harness, "refinement-close", () => panel.isConnected && Boolean(panel.querySelector('[role="alert"]')) && control<HTMLTextAreaElement>('[data-control="refinement-note"]', "retained refinement note").value === noteText);
  await closeRefinement(harness, "Retry refinement close");
  await harness.waitFor(() => !panel.isConnected, "closed refinement after workspace retry");
  const saved = await harness.api<RefinementSnapshot>(refinementPath(captureId));
  if (saved.inputDraft !== noteText) throw new Error("Successful close retry did not persist the exact workspace note");
  await harness.step("A one-shot native workspace-save failure kept the panel and error visible; the rendered Close retry persisted the exact note before closing.");
  await invoke("desktop_e2e_arm_one_shot_failure", { operation: "task-refinement.open" });
  harness.click(control<HTMLButtonElement>(`[data-entity-id="${CSS.escape(captureId)}"][data-control="task-card-refine"]`, "Capture refinement entry"), "Reopen refinement with failed read");
  await harness.waitFor(() => Boolean(document.querySelector('[data-control="refinement-retry"]')), "visible refinement read retry");
  const messageDraft = "Keep this message while retrying the read";
  harness.enter(control<HTMLTextAreaElement>('[data-control="refinement-message"]', "refinement message"), messageDraft);
  observeCoverage(harness, document, "task-refinement-close-retry");
  coverInteract(harness, "refinement-retry", () => harness.click(control<HTMLButtonElement>('[data-control="refinement-retry"]', "refinement retry"), "Retry failed refinement read"));
  await harness.waitFor(() => !document.querySelector('[data-control="refinement-retry"]') && Boolean(document.querySelector('.refinement-panel[data-refinement-session]:not([data-refinement-session=""])')), "recovered refinement read");
  coverEffect(harness, "refinement-retry", () => control<HTMLTextAreaElement>('[data-control="refinement-message"]', "preserved message").value === messageDraft);
  const recovered = await harness.api<RefinementSnapshot>(refinementPath(captureId));
  if (recovered.inputDraft !== noteText || recovered.messages?.length) throw new Error("Refinement read retry changed saved notes or submitted a message");
  await harness.step("The refinement Retry control recovered a native read failure while preserving the message draft and saved note without submitting a message.");

}

/** Prepare a durable workspace for the runner's real second packaged-app process. */
export async function prepareRefinementRelaunchScenario(harness: ChatScenarioHarness): Promise<string> {
  const captureText = `Relaunch refinement ${Date.now()}`;
  const noteText = `Persisted relaunch workspace ${Date.now()}`;
  await harness.create(captureText, "capture");
  const captureId = await openCaptureRefinement(harness, captureText);
  await harness.api("/provider/config", "PUT", { base_url: harness.providerUrl, model: "deterministic-test-model", api_key: "desktop-e2e-key" });
  const panel = control<HTMLElement>(".refinement-panel", "Capture refinement panel");
  observeCoverage(harness, panel, "task-refinement-relaunch");
  await send(harness, "Persist these exact message identities across relaunch.");
  await waitForTerminal(harness, refinementPath(captureId), "completed");
  await harness.waitFor(() => panel.dataset.refinementPolling === "false", "rendered terminal response before relaunch workspace save");
  coverInteract(harness, "refinement-note", () => harness.enter(control<HTMLTextAreaElement>('[data-control="refinement-note"]', "refinement note"), noteText));
  await harness.waitFor(() => Boolean(document.querySelector(".refinement-preview")), "visible proposal preview before relaunch");
  await closeRefinement(harness, "Persist refinement workspace before relaunch");
  await harness.waitFor(() => !panel.isConnected, "closed persisted refinement before relaunch");
  const saved = await harness.api<RefinementSnapshot>(refinementPath(captureId));
  if (saved.inputDraft !== noteText) throw new Error("Pre-relaunch workspace was not durably saved");
  const messageIds = saved.messages?.map(message => message.id) ?? [];
  if (messageIds.length !== 2 || new Set(messageIds).size !== messageIds.length)
    throw new Error("Pre-relaunch refinement did not persist distinct user and assistant message IDs");
  await harness.step(`Saved refinement workspace for ${captureId} before terminating the first packaged process.`);
  return JSON.stringify({ captureId, noteText, messageIds });
}

/** Closing a pending request detaches polling; the native job completes and appears only on reopen. */
export async function runRefinementClosePendingScenario(harness: ChatScenarioHarness): Promise<void> {
  const captureText = `Pending close ${Date.now()}`;
  await harness.api("/provider/config", "PUT", { base_url: harness.providerUrl, model: "deterministic-timeout", api_key: "desktop-e2e-key" });
  await harness.create(captureText, "capture");
  const captureId = await openCaptureRefinement(harness, captureText);
  const sessionPath = refinementPath(captureId);
  const panel = control<HTMLElement>(".refinement-panel", "Capture refinement panel");
  observeCoverage(harness, panel, "task-refinement-close-pending");
  await send(harness, "Finish this request after the refinement panel closes.");
  await harness.waitForAsync(async () => ["queued", "running"].includes((await harness.api<RefinementSnapshot>(sessionPath)).responseStatus ?? ""), "persisted pending refinement response");
  await closeRefinement(harness, "Close refinement while provider response is pending");
  await harness.waitFor(() => !panel.isConnected, "closed pending refinement panel");
  const otherText = `Pending isolation ${Date.now()}`;
  await harness.create(otherText, "capture");
  await openCaptureRefinement(harness, otherText);
  const otherPanel = control<HTMLElement>(".refinement-panel", "unrelated Capture refinement panel");
  await waitForTerminal(harness, sessionPath, "completed");
  if (otherPanel.querySelector(".message-assistant")) throw new Error("A late provider result leaked into an unrelated Capture refinement panel");
  await closeRefinement(harness, "Close unrelated refinement after late result");
  await harness.waitFor(() => !otherPanel.isConnected, "closed unrelated refinement panel");
  await openCaptureRefinement(harness, captureText);
  await harness.waitFor(() => document.querySelectorAll(".refinement-panel .message-assistant").length === 1, "late result rendered only after explicit reopen");
  await harness.step("Closing during a pending request detached the UI poll without claiming cancellation; the native job completed durably, did not reopen the panel, and appeared after explicit reopen.");
}

/**
 * Drives the shipping in-app tracking surface after the central legacy fixture
 * has opened its rendered Problem dialog. It neither creates DOM nor invokes a
 * runtime function to bypass the Refine control.
 */
export async function runLegacyChatTrackingScenario(harness: Pick<ChatScenarioHarness, "api" | "waitFor" | "waitForAsync" | "click" | "enter" | "step" | "coverage" | "providerUrl" | "prepareClick">, problemId?: string): Promise<void> {
  const dialog = control<HTMLDialogElement>("#chat-modal", "legacy chat dialog");
  if (!dialog.open) throw new Error("Legacy tracking scenario requires the rendered Problem chat dialog");
  await harness.api("/provider/config", "PUT", { base_url: harness.providerUrl, model: "deterministic-test-model", api_key: "desktop-e2e-key" });
  observeCoverage(harness, dialog, "task-legacy-chat-controls");
  const contextTab = control<HTMLButtonElement>("[data-control=chat-preview-context]", "legacy preview context tab");
  const workspaceDock = dialog.dataset.workspaceDock === "true";
  await harness.waitFor(() => !contextTab.disabled && document.getElementById("explore-preview-content")?.textContent !== "Loading current context…", "loaded legacy preview context");
  const contextWasOpen = contextTab.getAttribute("aria-expanded") === "true";
  if (harness.prepareClick) await harness.prepareClick(contextTab, "Show legacy preview context");
  coverInteract(harness, "chat-preview-context", () => harness.click(contextTab, "Show legacy preview context"));
  await harness.waitFor(() => workspaceDock ? contextTab.getAttribute("aria-expanded") === String(!contextWasOpen) : contextTab.getAttribute("aria-selected") === "true", "legacy preview context selected");
  coverEffect(harness, "chat-preview-context", () => workspaceDock ? contextTab.getAttribute("aria-expanded") === String(!contextWasOpen) : contextTab.getAttribute("aria-selected") === "true");
  const starterCount = document.querySelectorAll("#chat-log > *").length;
  coverInteract(harness, "chat-prompt", () => harness.click(control<HTMLButtonElement>("[data-chat-prompt]", "legacy quick prompt"), "Ask a legacy quick prompt"));
  await harness.waitFor(() => document.querySelectorAll("#chat-log > *").length > starterCount, "legacy quick-prompt response");
  coverEffect(harness, "chat-prompt", () => document.querySelectorAll("#chat-log > *").length > starterCount);
  const previewStatus = control<HTMLButtonElement>("[data-control=chat-preview-status]", "legacy preview status");
  coverInteract(harness, "chat-preview-status", () => harness.click(previewStatus, "Inspect legacy preview status"));
  await harness.waitFor(() => contextTab.getAttribute("aria-selected") === "true", "preview status opens context");
  coverEffect(harness, "chat-preview-status", () => contextTab.getAttribute("aria-selected") === "true");
  const detailTab = control<HTMLButtonElement>("[data-control=chat-preview-detail]", "legacy preview detail tab");
  await harness.waitFor(() => !detailTab.disabled, "generated legacy preview detail");
  if (harness.prepareClick) await harness.prepareClick(detailTab, "Show generated legacy preview detail");
  coverInteract(harness, "chat-preview-detail", () => harness.click(detailTab, "Show generated legacy preview detail"));
  await harness.waitFor(() => workspaceDock ? !document.getElementById("explore-preview-detail")?.hidden && document.getElementById("explore-preview-content")?.hidden === true : detailTab.getAttribute("aria-selected") === "true", "legacy preview detail selected");
  coverEffect(harness, "chat-preview-detail", () => workspaceDock ? !document.getElementById("explore-preview-detail")?.hidden && document.getElementById("explore-preview-content")?.hidden === true : detailTab.getAttribute("aria-selected") === "true");
  await harness.waitFor(() => Boolean(document.querySelector("#apply-refinement-preview")), "generated legacy preview apply");
  const appliedProblem = (await harness.api<{ categories: Array<{ items: Array<{ kind: string; problemId?: string; problemRevision?: number }> }> }>("/workbench"))
    .categories.flatMap(category => category.items)
    .find(item => item.kind === "refinement" && item.problemId && item.problemRevision);
  if (!appliedProblem?.problemId || !appliedProblem.problemRevision) throw new Error("Missing migrated Problem provenance before preview apply");
  const appliedProblemId = appliedProblem.problemId;
  const appliedProblemRevision = appliedProblem.problemRevision;
  const appliedDraftTitle = control<HTMLElement>("#explore-preview-detail .refinement-draft-head strong", "generated preview title").textContent ?? "";
  const applyPreview = control<HTMLButtonElement>("#apply-refinement-preview", "apply generated legacy preview");
  observeCoverage(harness, dialog, "task-legacy-chat-controls-preview-ready");
  coverInteract(harness, "chat-apply-preview", () => harness.click(applyPreview, "Apply generated legacy preview"));
  await harness.waitFor(() => applyPreview.disabled && document.getElementById("explore-preview-status")?.textContent === "APPLIED", "applied legacy preview");
  coverEffect(harness, "chat-apply-preview", () => applyPreview.disabled && document.getElementById("explore-preview-status")?.textContent === "APPLIED");
  await harness.step("Rendered migrated Problem preview was explicitly applied.");
  await harness.waitForAsync(async () => {
    const current = await harness.api<{ title?: string; problemRevision?: number }>(`/items/problems/${encodeURIComponent(appliedProblemId)}`);
    return current.title === appliedDraftTitle && current.problemRevision === appliedProblemRevision + 1;
  }, "applied migrated Problem revision and text");
  const track = control<HTMLButtonElement>("[data-control=chat-track-toggle]", "Track this chat");
  coverInteract(harness, "chat-track-toggle", () => harness.click(track, "Track this chat"));
  await harness.waitFor(() => track.textContent?.includes("Tracking") === true || Boolean(document.querySelector('[data-control="tracking-accept"]')), "tracking session or review card");
  const trackingReview = document.querySelector<HTMLButtonElement>('[data-control="tracking-accept"]');
  if (trackingReview) {
    observeCoverage(harness, dialog, "task-legacy-chat-tracking-review");
    coverInteract(harness, "tracking-accept", () => harness.click(trackingReview, "Accept chat tracking session"));
    await harness.waitFor(() => !trackingReview.isConnected, "accepted chat tracking review");
    coverEffect(harness, "tracking-accept", () => !trackingReview.isConnected);
  }
  await harness.waitFor(() => track.textContent?.includes("Tracking") === true, "attached chat tracking");
  coverEffect(harness, "chat-track-toggle", () => track.textContent?.includes("Tracking") === true);
  const workbench = await harness.api<{ categories: Array<{ items: Array<{ kind: string; title?: string; problemId?: string; problemRevision?: number }> }> }>("/workbench");
  const legacy = workbench.categories.flatMap(category => category.items).filter(item => item.kind === "refinement" && (!problemId || item.problemId === problemId));
  if (legacy.length !== 1 || !legacy[0].problemId || !legacy[0].problemRevision) throw new Error("Unable to identify the exact rendered migrated Problem revision for provenance assertion");
  problemId = legacy[0].problemId;
  const currentProblem = await harness.api<{ problemRevision?: number }>(`/items/problems/${encodeURIComponent(problemId)}`);
  const problemRevision = currentProblem.problemRevision ?? legacy[0].problemRevision;
  const problemTitle = legacy[0].title;
  await harness.waitFor(() => Boolean(document.querySelector('[data-control=chat-propose][data-action=create_task]')), "current Task proposal action");
  observeCoverage(harness, dialog, "task-legacy-chat-task-proposal");
  coverInteract(harness, "chat-propose", () => harness.click(control<HTMLButtonElement>('[data-control=chat-propose][data-action=create_task]', "Task proposal action"), "Review Task proposal"));
  const trackingActions = control<HTMLElement>('[aria-label][data-tracking-busy]', "tracking proposal actions");
  await harness.waitFor(
    () => trackingActions.dataset.trackingBusy === "true" ||
      Boolean(document.querySelector('[data-control=tracking-accept]')) ||
      Boolean(trackingActions.dataset.trackingError),
    "started Task proposal request",
  );
  await harness.waitFor(
    () => Boolean(document.querySelector('[data-control=tracking-accept]')) ||
      trackingActions.dataset.trackingBusy === "false",
    "finished Task proposal request",
  );
  if (!document.querySelector('[data-control=tracking-accept]')) {
    throw new Error(`Task proposal failed before review: ${trackingActions.dataset.trackingError || "no surfaced error"}`);
  }
  coverEffect(harness, "chat-propose", () => Boolean(document.querySelector("[data-control=tracking-accept]")));
  coverInteract(harness, "tracking-accept", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-accept]", "Task proposal accept"), "Accept Task proposal"));
  await harness.waitForAsync(async () => {
    const workbench = await harness.api<{ categories: Array<{ items: Array<{ kind: string; id: string }> }> }>("/workbench");
    const tasks = workbench.categories.flatMap(category => category.items).filter(item => item.kind === "task");
    for (const task of tasks) {
      const detail = await harness.api<{ title?: string; problemLinks?: Array<{ problemId: string; problemRevision: number }> }>(`/tasks/${encodeURIComponent(task.id)}`);
      if (detail.title === problemTitle && detail.problemLinks?.some(link => link.problemId === problemId && link.problemRevision === problemRevision)) return true;
    }
    return false;
  }, "accepted Task with exact Problem provenance");
  coverEffect(harness, "tracking-accept", () => !document.querySelector("[data-control=tracking-accept]"));
  await harness.step("Accepted current Task proposal and independently read back exact Problem provenance.");

  const message = control<HTMLTextAreaElement>("#chat-message", "legacy chat message");
  await harness.waitFor(() => !control<HTMLButtonElement>("#chat-form .primary", "legacy Ask").disabled && document.querySelector<HTMLFormElement>("#chat-form")?.dataset.sending === "false", "settled legacy sender before checkpoint");
  coverInteract(harness, "chat-message", () => harness.enter(message, "Record this checkpoint without changing the Problem."));
  coverInteract(harness, "chat-ask", () => message.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })));
  await harness.waitFor(() => Boolean(document.querySelector('[data-control=tracking-accept]')), "tracked checkpoint card");
  await harness.step("Rendered checkpoint card followed a real tracked chat response.");
  coverEffect(harness, "chat-message", () => message.value === "");
  coverEffect(harness, "chat-ask", () => Boolean(document.querySelector("[data-control=tracking-accept]")));

  const edit = control<HTMLButtonElement>("[data-control=tracking-edit]", "tracked card edit");
  coverInteract(harness, "tracking-edit", () => harness.click(edit, "Open tracked JSON editor"));
  await harness.waitFor(() => Boolean(document.querySelector("[data-control=tracking-json-edit]")), "opened tracked JSON editor");
  observeCoverage(harness, dialog, "task-legacy-chat-controls-json-editor");
  await harness.step("Rendered tracked checkpoint JSON editor opened.");
  const editor = control<HTMLTextAreaElement>("[data-control=tracking-json-edit]", "tracked JSON editor");
  coverInteract(harness, "tracking-json-edit", () => harness.enter(editor, "not valid JSON"));
  coverEffect(harness, "tracking-json-edit", () => editor.value === "not valid JSON");
  coverEffect(harness, "tracking-edit", () => Boolean(document.querySelector("[data-control=tracking-json-edit]")));
  coverInteract(harness, "tracking-json-preview", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-json-preview]", "tracked JSON preview"), "Preview invalid tracked JSON"));
  await harness.waitFor(() => Boolean(document.querySelector("#chat-column [role=alert]")), "invalid JSON alert");
  coverEffect(harness, "tracking-json-preview", () => Boolean(document.querySelector("#chat-column [role=alert]")));
  coverInteract(harness, "tracking-json-cancel", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-json-cancel]", "tracked JSON cancel"), "Cancel tracked JSON edit"));
  await harness.waitFor(() => !document.querySelector("[data-control=tracking-json-edit]"), "discarded JSON editor");
  coverEffect(harness, "tracking-json-cancel", () => !document.querySelector("[data-control=tracking-json-edit]"));

  coverInteract(harness, "tracking-edit", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-edit]", "tracked card edit"), "Reopen tracked JSON editor"));
  await harness.waitFor(() => Boolean(document.querySelector("[data-control=tracking-json-edit]")), "reopened tracked JSON editor");
  harness.enter(control<HTMLTextAreaElement>("[data-control=tracking-json-edit]", "tracked JSON editor"), '{"summary":"Edited checkpoint"}');
  coverInteract(harness, "tracking-json-preview", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-json-preview]", "tracked JSON preview"), "Preview edited checkpoint"));
  await harness.waitFor(() => !document.querySelector("[data-control=tracking-json-edit]"), "edited checkpoint preview");
  coverEffect(harness, "tracking-json-preview", () => !document.querySelector("[data-control=tracking-json-edit]"));
  coverInteract(harness, "tracking-reject", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-reject]", "tracked card reject"), "Reject tracked checkpoint"));
  await harness.waitFor(() => !document.querySelector('[data-control=tracking-reject]'), "rejected tracked card");
  coverEffect(harness, "tracking-reject", () => !document.querySelector("[data-control=tracking-reject]"));

  // A second real chat response yields a distinct card whose explicit Accept
  // action persists the checkpoint through the work-tracking adapter.
  await harness.waitFor(() => !control<HTMLButtonElement>("#chat-form .primary", "legacy Ask").disabled && document.querySelector<HTMLFormElement>("#chat-form")?.dataset.sending === "false", "settled legacy sender before accepted checkpoint");
  coverInteract(harness, "chat-message", () => harness.enter(message, "Record the accepted checkpoint."));
  coverInteract(harness, "chat-ask", () => harness.click(control<HTMLButtonElement>("#chat-form .primary", "legacy Ask"), "Ask legacy chat"));
  await harness.waitFor(() => Boolean(document.querySelector('[data-control=tracking-accept]')), "second tracked checkpoint card");
  coverInteract(harness, "tracking-accept", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-accept]", "tracked card accept"), "Accept tracked checkpoint"));
  await harness.waitFor(() => !document.querySelector('[data-control=tracking-accept]'), "accepted tracked card removed");

  const resumeCompletedFixture = async () => {
    const seeded = await invoke<{ sessionId: string }>("desktop_e2e_seed_completed_tracking");
    window.dispatchEvent(new CustomEvent("llm-wiki:tracked-resume", { detail: { sessionId: seeded.sessionId } }));
    await harness.waitFor(() => Boolean(document.querySelector("[data-control=tracking-review-draft]")), "offered Knowledge review card");
  };
  // The fixture establishes only the completed session. Each reviewed Knowledge
  // decision below is made through its rendered card.
  await resumeCompletedFixture();
  coverInteract(harness, "tracking-defer", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-defer]", "defer Knowledge publication"), "Defer Knowledge publication"));
  await harness.waitFor(() => !document.querySelector("[data-control=tracking-defer]"), "deferred Knowledge offer removed");
  coverEffect(harness, "tracking-defer", () => !document.querySelector("[data-control=tracking-defer]"));

  await resumeCompletedFixture();
  coverInteract(harness, "tracking-review-draft", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-review-draft]", "review Knowledge draft"), "Review Knowledge draft"));
  await harness.waitFor(() => Boolean(document.querySelector("[data-control=tracking-accept]")), "Knowledge draft review card");
  coverEffect(harness, "tracking-review-draft", () => Boolean(document.querySelector("[data-control=tracking-accept]")));
  coverInteract(harness, "tracking-accept", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-accept]", "accept Knowledge draft"), "Accept Knowledge draft"));
  await harness.waitFor(() => Boolean(document.querySelector("[data-control=tracking-publish]")), "saved Knowledge draft card");
  coverEffect(harness, "tracking-accept", () => Boolean(document.querySelector("[data-control=tracking-publish]")));
  const draftDetails = control<HTMLDetailsElement>("[data-control=tracking-knowledge-draft-details]", "saved Knowledge draft details");
  const draftSummary = draftDetails.querySelector<HTMLElement>("summary");
  if (!draftSummary) throw new Error("Saved Knowledge disclosure summary missing");
  coverInteract(harness, "tracking-knowledge-draft-details", () => harness.click(draftSummary, "Open saved Knowledge draft details"));
  await harness.waitFor(() => draftDetails.open, "expanded saved Knowledge draft");
  coverEffect(harness, "tracking-knowledge-draft-details", () => draftDetails.open && Boolean(draftDetails.querySelector("pre")));
  coverInteract(harness, "tracking-publish", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-publish]", "publish reviewed Knowledge draft"), "Publish reviewed Knowledge draft"));
  await harness.waitFor(() => !document.querySelector("[data-control=tracking-publish]"), "published Knowledge draft card removed");
  coverEffect(harness, "tracking-publish", () => !document.querySelector("[data-control=tracking-publish]"));

  await resumeCompletedFixture();
  coverInteract(harness, "tracking-review-draft", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-review-draft]", "review editable Knowledge draft"), "Review editable Knowledge draft"));
  await harness.waitFor(() => Boolean(document.querySelector("[data-control=tracking-accept]")), "editable Knowledge draft review");
  coverInteract(harness, "tracking-accept", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-accept]", "accept editable Knowledge draft"), "Accept editable Knowledge draft"));
  await harness.waitFor(() => Boolean(document.querySelector("[data-control=tracking-edit-draft]")), "editable saved Knowledge draft");
  coverInteract(harness, "tracking-edit-draft", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-edit-draft]", "edit saved Knowledge draft"), "Edit saved Knowledge draft"));
  await harness.waitFor(() => Boolean(document.querySelector("[data-control=tracking-json-edit]")), "Knowledge draft JSON editor");
  coverEffect(harness, "tracking-edit-draft", () => Boolean(document.querySelector("[data-control=tracking-json-edit]")));
  coverInteract(harness, "tracking-json-preview", () => harness.click(control<HTMLButtonElement>("[data-control=tracking-json-preview]", "preview edited Knowledge draft"), "Preview edited Knowledge draft"));
  await harness.waitFor(() => Boolean(document.querySelector("[data-control=tracking-accept]")), "edited Knowledge draft review");
  await harness.step("The rendered legacy Problem chat attached tracking, created checkpoint cards from actual chat responses, rejected and accepted separate cards, and exercised invalid, cancelled, and valid JSON editor states.");
}

/** The registry arms its E2E-only one-shot native context fault before it opens
 * the real migrated Problem Refine surface. The retry itself is always UI-driven. */
export async function runLegacyPreviewWarningRetryScenario(harness: Pick<ChatScenarioHarness, "waitFor" | "click" | "step" | "coverage">): Promise<void> {
  const dialog = control<HTMLDialogElement>("#chat-modal", "legacy Problem chat dialog");
  if (!dialog.open) throw new Error("Preview warning scenario requires the rendered migrated Problem dialog");
  observeCoverage(harness, dialog, "task-legacy-preview-warning");
  const warning = control<HTMLButtonElement>("[data-control=chat-preview-warning]", "refinement preview warning");
  await harness.waitFor(() => !warning.hidden, "one-shot refinement context warning");
  observeCoverage(harness, dialog, "task-legacy-preview-warning-visible");
  coverInteract(harness, "chat-preview-warning", () => harness.click(warning, "Retry refinement preview context"));
  await harness.waitFor(() => warning.hidden && !(control<HTMLElement>("#explore-refinement-preview", "refinement preview").hidden), "reloaded refinement preview context");
  coverEffect(harness, "chat-preview-warning", () => warning.hidden && !(control<HTMLElement>("#explore-refinement-preview", "refinement preview").hidden));
  await harness.step("A genuine one-shot native refinement-context failure displayed the retry warning; the rendered warning control reloaded the migrated Problem preview.");
}
