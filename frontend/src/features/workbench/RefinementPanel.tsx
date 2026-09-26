import { localizedTask } from "./taskContent";
import {
  InputImageAttachment,
  InputImagePreview,
} from "./InputImageAttachment";
import { useInputImage } from "./useInputImage";
import type {
  DocumentMention,
  ExactReferenceBinding,
  InputImage,
  ReferenceWorkspace,
  WorkPreviewFields,
  WorkPreviewVersion,
} from "../../types/taskWorkbench";
import {
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type Ref,
} from "react";
import { createPortal } from "react-dom";
import { taskClient } from "../../services/taskClient";
import type {
  RefinementProposal,
  RefinementSession,
  TaskAggregate,
  TaskApplicationEvent,
} from "../../types/taskWorkbench";
import { useTaskWorkbenchText } from "./taskWorkbenchText";
import { useModalInteraction } from "./useModalInteraction";
import { useReferenceWorkbenchText } from "./referenceWorkbenchText";
import { ReferenceAwarePreview } from "./ReferenceAwarePreview";
import { referenceKey } from "../../components/referenceIdentity";
import {
  deleteMentionRange,
  reconcileMentionEdit,
  removeMention,
} from "./referenceMentions";

export type RefinementPanelHandle = {
  requestLeave: (proceed: () => void) => void;
};

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

  return (
    <p>
      {reveal && <span className="sr-only">{body}</span>}
      <span aria-hidden={reveal}>{visibleBody}</span>
    </p>
  );
}

