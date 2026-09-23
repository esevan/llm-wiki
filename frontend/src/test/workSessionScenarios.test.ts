import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { interactWithCommittedAnswer, interactWithObservedControl, saveSettings, selectSession } from "./workSessionScenarios";

import { InteractionCoverage, type ControlSpec } from "./interactionCoverage";
import { TaskWorkSessions } from "../features/workbench/TaskWorkSessions";
import type { TaskAggregate } from "../types/taskWorkbench";

const execution = vi.hoisted(() => ({ subscribe: vi.fn(), respond: vi.fn() }));
vi.mock("../services/taskExecutionClient", () => ({ taskExecutionClient: execution }));

describe("packaged Work Session settings helper", () => {
  it("observes the completed UI after saving detaches the disabled editor button", async () => {
    const host = document.createElement("div");
    host.innerHTML = `
      <section class="task-detail" aria-busy="false">
        <div class="task-session-space">
          <button data-control="task-session-create">New session</button>
          <textarea data-control="task-session-message"></textarea>
          <section class="task-session-settings">
            <button data-control="task-session-prepare">Prepare conversation</button>
            <label>Model<select><option value="gpt-5.6-sol">Sol</option><option value="gpt-5.6-luna">Luna</option></select></label>
            <label>Project folder path<input /></label>
            <button data-control="task-session-settings-save">Save settings</button>
          </section>
        </div>
      </section>`;
    document.body.append(host);
    const save = host.querySelector<HTMLButtonElement>('[data-control="task-session-settings-save"]')!;
    try {
      await saveSettings({
        enter: (input, value) => { input.value = value; },
        prepareClick: async () => {},
        click: (button) => {
          expect(button).toBe(save);
          expect(host.querySelector("select")).toHaveValue("gpt-5.6-luna");
          expect(host.querySelector("input")).toHaveValue("/isolated/project");
          // React commits busy first, then unmounts the settings editor on success.
          save.disabled = true;
          save.remove();
          const status = document.createElement("p");
          status.setAttribute("role", "status");
          status.textContent = "Settings saved";
          host.querySelector(".task-session-settings")!.append(status);
        },
        waitFor: async (check) => { await waitFor(() => expect(check()).toBe(true)); },
      }, "gpt-5.6-luna", "/isolated/project");
      expect(save.isConnected).toBe(false);
      expect(save).toBeDisabled();
    } finally {
      host.remove();
    }
  });
});


describe("packaged Codex execution harness", () => {
  it("observes every newly rendered execution control before interacting", () => {
    const ids = ["task-session-prepare", "task-session-run", "task-session-approval-choice", "task-session-question-option", "task-session-question-text", "task-session-request-submit"];
    const controls: ControlSpec[] = ids.map((id) => ({ id, family: "F1", kind: id.endsWith("text") ? "textarea" : "button", selector: `[data-control="${id}"]`, source: "fixture", sourceEvidence: id, scenario: "task-codex-execution", effect: "recorded response" }));
    const coverage = new InteractionCoverage(controls, new Map([["fixture", ids.join(" ")]]));
    const host = document.createElement("div");
    document.body.append(host);
    try {
      for (const control of controls) {
        // Each request replaces the previous one, so observing only once at
        // scenario start cannot satisfy this test.
        host.innerHTML = `<${control.kind} data-control="${control.id}"></${control.kind}>`;
        let acted = false;
        interactWithObservedControl({ coverage }, control.id, () => {
          expect(coverage.report().renderedIds).toContain(control.id);
          acted = true;
        });
        coverage.assertEffect(control.id, () => acted);
      }
      expect(coverage.report().exercisedIds).toEqual(ids);
      expect(coverage.report().assertedIds).toEqual(ids);
    } finally {
      host.remove();
    }
  });

  it("reopens an executed instruction from Run history without requiring a duplicate manual note", async () => {
    const host = document.createElement("div");
    host.innerHTML = `<section class="task-session-space">
      <button data-control="task-session-create">New session</button>
      <select data-control="task-session-select"><option value="a">Session A</option></select>
      <div class="task-session-layout"><ol class="task-session-entries"></ol>
        <button data-control="task-session-run-open">Finished · Executed instruction</button>
        <textarea data-control="task-session-message"></textarea>
      </div>
    </section>`;
    document.body.append(host);
    try {
      await selectSession({ waitFor: async (check) => { await waitFor(() => expect(check()).toBe(true)); } }, "a", "Executed instruction", "run");
      expect(host.querySelector(".task-session-entries")).toBeEmptyDOMElement();
    } finally {
      host.remove();
    }
  });
});


