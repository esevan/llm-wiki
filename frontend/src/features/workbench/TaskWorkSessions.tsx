import { useEffect, useLayoutEffect, useRef, useState } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { taskClient } from "../../services/taskClient";
import { taskExecutionClient } from "../../services/taskExecutionClient";
import { formatSystemTime } from "../../services/systemTime";
import { chooseProjectFolder } from "../../services/projectFolderClient";
import type {
  TaskAggregate,
  TaskWorkSession,
  TaskWorkSessionAttachment,
  TaskWorkSessionRecord,
  TaskExecutionSnapshot,
  TaskExecutionFormalRequest,
  TaskExecutionEvidence,
  CodexThreadSummary,
  CodexThreadTranscript,
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
    link: "Link Codex session",
    linkTitle: "Available Codex sessions",
    linkHint: "Choose an existing Codex or VS Code conversation to continue it with this Task.",
    linkAction: "Link to Task",
    linked: "Codex session linked",
    alreadyLinked: "Linked",
    noCodexSessions: "No saved Codex sessions were found.",
    codexLoading: "Loading Codex sessions…",
    closePicker: "Close session picker",
    loadMore: "Load more",
    loadEarlier: "Load earlier messages",
    empty: "Create a session when you are ready to record focused work.",
    select: "Session",
    sessionTitle: "Session title",
    provider: "Provider",
    model: "Model",
    approval: "Approval reviewer",
    approvalHint: "Ask routes supported Codex approvals to you. Auto review needs a current opt-in.",
    askMe: "Ask me",
    automaticReview: "Automatic review",
    workspace: "Project folder path",
    workspaceHint:
      "Saved for a future Codex Run. The effective folder and safety policy appear before execution.",
    recentFolders: "Recent folders",
    browse: "Browse…",
    folderPickerFailed: "The folder picker could not be opened. Enter a path manually.",
    settings: "Save settings",
    saved: "Settings saved",
    message: "Instruction or note",
    placeholder: "Record what you tried, decided, or want to continue…",
    attach: "Attach file",
    send: "Save note",
    messageMode: "While Codex is running",
    noteMode: "Save as note",
    queueMode: "Queue for next Run",
    steerMode: "Steer current Run",
    queueUnavailable: "Queueing is unavailable until the native Queue API is connected. This draft is not sent.",
    steerUnavailable: "Steering is unavailable until the native turn/steer API is connected. This draft is not sent.",
    run: "Run with Codex",
    prepare: "Prepare conversation",
    checkSettings: "Check settings",
    prepareHint: "Connects this session and checks settings. Work starts only when you run an instruction.",
    editSettings: "Edit settings",
    runUnavailable: "Enter an instruction and save a project folder before running Codex.",
    effective: "Effective execution settings",
    runStatus: "Run status",
    runHistory: "Run history",
    runCommand: "Run command",
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
    syncSucceeded: "Work Log synced.",
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
    pending: "Attached to draft",
    sendingAttachment: "Sending with this Run",
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
    link: "Codex 세션 연결",
    linkTitle: "사용 가능한 Codex 세션",
    linkHint: "기존 Codex 또는 VS Code 대화를 선택해 이 Task에서 이어서 진행하세요.",
    linkAction: "Task에 연결",
    linked: "Codex 세션을 연결했습니다",
    alreadyLinked: "연결됨",
    noCodexSessions: "저장된 Codex 세션을 찾지 못했습니다.",
    codexLoading: "Codex 세션을 불러오는 중…",
    closePicker: "세션 선택 닫기",
    loadMore: "더 불러오기",
    loadEarlier: "이전 메시지 불러오기",
    empty: "집중해서 진행한 내용을 기록할 준비가 되면 세션을 만드세요.",
    select: "세션",
    sessionTitle: "세션 제목",
    provider: "제공자",
    model: "모델",
    approval: "승인 검토자",
    approvalHint: "지원되는 Codex 승인 요청을 사용자에게 보냅니다. 자동 검토는 현재 명시적 선택이 필요합니다.",
    askMe: "사용자에게 묻기",
    automaticReview: "자동 검토",
    workspace: "프로젝트 폴더 경로",
    workspaceHint:
      "향후 Codex 실행에 사용할 설정입니다. 실행 전 실제 폴더와 안전 정책을 표시합니다.",
    recentFolders: "최근 폴더",
    browse: "찾아보기…",
    folderPickerFailed: "폴더 선택기를 열지 못했습니다. 경로를 직접 입력하세요.",
    settings: "설정 저장",
    saved: "설정을 저장했습니다",
    message: "지시 또는 메모",
    placeholder: "시도한 내용, 결정, 다음에 이어갈 내용을 기록하세요…",
    attach: "파일 첨부",
    send: "메모 저장",
    messageMode: "Codex 실행 중 입력",
    noteMode: "메모로 저장",
    queueMode: "다음 실행에 Queue",
    steerMode: "현재 실행에 Steering",
    queueUnavailable: "native Queue API가 연결되기 전까지 Queueing을 사용할 수 없습니다. 이 초안은 전송되지 않았습니다.",
    steerUnavailable: "native turn/steer API가 연결되기 전까지 Steering을 사용할 수 없습니다. 이 초안은 전송되지 않았습니다.",
    run: "Codex로 실행",
    prepare: "대화 준비",
    checkSettings: "설정 확인",
    prepareHint: "이 세션을 연결하고 설정을 확인합니다. 지시를 실행할 때만 작업이 시작됩니다.",
    editSettings: "설정 편집",
    runUnavailable: "Codex를 실행하려면 지시와 프로젝트 폴더를 입력하세요.",
    effective: "실제 실행 설정",
    runStatus: "실행 상태",
    runHistory: "실행 기록",
    runCommand: "명령 실행",
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
    syncSucceeded: "Work Log를 동기화했습니다.",
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
    pending: "초안에 첨부됨",
    sendingAttachment: "이번 실행과 함께 전송 중",
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

function SessionMarkdown({ children }: { children: string }) {
  return (
    <div className="task-session-markdown" data-user-content>
      <Markdown remarkPlugins={[remarkGfm]} skipHtml>{children}</Markdown>
    </div>
  );
}

function duplicatesFinalReport(summary: string, finalReport?: string) {
  if (!finalReport) return false;
  const evidenceText = summary.trim();
  const finalText = finalReport.trim();
  return evidenceText === finalText
    || (evidenceText.endsWith("…") && finalText.startsWith(evidenceText.slice(0, -1)));
}

function EvidenceRow({ evidence, runCommand }: { evidence: TaskExecutionEvidence; runCommand: string }) {
  const isCommand = evidence.kind === "commandExecution" || Boolean(evidence.command);
  const detail = [evidence.command, evidence.summary].filter(Boolean).join("\n\n");
  const paths = Array.isArray(evidence.paths) ? evidence.paths.filter((path): path is string => typeof path === "string") : [];
  return (
    <details className="task-execution-event" data-kind={evidence.kind}>
      <summary>
        <span className="task-execution-event-icon" aria-hidden="true">{isCommand ? ">_" : "·"}</span>
        <strong>{isCommand ? runCommand : evidence.label}</strong>
        <span className="task-execution-event-status">{typeof evidence.exitCode === "number" ? `exit ${evidence.exitCode}` : evidence.status}</span>
      </summary>
      {detail && <pre>{detail}</pre>}
      {paths.length ? <ul>{paths.map((path) => <li key={path}>{path}</li>)}</ul> : null}
    </details>
  );
}

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
  const [settingsExpanded, setSettingsExpanded] = useState(false);
  const [selectedRunId, setSelectedRunId] = useState("");
  const [retryOfRunId, setRetryOfRunId] = useState<string>();
  const [syncBusy, setSyncBusy] = useState("");
  const [syncError, setSyncError] = useState<Record<string, string>>({});
  const [recentFolders, setRecentFolders] = useState<string[]>([]);
  const [showCodexSessions, setShowCodexSessions] = useState(false);
  const [codexSessions, setCodexSessions] = useState<CodexThreadSummary[]>();
  const [linkingThreadId, setLinkingThreadId] = useState("");
  const [externalTranscript, setExternalTranscript] = useState<CodexThreadTranscript>();
  const [codexNextCursor, setCodexNextCursor] = useState<string>();
  const [loadingMoreCodex, setLoadingMoreCodex] = useState(false);
  const [loadingEarlierTurns, setLoadingEarlierTurns] = useState(false);
  const generation = useRef(0);
  const activeRef = useRef("");
  const draftRef = useRef(draft);
  const attachmentRef = useRef(attachment);
  const timelineRef = useRef<HTMLOListElement>(null);
  const followLatestRef = useRef(true);
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
      return next.sessions;
    } catch (cause) {
      if (generation.current === expected) setError(String(cause));
      return undefined;
    }
  };
  const loadCodexSessions = async () => {
    setCodexSessions(undefined);
    setError("");
    const expected = generation.current;
    try {
      const result = await taskExecutionClient.externalThreads({ taskId: task.id, sessionId: activeRef.current || undefined, limit: 50 });
      if (generation.current === expected) {
        setCodexSessions(result.threads);
        setCodexNextCursor(result.nextCursor ?? undefined);
      }
    } catch (cause) {
      if (generation.current === expected) setError(String(cause));
    }
  };
  const loadLinkedTranscript = async (sessionId: string, expected: number) => {
    try {
      const transcript = await taskExecutionClient.externalThread({ taskId: task.id, sessionId, limit: 10 });
      if (generation.current === expected && activeRef.current === sessionId) setExternalTranscript(transcript);
    } catch {
      // The local session remains usable if Codex is unavailable.
    }
  };
  const loadEarlierTurns = async () => {
    if (!record || !externalTranscript?.nextCursor || loadingEarlierTurns) return;
    setLoadingEarlierTurns(true);
    const sessionId = record.session.id;
    const expected = generation.current;
    try {
      const page = await taskExecutionClient.externalThread({ taskId: task.id, sessionId, cursor: externalTranscript.nextCursor, limit: 10 });
      if (generation.current === expected && activeRef.current === sessionId) {
        setExternalTranscript((current) => current && ({ ...current, thread: page.thread, turns: [...page.turns, ...current.turns], nextCursor: page.nextCursor }));
      }
    } catch (cause) {
      if (generation.current === expected) setError(String(cause));
    } finally {
      setLoadingEarlierTurns(false);
    }
  };
  const loadMoreCodexSessions = async () => {
    if (!codexNextCursor || loadingMoreCodex) return;
    setLoadingMoreCodex(true);
    const expected = generation.current;
    try {
      const result = await taskExecutionClient.externalThreads({ taskId: task.id, sessionId: activeRef.current || undefined, cursor: codexNextCursor, limit: 50 });
      if (generation.current === expected) {
        setCodexSessions((current) => [...(current ?? []), ...result.threads]);
        setCodexNextCursor(result.nextCursor ?? undefined);
      }
    } catch (cause) {
      if (generation.current === expected) setError(String(cause));
    } finally {
      setLoadingMoreCodex(false);
    }
  };
  const toggleCodexSessions = () => {
    if (showCodexSessions) setShowCodexSessions(false);
    else {
      setShowCodexSessions(true);
      void loadCodexSessions();
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
    setSettingsExpanded(false);
    setSelectedRunId("");
    setRetryOfRunId(undefined);
    setSyncBusy("");
    setSyncError({});
    try {
      const stored: unknown = JSON.parse(localStorage.getItem("llm-wiki:recent-project-folders") ?? "[]");
      setRecentFolders(Array.isArray(stored) ? stored.filter((value): value is string => typeof value === "string").slice(0, 10) : []);
    } catch {
      setRecentFolders([]);
    }
    setShowCodexSessions(false);
    setCodexSessions(undefined);
    setLinkingThreadId("");
    setExternalTranscript(undefined);
    setCodexNextCursor(undefined);
    setLoadingMoreCodex(false);
    setLoadingEarlierTurns(false);
    void loadList(expected).then((next) => {
      if (generation.current === expected && !activeRef.current && !focusExecution?.sessionId && next?.[0]) void open(next[0].id);
    });
    return () => {
      generation.current += 1;
    };
  }, [task.id]);
  useEffect(() => {
    if (sessions !== undefined && focusExecution?.sessionId && focusExecution.sessionId !== activeRef.current) void open(focusExecution.sessionId);
  }, [focusExecution?.runId, focusExecution?.sessionId, sessions]);
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
    followLatestRef.current = true;
    setActiveId(id);
    setRecord(undefined);
    setExecution(undefined);
    setExternalTranscript(undefined);
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
      if (generation.current === expected && activeRef.current === id) {
        setRecord(next);
        void loadLinkedTranscript(id, expected);
      }
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
  const linkCodexSession = async (thread: CodexThreadSummary) => {
    if (linkingThreadId || busy) return;
    setLinkingThreadId(thread.id);
    setError("");
    const expected = generation.current;
    try {
      const linked = await taskExecutionClient.linkExternalThread({
        taskId: task.id,
        sessionId: activeRef.current || undefined,
        threadId: thread.id,
      });
      if (generation.current !== expected) return;
      await loadList(expected);
      await open(linked.sessionId);
      if (activeRef.current === linked.sessionId) setExternalTranscript(linked);
      setShowCodexSessions(false);
      setStatus(t.linked);
    } catch (cause) {
      if (generation.current === expected) setError(String(cause));
    } finally {
      setLinkingThreadId("");
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
        if (saved.workspacePath.trim()) {
          const next = [saved.workspacePath.trim(), ...recentFolders.filter((path) => path !== saved.workspacePath.trim())].slice(0, 10);
          setRecentFolders(next);
          localStorage.setItem("llm-wiki:recent-project-folders", JSON.stringify(next));
        }
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
  const browseProjectFolder = async () => {
    try {
      const selected = await chooseProjectFolder();
      if (selected && activeRef.current) updateSession({ workspacePath: selected });
    } catch (cause) {
      setError(`${t.folderPickerFailed} ${String(cause)}`);
    }
  };
  const run = async () => {
    if (!record || runSubmitting || execution?.activeRunId || !draft.trim() || !record.session.workspacePath.trim()) {
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
    const submittedAttachment = attachment;
    const payload = JSON.stringify({ instruction: submittedInstruction, settingsRevision: execution.effectiveConfig.settingsRevision, retryOfRunId, attachment: submittedAttachment });
    const savedDraft = drafts.get(id);
    const attempt = savedDraft?.executionAttempt?.payload === payload
      ? savedDraft.executionAttempt
      : { payload, operationId: operationId(), retryOfRunId };
    drafts.set(id, { body: draft, attachment, operationId: savedDraft?.operationId ?? operationId(), formalAnswers: answers, executionAttempt: attempt });
    setRunSubmitting(true); setError("");
    try {
      const snapshot = await taskExecutionClient.execute({ taskId: task.id, sessionId: id, operationId: attempt.operationId, instruction: submittedInstruction, settingsRevision: execution.effectiveConfig.settingsRevision, retryOfRunId: attempt.retryOfRunId, attachment: submittedAttachment }, (next) => {
        if (generation.current === expected && activeRef.current === id) receiveExecution(next);
      });
      if (generation.current === expected && activeRef.current === id) {
        receiveExecution(snapshot);
        const submittedInstructionStillCurrent = draftRef.current === submittedInstruction;
        const submittedAttachmentStillCurrent = attachmentRef.current === submittedAttachment;
        if (submittedInstructionStillCurrent) setDraft("");
        if (submittedAttachmentStillCurrent) setAttachment(undefined);
        const current = drafts.get(id);
        if (current?.executionAttempt?.operationId === attempt.operationId) {
          drafts.set(id, {
            ...current,
            body: submittedInstructionStillCurrent ? "" : draftRef.current,
            attachment: submittedAttachmentStillCurrent ? undefined : attachmentRef.current,
            executionAttempt: undefined,
          });
        }
        setRetryOfRunId(undefined);
      }
    } catch (cause) { if (generation.current === expected) setError(String(cause)); }
    finally { if (generation.current === expected) setRunSubmitting(false); }
  };
  const currentRun = execution?.runs.find((value) => value.id === execution.activeRunId);
  const activeRun = currentRun ?? execution?.runs.find((value) => value.id === selectedRunId) ?? execution?.selectedRun ?? execution?.runs[0];
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
  const localTurnIds = new Set((execution?.runs ?? []).flatMap((run) => run.turnId ? [run.turnId] : []));
  const conversationTimeline = [
    ...(externalTranscript?.turns.filter((turn) => !localTurnIds.has(turn.id)).map((turn, index) => ({
      type: "external" as const,
      turn,
      timestamp: Date.parse(turn.createdAt ?? turn.items?.find((item) => item.createdAt)?.createdAt ?? turn.completedAt ?? "") || index,
      sequence: index,
    })) ?? []),
    ...(record?.entries.filter((entry) => !linkedUserEntryIds.has(entry.id)).map((entry, index) => ({ type: "entry" as const, entry, timestamp: Date.parse(entry.createdAt) || 0, sequence: 10_000 + index })) ?? []),
    ...(execution?.runs.map((run, index) => ({ type: "run" as const, run, timestamp: Date.parse(run.startedAt ?? "") || 0, sequence: 20_000 + index })) ?? []),
  ].sort((left, right) => left.timestamp - right.timestamp || left.sequence - right.sequence);
  useLayoutEffect(() => {
    const timeline = timelineRef.current;
    if (!timeline || !followLatestRef.current) return;
    timeline.scrollTop = timeline.scrollHeight;
  }, [activeId, conversationTimeline.length, currentRun?.liveStatus?.output, currentRun?.liveStatus?.text, execution?.revision, externalTranscript?.turns.length]);
  return (
    <section className="task-session-space" aria-label={t.title}>
      <header className="task-session-toolbar">
        <div>
          <h3>{t.title}</h3>
          <p>{t.workspaceHint}</p>
        </div>
        <div className="task-session-toolbar-actions">
          <button type="button" data-control="task-session-link" disabled={busy} aria-expanded={showCodexSessions} onClick={toggleCodexSessions}>{showCodexSessions ? t.closePicker : t.link}</button>
          <button
            type="button"
            data-control="task-session-create"
            disabled={busy}
            onClick={() => void create()}
          >
            {t.create}
          </button>
        </div>
      </header>
      {error && (
        <p role="alert" className="task-session-error">
          {error}
        </p>
      )}
      {showCodexSessions && (
        <section className="task-session-link-panel" aria-label={t.linkTitle}>
          <header><div><h4>{t.linkTitle}</h4><p>{t.linkHint}</p></div></header>
          {codexSessions === undefined ? <p role="status">{t.codexLoading}</p> : codexSessions.length === 0 ? <p>{t.noCodexSessions}</p> : (
            <ol>
              {codexSessions.map((thread) => {
                const linkedHere = thread.linkedSessionId === activeId;
                return <li key={thread.id}>
                  <div>
                    <strong>{thread.title || thread.preview}</strong>
                    {thread.preview && thread.preview !== thread.title && <p>{thread.preview}</p>}
                    <span>{thread.source} · {thread.cwd || t.workspace} · {formatSystemTime(thread.updatedAt, document.documentElement.lang || navigator.language, { dateStyle: "medium", timeStyle: "short" })}</span>
                  </div>
                  <button type="button" data-control="task-session-link-choice" disabled={Boolean(linkingThreadId) || linkedHere} onClick={() => void linkCodexSession(thread)}>{linkedHere ? t.alreadyLinked : t.linkAction}</button>
                </li>;
              })}
            </ol>
          )}
          {codexNextCursor && <button type="button" data-control="task-session-link-more" disabled={loadingMoreCodex} onClick={() => void loadMoreCodexSessions()}>{t.loadMore}</button>}
        </section>
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
          <div className="task-session-layout">
              <nav className="task-session-rail" aria-label={t.title}>
                <p>{t.select}</p>
                <ol>
                  {sessions.map((session) => (
                    <li key={session.id}>
                      <button
                        type="button"
                        aria-current={activeId === session.id ? "page" : undefined}
                        onClick={() => void open(session.id)}
                      >
                        <strong>{session.title}</strong>
                        <span>{formatSystemTime(session.updatedAt ?? session.createdAt ?? "", document.documentElement.lang || navigator.language, { dateStyle: "medium" })}</span>
                      </button>
                    </li>
                  ))}
                </ol>
              </nav>
              {record ? <>
              <div className="task-session-chat">
                <header className="task-session-chat-header">
                  <div>
                    <h4>{record.session.title}</h4>
                    <p>{record.session.model} · {record.session.workspacePath || t.workspace}</p>
                  </div>
                  {currentRun && <span className="task-session-running" data-status={currentRun.status}><i aria-hidden="true" />{t.status[currentRun.status]}</span>}
                </header>
                <ol
                  className="task-session-entries"
                  ref={timelineRef}
                  onScroll={(event) => {
                    const timeline = event.currentTarget;
                    followLatestRef.current = timeline.scrollHeight - timeline.scrollTop - timeline.clientHeight < 80;
                  }}
                >
                  {externalTranscript?.nextCursor && <li className="task-session-load-earlier"><button type="button" disabled={loadingEarlierTurns} onClick={() => void loadEarlierTurns()}>{t.loadEarlier}</button></li>}
                  {conversationTimeline.map((item) => item.type === "external" ? (
                    <li className="task-session-external-turn" key={item.turn.id}>
                      {(item.turn.items ?? [
                        ...item.turn.messages.map((message) => ({ ...message, type: "message" as const })),
                        ...item.turn.activity.map((activity) => ({ ...activity, type: "activity" as const })),
                      ]).map((item) => item.type === "message" ? (
                          <article key={item.id} data-entry-author={item.role}>
                            <header><strong>{item.role === "user" ? t.user : "Codex"}</strong></header>
                            {item.role === "assistant" ? <SessionMarkdown>{item.body}</SessionMarkdown> : <p>{item.body}</p>}
                          </article>
                        ) : (
                          <details className="task-execution-event" key={item.id} data-kind={item.kind}>
                            <summary><span className="task-execution-event-icon" aria-hidden="true">{item.command ? ">_" : "·"}</span><strong>{item.command ? t.runCommand : item.label}</strong><span className="task-execution-event-status">{typeof item.exitCode === "number" ? `exit ${item.exitCode}` : item.status}</span></summary>
                            {(item.command || item.output) && <pre>{[item.command, item.output].filter(Boolean).join("\n\n")}</pre>}
                          </details>
                        ))}
                    </li>
                  ) : item.type === "entry" ? (
                    <li key={item.entry.id} data-entry-author={item.entry.author}>
                      <header>
                        <strong>
                          {item.entry.author === "user" ? t.user : item.entry.author}
                        </strong>
                        <span>
                          {item.entry.kind === "note" ? t.note : item.entry.kind}
                        </span>
                        <time>
                          {formatSystemTime(
                            item.entry.createdAt,
                            document.documentElement.lang || navigator.language,
                            { dateStyle: "medium", timeStyle: "short" },
                          )}
                        </time>
                      </header>
                      {item.entry.body && (item.entry.author === "assistant" ? <SessionMarkdown>{item.entry.body}</SessionMarkdown> : <p>{item.entry.body}</p>)}
                      {item.entry.attachment && (
                        <>
                          {item.entry.attachment.mediaType.startsWith("image/") && (
                            <img
                              className="task-session-saved-image"
                              src={`data:${item.entry.attachment.mediaType};base64,${item.entry.attachment.data}`}
                              alt={item.entry.attachment.name}
                            />
                          )}
                          <a
                            data-control="task-session-attachment-download"
                            download={item.entry.attachment.name}
                            href={`data:${item.entry.attachment.mediaType};base64,${item.entry.attachment.data}`}
                          >
                            {t.download}: {item.entry.attachment.name}
                          </a>
                        </>
                      )}
                    </li>
                  ) : (
                    <li className="task-session-run-turn" key={item.run.id}>
                      <article data-entry-author="user"><header><strong>{t.user}</strong><span>{t.status[item.run.status]}</span></header><p>{item.run.instruction}</p></article>
                      {item.run.evidence.map((evidence) => evidence.kind === "agentMessage"
                        ? duplicatesFinalReport(evidence.summary, item.run.finalReport)
                          ? null
                          : <article key={evidence.id} data-entry-author="assistant"><header><strong>Codex</strong></header><SessionMarkdown>{evidence.summary}</SessionMarkdown></article>
                        : <EvidenceRow key={evidence.id} evidence={evidence} runCommand={t.runCommand} />)}
                      {item.run.id === currentRun?.id && item.run.liveStatus?.text && <article data-entry-author="assistant"><header><strong>Codex</strong><span>{t.status.running}</span></header><SessionMarkdown>{item.run.liveStatus.text}</SessionMarkdown></article>}
                      {item.run.finalReport && <article data-entry-author="assistant"><header><strong>Codex</strong></header><SessionMarkdown>{item.run.finalReport}</SessionMarkdown></article>}
                      {!item.run.finalReport && ["succeeded", "failed", "cancelled", "interrupted", "needs_attention"].includes(item.run.status) && <p className="task-execution-meta">{t.noReport}</p>}
                      {item.run.error && <p role="alert">{item.run.error.message}</p>}
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
                    <p className="task-execution-live" role="status">{activeRun.stopRequested ? t.stopRequested : activeRun.status === "awaiting_response" && activeRun.formalRequests.some((request) => ["pending", "submitting", "error"].includes(request.status)) ? (activeRun.formalRequests.some((request) => request.kind === "user_input") ? t.questionWaiting : t.approvalWaiting) : ["interrupted", "needs_attention"].includes(activeRun.status) ? t.outcomeUnknown : activeRun.liveStatus ? `${t.liveKinds[activeRun.liveStatus.kind as keyof typeof t.liveKinds] ?? t.activeRun} · ${activeRun.liveStatus.status === "completed" ? t.status.succeeded : t.status.running}` : t.status[activeRun.status]}</p>
                    {activeRun.liveStatus && (activeRun.liveStatus.command || activeRun.liveStatus.output) && <details className="task-execution-event task-execution-event-live">
                      <summary><span className="task-execution-event-icon" aria-hidden="true">{activeRun.liveStatus.command ? ">_" : "·"}</span><strong>{activeRun.liveStatus.command ? t.runCommand : t.liveKinds[activeRun.liveStatus.kind as keyof typeof t.liveKinds] ?? t.activeRun}</strong><span className="task-execution-event-status">{activeRun.liveStatus.status === "completed" ? t.status.succeeded : t.status.running}</span></summary>
                      <pre>{[activeRun.liveStatus.command, activeRun.liveStatus.output].filter(Boolean).join("\n\n")}</pre>
                    </details>}
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
                    {activeRun.workLogEntryId && <button type="button" data-control="task-session-worklog-open" onClick={() => onOpenWorkLog?.(activeRun.workLogEntryId!)}>{t.workLog}</button>}
                    {activeRun.workLogSyncState === "synced" && <p className="task-execution-meta">{t.syncSucceeded}</p>}
                    {activeRun.workLogSyncState === "pending" && <p role="status">{t.syncPending}</p>}
                    {activeRun.workLogSyncState === "failed" && <><p role="alert">{t.syncFailed} {syncError[activeRun.id] || activeRun.workLogSyncError}</p><button type="button" data-control="task-session-worklog-sync" disabled={syncBusy === activeRun.id} onClick={() => void syncWorkLog(activeRun)}>{t.syncWorkLog}</button></>}
                    {["failed", "cancelled", "interrupted", "needs_attention"].includes(activeRun.status) && <><button type="button" data-control="task-session-run-retry" disabled={busy || Boolean(draft.trim())} onClick={() => { setDraft(activeRun.instruction); setRetryOfRunId(activeRun.id); }}>{t.retryRun}</button>{draft.trim() && <p>{t.retryDraftHint}</p>}</>}
                  </section>
                )}
                {execution?.effectiveConfig && <section className="task-execution-effective"><h5>{execution.effectiveConfig.provenance === "bound_thread" ? t.effective : t.prepare}</h5><dl><dt>{t.model}</dt><dd>{execution.effectiveConfig.model}</dd><dt>{t.workspace}</dt><dd>{execution.effectiveConfig.cwd}</dd><dt>{t.approval}</dt><dd>{execution.effectiveConfig.approvalPolicy} · {execution.effectiveConfig.approvalsReviewer}</dd><dt>Sandbox</dt><dd>{execution.effectiveConfig.sandbox}</dd></dl>{execution.effectiveConfig.readinessError && <p role="alert">{execution.effectiveConfig.readinessError.message}</p>}<button type="button" data-control="task-session-settings-edit" onClick={() => { setEditingSettings(true); setSettingsExpanded(true); }}>{t.editSettings}</button></section>}
                <label>
                  {t.message}
                  <textarea
                    data-control="task-session-message"
                    disabled={busy}
                    value={draft}
                    placeholder={t.placeholder}
                    onChange={(event) => setDraft(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing && event.nativeEvent.keyCode !== 229) {
                        event.preventDefault();
                        void run();
                      }
                    }}
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
                    <span aria-live="polite">
                      {runSubmitting ? t.sendingAttachment : t.pending}: {attachment.name}
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
                <details className="task-session-context">
                  <summary>{t.context}</summary>
                  <div>
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
                  </div>
                </details>
                <details className="task-session-settings" open={settingsExpanded} onToggle={(event) => setSettingsExpanded(event.currentTarget.open)}>
                  <summary>{t.effective}</summary>
                  <div className="task-session-settings-body">
                  <button type="button" data-control="task-session-prepare" disabled={busy} onClick={() => void prepare()}>{execution?.effectiveConfig ? t.checkSettings : t.prepare}</button>
                  <p>{t.prepareHint}</p>
                  {editingSettings && <>
                  <label>
                    {t.sessionTitle}
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
                      {!models.some(([id]) => id === record.session.model) && (
                        <option value={record.session.model}>{record.session.model}</option>
                      )}
                      {models.map(([id, label]) => (
                        <option key={id} value={id}>
                          {label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    {t.workspace}
                    <div className="task-session-workspace-picker">
                      <input
                        data-control="task-session-workspace"
                        disabled={busy}
                        value={record.session.workspacePath}
                        placeholder="/path/to/project"
                        onChange={(e) => updateSession({ workspacePath: e.target.value })}
                      />
                      <button type="button" data-control="task-session-workspace-browse" disabled={busy} onClick={() => void browseProjectFolder()}>{t.browse}</button>
                    </div>
                  </label>
                  {recentFolders.length > 0 && <label className="task-session-recent-folders">
                    {t.recentFolders}
                    <select data-control="task-session-workspace-recent" disabled={busy} value="" onChange={(e) => e.target.value && updateSession({ workspacePath: e.target.value })}>
                      <option value="">—</option>
                      {recentFolders.map((path) => <option key={path} value={path}>{path}</option>)}
                    </select>
                  </label>}
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
                  </div>
                </details>
              </aside>
              </> : <div className="task-session-empty"><p>{t.loading}</p></div>}
            </div>
        </>
      )}
    </section>
  );
}
