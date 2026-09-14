import { useCallback, useEffect, useImperativeHandle, useRef, useState, type Ref } from "react";
import { createPortal } from "react-dom";
import { taskClient } from "../../services/taskClient";
import type {
  RefinementProposal,
  RefinementSession,
  TaskAggregate,
} from "../../types/taskWorkbench";
import { useTaskWorkbenchText } from "./taskWorkbenchText";

export type RefinementPanelHandle = { requestLeave: (proceed: () => void) => void };

type RefinementScrollLock = {
  count: number;
  scrollY: number;
  rootOverflow: string;
  bodyOverflow: string;
  bodyPosition: string;
  bodyTop: string;
  bodyWidth: string;
};

let refinementScrollLock: RefinementScrollLock | undefined;

function RefinementMessage({
  body,
  reveal,
}: {
  body: string;
  reveal: boolean;
}) {
  const [visibleBody, setVisibleBody] = useState(reveal ? "" : body);

  useEffect(() => {
    if (!reveal) {
      setVisibleBody(body);
      return;
    }
    if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) {
      setVisibleBody(body);
      return;
    }
    let position = 0;
    setVisibleBody("");
    const step = Math.max(1, Math.ceil(body.length / 90));
    const interval = window.setInterval(() => {
      position = Math.min(body.length, position + step);
      setVisibleBody(body.slice(0, position));
      if (position >= body.length) window.clearInterval(interval);
    }, 24);
    return () => window.clearInterval(interval);
  }, [body, reveal]);

  return <p>
    {reveal && <span className="sr-only">{body}</span>}
    <span aria-hidden={reveal}>{visibleBody}</span>
  </p>;
}

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
  const [taskBaseline, setTaskBaseline] = useState<TaskAggregate | undefined>();
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
  const [loadError, setLoadError] = useState("");
  const [loading, setLoading] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  const pollTimer = useRef<number | undefined>(undefined);
  const sending = useRef(false);
  const pollRequesting = useRef(false);
  const loadingRef = useRef(false);
  const loadSequence = useRef(0);
  const hasLoadedSession = useRef(false);
  const knownMessageIds = useRef(new Set<string>());
  const [revealingMessageIds, setRevealingMessageIds] = useState<Set<string>>(new Set());
  const draftTouched = useRef(false);
  const lastSubmittedMessage = useRef("");
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
  const cancelLoad = useCallback(() => {
    ++loadSequence.current;
    loadingRef.current = false;
  }, []);
  const load = useCallback(async () => {
    if (loadingRef.current) return;
    loadingRef.current = true;
    setLoading(true);
    setLoadError("");
    setTaskBaseline(undefined);
    const sequence = ++loadSequence.current;
    try {
      const next = await (latest.current.session
        ? taskClient.refinementStatus(kind, subjectId)
        : taskClient.refinement(kind, subjectId));
      if (sequence !== loadSequence.current) return;
      const nextDraft = hasLoadedSession.current || draftTouched.current
        ? latest.current.draft
        : next.inputDraft ?? "";
      hasLoadedSession.current = true;
      latest.current = {
        session: next,
        draft: nextDraft,
        message: latest.current.message,
        activeTab: "conversation",
        scrollAnchor: next.scrollAnchor ?? "0",
      };
      knownMessageIds.current = new Set((next.messages ?? []).map(item => item.id));
      setRevealingMessageIds(new Set());
      setSession(next);
      setDraft(nextDraft);
      if (kind === "task" && next.taskId === subjectId) {
        const baseline = await taskClient.task(subjectId).catch(() => undefined);
        if (sequence !== loadSequence.current) return;
        if (baseline?.id === subjectId) setTaskBaseline(baseline);
      }
      requestAnimationFrame(() => {
        if (scrollRef.current)
          scrollRef.current.scrollTop = Number(next.scrollAnchor ?? 0);
      });
      const nextProposals = await taskClient.proposals(next.id);
      if (sequence === loadSequence.current) {
        setProposals(nextProposals);
        if (next.responseStatus && !["completed", "failed", "cancelled"].includes(next.responseStatus))
          setPolling(true);
        if (next.responseStatus === "failed" && !latest.current.message && lastSubmittedMessage.current) {
          setError(text.assistantFailure);
          setMessage(lastSubmittedMessage.current);
          latest.current.message = lastSubmittedMessage.current;
        }
      }
    } catch (e) {
      if (sequence === loadSequence.current)
        setLoadError(String(e instanceof Error ? e.message : e));
    } finally {
      if (sequence === loadSequence.current) {
        loadingRef.current = false;
        setLoading(false);
      }
    }
  }, [kind, subjectId, text.assistantFailure]);
  useEffect(() => {
    hasLoadedSession.current = false;
    void load();
    return () => {
      cancelLoad();
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
  }, [cancelLoad, kind, subjectId, load]);
  useEffect(() => {
    if (!polling || !session?.id) return;
    let cancelled = false;
    pollTimer.current = window.setInterval(
      () => {
        if (pollRequesting.current) return;
        pollRequesting.current = true;
        void taskClient
          .refinementStatus(kind, subjectId)
          .then(async (next) => {
            if (cancelled) return;
            const nextMessages = next.messages ?? [];
            const newAssistantIds = nextMessages
              .filter(item => item.role === "assistant" && !knownMessageIds.current.has(item.id))
              .map(item => item.id);
            // A normal turn adds one assistant message. If an older backend returns
            // several unseen messages at once, keep that recovered history readable
            // immediately instead of replaying it as a new answer.
            if (newAssistantIds.length === 1)
              setRevealingMessageIds(current => new Set([...current, ...newAssistantIds]));
            nextMessages.forEach(item => knownMessageIds.current.add(item.id));
            setSession(next);
            if (
              ["completed", "failed", "cancelled"].includes(
                next.responseStatus ?? "completed",
              )
            ) {
              let nextProposals: RefinementProposal[];
              try {
                nextProposals = await taskClient.proposals(next.id);
              } catch (e) {
                if (!cancelled) {
                  setLoadError(String(e instanceof Error ? e.message : e));
                  setPolling(false);
                }
                return;
              }
              if (cancelled) return;
              setProposals(nextProposals);
              setPolling(false);
              if (next.responseStatus === "failed")
                {
                  setError(text.assistantFailure);
                  if (!latest.current.message && lastSubmittedMessage.current) {
                    setMessage(lastSubmittedMessage.current);
                    latest.current.message = lastSubmittedMessage.current;
                  }
                }
            }
          })
          .catch((e) => {
            if (cancelled) return;
            setPolling(false);
            setLoadError(String(e instanceof Error ? e.message : e));
          })
          .finally(() => { pollRequesting.current = false; });
      },
      700,
    );
    return () => {
      cancelled = true;
      if (pollTimer.current) window.clearInterval(pollTimer.current);
    };
  }, [polling, session?.id, kind, subjectId, text.assistantFailure]);
  useEffect(() => {
    if (!revealingMessageIds.size || typeof ResizeObserver === "undefined") return;
    const element = scrollRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => {
      if (element.scrollHeight - element.scrollTop - element.clientHeight < 120)
        element.scrollTop = element.scrollHeight;
    });
    element.querySelectorAll<HTMLElement>(".message").forEach(message => observer.observe(message));
    return () => observer.disconnect();
  }, [revealingMessageIds]);
  const canonicalPayload = (proposal: RefinementProposal) => {
    const patch = proposal.payload.patch;
    if (patch && typeof patch === "object" && !Array.isArray(patch)) {
      const patchPayload = patch as Record<string, unknown>;
      return patchPayload.body !== undefined && patchPayload.detail === undefined
        ? { ...proposal.payload, patch: { ...patchPayload, detail: patchPayload.body } }
        : proposal.payload;
    }
    return proposal.payload.body !== undefined && proposal.payload.detail === undefined
      ? { ...proposal.payload, detail: proposal.payload.body }
      : proposal.payload;
  };
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
    draftTouched.current = true;
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
      requestAnimationFrame(restoreFocus);
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
  useEffect(() => {
    const application = document.querySelector<HTMLElement>(".app");
    if (!application) return;
    const wasInert = application.inert;
    application.inert = true;
    return () => { application.inert = wasInert; };
  }, []);
  useEffect(() => {
    const root = document.documentElement;
    const body = document.body;
    if (refinementScrollLock) {
      refinementScrollLock.count += 1;
    } else {
      const scrollY = window.scrollY;
      refinementScrollLock = {
        count: 1,
        scrollY,
        rootOverflow: root.style.overflow,
        bodyOverflow: body.style.overflow,
        bodyPosition: body.style.position,
        bodyTop: body.style.top,
        bodyWidth: body.style.width,
      };

      // The modal is portaled under body, so lock the document itself while it is
      // open. Fixed positioning preserves the user's page position even when the
      // browser normally exposes the body as the root scroll container.
      root.style.overflow = "hidden";
      body.style.overflow = "hidden";
      body.style.position = "fixed";
      body.style.top = `-${scrollY}px`;
      body.style.width = "100%";
    }

    return () => {
      const lock = refinementScrollLock;
      if (!lock || --lock.count > 0) return;
      root.style.overflow = lock.rootOverflow;
      body.style.overflow = lock.bodyOverflow;
      body.style.position = lock.bodyPosition;
      body.style.top = lock.bodyTop;
      body.style.width = lock.bodyWidth;
      refinementScrollLock = undefined;
      if (lock.scrollY) window.scrollTo(0, lock.scrollY);
    };
  }, []);
  const decide = async (
    proposal: RefinementProposal,
    decision: "accept" | "reject",
  ) => {
    if (!session || deciding) return;
    setDeciding(proposal.id);
    setError("");
    try {
      const normalizedPayload = canonicalPayload(proposal);
      await taskClient.proposalDecision(
        session.id,
        proposal.id,
        proposal.draftRevision,
        decision,
        decision === "accept" && (edits[proposal.id] || normalizedPayload !== proposal.payload)
          ? (edits[proposal.id] ?? normalizedPayload)
          : undefined,
      );
      setProposals((items) => items.filter((item) => item.id !== proposal.id));
      setEditing(undefined);
      setNotice(decision === "accept" ? text.previewApplied : text.previewRejected);
      if (decision === "accept") {
        onApplied?.();
        if (kind === "task") {
          const sequence = loadSequence.current;
          const baseline = await taskClient.task(subjectId).catch(() => undefined);
          if (sequence === loadSequence.current)
            setTaskBaseline(baseline?.id === subjectId ? baseline : undefined);
        }
      }
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally { setDeciding(undefined); }
  };
  const send = async () => {
    if (!session || !message.trim() || sending.current || polling || loadingRef.current || loadError) return;
    sending.current = true;
    setSaving(true);
    setError("");
    setNotice("");
    try {
      latest.current.scrollAnchor = String(scrollRef.current?.scrollTop ?? 0);
      await persistCurrent();
      const submittedMessage = message.trim();
      await taskClient.message(session.id, submittedMessage);
      lastSubmittedMessage.current = submittedMessage;
      setMessage("");
      latest.current.message = "";
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
  const meaningfulPayload = (payload: Record<string, unknown>, proposal?: RefinementProposal) => {
    const patch = payload.patch && typeof payload.patch === "object" && !Array.isArray(payload.patch)
      ? payload.patch as Record<string, unknown>
      : undefined;
    const source = patch ?? payload;
    const values: Record<string, unknown> = proposal?.type === "task_patch" && kind === "task" && taskBaseline?.id === subjectId
      && (!payload.taskId || payload.taskId === subjectId)
      ? {
          title: taskBaseline.title,
          detail: taskBaseline.detail,
          outcome: taskBaseline.outcome,
          scope: taskBaseline.scope,
          nonGoals: taskBaseline.nonGoals,
          validationCriteria: taskBaseline.validationCriteria,
          ...source,
        }
      : { ...source };
    if (values.body !== undefined && values.detail === undefined) values.detail = values.body;
    return values;
  };
  const fieldsFor = (payload: Record<string, unknown>, proposal?: RefinementProposal) => {
    const values = meaningfulPayload(payload, proposal);
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
  return createPortal(
    <div className="refinement-modal-layer">
      <div className="refinement-modal-backdrop" aria-hidden="true" />
      <section className="refinement-panel" role="dialog" aria-modal="true" aria-label={text.refining}
      onKeyDown={event => {
        if (event.key === "Escape" && !event.nativeEvent.isComposing && !event.defaultPrevented) {
          event.preventDefault(); event.stopPropagation(); void close(); return;
        }
        if (event.key !== "Tab") return;
        const focusable = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(
          "button:not(:disabled), textarea, input, select, summary, [tabindex]:not([tabindex='-1'])",
        )).filter((element) =>
          !element.closest("[hidden], [inert]") &&
          !element.hasAttribute("disabled") &&
          (!element.closest("details:not([open])") || element.matches("summary")),
        );
        if (!focusable.length) return;
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (event.shiftKey && (document.activeElement === first || document.activeElement === headingRef.current)) { event.preventDefault(); last.focus(); }
        else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
      }}
      data-refinement-session={session?.id ?? ""} data-refinement-sending={String(saving)} data-refinement-polling={String(polling)} data-refinement-message-length={message.length}>
      <header className="refinement-header">
        <div><small>{text.optionalAssistance}</small><h2 ref={headingRef} tabIndex={-1}>{subjectTitle ?? text.refining}</h2></div>
        <button type="button" data-control="refinement-close" aria-label={text.closeRefinement} onClick={() => void close()}>
          <span className="refinement-back">{kind === "task" ? text.backToTask : text.back}</span><span className="refinement-close-icon" aria-hidden="true">×</span>
        </button>
      </header>
      {(error || loadError) && <div role="alert" className="refinement-error">{error || loadError}
        {loadError && <button type="button" data-control="refinement-retry" disabled={loading} onClick={() => void load()}>{text.retry}</button>}
      </div>}
      <div className="refinement-workspace">
      <section className="refinement-preview" aria-label={text.previewTitle} aria-busy={polling}>
        <header><h3>{text.previewTitle}</h3><small>{text.previewUnapplied}</small></header>
        {notice && <p role="status">{notice}</p>}
        {polling && <p role="status" className="region-empty">{text.previewGenerating}</p>}
        {!proposals.length && !polling && <p className="region-empty">{text.previewEmpty}</p>}
        {proposals.map(proposal => {
          const payload = edits[proposal.id] ?? canonicalPayload(proposal);
          const previewValues = meaningfulPayload(payload, proposal);
          const fields = fieldsFor(payload, proposal);
          const isEditing = editing === proposal.id;
          return <article key={proposal.id} className="proposal proposal-document" data-proposal-id={proposal.id}>
            <small>{proposalLabel(proposal.type)}</small>
            <h4>{String(previewValues.title ?? payload.statement ?? proposalLabel(proposal.type))}</h4>
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
        <details className="refinement-private-note" data-control="refinement-note-details">
          <summary>{text.savedRefinementNote}</summary>
          <textarea aria-label={text.savedRefinementNote} data-control="refinement-note" value={draft} onChange={event => save(event.target.value)} placeholder={text.savedRefinementPlaceholder} />
        </details>
      </section>
      <section className="refinement-conversation" aria-label={text.conversation}>
        <div className="refinement-messages" ref={scrollRef} onScroll={event => {
          latest.current.scrollAnchor = String(event.currentTarget.scrollTop); scheduleSave();
        }}>
          {session?.messages?.map(item => <article key={item.id} className={`message message-${item.role}`}>
            <strong>{item.role === "user" ? text.you : text.assistant}</strong>
            <RefinementMessage body={item.body} reveal={revealingMessageIds.has(item.id)} />
          </article>)}
          {!session && <p className="region-empty">{text.loading}</p>}
        </div>
        {polling && <p className="refinement-thinking" role="status" aria-label={text.assistantGenerating} aria-live="polite">
          <span>{text.assistantGenerating}</span><span className="refinement-thinking-dots" aria-hidden="true"><span>...</span></span>
        </p>}
        <div className="refinement-composer">
          <textarea aria-label={text.refinementMessage} data-control="refinement-message" value={message}
            onChange={event => { latest.current.message = event.target.value; setMessage(event.target.value); }} onKeyDown={event => {
              if (event.key === "Enter" && (event.metaKey || event.ctrlKey) && !event.nativeEvent.isComposing) { event.preventDefault(); void send(); }
            }} placeholder={text.refinementPlaceholder} rows={2} />
          <button type="button" className="primary" disabled={!session || saving || polling || loading || Boolean(loadError) || !message.trim()}
            onClick={() => void send()} data-chat-control="refinement-send" data-control="refinement-send">{text.send}</button>
        </div>
      </section>
      </div>
      </section>
    </div>,
    document.body,
  );
}
