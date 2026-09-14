import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RefinementPanel } from "./RefinementPanel";

describe("Refinement panel", () => {
  it("retries a database-locked refinement read without losing local drafts", async () => {
    let attempts = 0;
    const request = vi.fn().mockImplementation(({ path }: { path: string; method?: string }) => {
      if (path.endsWith("/refinement")) {
        attempts += 1;
        if (attempts === 1) return Promise.resolve({ ok: false, status: 503, json: async () => ({ error: "database is locked" }), text: async () => "database is locked", body: null });
        return Promise.resolve({ ok: true, status: 200, json: async () => ({ id: "lock-retry", inputDraft: "stored note", messages: [] }), text: async () => "", body: null });
      }
      return Promise.resolve({ ok: true, status: 200, json: async () => [], text: async () => "", body: null });
    });
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="task" subjectId="locked-task" onClose={vi.fn()} />);

    expect(await screen.findByRole("alert")).toHaveTextContent("database is locked");
    fireEvent.change(screen.getByLabelText("Refinement message"), { target: { value: "Keep this message draft" } });
    fireEvent.click(screen.getByText("Saved refinement note"));
    fireEvent.change(screen.getByLabelText("Saved refinement note"), { target: { value: "Keep this private note" } });
    fireEvent.click(document.querySelector('[data-control="refinement-retry"]')!);

    await waitFor(() => expect(document.querySelector(".refinement-panel")).toHaveAttribute("data-refinement-session", "lock-retry"));
    expect(screen.getByLabelText("Refinement message")).toHaveValue("Keep this message draft");
    expect(screen.getByLabelText("Saved refinement note")).toHaveValue("Keep this private note");
    expect(request.mock.calls.filter(([input]) => input.path.endsWith("/messages"))).toHaveLength(0);
  });

  it("resumes a locked in-flight refinement without submitting its message twice", async () => {
    let refinementReads = 0;
    let completed = false;
    const request = vi.fn().mockImplementation(({ path }: { path: string; method?: string }) => {
      const result = (json: unknown) => Promise.resolve({ ok: true, status: 200, json: async () => json, text: async () => "", body: null });
      if (path.endsWith("/messages")) return result({});
      if (path.endsWith("/workspace")) return result({});
      if (path.endsWith("/proposals")) return result(completed ? [{ id: "recovered-proposal", type: "new_task", payload: { title: "Recovered proposal" }, draftRevision: 1 }] : []);
      refinementReads += 1;
      if (refinementReads === 2) return Promise.resolve({ ok: false, status: 503, json: async () => ({ error: "database is locked" }), text: async () => "database is locked", body: null });
      if (refinementReads === 4) completed = true;
      return result({ id: "in-flight-lock", draftRevision: 1, responseStatus: refinementReads === 4 ? "completed" : refinementReads === 3 ? "running" : undefined, messages: [] });
    });
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="task" subjectId="in-flight-lock" onClose={vi.fn()} />);
    const message = await screen.findByLabelText("Refinement message");
    fireEvent.click(screen.getByText("Saved refinement note"));
    fireEvent.change(screen.getByLabelText("Saved refinement note"), { target: { value: "Keep this note while status recovers" } });
    fireEvent.change(message, { target: { value: "Only submit this once" } });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));
    await waitFor(() => expect(request.mock.calls.filter(([input]) => input.path.endsWith("/messages"))).toHaveLength(1));
    expect((await screen.findByRole("alert", {}, { timeout: 2_000 }))).toHaveTextContent("database is locked");
      expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();
      fireEvent.click(document.querySelector('[data-control="refinement-retry"]')!);
      await waitFor(() => expect(document.querySelector(".refinement-panel")).toHaveAttribute("data-refinement-polling", "true"));
      expect(request.mock.calls.filter(([input]) => input.path.endsWith("/messages"))).toHaveLength(1);
      expect(screen.getByLabelText("Saved refinement note")).toHaveValue("Keep this note while status recovers");

    expect(await screen.findByText("Recovered proposal", {}, { timeout: 2_000 })).toBeInTheDocument();
    expect(request.mock.calls.filter(([input]) => input.path.endsWith("/messages"))).toHaveLength(1);
    const refinementMethods = request.mock.calls
      .map(([input]) => input)
      .filter((input) => input.path.endsWith("/refinement"))
      .map((input) => input.method);
    expect(refinementMethods).toEqual(["POST", "GET", "GET", "GET"]);
  });

  it("restores the submitted message only after a confirmed failed refinement response", async () => {
    let reads = 0;
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => {
      const result = (json: unknown) => Promise.resolve({ ok: true, status: 200, json: async () => json, text: async () => "", body: null });
      if (path.endsWith("/messages") || path.endsWith("/proposals") || path.endsWith("/workspace")) return result([]);
      reads += 1;
      return result({ id: "failed-turn", draftRevision: 1, responseStatus: reads === 2 ? "failed" : undefined, messages: [] });
    });
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="task" subjectId="failed-turn" onClose={vi.fn()} />);
    const message = await screen.findByLabelText("Refinement message");
    fireEvent.change(message, { target: { value: "Let me choose whether to retry" } });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));
    await waitFor(() => expect(request.mock.calls.filter(([input]) => input.path.endsWith("/messages"))).toHaveLength(1));
    expect(await screen.findByRole("alert", {}, { timeout: 2_000 })).toHaveTextContent("The assistant could not finish. Your draft is still saved.");
    expect(message).toHaveValue("Let me choose whether to retry");
    expect(screen.getByRole("button", { name: "Send" })).toBeEnabled();
  });

  it("retries a locked terminal proposal read without submitting the message again", async () => {
    let refinementReads = 0;
    let proposalReads = 0;
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => {
      const result = (json: unknown) => Promise.resolve({ ok: true, status: 200, json: async () => json, text: async () => "", body: null });
      if (path.endsWith("/messages") || path.endsWith("/workspace")) return result({});
      if (path.endsWith("/proposals")) {
        proposalReads += 1;
        if (proposalReads === 2) return Promise.resolve({ ok: false, status: 503, json: async () => ({ error: "database is locked" }), text: async () => "database is locked", body: null });
        return result(proposalReads > 2 ? [{ id: "proposal-after-retry", type: "new_task", payload: { title: "Recovered terminal proposal" }, draftRevision: 1 }] : []);
      }
      refinementReads += 1;
      return result({ id: "terminal-proposal-lock", draftRevision: 1, responseStatus: refinementReads > 1 ? "completed" : undefined, messages: [] });
    });
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="task" subjectId="terminal-proposal-lock" onClose={vi.fn()} />);
    const message = await screen.findByLabelText("Refinement message");
    fireEvent.change(message, { target: { value: "Generate this result once" } });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));
    await waitFor(() => expect(request.mock.calls.filter(([input]) => input.path.endsWith("/messages"))).toHaveLength(1));
    expect(await screen.findByRole("alert", {}, { timeout: 2_000 })).toHaveTextContent("database is locked");
    expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();

    fireEvent.click(document.querySelector('[data-control="refinement-retry"]')!);
    expect(await screen.findByText("Recovered terminal proposal")).toBeInTheDocument();
    expect(request.mock.calls.filter(([input]) => input.path.endsWith("/messages"))).toHaveLength(1);
  });

  it("keeps an existing Solution draft visible without a Problem approval gate", async () => {
    window.llmWikiApplication = {
      request: vi.fn().mockImplementation(({ path }: { path: string }) => Promise.resolve({
        ok: true,
        status: 200,
        json: async () => path.endsWith("/proposals") ? [] : ({
          id: "r-preserved",
          inputDraft: "Existing Solution: preserve the researched implementation.",
          messages: [],
        }),
        text: async () => "",
        body: null,
      })),
    };
    render(
      <RefinementPanel
        kind="capture"
        subjectId="capture-1"
        onClose={vi.fn()}
      />,
    );
    fireEvent.click(await screen.findByText("Saved refinement note"));
    expect(await screen.findByLabelText("Saved refinement note")).toHaveValue("Existing Solution: preserve the researched implementation.");
    expect(screen.getByLabelText("Refinement message")).toHaveValue("");
    expect(screen.queryByText(/approve problem/i)).not.toBeInTheDocument();
  });

  it("flushes the latest saved note before closing", async () => {
    const onClose = vi.fn();
    const request = vi.fn().mockImplementation(({ path }: { path: string }) =>
      Promise.resolve({
        ok: true,
        status: 200,
        json: async () =>
          path.endsWith("/proposals")
            ? []
            : { id: "r-flush", draftRevision: 0, messages: [] },
        text: async () => "",
        body: null,
      }),
    );
    window.llmWikiApplication = { request };
    render(
      <RefinementPanel kind="task" subjectId="task-1" onClose={onClose} />,
    );
    fireEvent.click(await screen.findByText("Saved refinement note"));
    fireEvent.change(await screen.findByLabelText("Saved refinement note"), {
      target: { value: "latest private note" },
    });
    const messages = document.querySelector<HTMLDivElement>(
      ".refinement-messages",
    )!;
    Object.defineProperty(messages, "scrollTop", { value: 37, writable: true });
    fireEvent.scroll(messages);
    fireEvent.click(screen.getByRole("button", { name: "Close refinement" }));

    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    const saved = request.mock.calls
      .map(([input]) => input)
      .find(
        (input) =>
          input.path === "/refinement/r-flush/workspace" &&
          input.method === "PUT",
      );
    expect(JSON.parse(saved.body)).toMatchObject({
      inputDraft: "latest private note",
      activeTab: "conversation",
      scrollAnchor: "37",
      baseDraftRevision: 0,
    });
  });

  it("sends a message and exposes independently actionable proposals", async () => {
    const request = vi.fn().mockImplementation(({ path }: { path: string }) =>
      Promise.resolve({
        ok: true,
        status: 200,
        json: async () =>
          path.endsWith("/refinement")
            ? { id: "r1", messages: [] }
            : path.endsWith("/proposals")
              ? [
                  {
                    id: "p1",
                    type: "new_task",
                    payload: { title: "Investigate the evidence" },
                    draftRevision: 1,
                  },
                ]
              : {
                  id: "r1",
                  messages: [
                    {
                      id: "m1",
                      role: "assistant",
                      body: "Here is a proposal.",
                    },
                  ],
                },
        text: async () => "",
        body: null,
      }),
    );
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="capture" subjectId="c1" onClose={vi.fn()} />);
    await screen.findByRole("region", { name: "Conversation" });
    fireEvent.change(screen.getByLabelText("Refinement message"), {
      target: { value: "Help structure this." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));
    await waitFor(() =>
      expect(request).toHaveBeenCalledWith(
        expect.objectContaining({
          path: "/refinement/r1/messages",
          method: "POST",
        }),
      ),
    );
  });

  it("keeps terminal polling alive through delayed proposals, accepts an edited proposal, and never auto-applies a sibling", async () => {
    let poll = false;
    let releaseProposals: (() => void) | undefined;
    const request = vi.fn().mockImplementation(({ path, body }: { path: string; body?: string }) => {
      const reply = (json: unknown) => Promise.resolve({ ok: true, status: 200, json: async () => json, text: async () => "", body: null });
      if (path.endsWith("/proposals")) {
        if (!poll) return reply([]);
        return new Promise(resolve => {
          releaseProposals = () => resolve(reply([
            { id: "accept", type: "new_task", payload: { title: "Existing Solution" }, draftRevision: 3 },
            { id: "reject", type: "new_task", payload: { title: "Do not create" }, draftRevision: 3 },
          ]));
        });
      }
      if (path.endsWith("/refinement")) return reply(poll ? {
        id: "multi", draftRevision: 3, responseStatus: "completed", messages: [
          { id: "u1", role: "user", body: "retain solution" },
          { id: "a1", role: "assistant", body: "first response" },
          { id: "u2", role: "user", body: "correction" },
          { id: "a2", role: "assistant", body: "second response" },
          { id: "u3", role: "user", body: "proposal only" },
          { id: "a3", role: "assistant", body: "third response" },
        ],
      } : { id: "multi", draftRevision: 0, messages: [] });
      if (path.endsWith("/messages")) {
        expect(JSON.parse(body!)).toMatchObject({ message: "retain solution" });
        poll = true;
      }
      return reply({});
    });
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="capture" subjectId="solution-capture" onClose={vi.fn()} />);
    const message = await screen.findByLabelText("Refinement message");
    vi.useFakeTimers();
    try {
      fireEvent.change(message, { target: { value: "retain solution" } });
      fireEvent.click(screen.getByRole("button", { name: "Send" }));
      fireEvent.change(message, { target: { value: "unsent correction while response is pending" } });
      expect(message).toHaveValue("unsent correction while response is pending");
      await act(async () => undefined);
      await act(async () => { await vi.advanceTimersByTimeAsync(700); });
      expect(screen.getByText("third response")).toBeInTheDocument();
      expect(document.querySelector(".refinement-panel")).toHaveAttribute("data-refinement-polling", "true");
      await act(async () => { releaseProposals!(); });
      expect(document.querySelector(".refinement-panel")).toHaveAttribute("data-refinement-polling", "false");
      expect(screen.getByText("Existing Solution")).toBeInTheDocument();
      expect(request.mock.calls.filter(([input]) => input.path.endsWith("/proposal-decisions"))).toHaveLength(0);
      fireEvent.click(screen.getAllByRole("button", { name: "Edit" })[0]);
      fireEvent.change(screen.getByLabelText("Edit Title"), { target: { value: "Edited existing Solution" } });
      fireEvent.click(screen.getAllByRole("button", { name: "Apply" })[0]);
      expect(request).toHaveBeenCalledWith(expect.objectContaining({ path: "/refinement/multi/proposal-decisions", method: "POST" }));
      const decision = request.mock.calls.find(([input]) => input.path.endsWith("/proposal-decisions"))?.[0];
      expect(JSON.parse(decision.body)).toMatchObject({ proposalId: "accept", decision: "accept", editedPayload: { title: "Edited existing Solution" } });
      expect(JSON.parse(decision.body).editedPayload).not.toHaveProperty("detail");
      expect(screen.getByText("Do not create")).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps the conversation and proposal preview visible together", async () => {
    const request = vi.fn().mockImplementation(({ path }: { path: string }) =>
      Promise.resolve({
        ok: true,
        status: 200,
        json: async () => path.endsWith("/proposals")
          ? [{ id: "preview-1", type: "new_task", payload: { title: "Preview this Task", body: "Preserve the authored body" }, draftRevision: 1 }]
          : { id: "preview-session", messages: [{ id: "assistant-1", role: "assistant", body: "Conversation remains available." }] },
        text: async () => "",
        body: null,
      }),
    );
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="task" subjectId="preview-task" onClose={vi.fn()} />);

    expect(await screen.findByText("Conversation remains available.")).toBeInTheDocument();
    expect(screen.getByText("Preview this Task")).toBeInTheDocument();
    expect(screen.getByText("Preserve the authored body")).toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Refining" })).toHaveAttribute("aria-modal", "true");
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() => expect(request.mock.calls.some(([input]) => input.path.endsWith("/proposal-decisions"))).toBe(true));
    const decision = request.mock.calls.find(([input]) => input.path.endsWith("/proposal-decisions"))?.[0];
    expect(JSON.parse(decision.body).editedPayload).toMatchObject({ detail: "Preserve the authored body" });
    expect(document.querySelector(".refinement-messages")).toBeInTheDocument();
    expect(document.querySelector(".refinement-preview[aria-label='Proposed result']")).toBeInTheDocument();
    expect(document.querySelector('[data-control="refinement-note-details"]')).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Conversation" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Proposals" })).not.toBeInTheDocument();
  });

  it("shows every meaningful Task field in a task patch preview", async () => {
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => Promise.resolve({
      ok: true,
      status: 200,
      json: async () => path.endsWith("/proposals")
        ? [{ id: "nested-task", type: "task_patch", payload: {
            expectedTaskRevision: 4,
            patch: {
              title: "Keep the complete Task result",
              detail: "The full authored description remains reviewable.",
              outcome: "Reviewers can see the intended outcome.",
              scope: "Only the refinement preview changes.",
              nonGoals: "No unrelated Workbench changes.",
              validationCriteria: "All six Task fields are visible.",
            },
          }, draftRevision: 4 }]
        : { id: "nested-session", messages: [] },
      text: async () => "",
      body: null,
    }));
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="task" subjectId="nested-task" onClose={vi.fn()} />);

    for (const value of [
      "The full authored description remains reviewable.",
      "Reviewers can see the intended outcome.",
      "Only the refinement preview changes.",
      "No unrelated Workbench changes.",
      "All six Task fields are visible.",
    ]) expect(await screen.findByText(value)).toBeInTheDocument();
  });

  it.each(["reject", "unmount"])("handles a delayed terminal proposal %s without stale updates", async (outcome) => {
    let sent = false;
    let finish: ((value?: unknown) => void) | undefined;
    const reply = (json: unknown) => Promise.resolve({ ok: true, status: 200, json: async () => json, text: async () => "", body: null });
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => {
      if (path.endsWith("/proposals")) {
        if (!sent) return reply([]);
        return new Promise((resolve, reject) => {
          finish = outcome === "reject" ? () => reject(new Error("Proposal readback unavailable")) : () => resolve(reply([{ id: "late", type: "new_task", payload: { title: "Late proposal" }, draftRevision: 1 }]));
        });
      }
      if (path.endsWith("/messages")) sent = true;
      return reply({ id: "delayed", draftRevision: 1, responseStatus: sent ? "completed" : undefined, messages: [] });
    });
    window.llmWikiApplication = { request };
    const view = render(<RefinementPanel kind="capture" subjectId="delayed-capture" onClose={vi.fn()} />);
    const message = await screen.findByLabelText("Refinement message");
    vi.useFakeTimers();
    try {
      fireEvent.change(message, { target: { value: "Create a reviewable proposal" } });
      fireEvent.click(screen.getByRole("button", { name: "Send" }));
      await act(async () => undefined);
      await act(async () => { await vi.advanceTimersByTimeAsync(700); });
      expect(finish).toBeDefined();
      if (outcome === "unmount") view.unmount();
      await act(async () => { finish!(); });
      if (outcome === "reject") {
        expect(screen.getByRole("alert")).toHaveTextContent("Proposal readback unavailable");
        expect(document.querySelector(".refinement-panel")).toHaveAttribute("data-refinement-polling", "false");
        fireEvent.change(message, { target: { value: "Retry" } });
        expect(document.querySelector('[data-control="refinement-retry"]')).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Send" })).toBeDisabled();
      } else {
        expect(screen.queryByText("Late proposal")).not.toBeInTheDocument();
      }
    } finally {
      vi.useRealTimers();
    }
  });

  it("guards duplicate sends, supports control-or-command Enter, and allows retry after a provider error", async () => {
    let attempts = 0;
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => {
      const result = (json: unknown) => Promise.resolve({ ok: true, status: 200, json: async () => json, text: async () => "", body: null });
      if (path.endsWith("/refinement")) return result({ id: "retry", draftRevision: 0, messages: [] });
      if (path.endsWith("/messages")) {
        attempts += 1;
        if (attempts === 1) return Promise.resolve({ ok: false, status: 502, json: async () => ({ detail: "provider unavailable" }), text: async () => "provider unavailable", body: null });
      }
      return result([]);
    });
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="task" subjectId="retry-task" onClose={vi.fn()} />);
    const message = await screen.findByLabelText("Refinement message");
    fireEvent.change(message, { target: { value: "retry this turn" } });
    fireEvent.keyDown(message, { key: "Enter", ctrlKey: true });
    await screen.findByRole("alert");
    expect(message).toHaveValue("retry this turn");
    fireEvent.click(screen.getByRole("button", { name: "Send" }));
    fireEvent.click(screen.getByRole("button", { name: "Send" }));
    await waitFor(() => expect(attempts).toBe(2));
    expect(request.mock.calls.filter(([input]) => input.path.endsWith("/messages"))).toHaveLength(2);
  });

  it("does not send control Enter while a Korean IME composition is active", async () => {
    const request = vi.fn().mockImplementation(({ path }: { path: string }) =>
      Promise.resolve({ ok: true, status: 200, json: async () => path.endsWith('/refinement') ? { id: 'ime', messages: [] } : [], text: async () => '', body: null }),
    );
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="task" subjectId="ime-task" onClose={vi.fn()} />);
    const message = await screen.findByLabelText('Refinement message');
    fireEvent.change(message, { target: { value: '조합 중' } });
    fireEvent.keyDown(message, { key: 'Enter', ctrlKey: true, isComposing: true });
    expect(request.mock.calls.some(([input]) => input.path.endsWith('/messages'))).toBe(false);
  });

  it("traps focus in the modal, makes the background inert, and restores the opener on Escape after saving", async () => {
    const onClose = vi.fn();
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => Promise.resolve({ ok: true, status: 200, json: async () => path.endsWith("/proposals") ? [] : ({ id: 'focus', messages: [] }), text: async () => '', body: null }));
    window.llmWikiApplication = { request };
    const opener = document.createElement('button');
    opener.textContent = 'Open refinement';
    document.body.append(opener);
    opener.focus();
    const app = document.createElement("div");
    app.className = "app";
    document.body.append(app);
    const view = render(<RefinementPanel kind="task" subjectId="focus-task" onClose={onClose} />, { container: app });
    const heading = await screen.findByRole('heading', { name: 'Refining' });
    expect(document.activeElement).toBe(heading);
    expect(app).toHaveProperty("inert", true);
    const summary = screen.getByText("Saved refinement note", { selector: "summary" });
    expect(summary).toBeInTheDocument();
    const send = screen.getByRole("button", { name: "Send" });
    fireEvent.change(screen.getByLabelText("Refinement message"), { target: { value: "Keep this draft" } });
    send.focus();
    fireEvent.keyDown(send, { key: "Tab" });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Close refinement" }));
    heading.focus();
    fireEvent.keyDown(heading, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(send);
    fireEvent.keyDown(screen.getByRole("region", { name: "Conversation" }), { key: 'Escape' });
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    await waitFor(() => expect(document.activeElement).toBe(opener));
    view.unmount();
    expect(app.inert).not.toBe(true);
    app.remove();
    opener.remove();
  });

  it("locks document scrolling while open and restores styles and position on unmount", async () => {
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => Promise.resolve({
      ok: true,
      status: 200,
      json: async () => path.endsWith("/proposals") ? [] : ({ id: "scroll-lock", messages: [] }),
      text: async () => "",
      body: null,
    }));
    window.llmWikiApplication = { request };
    const root = document.documentElement;
    const body = document.body;
    root.style.overflow = "auto";
    body.style.overflow = "scroll";
    body.style.position = "relative";
    body.style.top = "12px";
    body.style.width = "90%";
    const scrollYDescriptor = Object.getOwnPropertyDescriptor(window, "scrollY");
    Object.defineProperty(window, "scrollY", { configurable: true, value: 143 });
    const scrollTo = vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    const view = render(<RefinementPanel kind="capture" subjectId="scroll-lock" onClose={vi.fn()} />);

    await screen.findByRole("dialog", { name: "Refining" });
    expect(root.style.overflow).toBe("hidden");
    expect(body.style.overflow).toBe("hidden");
    expect(body.style.position).toBe("fixed");
    expect(body.style.top).toBe("-143px");
    expect(body.style.width).toBe("100%");

    const nested = render(<RefinementPanel kind="capture" subjectId="nested-scroll-lock" onClose={vi.fn()} />);
    view.unmount();
    expect(root.style.overflow).toBe("hidden");
    nested.unmount();
    expect(root.style.overflow).toBe("auto");
    expect(body.style.overflow).toBe("scroll");
    expect(body.style.position).toBe("relative");
    expect(body.style.top).toBe("12px");
    expect(body.style.width).toBe("90%");
    expect(scrollTo).toHaveBeenCalledWith(0, 143);
    scrollTo.mockRestore();
    root.style.overflow = "";
    body.style.overflow = "";
    body.style.position = "";
    body.style.top = "";
    body.style.width = "";
    if (scrollYDescriptor) Object.defineProperty(window, "scrollY", scrollYDescriptor);
    else delete (window as unknown as { scrollY?: number }).scrollY;
  });

  it("ignores a late response from a closed subject after switching to another refinement", async () => {
    let resolveFirst!: (value: unknown) => void;
    const first = new Promise<unknown>((resolve) => { resolveFirst = resolve; });
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => {
      const reply = (json: unknown) => ({ ok: true, status: 200, json: async () => json, text: async () => "", body: null });
      if (path === "/captures/a/refinement") return first.then(reply);
      if (path === "/captures/b/refinement") return Promise.resolve(reply({ id: "b", messages: [{ id: "b1", role: "assistant", body: "B only" }] }));
      return Promise.resolve(reply([]));
    });
    window.llmWikiApplication = { request };
    const view = render(<RefinementPanel kind="capture" subjectId="a" onClose={vi.fn()} />);
    view.rerender(<RefinementPanel kind="capture" subjectId="b" onClose={vi.fn()} />);
    resolveFirst({ id: "a", messages: [{ id: "a1", role: "assistant", body: "late A" }] });
    expect(await screen.findByText("B only")).toBeInTheDocument();
    await act(async () => undefined);
    expect(screen.queryByText("late A")).not.toBeInTheDocument();
  });

  it("reveals only a newly arrived assistant answer and exposes an accessible generating state", async () => {
    let completed = false;
    let pollReads = 0;
    const answer = "A newly generated answer arrives progressively.";
    const reply = (json: unknown) => Promise.resolve({ ok: true, status: 200, json: async () => json, text: async () => "", body: null });
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => {
      if (path.endsWith("/messages")) {
        completed = true;
        return reply({});
      }
      if (path.endsWith("/proposals")) return reply([]);
      if (completed) pollReads += 1;
      return reply({
        id: "animated",
        responseStatus: completed ? (pollReads > 1 ? "completed" : "running") : undefined,
        messages: completed
          ? [{ id: "stored", role: "assistant", body: "Stored history" }, { id: "fresh", role: "assistant", body: answer }]
          : [{ id: "stored", role: "assistant", body: "Stored history" }],
      });
    });
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="capture" subjectId="animated-capture" onClose={vi.fn()} />);
    expect(await screen.findByText("Stored history")).toBeInTheDocument();
    vi.useFakeTimers();
    try {
      const message = screen.getByLabelText("Refinement message");
      fireEvent.change(message, { target: { value: "Answer this" } });
      fireEvent.click(screen.getByRole("button", { name: "Send" }));
      await act(async () => { await vi.advanceTimersByTimeAsync(700); });
      await act(async () => undefined);
      expect(screen.getByRole("status", { name: "Assistant is responding" })).toBeInTheDocument();
      expect(screen.getByText("Stored history")).toBeInTheDocument();
      expect(Array.from(document.querySelectorAll('.refinement-messages [aria-hidden="true"]')).some(element => element.textContent === answer)).toBe(false);
      await act(async () => { await vi.advanceTimersByTimeAsync(700); });
      await act(async () => { await vi.advanceTimersByTimeAsync(2_500); });
      expect(Array.from(document.querySelectorAll('.refinement-messages [aria-hidden="true"]')).some(element => element.textContent === answer)).toBe(true);
    } finally {
      vi.useRealTimers();
    }
  });
  it("previews unchanged task details while applying only the sparse proposal", async () => {
    const patch = { outcome: "Updated outcome" };
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => {
      const value = path === "/tasks/full-preview"
        ? { id: "full-preview", title: "Existing title", detail: "Full existing detail", scope: "Existing scope", outcome: "Old outcome", nonGoals: "Existing exclusions", validationCriteria: "Existing validation" }
        : path.endsWith("/proposals")
          ? [{ id: "sparse", type: "task_patch", draftRevision: 1, payload: { taskId: "full-preview", expectedTaskRevision: 1, patch } }]
          : { id: "preview-session", taskId: "full-preview", messages: [] };
      return Promise.resolve({ ok: true, status: 200, json: async () => value, text: async () => "", body: null });
    });
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="task" subjectId="full-preview" onClose={vi.fn()} />);
    expect(await screen.findByRole("heading", { name: "Existing title" })).toBeInTheDocument();
    expect(screen.getByText("Full existing detail")).toBeInTheDocument();
    expect(screen.getByText("Existing scope")).toBeInTheDocument();
    expect(screen.getByText("Existing exclusions")).toBeInTheDocument();
    expect(screen.getByText("Existing validation")).toBeInTheDocument();
    expect(screen.getByText("Updated outcome")).toBeInTheDocument();
    fireEvent.click(document.querySelector('[data-control="refinement-proposal-accept"]')!);
    await waitFor(() => expect(request).toHaveBeenCalledWith(expect.objectContaining({
      path: "/refinement/preview-session/proposal-decisions",
      method: "POST",
    })));
    const decision = request.mock.calls.map(([input]) => input).find(input => input.path.endsWith("/proposal-decisions"));
    expect(JSON.parse(decision.body).editedPayload).toBeUndefined();
  });

});