it("commits mixed provider answers between synthetic events before submitting", async () => {
  const session = { id: "session-a", taskId: "task-a", title: "Session", provider: "codex", model: "gpt-5.6-luna", approvalMode: "ask", workspacePath: "/project" };
  const request = { id: "request-a", kind: "user_input", status: "pending", isBlocking: true, title: "Questions", prompt: "Answer both", choices: [], questions: [
    { id: "choice", header: "Mode", prompt: "Choose mode", options: [{ value: "Proceed", label: "Proceed" }], allowOther: false, isSecret: false },
    { id: "text", header: "Note", prompt: "Add note", options: [], allowOther: false, isSecret: false },
  ] };
  const run = { id: "run-a", taskId: "task-a", sessionId: "session-a", instruction: "Ask", status: "awaiting_response", provider: "codex", model: "gpt-5.6-luna", workspacePath: "/project", stopRequested: false, evidence: [], formalRequests: [request], workLogSyncState: "synced", revision: 1 };
  const snapshot = { runs: [run], activeRunId: run.id, selectedRun: run };
  execution.subscribe.mockResolvedValue(snapshot);
  execution.respond.mockResolvedValue(snapshot);
  window.llmWikiApplication = { request: vi.fn().mockImplementation(({ path }: { path: string }) => Promise.resolve({ ok: true, status: 200, json: async () => path.endsWith("work-sessions") ? { sessions: [session] } : { session, entries: [] }, text: async () => "", body: null })) };
  render(createElement(TaskWorkSessions, { task: { id: "task-a", taskRevision: 1, state: "in_progress", title: "Fixture", problemLinks: [] } as unknown as TaskAggregate }));
  fireEvent.change(await screen.findByLabelText("Session"), { target: { value: "session-a" } });
  const option = await screen.findByRole("button", { name: "Proceed" });
  const text = screen.getByLabelText("Note") as HTMLTextAreaElement;
  const submit = screen.getByRole("button", { name: "Submit answers" });
  const typeNote = () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(text, "controlled fixture answer");
    text.dispatchEvent(new Event("input", { bubbles: true }));
    text.dispatchEvent(new Event("change", { bubbles: true }));
  };
  // Reproduce the old harness: both handlers read the same render's answers,
  // and Submit still sees missing inputs. Native receives no response.
  act(() => { option.click(); typeNote(); submit.click(); });
  expect(execution.respond).not.toHaveBeenCalled();
  expect(option).toHaveAttribute("aria-pressed", "false");
  expect(text).toHaveValue("controlled fixture answer");

  const ids = ["task-session-question-option", "task-session-question-text"];
  const controls: ControlSpec[] = ids.map((id) => ({ id, family: "F1", kind: "button", selector: `[data-control="${id}"]`, source: "fixture", sourceEvidence: id, scenario: "fixture", effect: "response" }));
  const coverage = new InteractionCoverage(controls, new Map([["fixture", ids.join(" ")]]));
  const h = { coverage, waitFor: async (check: () => boolean) => { await waitFor(() => expect(check()).toBe(true)); } };
  await interactWithCommittedAnswer(h, ids[0], () => act(() => option.click()), () => option.getAttribute("aria-pressed") === "true", "selected option");
  await interactWithCommittedAnswer(h, ids[1], () => act(typeNote), () => text.value === "controlled fixture answer" && text.defaultValue === "controlled fixture answer", "committed note");
  fireEvent.click(submit);
  await waitFor(() => expect(execution.respond).toHaveBeenCalledWith(expect.objectContaining({ response: { answers: { choice: { answers: ["Proceed"] }, text: { answers: ["controlled fixture answer"] } } } })));
});
