import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RefinementPanel } from "./RefinementPanel";

describe("Refinement panel", () => {
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
          ? [{ id: "preview-1", type: "new_task", payload: { title: "Preview this Task", detail: "Preserve the authored detail" }, draftRevision: 1 }]
          : { id: "preview-session", messages: [{ id: "assistant-1", role: "assistant", body: "Conversation remains available." }] },
        text: async () => "",
        body: null,
      }),
    );
    window.llmWikiApplication = { request };
    render(<RefinementPanel kind="task" subjectId="preview-task" onClose={vi.fn()} />);

    expect(await screen.findByText("Conversation remains available.")).toBeInTheDocument();
    expect(screen.getByText("Preview this Task")).toBeInTheDocument();
    expect(document.querySelector(".refinement-messages")).toBeInTheDocument();
    expect(document.querySelector(".refinement-preview[aria-label='Proposed result']")).toBeInTheDocument();
    expect(document.querySelector('[data-control="refinement-note-details"]')).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Conversation" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Proposals" })).not.toBeInTheDocument();
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
        expect(screen.getByRole("button", { name: "Send" })).toBeEnabled();
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

  it("moves focus to the non-modal heading and restores the opener on Escape after saving", async () => {
    const onClose = vi.fn();
    const request = vi.fn().mockImplementation(({ path }: { path: string }) => Promise.resolve({ ok: true, status: 200, json: async () => path.endsWith("/proposals") ? [] : ({ id: 'focus', messages: [] }), text: async () => '', body: null }));
    window.llmWikiApplication = { request };
    const opener = document.createElement('button');
    opener.textContent = 'Open refinement';
    document.body.append(opener);
    opener.focus();
    render(<RefinementPanel kind="task" subjectId="focus-task" onClose={onClose} />);
    const heading = await screen.findByRole('heading', { name: 'Refining' });
    expect(document.activeElement).toBe(heading);
    fireEvent.keyDown(screen.getByRole("region", { name: "Conversation" }), { key: 'Escape' });
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    await waitFor(() => expect(document.activeElement).toBe(opener));
    opener.remove();
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
});
