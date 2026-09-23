import { useEffect, useRef, useState } from "react";
import { taskClient } from "../../services/taskClient";
import { taskExecutionClient } from "../../services/taskExecutionClient";
import { formatSystemTime } from "../../services/systemTime";
import type {
  TaskAggregate,
  TaskWorkSession,
  TaskWorkSessionAttachment,
  TaskWorkSessionRecord,
  TaskExecutionSnapshot,
  TaskExecutionFormalRequest,
} from "../../types/taskWorkbench";

export type WorkSessionDraft = {
  body: string;
  attachment?: TaskWorkSessionAttachment;
  operationId: string;
  attemptedPayload?: string;
  formalAnswers?: Record<string, string[]>;
  executionAttempt?: { payload: string; operationId: string; retryOfRunId?: string };
};
const models = [
  ["gpt-5.6-sol", "GPT-5.6-Sol"],
  ["gpt-5.6-terra", "GPT-5.6-Terra"],
  ["gpt-5.6-luna", "GPT-5.6-Luna"],
  ["gpt-6-astra", "GPT-6-Astra"],
  ["gpt-5.5", "GPT-5.5"],
] as const;
const copy = {
  en: {
    title: "Work sessions",
    create: "New session",
    empty: "Create a session when you are ready to record focused work.",
    select: "Session",
    provider: "Provider",
    model: "Model",
    approval: "Approval reviewer",
    approvalHint: "Ask routes supported Codex approvals to you. Auto review needs a current opt-in.",
    askMe: "Ask me",
    automaticReview: "Automatic review",
    workspace: "Project folder path",
    workspaceHint:
      "Saved for a future Codex Run. The effective folder and safety policy appear before execution.",
    settings: "Save settings",
    saved: "Settings saved",
    message: "Instruction or note",
    placeholder: "Record what you tried, decided, or want to continue…",
    attach: "Attach file",
    send: "Save note",
    run: "Run with Codex",
    prepare: "Prepare conversation",
    checkSettings: "Check settings",
    prepareHint: "Connects this session and checks settings. Work starts only when you run an instruction.",
    editSettings: "Edit settings",
    runUnavailable: "Enter an instruction and save a project folder before running Codex.",
    effective: "Effective execution settings",
    runStatus: "Run status",
    runHistory: "Run history",
    activeRun: "Active Run",
    liveKinds: { commandExecution: "Command", fileChange: "File change", agentMessage: "Codex response", reasoning: "Reasoning", webSearch: "Web search", mcpToolCall: "Tool call" },
    openRun: "Open Run",
    status: { queued: "Queued", running: "Running", awaiting_response: "Response needed", succeeded: "Finished", failed: "Failed", cancelled: "Cancelled", interrupted: "Interrupted", needs_attention: "Needs attention" },
    stop: "Stop",
    stopRequested: "Stop requested — waiting for Codex terminal status.",
    outcomeUnknown: "Outcome unknown — work may already have happened. Check the saved evidence before starting another Run.",
    continues: "Codex can continue while you answer this request.",
    blocking: "Codex is waiting for this response.",
    retryRun: "Use instruction again",
    retryDraftHint: "Save or clear your current draft before reusing this instruction.",
    workLog: "Open Work Log entry",
    syncWorkLog: "Retry Work Log sync",
    syncPending: "Work Log sync pending.",
    syncFailed: "Work Log sync failed.",
    evidence: "Observed evidence",
    report: "Codex final report",
    noReport: "Codex did not provide a final report.",
    approvalWaiting: "Codex is waiting for your approval.",
    questionWaiting: "Codex is waiting for your input.",
    submitAnswers: "Submit answers",
    other: "Additional detail",
    secret: "This request needs secret handling in Codex and cannot be answered here.",
    requestAnswered: "Response recorded",
    requestStale: "This request is no longer actionable.",
    user: "You",
    note: "Note",
    context: "Current Task context",
    outcome: "Outcome",
    references: "Existing references",
    noReferences: "No linked references",
    errorFile: "Choose a file up to 10 MB.",
    download: "Download attachment",
    pending: "Unsaved attachment",
    createName: "Session",
    retry: "Save failed. Your text and attachment are still here; try again.",
    reload:
      "The record was saved, but the refreshed session could not be loaded. Reopen the session to see it.",
    loading: "Loading sessions…",
    remove: "Remove attachment",
  },
  ko: {
    title: "작업 세션",
    create: "새 세션",
    empty: "집중해서 진행한 내용을 기록할 준비가 되면 세션을 만드세요.",
    select: "세션",
    provider: "제공자",
    model: "모델",
    approval: "승인 검토자",
    approvalHint: "지원되는 Codex 승인 요청을 사용자에게 보냅니다. 자동 검토는 현재 명시적 선택이 필요합니다.",
    askMe: "사용자에게 묻기",
    automaticReview: "자동 검토",
    workspace: "프로젝트 폴더 경로",
    workspaceHint:
      "향후 Codex 실행에 사용할 설정입니다. 실행 전 실제 폴더와 안전 정책을 표시합니다.",
    settings: "설정 저장",
    saved: "설정을 저장했습니다",
    message: "지시 또는 메모",
    placeholder: "시도한 내용, 결정, 다음에 이어갈 내용을 기록하세요…",
    attach: "파일 첨부",
    send: "메모 저장",
    run: "Codex로 실행",
    prepare: "대화 준비",
    checkSettings: "설정 확인",
    prepareHint: "이 세션을 연결하고 설정을 확인합니다. 지시를 실행할 때만 작업이 시작됩니다.",
    editSettings: "설정 편집",
    runUnavailable: "Codex를 실행하려면 지시와 프로젝트 폴더를 입력하세요.",
    effective: "실제 실행 설정",
    runStatus: "실행 상태",
    runHistory: "실행 기록",
    activeRun: "진행 중인 실행",
    liveKinds: { commandExecution: "명령", fileChange: "파일 변경", agentMessage: "Codex 응답", reasoning: "추론", webSearch: "웹 검색", mcpToolCall: "도구 호출" },
    openRun: "실행 열기",
    status: { queued: "대기 중", running: "실행 중", awaiting_response: "응답 필요", succeeded: "완료됨", failed: "실패", cancelled: "취소됨", interrupted: "중단됨", needs_attention: "확인 필요" },
    stop: "중지",
    stopRequested: "중지를 요청했습니다. Codex의 최종 상태를 기다리는 중입니다.",
    outcomeUnknown: "결과를 알 수 없습니다. 작업이 이미 수행되었을 수 있으니 새 실행 전 저장된 근거를 확인하세요.",
    continues: "이 요청에 답하는 동안 Codex는 계속 진행할 수 있습니다.",
    blocking: "Codex가 이 응답을 기다리고 있습니다.",
    retryRun: "지시 다시 사용",
    retryDraftHint: "이 지시를 다시 사용하려면 현재 초안을 저장하거나 비우세요.",
    workLog: "Work Log 항목 열기",
    syncWorkLog: "Work Log 동기화 다시 시도",
    syncPending: "Work Log 동기화가 보류 중입니다.",
    syncFailed: "Work Log 동기화에 실패했습니다.",
    evidence: "관찰된 근거",
    report: "Codex 최종 보고",
    noReport: "Codex가 최종 보고를 제공하지 않았습니다.",
    approvalWaiting: "Codex가 사용자의 승인을 기다리고 있습니다.",
    questionWaiting: "Codex가 사용자의 입력을 기다리고 있습니다.",
    submitAnswers: "답변 제출",
    other: "추가 세부 사항",
    secret: "이 요청은 Codex의 비밀값 처리 기능이 필요하므로 여기에서 답변할 수 없습니다.",
    requestAnswered: "응답을 기록했습니다",
    requestStale: "이 요청은 더 이상 처리할 수 없습니다.",
    user: "사용자",
    note: "메모",
    context: "현재 Task 맥락",
    outcome: "목표 결과",
    references: "기존 참조",
    noReferences: "연결된 참조 없음",
    errorFile: "10 MB 이하의 파일을 선택하세요.",
    download: "첨부 파일 다운로드",
    pending: "저장되지 않은 첨부",
    createName: "세션",
    retry:
      "저장하지 못했습니다. 작성 내용과 첨부는 유지됩니다. 다시 시도하세요.",
    reload:
      "기록은 저장했지만 세션을 다시 불러오지 못했습니다. 세션을 다시 열어 확인하세요.",
    loading: "세션을 불러오는 중…",
    remove: "첨부 제거",
  },
};
const operationId = () =>
  globalThis.crypto?.randomUUID?.() ??
  `session-entry-${Date.now()}-${Math.random()}`;
