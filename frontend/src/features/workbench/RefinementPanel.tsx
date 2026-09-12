import { useCallback, useEffect, useRef, useState } from "react";
import { taskClient } from "../../services/taskClient";
import type {
  RefinementProposal,
  RefinementSession,
} from "../../types/taskWorkbench";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

export function RefinementPanel({
  kind,
  subjectId,
  onClose,
}: {
  kind: "capture" | "task";
  subjectId: string;
  onClose: () => void;
}) {
  const text = useTaskWorkbenchText();
  const [session, setSession] = useState<RefinementSession | undefined>(
    undefined,
  );
  const [proposals, setProposals] = useState<RefinementProposal[]>([]);
  const [draft, setDraft] = useState("");
  const [message, setMessage] = useState("");
  const [editing, setEditing] = useState<string>();
  const [activeTab, setActiveTab] = useState<"conversation" | "proposals">(
    "conversation",
  );
  const [saving, setSaving] = useState(false);
  const [polling, setPolling] = useState(false);
  const [error, setError] = useState("");
  const timer = useRef<number | undefined>(undefined);
  const pollTimer = useRef<number | undefined>(undefined);
  const sending = useRef(false);
  const skipCleanupFlush = useRef(false);
  const latest = useRef({
    session: undefined as RefinementSession | undefined,
    draft: "",
    message: "",
    activeTab: "conversation",
    scrollAnchor: "top",
  });
  const scrollRef = useRef<HTMLDivElement>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const openerRef = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (!openerRef.current)
      openerRef.current = document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    latest.current.session = session;
    latest.current.draft = draft;
    latest.current.message = message;
    latest.current.activeTab = activeTab;
  }, [session, draft, message, activeTab]);
  useEffect(() => {
    let cancelled = false;
    void taskClient
      .refinement(kind, subjectId)
      .then(async (next) => {
        if (cancelled) return;
        const restoredTab =
          next.activeTab === "proposals" ? "proposals" : "conversation";
        latest.current = {
          session: next,
          draft: next.inputDraft ?? "",
          message: "",
          activeTab: restoredTab,
          scrollAnchor: next.scrollAnchor ?? "0",
        };
        setSession(next);
        setDraft(next.inputDraft ?? "");
        setMessage("");
        setActiveTab(restoredTab);
        requestAnimationFrame(() => {
          if (scrollRef.current)
            scrollRef.current.scrollTop = Number(next.scrollAnchor ?? 0);
        });
        const nextProposals = await taskClient.proposals(next.id);
        if (!cancelled) setProposals(nextProposals);
      })
      .catch((e) => !cancelled && setError(String(e.message ?? e)));
    return () => {
      cancelled = true;
      if (timer.current) window.clearTimeout(timer.current);
      if (pollTimer.current) window.clearInterval(pollTimer.current);
      const current = latest.current;
      if (current.session && !skipCleanupFlush.current)
        void taskClient
          .saveWorkspace(current.session.id, {
            inputDraft: current.draft,
            activeTab: current.activeTab,
            scrollAnchor: current.scrollAnchor,
            baseDraftRevision: current.session.draftRevision,
          })
          .catch(() => undefined);
    };
  }, [kind, subjectId]);
  useEffect(() => {
    if (!polling || !session?.id) return;
    let cancelled = false;
    pollTimer.current = window.setInterval(
      () =>
        void taskClient
          .refinement(kind, subjectId)
          .then(async (next) => {
            if (cancelled) return;
            setSession(next);
            if (
              ["completed", "failed", "cancelled"].includes(
                next.responseStatus ?? "completed",
              )
            ) {
              const nextProposals = await taskClient.proposals(next.id);
              if (cancelled) return;
              setProposals(nextProposals);
              setPolling(false);
              if (next.responseStatus === "failed")
                setError(
                  text.assistantFailure,
                );
            }
          })
          .catch((e) => {
            if (cancelled) return;
            setPolling(false);
            setError(String(e instanceof Error ? e.message : e));
          }),
      700,
    );
    return () => {
      cancelled = true;
      if (pollTimer.current) window.clearInterval(pollTimer.current);
    };
  }, [polling, session?.id, kind, subjectId, text.assistantFailure]);
  const persistCurrent = () => {
    const current = latest.current;
    if (!current.session) return Promise.resolve();
    return taskClient.saveWorkspace(current.session.id, {
      inputDraft: current.draft,
      activeTab: current.activeTab,
      scrollAnchor: current.scrollAnchor,
      baseDraftRevision: current.session.draftRevision,
    });
  };
  const scheduleSave = () => {
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(
      () =>
        void persistCurrent()
          .catch((e) => setError(String(e.message ?? e))),
      500,
    );
  };
  const save = (value: string) => {
    setDraft(value);
    latest.current.draft = value;
    scheduleSave();
  };
  const selectTab = (tab: "conversation" | "proposals") => {
    if (scrollRef.current)
      latest.current.scrollAnchor = String(scrollRef.current.scrollTop);
    latest.current.activeTab = tab;
    setActiveTab(tab);
    scheduleSave();
  };
  const restoreFocus = useCallback(() => {
    const opener = openerRef.current;
    if (opener?.isConnected) {
      opener.focus();
      return;
    }
    document.querySelector<HTMLElement>("#workbench h1")?.focus();
  }, []);
  const close = useCallback(async () => {
    const current = latest.current;
    if (!current.session) {
      onClose();
      return;
    }
    if (timer.current) window.clearTimeout(timer.current);
    try {
      await persistCurrent();
      skipCleanupFlush.current = true;
      restoreFocus();
      onClose();
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    }
  }, [onClose, restoreFocus]);
  useEffect(() => {
    headingRef.current?.focus();
  }, []);
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void close();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [close]);
  const decide = async (
    proposal: RefinementProposal,
    decision: "accept" | "reject",
  ) => {
    if (!session) return;
    try {
      await taskClient.proposalDecision(
        session.id,
        proposal.id,
        proposal.draftRevision,
        decision,
        editing === proposal.id ? proposal.payload : undefined,
      );
      setProposals((items) => items.filter((item) => item.id !== proposal.id));
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    }
  };
  const send = async () => {
    if (!session || !message.trim() || sending.current || polling) return;
    sending.current = true;
    setSaving(true);
    setError("");
    try {
      await taskClient.saveWorkspace(session.id, {
        // The conversation message is persisted by taskClient.message. Keep the
        // separate workspace draft intact so an existing Capture Solution stays
        // available to every later provider turn.
        inputDraft: latest.current.draft,
        activeTab,
        scrollAnchor: String(scrollRef.current?.scrollTop ?? 0),
        baseDraftRevision: session.draftRevision,
      });
      await taskClient.message(session.id, message);
      setMessage("");
      setPolling(true);
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      sending.current = false;
      setSaving(false);
    }
  };
  return (
    <section className="task-panel refinement-panel" aria-label={text.refining} data-refinement-session={session?.id ?? ""} data-refinement-sending={String(saving)} data-refinement-polling={String(polling)} data-refinement-message-length={message.length}>
      <header>
        <div>
          <small>{text.optionalAssistance}</small>
          <h3 ref={headingRef} tabIndex={-1}>{text.refining}</h3>
        </div>
        <button type="button" data-control="refinement-close" onClick={() => void close()}>
          ×
        </button>
      </header>
      {error && <p role="alert">{error}</p>}
      <div className="panel-tabs">
        <button
          type="button"
          data-control="refinement-tab-conversation"
          aria-pressed={activeTab === "conversation"}
          onClick={() => selectTab("conversation")}
        >
          {text.conversation}
        </button>
        <button
          type="button"
          data-control="refinement-tab-proposals"
          aria-pressed={activeTab === "proposals"}
          onClick={() => selectTab("proposals")}
        >
          {text.proposals}
        </button>
      </div>
      {activeTab === "conversation" && (
        <>
          <div
            className="refinement-messages"
            ref={scrollRef}
            onScroll={(event) => {
              latest.current.scrollAnchor = String(event.currentTarget.scrollTop);
              scheduleSave();
            }}
          >
            {session?.messages?.map((item) => (
              <article key={item.id} className={`message message-${item.role}`}>
                <strong>{item.role === "user" ? text.you : text.assistant}</strong>
                <p>{item.body}</p>
              </article>
            ))}
          </div>
          <textarea
            aria-label={text.refinementMessage}
            data-control="refinement-message"
            value={message}
            onChange={(event) => setMessage(event.target.value)}
            onKeyDown={(event) => {
              if (
                event.key === "Enter" &&
                (event.metaKey || event.ctrlKey) &&
                !event.nativeEvent.isComposing
              ) {
                event.preventDefault();
                void send();
              }
            }}
            placeholder={text.refinementPlaceholder}
          />
          <button
            type="button"
            disabled={saving || polling || !message.trim()}
            onClick={() => void send()}
            data-chat-control="refinement-send"
            data-control="refinement-send"
          >
            {text.send}
          </button>
          <textarea
            aria-label={text.savedRefinementNote}
            data-control="refinement-note"
            value={draft}
            onChange={(event) => save(event.target.value)}
            placeholder={text.savedRefinementPlaceholder}
          />
        </>
      )}
      {activeTab === "proposals" && proposals.length > 0 && (
        <div>
          <h4>{text.proposals}</h4>
          {proposals.map((proposal) => (
            <article key={proposal.id} className="proposal" data-proposal-id={proposal.id}>
              <strong>{proposal.type.replaceAll("_", " ")}</strong>
              <p>
                {String(
                  proposal.payload.title ??
                    proposal.payload.statement ??
                    proposal.payload.detail ??
                    text.proposedChange,
                )}
              </p>
              {editing === proposal.id && (
                <textarea
                  data-control="refinement-proposal-editor"
                  aria-label={`Edit ${proposal.type} proposal`}
                  defaultValue={String(
                    proposal.payload.detail ?? proposal.payload.title ?? "",
                  )}
                  onChange={(event) => {
                    proposal.payload.detail = event.target.value;
                  }}
                />
              )}
              <footer>
                <button
                  type="button"
                  data-control="refinement-proposal-edit"
                  onClick={() =>
                    setEditing(
                      editing === proposal.id ? undefined : proposal.id,
                    )
                  }
                >
                  {text.edit}
                </button>
                <button
                  type="button"
                  data-control="refinement-proposal-reject"
                  onClick={() => void decide(proposal, "reject")}
                >
                  {text.reject}
                </button>
                <button
                  type="button"
                  data-control="refinement-proposal-accept"
                  onClick={() => void decide(proposal, "accept")}
                >
                  {text.apply}
                </button>
              </footer>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
