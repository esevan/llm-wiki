import { useCallback, useEffect, useImperativeHandle, useRef, useState, type Ref } from "react";
import { taskClient } from "../../services/taskClient";
import type {
  RefinementProposal,
  RefinementSession,
} from "../../types/taskWorkbench";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

export type RefinementPanelHandle = { requestLeave: (proceed: () => void) => void };

export function RefinementPanel({
  kind,
  subjectId,
  onClose,
  onApplied,
  subjectTitle,
  messageDrafts,
  ref,
}: {
  kind: "capture" | "task";
  subjectId: string;
  onClose: () => void;
  onApplied?: () => void;
  subjectTitle?: string;
  messageDrafts?: Map<string, string>;
  ref?: Ref<RefinementPanelHandle>;
}) {
  const text = useTaskWorkbenchText();
  const [session, setSession] = useState<RefinementSession | undefined>(
    undefined,
  );
  const [proposals, setProposals] = useState<RefinementProposal[]>([]);
  const [draft, setDraft] = useState("");
  const messageKey = `${kind}:${subjectId}`;
  const [message, setMessage] = useState(messageDrafts?.get(messageKey) ?? "");
  useEffect(() => { messageDrafts?.set(messageKey, message); }, [messageDrafts, messageKey, message]);
  const [editing, setEditing] = useState<string>();
  const activeTab = "conversation";
  const [edits, setEdits] = useState<Record<string, Record<string, unknown>>>({});
  const [deciding, setDeciding] = useState<string>();
  const [notice, setNotice] = useState("");
  const [saving, setSaving] = useState(false);
  const [polling, setPolling] = useState(false);
  const [error, setError] = useState("");
  const timer = useRef<number | undefined>(undefined);
  const pollTimer = useRef<number | undefined>(undefined);
  const sending = useRef(false);
  const workspaceQueue = useRef<Promise<unknown>>(Promise.resolve());
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
        const restoredTab = "conversation";
        latest.current = {
          session: next,
          draft: next.inputDraft ?? "",
          message: "",
          activeTab: restoredTab,
          scrollAnchor: next.scrollAnchor ?? "0",
        };
        setSession(next);
        setDraft(next.inputDraft ?? "");


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
  const persistCurrent = useCallback(() => {
    const pending = workspaceQueue.current.then(async () => {
      const current = latest.current;
      if (!current.session) return;
      const saveWorkspace = () => taskClient.saveWorkspace(current.session!.id, {
        inputDraft: current.draft, activeTab: "conversation", scrollAnchor: current.scrollAnchor,
        baseDraftRevision: current.session!.draftRevision,
      });
      try { return await saveWorkspace(); }
      catch (error) {
        if (!String(error).includes("draft_conflict")) throw error;
        const fresh = await taskClient.refinement(kind, subjectId);
        current.session = fresh;
        setSession(fresh);
        return saveWorkspace();
      }
    });
    workspaceQueue.current = pending.catch(() => undefined);
    return pending;
  }, [kind, subjectId]);
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
  const restoreFocus = useCallback(() => {
    const opener = openerRef.current;
    if (opener?.isConnected) {
      opener.focus();
      return;
    }
    document.querySelector<HTMLElement>("#workbench h1")?.focus();
  }, []);
  const close = useCallback(async (proceed: () => void = onClose) => {
    const current = latest.current;
    if (!current.session) {
      proceed();
      return;
    }
    if (timer.current) window.clearTimeout(timer.current);
    try {
      await persistCurrent();
      skipCleanupFlush.current = true;
      proceed();
      requestAnimationFrame(restoreFocus);
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    }
  }, [onClose, restoreFocus, persistCurrent]);
  useImperativeHandle(ref, () => ({ requestLeave: (proceed) => { void close(proceed); } }), [close]);
  useEffect(() => {
    headingRef.current?.focus();
  }, []);
  const decide = async (
    proposal: RefinementProposal,
    decision: "accept" | "reject",
  ) => {
    if (!session || deciding) return;
    setDeciding(proposal.id);
    setError("");
    try {
      await taskClient.proposalDecision(
        session.id,
        proposal.id,
        proposal.draftRevision,
        decision,
        decision === "accept" ? edits[proposal.id] : undefined,
      );
      setProposals((items) => items.filter((item) => item.id !== proposal.id));
      setEditing(undefined);
      setNotice(decision === "accept" ? text.previewApplied : text.previewRejected);
      if (decision === "accept") onApplied?.();
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally { setDeciding(undefined); }
  };
  const send = async () => {
    if (!session || !message.trim() || sending.current || polling) return;
    sending.current = true;
    setSaving(true);
    setError("");
    setNotice("");
    try {
      latest.current.scrollAnchor = String(scrollRef.current?.scrollTop ?? 0);
      await persistCurrent();
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
  const labels: Record<string, string> = {
    title: text.title, detail: text.detail, outcome: text.outcome, scope: text.scope,
    nonGoals: text.nonGoals, validationCriteria: text.criteria, statement: text.newProblemStatement,
    category: text.previewCategory, note: text.previewNote, relationship: text.relationshipKind,
    problemId: text.problem, problemRevision: text.problemRevision, taskId: text.task,
  };
  const fieldsFor = (payload: Record<string, unknown>) => {
    const values = payload.patch && typeof payload.patch === "object" ? payload.patch as Record<string, unknown> : payload;
    return Object.entries(values).filter(([key, value]) => labels[key] && value !== null && value !== undefined);
  };
  const editField = (proposal: RefinementProposal, key: string, input: string) => setEdits(current => {
    const value = key === "problemRevision" ? Number(input) : input;
    const payload = current[proposal.id] ?? proposal.payload;
    return { ...current, [proposal.id]: payload.patch && typeof payload.patch === "object"
      ? { ...payload, patch: { ...payload.patch as Record<string, unknown>, [key]: value } }
      : { ...payload, [key]: value } };
  });
  const proposalLabel = (type: string) => ({ new_task: text.previewNewTask, task_patch: text.previewTaskChange,
    problem_snapshot: text.previewProblem, task_problem_link: text.previewConnection }[type] ?? text.proposedChange);
  return (
    <section className="refinement-panel" aria-label={text.refining}
      onKeyDown={event => { if (event.key === "Escape" && !event.nativeEvent.isComposing && !event.defaultPrevented) { event.preventDefault(); event.stopPropagation(); void close(); } }}
      data-refinement-session={session?.id ?? ""} data-refinement-sending={String(saving)} data-refinement-polling={String(polling)} data-refinement-message-length={message.length}>
      <header className="refinement-header">
        <div><small>{text.optionalAssistance}</small><h2 ref={headingRef} tabIndex={-1}>{subjectTitle ?? text.refining}</h2></div>
        <button type="button" data-control="refinement-close" aria-label={text.closeRefinement} onClick={() => void close()}>
          <span className="refinement-back">{kind === "task" ? text.backToTask : text.back}</span><span className="refinement-close-icon" aria-hidden="true">×</span>
        </button>
      </header>
      {error && <p role="alert" className="workbench-error">{error}</p>}
      <section className="refinement-conversation" aria-label={text.conversation}>
        <div className="refinement-messages" ref={scrollRef} onScroll={event => {
          latest.current.scrollAnchor = String(event.currentTarget.scrollTop); scheduleSave();
        }}>
          {session?.messages?.map(item => <article key={item.id} className={`message message-${item.role}`}>
            <strong>{item.role === "user" ? text.you : text.assistant}</strong><p>{item.body}</p>
          </article>)}
          {!session && <p className="region-empty">{text.loading}</p>}
        </div>
        <div className="refinement-composer">
          <textarea aria-label={text.refinementMessage} data-control="refinement-message" value={message}
            onChange={event => setMessage(event.target.value)} onKeyDown={event => {
              if (event.key === "Enter" && (event.metaKey || event.ctrlKey) && !event.nativeEvent.isComposing) { event.preventDefault(); void send(); }
            }} placeholder={text.refinementPlaceholder} rows={2} />
          <button type="button" className="primary" disabled={!session || saving || polling || !message.trim()}
            onClick={() => void send()} data-chat-control="refinement-send" data-control="refinement-send">{text.send}</button>
        </div>
      </section>
      <section className="refinement-preview" aria-label={text.previewTitle} aria-busy={polling}>
        <header><h3>{text.previewTitle}</h3><small>{text.previewUnapplied}</small></header>
        {notice && <p role="status">{notice}</p>}
        {polling && <p role="status" className="region-empty">{text.previewGenerating}</p>}
        {!proposals.length && !polling && <p className="region-empty">{text.previewEmpty}</p>}
        {proposals.map(proposal => {
          const payload = edits[proposal.id] ?? proposal.payload;
          const fields = fieldsFor(payload);
          const isEditing = editing === proposal.id;
          return <article key={proposal.id} className="proposal proposal-document" data-proposal-id={proposal.id}>
            <small>{proposalLabel(proposal.type)}</small>
            <h4>{String(payload.title ?? (payload.patch as Record<string, unknown> | undefined)?.title ?? payload.statement ?? proposalLabel(proposal.type))}</h4>
            {isEditing ? <div className="proposal-editor">{fields.map(([key, value]) => <label key={key}>{labels[key]}
              <textarea data-control="refinement-proposal-editor" aria-label={`${text.edit} ${labels[key]}`} value={String(value)}
                onChange={event => editField(proposal, key, event.target.value)} />
            </label>)}</div> : <dl className="proposal-fields">{fields.filter(([key]) => key !== "title").map(([key, value]) =>
              <div key={key}><dt>{labels[key]}</dt><dd>{String(value)}</dd></div>)}</dl>}
            <footer>
              <button type="button" data-control="refinement-proposal-edit" disabled={Boolean(deciding)}
                onClick={() => setEditing(isEditing ? undefined : proposal.id)}>{isEditing ? text.previewFinishEdit : text.edit}</button>
              <button type="button" data-control="refinement-proposal-reject" disabled={Boolean(deciding)} onClick={() => void decide(proposal, "reject")}>{text.reject}</button>
              <button type="button" className="primary" data-control="refinement-proposal-accept" disabled={Boolean(deciding) || polling}
                onClick={() => void decide(proposal, "accept")}>{text.apply}</button>
            </footer>
          </article>;
        })}
      </section>
      <details className="refinement-private-note" data-control="refinement-note-details">
        <summary>{text.savedRefinementNote}</summary>
        <textarea aria-label={text.savedRefinementNote} data-control="refinement-note" value={draft} onChange={event => save(event.target.value)} placeholder={text.savedRefinementPlaceholder} />
      </details>
    </section>
  );
}