export function RefinementPanel({
  kind,
  subjectId,
  onClose,
  onApplied,
  subjectTitle,
  messageDrafts,
  imageDrafts,
  ref,
}: {
  kind: "capture" | "task";
  subjectId: string;
  onClose: () => void;
  onApplied?: (application?: TaskApplicationEvent) => void;
  subjectTitle?: string;
  messageDrafts?: Map<string, string>;
  imageDrafts?: Map<string, InputImage[]>;
  ref?: Ref<RefinementPanelHandle>;
}) {
  const text = useTaskWorkbenchText();
  const referenceText = useReferenceWorkbenchText();
  const [session, setSession] = useState<RefinementSession | undefined>(
    undefined,
  );
  const [proposals, setProposals] = useState<RefinementProposal[]>([]);
  const [referenceWorkspace, setReferenceWorkspace] =
    useState<ReferenceWorkspace>();
  const [mentions, setMentions] = useState<DocumentMention[]>([]);
  const [mentionOptions, setMentionOptions] = useState<ExactReferenceBinding[]>(
    [],
  );
  const [mentionIndex, setMentionIndex] = useState(0);
  const [selectedMentionOptions, setSelectedMentionOptions] = useState<
    ExactReferenceBinding[]
  >([]);
  const [mentionDismissed, setMentionDismissed] = useState(false);
  useEffect(() => {
    setSelectedMentionOptions([]);
    setMentionOptions([]);
  }, [kind, subjectId]);
  const composerRef = useRef<HTMLTextAreaElement>(null);
  const mentionRef = useRef<DocumentMention[]>([]);
  const mentionDirty = useRef(false);
  const mentionRevision = useRef(0);
  const lastSubmittedMentions = useRef<DocumentMention[]>([]);
  const previewDirty = useRef(false);
  const initialGeneration = useRef(new Set<string>());
  const referencesLoaded = useRef(false);
  const [referenceError, setReferenceError] = useState("");
  const [taskBaseline, setTaskBaseline] = useState<TaskAggregate | undefined>();
  const [appliedTask, setAppliedTask] = useState<TaskAggregate>();
  const currentTask = appliedTask ?? taskBaseline;
  const [draft, setDraft] = useState("");
  const messageKey = `${kind}:${subjectId}`;
  const attachment = useInputImage(imageDrafts?.get(messageKey));
  useEffect(() => {
    if (attachment.images.length)
      imageDrafts?.set(messageKey, attachment.images);
    else imageDrafts?.delete(messageKey);
  }, [attachment.images, imageDrafts, messageKey]);
  const [message, setMessage] = useState(messageDrafts?.get(messageKey) ?? "");
  useEffect(() => {
    messageDrafts?.set(messageKey, message);
  }, [messageDrafts, messageKey, message]);
  const [editing, setEditing] = useState<string>();
  const activeTab = "conversation";
  const [resultTab, setResultTab] = useState<"preview" | "status">("preview");
  const [statusBusy, setStatusBusy] = useState(false);
  const [edits, setEdits] = useState<Record<string, Record<string, unknown>>>(
    {},
  );
  const [deciding, setDeciding] = useState<string>();
  const [notice, setNotice] = useState("");
  const [saving, setSaving] = useState(false);
  const [polling, setPolling] = useState(false);
  const [responding, setResponding] = useState(false);
  const activeStatus = (status?: string) =>
    Boolean(status && ["queued", "running", "retryable"].includes(status));
  const previewBusy = responding || activeStatus(session?.previewStatus);

  const [error, setError] = useState("");
  const [loadError, setLoadError] = useState("");
  const [loading, setLoading] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  const pollTimer = useRef<number | undefined>(undefined);
  const sending = useRef(false);
  const turnGeneration = useRef(0);
  const pollRequesting = useRef(false);
  const loadingRef = useRef(false);
  const loadSequence = useRef(0);
  const hasLoadedSession = useRef(false);
  const knownMessageIds = useRef(new Set<string>());
  const [revealingMessageIds, setRevealingMessageIds] = useState<Set<string>>(
    new Set(),
  );
  const draftTouched = useRef(false);
  const lastSubmittedMessage = useRef("");
  const lastSubmittedImage = useRef<InputImage[]>([]);
  const currentImage = useRef(attachment.images);
  currentImage.current = attachment.images;
  const restoreImage = attachment.restore;
  const workspaceQueue = useRef<Promise<unknown>>(Promise.resolve());
  const skipCleanupFlush = useRef(false);
  const latest = useRef({
    session: undefined as RefinementSession | undefined,
    draft: "",
    message: "",
    activeTab: "conversation",
    scrollAnchor: "top",
  });
  const referenceReadSequence = useRef(0);
  const referenceSession = useRef<string | undefined>(undefined);
  referenceSession.current = session?.id;
  const refreshReferences = useCallback(async () => {
    const id = referenceSession.current;
    if (!id) return;
    const sequence = ++referenceReadSequence.current;
    const result = await taskClient.referenceWorkspace(id);
    if (
      sequence !== referenceReadSequence.current ||
      referenceSession.current !== id ||
      !result ||
      !("preview" in result)
    )
      return;
    setReferenceWorkspace(result);
    setReferenceError("");
    if (
      !referencesLoaded.current &&
      !mentionDirty.current &&
      !latest.current.message &&
      result.mentionDraft
    ) {
      setMessage(result.mentionDraft.text);
      latest.current.message = result.mentionDraft.text;
      setMentions(result.mentionDraft.mentions);
      mentionRef.current = result.mentionDraft.mentions;
    }
    referencesLoaded.current = true;
  }, []);
  useEffect(() => {
    if (!session?.id) return;
    referenceSession.current = session.id;
    let cancelled = false;
    let reading = false;
    const refresh = () => {
      if (cancelled || reading) return;
      reading = true;
      void refreshReferences()
        .catch(() => {
          if (!cancelled) setReferenceError(referenceText.actionFailed);
        })
        .finally(() => {
          reading = false;
        });
    };
    refresh();
    const interval = window.setInterval(refresh, 700);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
      referenceSession.current = undefined;
    };
  }, [session?.id, refreshReferences, referenceText.actionFailed]);
  useEffect(() => {
    if (
      !session ||
      (session.taskId && !taskBaseline) ||
      !referenceWorkspace ||
      referenceWorkspace.preview ||
      referenceWorkspace.generation ||
      activeStatus(session.responseStatus) ||
      activeStatus(session.previewStatus) ||
      initialGeneration.current.has(session.id)
    )
      return;
    initialGeneration.current.add(session.id);
    void taskClient
      .generatePreview(
        session.id,
        `refinement:${session.draftRevision ?? 0}`,
        taskBaseline?.taskRevision,
      )
      .then(refreshReferences)
      .then(() => setPolling(true))
      .catch(() => setReferenceError(referenceText.actionFailed));
  }, [
    session,
    referenceWorkspace,
    taskBaseline,
    refreshReferences,
    referenceText.actionFailed,
  ]);
  const mentionQuery = !mentionDismissed
    ? message.match(/(?:^|\s)@([^@\n]*)$/)?.[1]
    : undefined;
  useEffect(() => {
    if (mentionQuery === undefined || !session?.id) {
      setMentionOptions([]);
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void taskClient
        .references(session.id, mentionQuery)
        .then((result) => {
          if (!cancelled) {
            setMentionOptions(result.items ?? []);
            setMentionIndex(0);
          }
        })
        .catch(() => {
          if (!cancelled) setMentionOptions([]);
        });
    }, 100);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [mentionQuery, session?.id]);
  const setComposer = (value: string, bindings?: DocumentMention[]) => {
    const kept =
      bindings ??
      reconcileMentionEdit(latest.current.message, value, mentionRef.current);
    latest.current.message = value;
    setMessage(value);
    setMentionDismissed(false);
    mentionRef.current = kept;
    setMentions(kept);
    mentionDirty.current = true;
    mentionRevision.current += 1;
    scheduleSave();
  };
  const focusComposer = (caret: number) =>
    requestAnimationFrame(() => {
      composerRef.current?.focus({ preventScroll: true });
      composerRef.current?.setSelectionRange(caret, caret);
    });
  const insertMentions = (bindings: ExactReferenceBinding[]) => {
    const before = latest.current.message;
    const match = /@([^@\n]*)$/.exec(before);
    const start = match?.index ?? before.length;
    const kept = reconcileMentionEdit(
      before,
      before.slice(0, start),
      mentionRef.current,
    );
    let value = before.slice(0, start);
    const inserted: DocumentMention[] = [];
    for (const binding of bindings) {
      const baseLabel = `${binding.title || binding.documentId}${binding.section ? ` > ${binding.section}` : ""}`;
      const duplicate = mentionOptions.some(
        (other) =>
          other.title === binding.title &&
          other.section === binding.section &&
          referenceKey(other) !== referenceKey(binding),
      );
      const label = duplicate
        ? `${baseLabel} [${binding.documentId} · ${binding.documentVersion}]`
        : baseLabel;
      const token = `@${label}`;
      inserted.push({
        ...binding,
        mentionId: crypto.randomUUID(),
        displayLabel: label,
        start: value.length,
        end: value.length + token.length,
      });
      value += `${token} `;
    }
    setComposer(value, [...kept, ...inserted]);
    setMentionDismissed(true);
    setMentionOptions([]);
    setSelectedMentionOptions([]);
    focusComposer(value.length);
  };
  const toggleMention = (binding: ExactReferenceBinding) =>
    setSelectedMentionOptions((previous) =>
      previous.some((item) => referenceKey(item) === referenceKey(binding))
        ? previous.filter(
            (item) => referenceKey(item) !== referenceKey(binding),
          )
        : [...previous, binding],
    );
  const scrollRef = useRef<HTMLDivElement>(null);
  const followBottom = useRef(true);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const panelRef = useRef<HTMLElement>(null);
  const closing = useRef(false);
  const openerRef = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (!openerRef.current)
      openerRef.current =
        document.activeElement instanceof HTMLElement
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
      const nextDraft =
        hasLoadedSession.current || draftTouched.current
          ? latest.current.draft
          : (next.inputDraft ?? "");
      hasLoadedSession.current = true;
      latest.current = {
        session: next,
        draft: nextDraft,
        message: latest.current.message,
        activeTab: "conversation",
        scrollAnchor: next.scrollAnchor ?? "0",
      };
      knownMessageIds.current = new Set(
        (next.messages ?? []).map((item) => item.id),
      );
      setRevealingMessageIds(new Set());
      setSession(next);
      if (next.state === "completed" && next.taskId) setResultTab("status");
      setDraft(nextDraft);
      if (next.taskId) {
        const baseline = await taskClient
          .task(next.taskId)
          .catch(() => undefined);
        if (sequence !== loadSequence.current) return;
        if (baseline?.id === next.taskId) setTaskBaseline(baseline);
      }
      requestAnimationFrame(() => {
        if (sequence === loadSequence.current && scrollRef.current) {
          const saved = Number(next.scrollAnchor);
          scrollRef.current.scrollTop = Number.isFinite(saved)
            ? saved
            : scrollRef.current.scrollHeight;
          followBottom.current =
            scrollRef.current.scrollHeight -
              scrollRef.current.scrollTop -
              scrollRef.current.clientHeight <
            120;
        }
      });
      const nextProposals = await taskClient.proposals(next.id);
      if (sequence === loadSequence.current) {
        setProposals(nextProposals);
        setResponding(activeStatus(next.responseStatus));
        setPolling(
          activeStatus(next.responseStatus) || activeStatus(next.previewStatus),
        );
        if (
          next.responseStatus === "failed" &&
          !latest.current.message &&
          !currentImage.current.length &&
          (lastSubmittedMessage.current || lastSubmittedImage.current.length)
        ) {
          setError(text.assistantFailure);
          setMessage(lastSubmittedMessage.current);
          latest.current.message = lastSubmittedMessage.current;
          mentionRef.current = lastSubmittedMentions.current;
          setMentions(lastSubmittedMentions.current);
          mentionDirty.current = true;
          mentionRevision.current += 1;
          if (!currentImage.current.length)
            restoreImage(lastSubmittedImage.current);
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
  }, [kind, subjectId, text.assistantFailure, restoreImage]);
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
    const onQueueChanged = (event: Event) => {
      const jobs = (
        event as CustomEvent<
          Array<{
            id: string;
            entity_id: string;
            task_kind: string;
            status: string;
          }>
        >
      ).detail;
      const job = jobs?.find(
        (item) =>
          item.task_kind === "refinement_preview" &&
          item.entity_id === session?.id,
      );
      if (
        job &&
        (job.id !== session?.previewJobId ||
          job.status !== session?.previewStatus)
      )
        setPolling(true);
    };
    window.addEventListener("llm-wiki:queue-changed", onQueueChanged);
    return () =>
      window.removeEventListener("llm-wiki:queue-changed", onQueueChanged);
  }, [session?.id, session?.previewJobId, session?.previewStatus]);
  useEffect(() => {
    if (!polling || !session?.id) return;
    let cancelled = false;
    pollTimer.current = window.setInterval(() => {
      if (pollRequesting.current) return;
      pollRequesting.current = true;
      const generation = turnGeneration.current;
      void taskClient
        .refinementStatus(kind, subjectId)
        .then(async (next) => {
          if (cancelled || generation !== turnGeneration.current) return;
          const nextMessages = next.messages ?? [];
          const newAssistantIds = nextMessages
            .filter(
              (item) =>
                item.role === "assistant" &&
                !knownMessageIds.current.has(item.id),
            )
            .map((item) => item.id);
          // A normal turn adds one assistant message. If an older backend returns
          // several unseen messages at once, keep that recovered history readable
          // immediately instead of replaying it as a new answer.
          if (newAssistantIds.length === 1)
            setRevealingMessageIds(
              (current) => new Set([...current, ...newAssistantIds]),
            );
          nextMessages.forEach((item) => knownMessageIds.current.add(item.id));
          setSession(next);
          setResponding(activeStatus(next.responseStatus));
          if (
            ["completed", "failed", "cancelled"].includes(
              next.responseStatus ?? "completed",
            )
          ) {
            let nextProposals: RefinementProposal[];
            try {
              nextProposals = await taskClient.proposals(next.id);
            } catch (e) {
              if (!cancelled && generation === turnGeneration.current) {
                setLoadError(String(e instanceof Error ? e.message : e));
                setPolling(false);
              }
              return;
            }
            if (cancelled || generation !== turnGeneration.current) return;
            setProposals(nextProposals);
            setPolling(activeStatus(next.previewStatus));
            if (next.responseStatus === "failed") {
              setError(text.assistantFailure);
              if (
                !latest.current.message &&
                !currentImage.current.length &&
                (lastSubmittedMessage.current ||
                  lastSubmittedImage.current.length)
              ) {
                setMessage(lastSubmittedMessage.current);
                latest.current.message = lastSubmittedMessage.current;
                mentionRef.current = lastSubmittedMentions.current;
                setMentions(lastSubmittedMentions.current);
                mentionDirty.current = true;
                mentionRevision.current += 1;
                if (!currentImage.current.length)
                  restoreImage(lastSubmittedImage.current);
              }
            }
          }
        })
        .catch((e) => {
          if (cancelled || generation !== turnGeneration.current) return;
          setPolling(false);
          setLoadError(String(e instanceof Error ? e.message : e));
        })
        .finally(() => {
          pollRequesting.current = false;
        });
    }, 700);
    return () => {
      cancelled = true;
      if (pollTimer.current) window.clearInterval(pollTimer.current);
    };
  }, [
    polling,
    session?.id,
    kind,
    subjectId,
    text.assistantFailure,
    restoreImage,
  ]);
  useEffect(() => {
    if (typeof ResizeObserver === "undefined") return;
    const element = scrollRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => {
      if (followBottom.current) element.scrollTop = element.scrollHeight;
    });
    element
      .querySelectorAll<HTMLElement>(".message")
      .forEach((message) => observer.observe(message));
    return () => observer.disconnect();
  }, [session?.messages?.length, session?.id]);
  const lastMessageId = session?.messages?.at(-1)?.id;
  useEffect(() => {
    if (!lastMessageId || !scrollRef.current) return;
    followBottom.current = true;
    scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
  }, [lastMessageId]);
  const canonicalPayload = (proposal: RefinementProposal) => {
    const patch = proposal.payload.patch;
    if (patch && typeof patch === "object" && !Array.isArray(patch)) {
      const patchPayload = patch as Record<string, unknown>;
      return patchPayload.body !== undefined &&
        patchPayload.detail === undefined
        ? {
            ...proposal.payload,
            patch: { ...patchPayload, detail: patchPayload.body },
          }
        : proposal.payload;
    }
    return proposal.payload.body !== undefined &&
      proposal.payload.detail === undefined
      ? { ...proposal.payload, detail: proposal.payload.body }
      : proposal.payload;
  };
  const persistCurrent = useCallback(() => {
    const pending = workspaceQueue.current.then(async () => {
      const current = latest.current;
      if (!current.session) return;
      const saveWorkspace = () =>
        taskClient.saveWorkspace(current.session!.id, {
          inputDraft: current.draft,
          activeTab: "conversation",
          scrollAnchor: current.scrollAnchor,
          baseDraftRevision: current.session!.draftRevision,
        });
      const saveMentions = async (session: RefinementSession) => {
        if (!mentionDirty.current) return;
        const revision = mentionRevision.current;
        await taskClient.saveMentionDraft(
          session.id,
          mentionRef.current,
          latest.current.message,
          session.draftRevision,
        );
        if (mentionRevision.current === revision) mentionDirty.current = false;
      };
      try {
        const result = await saveWorkspace();
        await saveMentions(current.session);
        return result;
      } catch (error) {
        if (!String(error).includes("draft_conflict")) throw error;
        const fresh = await taskClient.refinement(kind, subjectId);
        current.session = fresh;
        setSession(fresh);
        const result = await saveWorkspace();
        await saveMentions(fresh);
        return result;
      }
    });
    workspaceQueue.current = pending.catch(() => undefined);
    return pending;
  }, [kind, subjectId]);
  const scheduleSave = () => {
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(
      () =>
        void persistCurrent().catch((e) => setError(String(e.message ?? e))),
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
  const close = useCallback(
    async (proceed: () => void = onClose) => {
      if (closing.current) return;
      if (previewDirty.current) {
        setError(referenceText.dirty);
        return;
      }
      const current = latest.current;
      if (!current.session) {
        proceed();
        return;
      }
      closing.current = true;
      if (timer.current) window.clearTimeout(timer.current);
      try {
        await persistCurrent();
        skipCleanupFlush.current = true;
        proceed();
      } catch (e) {
        setError(String(e instanceof Error ? e.message : e));
      } finally {
        closing.current = false;
      }
    },
    [onClose, persistCurrent, referenceText.dirty],
  );
  useModalInteraction(panelRef, 20, () => {
    void close();
  });
  useEffect(
    () => () => {
      // Wait for unmount to release the focus trap and background inert state.
      // Strict Mode's effect replay must not move focus out of a mounted panel.
      requestAnimationFrame(() => {
        if (!panelRef.current?.isConnected) restoreFocus();
      });
    },
    [restoreFocus],
  );
  useImperativeHandle(
    ref,
    () => ({
      requestLeave: (proceed) => {
        void close(proceed);
      },
    }),
    [close],
  );
  useEffect(() => {
    headingRef.current?.focus();
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
    intent?: "split",
  ) => {
    if (!session || deciding) return;
    setDeciding(proposal.id);
    setError("");
    try {
      const normalizedPayload = canonicalPayload(proposal);
      const applied = await taskClient.proposalDecision(
        session.id,
        proposal.id,
        proposal.draftRevision,
        decision,
        decision === "accept"
          ? (edits[proposal.id] ?? normalizedPayload)
          : undefined,
        intent,
      );
      setProposals((items) => items.filter((item) => item.id !== proposal.id));
      setEditing(undefined);
      setNotice(
        decision === "accept" ? text.previewApplied : text.previewRejected,
      );
      if (decision === "accept") {
        const result = (applied ?? {}) as {
          id?: string;
          taskRevision?: number;
        };
        onApplied?.(
          proposal.type !== "subtask" && result.id && result.taskRevision
            ? {
                taskId: result.id,
                revision: result.taskRevision,
                cause: "preview_adopted",
              }
            : undefined,
        );
        const taskId = result.taskRevision ? result.id : taskBaseline?.id;
        if (taskId) {
          const sequence = loadSequence.current;
          const baseline = await taskClient.task(taskId).catch(() => undefined);
          if (sequence === loadSequence.current) {
            setAppliedTask(baseline);
            if (proposal.type !== "subtask") setTaskBaseline(baseline);
            setResultTab("status");
          }
        }
      }
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setDeciding(undefined);
    }
  };
  const send = async () => {
    if (
      !session ||
      (!message.trim() && !attachment.image) ||
      attachment.reading ||
      sending.current ||
      responding ||
      loadingRef.current ||
      loadError
    )
      return;
    sending.current = true;
    setSaving(true);
    setError("");
    setNotice("");
    try {
      latest.current.scrollAnchor = String(scrollRef.current?.scrollTop ?? 0);
      await persistCurrent();
      const submittedMessage = message.trim();
      const submittedMentions = [...mentionRef.current];
      await taskClient.message(
        session.id,
        submittedMessage,
        attachment.images,
        submittedMentions,
      );
      lastSubmittedMentions.current = submittedMentions;
      lastSubmittedImage.current = attachment.images;
      attachment.clear();
      turnGeneration.current += 1;
      lastSubmittedMessage.current = submittedMessage;
      setMessage("");
      setMentions([]);
      mentionRef.current = [];
      mentionDirty.current = true;
      mentionRevision.current += 1;
      latest.current.message = "";
      setResponding(true);
      setPolling(true);
      setResultTab("preview");
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      sending.current = false;
      setSaving(false);
    }
  };
  const generateReferencePreview = async () => {
    if (!session) return;
    await taskClient.generatePreview(
      session.id,
      `refinement:${session.draftRevision ?? 0}`,
      taskBaseline?.taskRevision,
    );
    await refreshReferences();
    setPolling(true);
  };
  const investigateReferences = async () => {
    if (!session) return;
    await taskClient.investigate(session.id);
    await refreshReferences();
  };
  const openReference = async (
    binding: ExactReferenceBinding,
    version: number,
  ) => {
    const result = await taskClient.openReference(
      session!.id,
      binding,
      version,
    );
    return {
      ...binding,
      markdown: result.source.markdown ?? result.source.body ?? "",
    };
  };
  const selectReferenceVersion = useCallback(
    (version: number) =>
      taskClient.previewVersion(referenceSession.current!, version),
    [],
  );
  const compareReferenceVersions = async (left: number, right: number) => {
    const result = await taskClient.comparePreview(session!.id, left, right);
    return {
      left: result.left,
      right: result.right,
      fields: result.comparison.fields,
    };
  };
  const saveReferencePreview = async (
    base: WorkPreviewVersion,
    fields: WorkPreviewFields,
  ) => {
    await taskClient.editPreview(
      session!.id,
      base.version,
      fields,
      base.version,
      base.contentHash,
    );
    await refreshReferences();
  };
  const applyReferencePreview = async (version: WorkPreviewVersion) => {
    const application = await taskClient.applyPreview(
      session!.id,
      version.version,
      referenceWorkspace!.preview!.currentVersion,
      version.contentHash,
      version.taskRevision,
    );
    onApplied?.({
      taskId: application.task.id,
      revision: application.task.taskRevision,
      cause: "preview_adopted",
    });
    const next = await taskClient.refinementStatus(kind, subjectId);
    setSession(next);
    latest.current.session = next;
    if (next.taskId) {
      const task = await taskClient.task(next.taskId);
      setAppliedTask(task);
      setTaskBaseline(task);
    }
    await refreshReferences();
  };
  const setPreviewDirty = useCallback(
    (dirty: boolean) => {
      previewDirty.current = dirty;
      if (!dirty)
        setError((current) => (current === referenceText.dirty ? "" : current));
    },
    [referenceText.dirty],
  );
  const labels: Record<string, string> = {
    boundaryReason: text.boundaryReason,
    parentTaskId: text.parentTask,
    title: text.title,
    detail: text.detail,
    outcome: text.outcome,
    scope: text.scope,
    nonGoals: text.nonGoals,
    validationCriteria: text.criteria,
    statement: text.newProblemStatement,
    category: text.previewCategory,
    note: text.previewNote,
    relationship: text.relationshipKind,
    problemId: text.problem,
    problemRevision: text.problemRevision,
    taskId: text.task,
  };
  const meaningfulPayload = (
    payload: Record<string, unknown>,
    proposal?: RefinementProposal,
  ) => {
    const patch =
      payload.patch &&
      typeof payload.patch === "object" &&
      !Array.isArray(payload.patch)
        ? (payload.patch as Record<string, unknown>)
        : undefined;
    const source = patch ?? payload;
    const values: Record<string, unknown> =
      proposal?.type === "task_patch" &&
      taskBaseline &&
      (!payload.taskId || payload.taskId === taskBaseline?.id)
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
    if (values.body !== undefined && values.detail === undefined)
      values.detail = values.body;
    if (proposal && !edits[proposal.id] && editing !== proposal.id) {
      const translated = localizedTask({
        contentVersions: proposal.localizedFields,
      });
      const { contentVersions: _versions, ...fields } = translated;
      void _versions;
      return { ...values, ...fields };
    }
    return values;
  };
  const fieldsFor = (
    payload: Record<string, unknown>,
    proposal?: RefinementProposal,
  ) => {
    const values = meaningfulPayload(payload, proposal);
    return Object.entries(values).filter(
      ([key, value]) => labels[key] && value !== null && value !== undefined,
    );
  };
  const editField = (
    proposal: RefinementProposal,
    key: string,
    input: string,
  ) =>
    setEdits((current) => {
      const value = key === "problemRevision" ? Number(input) : input;
      const payload = current[proposal.id] ?? proposal.payload;
      return {
        ...current,
        [proposal.id]:
          payload.patch && typeof payload.patch === "object"
            ? {
                ...payload,
                patch: {
                  ...(payload.patch as Record<string, unknown>),
                  [key]: value,
                },
              }
            : { ...payload, [key]: value },
      };
    });
  const proposalLabel = (type: string) =>
    ({
      new_task: text.previewNewTask,
      subtask: text.subtask,
      task_patch: text.previewTaskChange,
      problem_snapshot: text.previewProblem,
      task_problem_link: text.previewConnection,
    })[type] ?? text.proposedChange;
  return createPortal(
    <div className="refinement-modal-layer">
      <div className="refinement-modal-backdrop" aria-hidden="true" />
      <section
        ref={panelRef}
        tabIndex={-1}
        className="refinement-panel"
        role="dialog"
        aria-modal="true"
        aria-label={text.refining}
        data-refinement-session={session?.id ?? ""}
        data-refinement-sending={String(saving)}
        data-refinement-polling={String(polling)}
        data-refinement-message-length={message.length}
      >
        <header className="refinement-header">
          <div>
            <small>{text.optionalAssistance}</small>
            <h2 ref={headingRef} tabIndex={-1}>
              {subjectTitle ?? text.refining}
            </h2>
          </div>
          <button
            type="button"
            data-control="refinement-close"
            aria-label={text.closeRefinement}
            onClick={() => void close()}
          >
            <span className="refinement-back">
              {kind === "task" ? text.backToTask : text.back}
            </span>
            <span className="refinement-close-icon" aria-hidden="true">
              ×
            </span>
          </button>
        </header>
        {(error || loadError) && (
          <div role="alert" className="refinement-error">
            {error || loadError}
            {loadError && (
              <button
                type="button"
                data-control="refinement-retry"
                disabled={loading}
                onClick={() => void load()}
              >
                {text.retry}
              </button>
            )}
          </div>
        )}
        <div className="refinement-workspace">
          <section
            className="refinement-preview"
            aria-label={text.previewTitle}
            aria-busy={previewBusy}
          >
            <nav
              className="task-detail-tabs"
              role="tablist"
              aria-label={text.proposals}
              onKeyDown={(event) => {
                if (
                  !currentTask ||
                  !["ArrowLeft", "ArrowRight", "Home", "End"].includes(
                    event.key,
                  )
                )
                  return;
                event.preventDefault();
                const next =
                  event.key === "Home"
                    ? "preview"
                    : event.key === "End"
                      ? "status"
                      : resultTab === "preview"
                        ? "status"
                        : "preview";
                setResultTab(next);
                document.getElementById(`refinement-tab-${next}`)?.focus();
              }}
            >
              <button
                type="button"
                id="refinement-tab-preview"
                data-control="refinement-tab-preview"
                role="tab"
                aria-selected={resultTab === "preview"}
                aria-controls="refinement-result-preview"
                tabIndex={resultTab === "preview" ? 0 : -1}
                onClick={() => setResultTab("preview")}
              >
                {text.previewTab}
              </button>
              {currentTask && (
                <button
                  type="button"
                  id="refinement-tab-status"
                  data-control="refinement-tab-status"
                  role="tab"
                  aria-selected={resultTab === "status"}
                  aria-controls="refinement-result-status"
                  tabIndex={resultTab === "status" ? 0 : -1}
                  onClick={() => setResultTab("status")}
                >
                  {text.workStatus}
                </button>
              )}
            </nav>
            {currentTask && resultTab === "status" && (
              <div
                id="refinement-result-status"
                role="tabpanel"
                aria-labelledby="refinement-tab-status"
                hidden={resultTab !== "status"}
              >
                <p className="refined-status">
                  {currentTask.refinedRevision
                    ? `${text.refined} ${currentTask.refinedRevision}`
                    : text.ready}
                </p>
                <h3>{localizedTask(currentTask).title}</h3>
                <p>
                  {currentTask.state === "in_progress"
                    ? text.inProgress
                    : currentTask.state === "completed"
                      ? text.completed
                      : text.ready}
                </p>
                <dl className="proposal-fields">
                  {[
                    "detail",
                    "outcome",
                    "scope",
                    "nonGoals",
                    "validationCriteria",
                  ].map((key) => {
                    const value = currentTask[key as keyof TaskAggregate];
                    return typeof value === "string" && value ? (
                      <div key={key}>
                        <dt>{labels[key]}</dt>
                        <dd>{value}</dd>
                      </div>
                    ) : null;
                  })}
                </dl>
                {currentTask.hierarchy?.parent && (
                  <p>
                    {text.parentTask}:{" "}
                    <strong>{currentTask.hierarchy.parent.title}</strong>
                  </p>
                )}
                {currentTask.state !== "in_progress" && (
                  <button
                    type="button"
                    className="primary"
                    disabled={statusBusy}
                    data-control="refinement-start-work"
                    onClick={() => {
                      setStatusBusy(true);
                      setError("");
                      void taskClient
                        .transition(
                          currentTask.id,
                          currentTask.taskRevision,
                          currentTask.state === "completed"
                            ? "reopen"
                            : "in_progress",
                        )
                        .then(() => taskClient.task(currentTask.id))
                        .then((task) => {
                          setAppliedTask(task);
                          onApplied?.();
                        })
                        .catch((e) =>
                          setError(String(e instanceof Error ? e.message : e)),
                        )
                        .finally(() => setStatusBusy(false));
                    }}
                  >
                    {currentTask.state === "completed"
                      ? text.reopen
                      : text.start}
                  </button>
                )}
              </div>
            )}
            <div
              id="refinement-result-preview"
              role="tabpanel"
              aria-labelledby="refinement-tab-preview"
              hidden={resultTab !== "preview"}
            >
              {referenceError && (
                <p role="alert">
                  {referenceError}{" "}
                  <button
                    data-control="reference-workspace-retry"
                    type="button"
                    onClick={() =>
                      void refreshReferences().catch(() =>
                        setReferenceError(referenceText.actionFailed),
                      )
                    }
                  >
                    {text.retry}
                  </button>
                </p>
              )}
              {referenceWorkspace && (
                <ReferenceAwarePreview
                  workspace={referenceWorkspace}
                  onGenerate={generateReferencePreview}
                  onInvestigate={investigateReferences}
                  onSave={saveReferencePreview}
                  onApply={applyReferencePreview}
                  onCompare={compareReferenceVersions}
                  onDirtyChange={setPreviewDirty}
                  onUsage={async (binding, kind) => {
                    await taskClient.referenceUsage(session!.id, binding, kind);
                    await refreshReferences();
                  }}
                  onRestore={async (version) => {
                    await taskClient.restorePreview(
                      session!.id,
                      version.version,
                      referenceWorkspace.preview!.currentVersion,
                    );
                    await refreshReferences();
                  }}
                  onSelect={selectReferenceVersion}
                  onOpenReference={openReference}
                />
              )}
              {!referenceWorkspace?.preview && (
                <>
                  {" "}
                  <header>
                    <h3>{text.previewTitle}</h3>
                    <small>{text.previewUnapplied}</small>
                  </header>
                  {notice && <p role="status">{notice}</p>}
                  {previewBusy && (
                    <p role="status" className="region-empty">
                      {text.previewGenerating}
                    </p>
                  )}
                  {session?.previewStatus &&
                    ["failed", "cancelled", "stale"].includes(
                      session.previewStatus,
                    ) && (
                      <p role="status" className="region-empty">
                        {text.previewQueueRecovery}
                      </p>
                    )}
                  {!proposals.length && !previewBusy && (
                    <p className="region-empty">{text.previewEmpty}</p>
                  )}
                  {proposals.map((proposal) => {
                    const payload =
                      edits[proposal.id] ?? canonicalPayload(proposal);
                    const previewValues = meaningfulPayload(payload, proposal);
                    const fields = fieldsFor(payload, proposal);
                    const isEditing = editing === proposal.id;
                    return (
                      <article
                        key={proposal.id}
                        className="proposal proposal-document"
                        data-proposal-id={proposal.id}
                      >
                        <small>{proposalLabel(proposal.type)}</small>
                        {proposal.type === "subtask" && (
                          <p>{text.subtaskHint}</p>
                        )}
                        <h4>
                          {String(
                            previewValues.title ??
                              payload.statement ??
                              proposalLabel(proposal.type),
                          )}
                        </h4>
                        {isEditing ? (
                          <div className="proposal-editor">
                            {fields.map(([key, value]) => (
                              <label key={key}>
                                {labels[key]}
                                <textarea
                                  data-control="refinement-proposal-editor"
                                  aria-label={`${text.edit} ${labels[key]}`}
                                  value={String(value)}
                                  onChange={(event) =>
                                    editField(proposal, key, event.target.value)
                                  }
                                />
                              </label>
                            ))}
                          </div>
                        ) : (
                          <dl className="proposal-fields">
                            {fields
                              .filter(([key]) => key !== "title")
                              .map(([key, value]) => (
                                <div key={key}>
                                  <dt>{labels[key]}</dt>
                                  <dd>{String(value)}</dd>
                                </div>
                              ))}
                          </dl>
                        )}
                        <footer>
                          <button
                            type="button"
                            data-control="refinement-proposal-edit"
                            disabled={Boolean(deciding)}
                            onClick={() =>
                              setEditing(isEditing ? undefined : proposal.id)
                            }
                          >
                            {isEditing ? text.previewFinishEdit : text.edit}
                          </button>
                          <button
                            type="button"
                            data-control="refinement-proposal-reject"
                            disabled={Boolean(deciding)}
                            onClick={() => void decide(proposal, "reject")}
                          >
                            {text.reject}
                          </button>
                          <button
                            type="button"
                            className="primary"
                            data-control="refinement-proposal-accept"
                            disabled={
                              Boolean(deciding) ||
                              previewBusy ||
                              Boolean(
                                session?.previewStatus &&
                                session.previewStatus !== "completed",
                              )
                            }
                            onClick={() =>
                              void decide(
                                proposal,
                                "accept",
                                proposal.type === "subtask"
                                  ? "split"
                                  : undefined,
                              )
                            }
                          >
                            {proposal.type === "subtask"
                              ? text.splitTask
                              : text.apply}
                          </button>
                        </footer>
                      </article>
                    );
                  })}
                </>
              )}
              {taskBaseline?.hierarchy &&
                (taskBaseline.hierarchy.parent ||
                  taskBaseline.hierarchy.children.length > 0) && (
                  <details
                    className="refinement-boundaries"
                    data-control="refinement-boundaries"
                  >
                    <summary>{text.boundaryContext}</summary>
                    {[
                      ...(taskBaseline.hierarchy.parent
                        ? [taskBaseline.hierarchy.parent]
                        : []),
                      ...taskBaseline.hierarchy.siblings,
                      ...taskBaseline.hierarchy.children,
                    ].map((task) => (
                      <article key={task.id}>
                        <h4>{task.title}</h4>
                        <p>
                          {text.scope}: {task.scope}
                        </p>
                        <p>
                          {text.nonGoals}: {task.nonGoals}
                        </p>
                      </article>
                    ))}
                  </details>
                )}
            </div>
            {kind === "capture" && session?.captureDistillation && (
              <section
                className="capture-distillation-detail"
                aria-label={text.captureSource}
              >
                <header>
                  <h3>{text.captureSource}</h3>
                  {session.captureDistillation.distillation && (
                    <small
                      className={`capture-distillation-status status-${session.captureDistillation.distillation.status}`}
                      role="status"
                    >
                      {session.captureDistillation.distillation.status ===
                        "queued" ||
                      session.captureDistillation.distillation.status ===
                        "running"
                        ? text.captureOrganizing
                        : session.captureDistillation.distillation
                              .executionOutcome === "failed"
                          ? (session.captureDistillation.distillation.safeError
                              ?.message ?? text.captureOrganizationFailed)
                          : session.captureDistillation.distillation
                                .applicationDisposition === "applied"
                            ? text.captureOrganized
                            : ""}
                    </small>
                  )}
                </header>
                {session.captureDistillation.source.text && (
                  <p className="capture-source-text">
                    {session.captureDistillation.source.text}
                  </p>
                )}
                {session.captureDistillation.source.images.length > 0 && (
                  <small>
                    {session.captureDistillation.source.images
                      .map((image) => image.name)
                      .join(", ")}
                  </small>
                )}
                {session.captureDistillation.display.context && (
                  <div>
                    <h4>{text.captureContext}</h4>
                    <p>{session.captureDistillation.display.context}</p>
                  </div>
                )}
                {session.captureDistillation.display.explicitRequests.length >
                  0 && (
                  <div>
                    <h4>{text.captureRequests}</h4>
                    <ul>
                      {session.captureDistillation.display.explicitRequests.map(
                        (request, index) => (
                          <li key={index}>{request}</li>
                        ),
                      )}
                    </ul>
                  </div>
                )}
                {session.captureDistillation.proposal && (
                  <div className="capture-distillation-proposal">
                    <h4>{text.captureProposal}</h4>
                    <strong>
                      {session.captureDistillation.proposal.title}
                    </strong>
                    <p>{session.captureDistillation.proposal.content}</p>
                  </div>
                )}
                {session.captureDistillation.distillation?.retryAllowed && (
                  <button
                    type="button"
                    disabled={saving}
                    data-control="capture-distillation-retry"
                    onClick={() => {
                      setSaving(true);
                      setError("");
                      void taskClient
                        .retryJob(
                          session.captureDistillation!.distillation!.jobId,
                        )
                        .then(() => load())
                        .catch((e) =>
                          setError(String(e instanceof Error ? e.message : e)),
                        )
                        .finally(() => setSaving(false));
                    }}
                  >
                    {text.retry}
                  </button>
                )}
              </section>
            )}
            <details
              className="refinement-private-note"
              data-control="refinement-note-details"
            >
              <summary>{text.savedRefinementNote}</summary>
              <textarea
                aria-label={text.savedRefinementNote}
                data-control="refinement-note"
                value={draft}
                onChange={(event) => save(event.target.value)}
                placeholder={text.savedRefinementPlaceholder}
              />
            </details>
          </section>
          <section
            className="refinement-conversation"
            aria-label={text.conversation}
          >
            <div
              className="refinement-messages"
              ref={scrollRef}
              onScroll={(event) => {
                followBottom.current =
                  event.currentTarget.scrollHeight -
                    event.currentTarget.scrollTop -
                    event.currentTarget.clientHeight <
                  120;
                latest.current.scrollAnchor = String(
                  event.currentTarget.scrollTop,
                );
                scheduleSave();
              }}
            >
              {(
                session?.captureImages ??
                (session?.captureImage ? [session.captureImage] : [])
              ).map((image, index) => (
                <article key={index} className="message message-user">
                  <strong>{text.capture}</strong>
                  <InputImagePreview image={image} />
                </article>
              ))}
              {session?.messages?.map((item) => (
                <article
                  key={item.id}
                  className={`message message-${item.role}`}
                >
                  <strong>
                    {item.role === "user" ? text.you : text.assistant}
                  </strong>
                  <RefinementMessage
                    body={item.body}
                    reveal={revealingMessageIds.has(item.id)}
                  />
                  {(item.images ?? (item.image ? [item.image] : [])).map(
                    (image, index) => (
                      <InputImagePreview key={index} image={image} />
                    ),
                  )}
                </article>
              ))}
              {!session && <p className="region-empty">{text.loading}</p>}
            </div>
            {responding && (
              <p
                className="refinement-thinking"
                role="status"
                aria-label={text.assistantGenerating}
                aria-live="polite"
              >
                <span>{text.assistantGenerating}</span>
                <span className="refinement-thinking-dots" aria-hidden="true">
                  <span>...</span>
                </span>
              </p>
            )}
            <div className="refinement-composer">
              <div className="refinement-composer-input">
                {mentions.length > 0 && (
                  <div
                    className="reference-mention-tokens"
                    aria-label={referenceText.mentions}
                  >
                    {mentions.map((mention) => (
                      <span key={mention.mentionId}>
                        {mention.displayLabel}
                        <button
                          data-control="reference-mention-remove"
                          type="button"
                          aria-label={`${referenceText.removeMention}: ${mention.displayLabel}`}
                          onClick={() => {
                            const result = removeMention(
                              latest.current.message,
                              mentionRef.current,
                              mention.mentionId,
                            );
                            setComposer(result.value, result.mentions);
                            focusComposer(result.caret);
                          }}
                        >
                          ×
                        </button>
                      </span>
                    ))}
                  </div>
                )}
                {(mentionOptions.length > 0 ||
                  selectedMentionOptions.length > 0) && (
                  <div className="reference-mention-picker">
                    <div
                      id="reference-mention-options"
                      className="reference-mention-lookup"
                      role="listbox"
                      aria-multiselectable="true"
                      aria-label={referenceText.mentions}
                    >
                      {mentionOptions.slice(0, 12).map((reference, index) => (
                        <button
                          type="button"
                          role="option"
                          aria-selected={selectedMentionOptions.some(
                            (item) =>
                              referenceKey(item) === referenceKey(reference),
                          )}
                          id={`reference-mention-${index}`}
                          data-control="reference-mention-option"
                          key={referenceKey(reference)}
                          onMouseDown={(event) => event.preventDefault()}
                          onClick={() => toggleMention(reference)}
                        >
                          <span>
                            {reference.title || reference.documentId}
                            {reference.section ? ` · ${reference.section}` : ""}
                          </span>
                          <small>
                            {reference.documentId} · {reference.documentVersion}
                            {reference.status
                              ? ` · ${reference.status === "current" ? referenceText.currentStatus : reference.status === "historical" ? referenceText.historicalStatus : reference.status === "unverified" ? referenceText.unverified : reference.status === "deferred" ? referenceText.deferred : reference.status === "rejected" ? referenceText.rejected : reference.status}`
                              : ""}
                          </small>
                        </button>
                      ))}
                    </div>
                    <div className="reference-mention-batch">
                      <span>
                        {referenceText.selectedMentions}:{" "}
                        {selectedMentionOptions.length}
                      </span>
                      <button
                        type="button"
                        data-control="reference-mention-insert-selected"
                        disabled={!selectedMentionOptions.length}
                        onClick={() => insertMentions(selectedMentionOptions)}
                      >
                        {referenceText.insertSelectedMentions}
                      </button>
                    </div>
                  </div>
                )}
                <textarea
                  ref={composerRef}
                  disabled={saving}
                  onPaste={
                    !saving && !responding ? attachment.paste : undefined
                  }
                  aria-label={text.refinementMessage}
                  aria-describedby="reference-mention-hint"
                  aria-controls={
                    mentionOptions.length
                      ? "reference-mention-options"
                      : undefined
                  }
                  aria-activedescendant={
                    mentionOptions.length
                      ? `reference-mention-${mentionIndex}`
                      : undefined
                  }
                  data-control="refinement-message"
                  value={message}
                  onChange={(event) => setComposer(event.target.value)}
                  onKeyDown={(event) => {
                    if (
                      event.nativeEvent.isComposing ||
                      event.nativeEvent.keyCode === 229
                    )
                      return;
                    if (event.key === "Backspace" || event.key === "Delete") {
                      const result = deleteMentionRange(
                        latest.current.message,
                        mentionRef.current,
                        event.currentTarget.selectionStart,
                        event.currentTarget.selectionEnd,
                        event.key,
                      );
                      if (result) {
                        event.preventDefault();
                        setComposer(result.value, result.mentions);
                        focusComposer(result.caret);
                        return;
                      }
                    }
                    if (
                      mentionOptions.length &&
                      ["ArrowDown", "ArrowUp", "Enter", " ", "Escape"].includes(
                        event.key,
                      )
                    ) {
                      event.preventDefault();
                      event.stopPropagation();
                      if (event.key === "Escape") {
                        setMentionDismissed(true);
                        setMentionOptions([]);
                        setSelectedMentionOptions([]);
                      } else if (event.key === "Enter" || event.key === " ")
                        insertMentions(
                          selectedMentionOptions.length
                            ? selectedMentionOptions
                            : [mentionOptions[mentionIndex]],
                        );
                      else
                        setMentionIndex(
                          (index) =>
                            (index +
                              (event.key === "ArrowDown" ? 1 : -1) +
                              Math.min(12, mentionOptions.length)) %
                            Math.min(12, mentionOptions.length),
                        );
                      return;
                    }
                    if (event.key === "Enter" && !event.shiftKey) {
                      event.preventDefault();
                      void send();
                    }
                  }}
                  placeholder={text.refinementPlaceholder}
                  rows={2}
                />
                <small id="reference-mention-hint">
                  {referenceText.mentionHint}
                </small>
                <InputImageAttachment
                  attachment={attachment}
                  disabled={saving || responding}
                />
              </div>
              <button
                type="button"
                className="primary"
                disabled={
                  !session ||
                  saving ||
                  responding ||
                  loading ||
                  Boolean(loadError) ||
                  attachment.reading ||
                  (!message.trim() && !attachment.image)
                }
                onClick={() => void send()}
                data-chat-control="refinement-send"
                data-control="refinement-send"
              >
                {text.send}
              </button>
            </div>
          </section>
        </div>
      </section>
    </div>,
    document.body,
  );
}