const readAttachment = (file: File): Promise<TaskWorkSessionAttachment> =>
  new Promise((resolve, reject) => {
    if (file.size > 10 * 1024 * 1024) {
      reject(new Error("size"));
      return;
    }
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error);
    reader.onload = () =>
      resolve({
        name: file.name,
        mediaType: file.type || "application/octet-stream",
        data: String(reader.result).split(",")[1] ?? "",
      });
    reader.readAsDataURL(file);
  });

export function TaskWorkSessions({
  task,
  draftStore,
  focusExecution,
  onOpenWorkLog,
}: {
  task: TaskAggregate;
  draftStore?: Map<string, WorkSessionDraft>;
  focusExecution?: { sessionId: string; runId: string };
  onOpenWorkLog?: (entryId: string) => void;
}) {
  const t = document.documentElement.lang.startsWith("ko") ? copy.ko : copy.en;
  const fallback = useRef(new Map<string, WorkSessionDraft>());
  const drafts = draftStore ?? fallback.current;
  const [sessions, setSessions] = useState<TaskWorkSession[]>();
  const [activeId, setActiveId] = useState("");
  const [record, setRecord] = useState<TaskWorkSessionRecord>();
  const [draft, setDraft] = useState("");
  const [attachment, setAttachment] = useState<TaskWorkSessionAttachment>();
  const [error, setError] = useState("");
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const [runSubmitting, setRunSubmitting] = useState(false);
  const [execution, setExecution] = useState<TaskExecutionSnapshot>();
  const receiveExecution = (snapshot: TaskExecutionSnapshot) => setExecution((current) =>
    current?.revision !== undefined && snapshot.revision !== undefined && snapshot.revision < current.revision
      ? current : snapshot);
  const [answers, setAnswers] = useState<Record<string, string[]>>({});
  const [requestBusy, setRequestBusy] = useState("");
  const [editingSettings, setEditingSettings] = useState(true);
  const [selectedRunId, setSelectedRunId] = useState("");
  const [retryOfRunId, setRetryOfRunId] = useState<string>();
  const [syncBusy, setSyncBusy] = useState("");
  const [syncError, setSyncError] = useState<Record<string, string>>({});
  const generation = useRef(0);
  const activeRef = useRef("");
  const draftRef = useRef(draft);
  const attachmentRef = useRef(attachment);
  useEffect(() => {
    draftRef.current = draft;
    attachmentRef.current = attachment;
    if (activeRef.current) {
      const existing = drafts.get(activeRef.current);
      drafts.set(activeRef.current, {
        ...existing,
        body: draft,
        attachment,
        operationId: existing?.operationId ?? operationId(),
        attemptedPayload: existing?.attemptedPayload,
      });
    }
  }, [attachment, draft, drafts]);
  const loadList = async (expected: number) => {
    try {
      const next = await taskClient.workSessions(task.id);
      if (generation.current === expected) {
        setSessions(next.sessions);
        setError("");
      }
    } catch (cause) {
      if (generation.current === expected) setError(String(cause));
    }
  };
  useEffect(() => {
    const expected = ++generation.current;
    activeRef.current = "";
    setSessions(undefined);
    setActiveId("");
    setRecord(undefined);
    setDraft("");
    setAttachment(undefined);
    setBusy(false);
    setRunSubmitting(false);
    setStatus("");
    setError("");
    setExecution(undefined);
    setAnswers({});
    setRequestBusy("");
    setEditingSettings(true);
    setSelectedRunId("");
    setRetryOfRunId(undefined);
    setSyncBusy("");
    setSyncError({});
    void loadList(expected);
    return () => {
      generation.current += 1;
    };
  }, [task.id]);
  useEffect(() => {
    if (focusExecution?.sessionId && focusExecution.sessionId !== activeRef.current) void open(focusExecution.sessionId);
  }, [focusExecution?.runId, focusExecution?.sessionId]);
  const open = async (id: string) => {
    if (busy) return;
    if (activeRef.current) {
      const existing = drafts.get(activeRef.current);
      drafts.set(activeRef.current, {
        ...existing,
        body: draftRef.current,
        attachment: attachmentRef.current,
        operationId: existing?.operationId ?? operationId(),
        attemptedPayload: existing?.attemptedPayload,
      });
    }
    const expected = ++generation.current;
    activeRef.current = id;
    setActiveId(id);
    setRecord(undefined);
    setExecution(undefined);
    setSelectedRunId("");
    setRetryOfRunId(undefined);
    setRequestBusy("");
    setRunSubmitting(false);
    setError("");
    setStatus("");
    const saved = drafts.get(id);
    setDraft(saved?.body ?? "");
    setAttachment(saved?.attachment);
    setAnswers(saved?.formalAnswers ?? {});
    setEditingSettings(true);
    if (!id) return;
    try {
      const next = await taskClient.workSession(task.id, id);
      if (generation.current === expected && activeRef.current === id)
        setRecord(next);
    } catch (cause) {
      if (generation.current === expected) setError(String(cause));
    }
  };
  useEffect(() => {
    if (!record || activeRef.current !== record.session.id) return;
    const expected = generation.current;
    void taskExecutionClient.subscribe({ taskId: task.id, sessionId: record.session.id, runId: focusExecution?.sessionId === record.session.id ? focusExecution.runId : undefined }, (snapshot) => {
      if (generation.current === expected && activeRef.current === record.session.id) receiveExecution(snapshot);
    }).then((snapshot) => {
      if (generation.current === expected && activeRef.current === record.session.id) receiveExecution(snapshot);
    }).catch((cause) => {
      if (generation.current === expected) setError(String(cause));
    });
  }, [focusExecution?.runId, focusExecution?.sessionId, record?.session.id, task.id]);
  const create = async () => {
    if (busy) return;
    setBusy(true);
    const expected = generation.current;
    try {
      const created = await taskClient.createWorkSession(
        task.id,
        `${t.createName} ${(sessions?.length ?? 0) + 1}`,
      );
      if (generation.current !== expected) return;
      setSessions((current) => [created, ...(current ?? [])]);
      setBusy(false);
      await open(created.id);
    } catch (cause) {
      if (generation.current === expected) setError(String(cause));
    } finally {
      setBusy(false);
    }
  };
  const updateSession = (patch: Partial<TaskWorkSession>) =>
    setRecord(
      (current) =>
        current && { ...current, session: { ...current.session, ...patch } },
    );
  const saveSettings = async () => {
    if (!record || busy) return;
    const id = record.session.id;
    const expected = generation.current;
    setBusy(true);
    setStatus("");
    try {
      const saved = await taskClient.saveWorkSession(task.id, record.session);
      if (generation.current === expected && activeRef.current === id) {
        setRecord(
          (current) =>
            current && {
              ...current,
              session: { ...current.session, ...saved },
            },
        );
        setSessions((current) =>
          current?.map((item) =>
            item.id === id ? { ...item, ...saved } : item,
          ),
        );
        setStatus(t.saved);
        setExecution(undefined);
        setEditingSettings(false);
      }
    } catch (cause) {
      if (generation.current === expected) setError(String(cause));
    } finally {
      if (generation.current === expected) setBusy(false);
    }
  };
  const prepare = async () => {
    if (!record || busy) return;
    const id = record.session.id;
    const expected = generation.current;
    setBusy(true); setError("");
    try {
      const snapshot = await taskExecutionClient.prepare({ taskId: task.id, sessionId: id }, (next) => {
        if (generation.current === expected && activeRef.current === id) receiveExecution(next);
      });
      if (generation.current === expected && activeRef.current === id) receiveExecution(snapshot);
    } catch (cause) { if (generation.current === expected) setError(String(cause)); }
    finally { if (generation.current === expected) setBusy(false); }
  };
  const send = async () => {
    if (!record || busy || (!draft.trim() && !attachment)) return;
    const id = record.session.id;
    const expected = generation.current;
    const payload = JSON.stringify({ body: draft, attachment });
    const existing = drafts.get(id);
    const pending =
      existing?.attemptedPayload === payload
        ? existing
        : { body: draft, attachment, operationId: operationId() };
    pending.body = draft;
    pending.attachment = attachment;
    pending.attemptedPayload = payload;
    drafts.set(id, pending);
    setBusy(true);
    setError("");
    try {
      await taskClient.appendWorkSessionEntry(
        task.id,
        id,
        pending.body,
        pending.attachment,
        pending.operationId,
      );
      if (generation.current !== expected || activeRef.current !== id) return;
      drafts.delete(id);
      setDraft("");
      setAttachment(undefined);
      try {
        const next = await taskClient.workSession(task.id, id);
        if (generation.current === expected && activeRef.current === id)
          setRecord(next);
        await loadList(expected);
      } catch {
        if (generation.current === expected) setError(t.reload);
      }
    } catch {
      if (generation.current === expected && activeRef.current === id)
        setError(t.retry);
    } finally {
      if (generation.current === expected) setBusy(false);
    }
  };
  const run = async () => {
    if (!record || runSubmitting || !draft.trim() || !record.session.workspacePath.trim()) {
      if (record && !record.session.workspacePath.trim()) setError(t.runUnavailable);
      return;
    }
    if (!execution?.effectiveConfig?.ready) {
      setError(t.prepareHint);
      return;
    }
    const id = record.session.id;
    const expected = generation.current;
    const submittedInstruction = draft;
    const payload = JSON.stringify({ instruction: submittedInstruction, settingsRevision: execution.effectiveConfig.settingsRevision, retryOfRunId, attachment });
    const savedDraft = drafts.get(id);
    const attempt = savedDraft?.executionAttempt?.payload === payload
      ? savedDraft.executionAttempt
      : { payload, operationId: operationId(), retryOfRunId };
    drafts.set(id, { body: draft, attachment, operationId: savedDraft?.operationId ?? operationId(), formalAnswers: answers, executionAttempt: attempt });
    setRunSubmitting(true); setError("");
    try {
      const snapshot = await taskExecutionClient.execute({ taskId: task.id, sessionId: id, operationId: attempt.operationId, instruction: submittedInstruction, settingsRevision: execution.effectiveConfig.settingsRevision, retryOfRunId: attempt.retryOfRunId, attachment }, (next) => {
        if (generation.current === expected && activeRef.current === id) receiveExecution(next);
      });
      if (generation.current === expected && activeRef.current === id) {
        receiveExecution(snapshot);
        if (draftRef.current === submittedInstruction) setDraft("");
        const current = drafts.get(id);
        if (current?.executionAttempt?.operationId === attempt.operationId) {
          drafts.set(id, { ...current, body: draftRef.current === submittedInstruction ? "" : draftRef.current, executionAttempt: undefined });
        }
        setRetryOfRunId(undefined);
      }
    } catch (cause) { if (generation.current === expected) setError(String(cause)); }
    finally { if (generation.current === expected) setRunSubmitting(false); }
  };
  const activeRun = execution?.runs.find((value) => value.id === selectedRunId) ?? execution?.selectedRun ?? execution?.runs.find((value) => value.id === execution.activeRunId) ?? execution?.runs[0];
  const runCanAcceptResponse = Boolean(activeRun && ["queued", "running", "awaiting_response"].includes(activeRun.status));
  const requestIsLive = (request: TaskExecutionFormalRequest) => runCanAcceptResponse && ["pending", "submitting", "error"].includes(request.status);
  const requestCanAcceptResponse = (request: TaskExecutionFormalRequest) => runCanAcceptResponse && ["pending", "error"].includes(request.status);
  useEffect(() => {
    if (focusExecution?.runId !== activeRun?.id) return;
    const frame = requestAnimationFrame(() => document.querySelector<HTMLElement>("[data-control='task-execution-heading']")?.focus({ preventScroll: true }));
    return () => cancelAnimationFrame(frame);
  }, [activeRun?.id, focusExecution?.runId]);
  const answerKey = (runId: string, requestId: string, questionId: string) => `${runId}/${requestId}/${questionId}`;
  const answerValues = (runId: string, request: TaskExecutionFormalRequest, questionId: string) =>
    answers[answerKey(runId, request.id, questionId)] ?? request.proposedResponse?.answers?.[questionId]?.answers ?? [];
  const savedAnswerSummary = (request: TaskExecutionFormalRequest) => {
    const saved = request.status === "answered" ? request.response : request.proposedResponse;
    return saved?.decision ?? request.questions.map((question) =>
      (saved?.answers?.[question.id]?.answers ?? answers[answerKey(activeRun?.id ?? "", request.id, question.id)] ?? []).join(", "),
    ).filter(Boolean).join("; ");
  };
  const rememberAnswers = (next: Record<string, string[]>) => {
    setAnswers(next);
    if (activeRef.current) {
      const existing = drafts.get(activeRef.current) ?? { body: draftRef.current, attachment: attachmentRef.current, operationId: operationId() };
      drafts.set(activeRef.current, { ...existing, formalAnswers: next });
    }
  };
  const submitResponse = async (request: TaskExecutionFormalRequest, decision?: string) => {
    if (!record || !activeRun || !requestCanAcceptResponse(request)) return;
    const nonSecret = request.questions.filter((question) => !question.isSecret);
    if (!decision && request.questions.some((question) => question.isSecret)) return;
    const missing = !decision && nonSecret.find((question) => !(answerValues(activeRun.id, request, question.id)).some(Boolean));
    if (missing) {
      Array.from(document.querySelectorAll<HTMLElement>("[data-question-id]"))
        .find((element) => element.dataset.questionId === missing.id)?.focus();
      return;
    }
    setRequestBusy(request.id); setError("");
    const expected = generation.current;
    try {
      const snapshot = await taskExecutionClient.respond({ taskId: task.id, sessionId: record.session.id, runId: activeRun.id, requestId: request.id, response: decision ? { decision } : { answers: Object.fromEntries(nonSecret.map((question) => [question.id, { answers: answerValues(activeRun.id, request, question.id) }])) } });
      if (generation.current === expected) receiveExecution(snapshot);
    } catch (cause) { if (generation.current === expected) setError(String(cause)); }
    finally { if (generation.current === expected) setRequestBusy(""); }
  };
  const stop = async () => {
    if (!record || !activeRun || busy) return;
    setBusy(true); setError("");
    const expected = generation.current;
    try {
      const snapshot = await taskExecutionClient.interrupt({ taskId: task.id, sessionId: record.session.id, runId: activeRun.id });
      if (generation.current === expected) receiveExecution(snapshot);
    }
    catch (cause) { if (generation.current === expected) setError(String(cause)); }
    finally { if (generation.current === expected) setBusy(false); }
  };
  const syncWorkLog = async (run: NonNullable<typeof activeRun>) => {
    if (!record || syncBusy || !run.workLogEntryId) return;
    const expected = generation.current;
    setSyncBusy(run.id);
    setSyncError((current) => ({ ...current, [run.id]: "" }));
    try {
      const snapshot = await taskExecutionClient.syncWorkLog({ taskId: task.id, sessionId: record.session.id, runId: run.id });
      if (generation.current === expected && activeRef.current === record.session.id) receiveExecution(snapshot);
    } catch (cause) {
      if (generation.current === expected) setSyncError((current) => ({ ...current, [run.id]: String(cause) }));
    } finally {
      if (generation.current === expected) setSyncBusy("");
    }
  };
  const chooseFile = (file?: File) => {
    if (!file) return;
    const expected = generation.current;
    const id = activeRef.current;
    void readAttachment(file)
      .then((value) => {
        if (generation.current === expected && activeRef.current === id) {
          setAttachment(value);
          setError("");
        }
      })
      .catch(() => {
        if (generation.current === expected) setError(t.errorFile);
      });
  };
  const references = [
    ...(task.problemLinks ?? []).map(
      (link) => `Problem ${link.problemId} · r${link.problemRevision}`,
    ),
    ...(task.relationships ?? []).map(
      (link) => `${link.kind} · ${link.targetTaskId}`,
    ),
  ];
  const linkedUserEntryIds = new Set((execution?.runs ?? []).flatMap((run) => run.userEntryId ? [run.userEntryId] : []));
  return (
    <section className="task-session-space" aria-label={t.title}>
      <header className="task-session-toolbar">
        <div>
          <h3>{t.title}</h3>
          <p>{t.workspaceHint}</p>
        </div>
        <button
          type="button"
          data-control="task-session-create"
          disabled={busy}
          onClick={() => void create()}
        >
          {t.create}
        </button>
      </header>
      {error && (
        <p role="alert" className="task-session-error">
          {error}
        </p>
      )}
      {sessions === undefined ? (
        <p>{t.loading}</p>
      ) : sessions.length === 0 ? (
        <div className="task-session-empty">
          <p>{t.empty}</p>
        </div>
      ) : (
        <>
          <label className="task-session-picker">
            {t.select}
            <select
              data-control="task-session-select"
              disabled={busy}
              value={activeId}
              onChange={(event) => void open(event.target.value)}
            >
              <option value="">—</option>
              {sessions.map((session) => (
                <option key={session.id} value={session.id}>
                  {session.title}
                </option>
              ))}
            </select>
          </label>
          {record && (
            <div className="task-session-layout">
              <div className="task-session-chat">
                <ol className="task-session-entries">
                  {record.entries.filter((entry) => !linkedUserEntryIds.has(entry.id)).map((entry) => (
                    <li key={entry.id} data-entry-author={entry.author}>
                      <header>
                        <strong>
                          {entry.author === "user" ? t.user : entry.author}
                        </strong>
                        <span>
                          {entry.kind === "note" ? t.note : entry.kind}
                        </span>
                        <time>
                          {formatSystemTime(
                            entry.createdAt,
                            document.documentElement.lang || navigator.language,
                            { dateStyle: "medium", timeStyle: "short" },
                          )}
                        </time>
                      </header>
                      {entry.body && <p>{entry.body}</p>}
                      {entry.attachment && (
                        <>
                          {entry.attachment.mediaType.startsWith("image/") && (
                            <img
                              className="task-session-saved-image"
                              src={`data:${entry.attachment.mediaType};base64,${entry.attachment.data}`}
                              alt={entry.attachment.name}
                            />
                          )}
                          <a
                            data-control="task-session-attachment-download"
                            download={entry.attachment.name}
                            href={`data:${entry.attachment.mediaType};base64,${entry.attachment.data}`}
                          >
                            {t.download}: {entry.attachment.name}
                          </a>
                        </>
                      )}
                    </li>
                  ))}
                </ol>
                {execution?.runs.length ? <section className="task-execution-history" aria-label={t.runHistory}>
                  <h4>{t.runHistory}</h4>
                  <ol>{execution.runs.map((run) => <li key={run.id}><button type="button" data-control="task-session-run-open" aria-pressed={activeRun?.id === run.id} onClick={() => setSelectedRunId(run.id)}>{run.id === execution.activeRunId ? `${t.activeRun} · ` : ""}{t.status[run.status]} · {run.instruction}</button></li>)}</ol>
                </section> : null}
                {activeRun && (
                  <section className="task-execution" aria-label={t.runStatus}>
                    <header>
                      <div>
                        <h4 data-control="task-execution-heading" tabIndex={-1}>{t.runStatus}: {t.status[activeRun.status]}</h4>
                        <p className="task-execution-meta">{activeRun.model} · {activeRun.workspacePath}</p>
                      </div>
                      {["queued", "running", "awaiting_response"].includes(activeRun.status) && (
                        <button type="button" data-control="task-session-stop" disabled={busy || activeRun.stopRequested} onClick={() => void stop()}>{activeRun.stopRequested ? t.stopRequested : t.stop}</button>
                      )}
                    </header>
                    <p>{activeRun.instruction}</p>
                    <p className="task-execution-live" role="status">{activeRun.stopRequested ? t.stopRequested : activeRun.status === "awaiting_response" && activeRun.formalRequests.some((request) => ["pending", "submitting", "error"].includes(request.status)) ? (activeRun.formalRequests.some((request) => request.kind === "user_input") ? t.questionWaiting : t.approvalWaiting) : ["interrupted", "needs_attention"].includes(activeRun.status) ? t.outcomeUnknown : activeRun.liveStatus ? `${t.liveKinds[activeRun.liveStatus.kind as keyof typeof t.liveKinds] ?? t.activeRun} · ${activeRun.liveStatus.status === "completed" ? t.status.succeeded : t.status.running}` : t.status[activeRun.status]}</p>
                    {activeRun.formalRequests.map((request) => (
                      <section className="task-execution-request" key={request.id} aria-label={request.title}>
                        <h5>{request.title}</h5><p>{request.prompt}</p>
                        {requestIsLive(request) && <p><strong>{request.isBlocking ? t.blocking : t.continues}</strong></p>}
                        {request.status === "answered" ? <p>{t.requestAnswered}: {savedAnswerSummary(request)}</p> : request.status === "stale" || !runCanAcceptResponse ? <p>{t.requestStale}: {savedAnswerSummary(request)}</p> : (
                          <>
                            {request.choices.map((choice) => <button key={choice.value} data-control="task-session-approval-choice" type="button" disabled={requestBusy === request.id || !["pending", "error"].includes(request.status)} onClick={() => void submitResponse(request, choice.value)}>{choice.label}{choice.description ? ` — ${choice.description}` : ""}</button>)}
                            {request.questions.map((question) => <fieldset key={question.id} data-question-id={question.id} tabIndex={-1} disabled={requestBusy === request.id || !requestCanAcceptResponse(request)}>
                              <legend>{question.header}</legend><p>{question.prompt}</p>
                              {question.isSecret ? <p>{t.secret}</p> : question.options.length === 0 ? <textarea data-question-id={question.id} data-control="task-session-question-text" aria-label={question.header} value={(answerValues(activeRun.id, request, question.id))[0] ?? ""} onChange={(event) => rememberAnswers({ ...answers, [answerKey(activeRun.id, request.id, question.id)]: [event.target.value] })} /> : <>
                                <div className="task-execution-options">{question.options.map((option) => <button key={option.value} data-control="task-session-question-option" type="button" aria-pressed={(answerValues(activeRun.id, request, question.id)).includes(option.value)} onClick={() => rememberAnswers({ ...answers, [answerKey(activeRun.id, request.id, question.id)]: [option.value] })}>{option.label}{option.description ? ` — ${option.description}` : ""}</button>)}</div>
                                {question.allowOther && <label>{t.other}<textarea data-control="task-session-question-other" value={(answerValues(activeRun.id, request, question.id)).find((value) => !question.options.some((option) => option.value === value)) ?? ""} onChange={(event) => rememberAnswers({ ...answers, [answerKey(activeRun.id, request.id, question.id)]: [event.target.value].filter(Boolean) })} /></label>}</>}
                            </fieldset>)}
                            {request.questions.length > 0 && !request.questions.some((question) => question.isSecret) && <button type="button" data-control="task-session-request-submit" disabled={requestBusy === request.id || !["pending", "error"].includes(request.status)} onClick={() => void submitResponse(request)}>{t.submitAnswers}</button>}
                            {request.status === "error" && <p role="alert">{request.error}</p>}
                          </>
                        )}
                      </section>
                    ))}
                    {activeRun.finalReport ? <section><h5>{t.report}</h5><p>{activeRun.finalReport}</p></section> : ["succeeded", "failed", "cancelled", "interrupted", "needs_attention"].includes(activeRun.status) && <p>{t.noReport}</p>}
                    {activeRun.error && <p role="alert">{activeRun.error.message}</p>}
                    {!!activeRun.evidence.length && <section><h5>{t.evidence}</h5><ul>{activeRun.evidence.map((evidence) => <li key={evidence.id}><strong>{evidence.label}</strong>: {evidence.summary}</li>)}</ul></section>}
                    {activeRun.workLogEntryId && <button type="button" data-control="task-session-worklog-open" onClick={() => onOpenWorkLog?.(activeRun.workLogEntryId!)}>{t.workLog}</button>}
                    {activeRun.workLogSyncState === "pending" && <p role="status">{t.syncPending}</p>}
                    {activeRun.workLogSyncState === "failed" && <><p role="alert">{t.syncFailed} {syncError[activeRun.id] || activeRun.workLogSyncError}</p><button type="button" data-control="task-session-worklog-sync" disabled={syncBusy === activeRun.id} onClick={() => void syncWorkLog(activeRun)}>{t.syncWorkLog}</button></>}
                    {["failed", "cancelled", "interrupted", "needs_attention"].includes(activeRun.status) && <><button type="button" data-control="task-session-run-retry" disabled={busy || Boolean(draft.trim())} onClick={() => { setDraft(activeRun.instruction); setRetryOfRunId(activeRun.id); }}>{t.retryRun}</button>{draft.trim() && <p>{t.retryDraftHint}</p>}</>}
                  </section>
                )}
                {execution?.effectiveConfig && <section className="task-execution-effective"><h5>{execution.effectiveConfig.provenance === "bound_thread" ? t.effective : t.prepare}</h5><dl><dt>{t.model}</dt><dd>{execution.effectiveConfig.model}</dd><dt>{t.workspace}</dt><dd>{execution.effectiveConfig.cwd}</dd><dt>{t.approval}</dt><dd>{execution.effectiveConfig.approvalPolicy} · {execution.effectiveConfig.approvalsReviewer}</dd><dt>Sandbox</dt><dd>{execution.effectiveConfig.sandbox}</dd></dl>{execution.effectiveConfig.readinessError && <p role="alert">{execution.effectiveConfig.readinessError.message}</p>}<button type="button" data-control="task-session-settings-edit" onClick={() => setEditingSettings(true)}>{t.editSettings}</button></section>}
                <label>
                  {t.message}
                  <textarea
                    data-control="task-session-message"
                    disabled={busy}
                    value={draft}
                    placeholder={t.placeholder}
                    onChange={(event) => setDraft(event.target.value)}
                    onPaste={(event) => {
                      const file = [...event.clipboardData.files][0];
                      if (file) {
                        event.preventDefault();
                        chooseFile(file);
                      }
                    }}
                  />
                </label>
                {attachment && (
                  <div className="task-session-pending">
                    <span>
                      {t.pending}: {attachment.name}
                    </span>
                    {attachment.mediaType.startsWith("image/") && (
                      <img
                        src={`data:${attachment.mediaType};base64,${attachment.data}`}
                        alt={attachment.name}
                      />
                    )}
                    <button
                      type="button"
                      data-control="task-session-attachment-remove"
                      aria-label={t.remove}
                      disabled={busy}
                      onClick={() => setAttachment(undefined)}
                    >
                      ×
                    </button>
                  </div>
                )}
                <footer>
                  <label className="task-session-attach">
                    {t.attach}
                    <input
                      type="file"
                      data-control="task-session-attachment"
                      disabled={busy}
                      onChange={(event) => {
                        chooseFile(event.target.files?.[0]);
                        event.target.value = "";
                      }}
                    />
                  </label>
                  <button
                    className="primary"
                    type="button"
                    data-control="task-session-send"
                    disabled={busy || (!draft.trim() && !attachment)}
                    onClick={() => void send()}
                  >
                    {t.send}
                  </button>
                  <button className="primary" type="button" data-control="task-session-run" disabled={runSubmitting || !draft.trim() || !record.session.workspacePath.trim() || Boolean(execution?.activeRunId) || !execution?.effectiveConfig?.ready} onClick={() => void run()}>{t.run}</button>
                </footer>
              </div>
              <aside className="task-session-sidebar">
                <section>
                  <h4>{t.context}</h4>
                  {task.detail && <p>{task.detail}</p>}
                  {task.outcome && (
                    <p>
                      <strong>{t.outcome}:</strong> {task.outcome}
                    </p>
                  )}
                  <h4>{t.references}</h4>
                  {references.length ? (
                    <ul>
                      {references.map((value) => (
                        <li key={value}>{value}</li>
                      ))}
                    </ul>
                  ) : (
                    <p>{t.noReferences}</p>
                  )}
                </section>
                <section className="task-session-settings">
                  <button type="button" data-control="task-session-prepare" disabled={busy} onClick={() => void prepare()}>{execution?.effectiveConfig ? t.checkSettings : t.prepare}</button>
                  <p>{t.prepareHint}</p>
                  {editingSettings && <>
                  <label>
                    {t.select}
                    <input
                      data-control="task-session-title"
                      disabled={busy}
                      value={record.session.title}
                      onChange={(e) => updateSession({ title: e.target.value })}
                    />
                  </label>
                  <label>
                    {t.provider}
                    <select
                      data-control="task-session-provider"
                      disabled={busy}
                      value={record.session.provider}
                      onChange={(e) =>
                        updateSession({ provider: e.target.value as "codex" })
                      }
                    >
                      <option value="codex">Codex</option>
                    </select>
                  </label>
                  <label>
                    {t.model}
                    <select
                      data-control="task-session-model"
                      disabled={busy}
                      value={record.session.model}
                      onChange={(e) => updateSession({ model: e.target.value })}
                    >
                      {models.map(([id, label]) => (
                        <option key={id} value={id}>
                          {label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    {t.workspace}
                    <input
                      data-control="task-session-workspace"
                      disabled={busy}
                      value={record.session.workspacePath}
                      placeholder="/path/to/project"
                      onChange={(e) =>
                        updateSession({ workspacePath: e.target.value })
                      }
                    />
                  </label>
                  <label className="task-session-approval">
                    <select
                      data-control="task-session-approval"
                      disabled={busy}
                      value={record.session.approvalsReviewer ?? "user"}
                      onChange={(e) =>
                        updateSession({
                          approvalsReviewer: e.target.value as "user" | "auto_review",
                        })
                      }
                    ><option value="user">{t.askMe}</option><option value="auto_review">{t.automaticReview}</option></select>
                    <span>
                      <strong>{t.approval}</strong>
                      <small>{t.approvalHint}</small>
                    </span>
                  </label>
                  <button
                    type="button"
                    data-control="task-session-settings-save"
                    disabled={busy || !record.session.title.trim()}
                    onClick={() => void saveSettings()}
                  >
                    {t.settings}
                  </button>
                  </>}
                  {status && <p role="status">{status}</p>}
                </section>
              </aside>
            </div>
          )}
        </>
      )}
    </section>
  );
}
