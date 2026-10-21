import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { KnowledgeMarkdown } from "../../components/KnowledgeMarkdown";
import { taskClient } from "../../services/taskClient";
import type { KnowledgeVersion, KnowledgeRevisionDiagnostic, KnowledgeRevisionTurn } from "../../types/taskWorkbench";
import { useKnowledgeRevisionText } from "./knowledgeRevisionText";
import "./knowledge-revision-chat.css";

// Same-app sessions outlive a modal. Requests and input belong to the exact Task/version.
interface ChatSession { input: string; pending?: Promise<unknown>; requestId?: string; error?: string; previousTurnId?: string }
const appChatSessions = new WeakMap<object, Map<string, ChatSession>>();

export function KnowledgeRevisionChat({ taskId, version, disabled, publishedRevision, onSelectPublished, onStateChange, onApplied, reviewContainer, versions }: {
  reviewContainer?: HTMLElement | null; versions?: KnowledgeVersion[];
  taskId: string; version: KnowledgeVersion; disabled: boolean; publishedRevision?: number; onSelectPublished?: () => void;
  onStateChange: (dirty: boolean, pending: boolean) => void;
  onApplied: (revision: number) => Promise<void>;
}) {
  const t = useKnowledgeRevisionText();
  const application = window.llmWikiApplication ?? window;
  const sessionKey = `${taskId}:${version.revision}`;
  const session = useMemo(() => {
    const sessions = appChatSessions.get(application) ?? new Map<string, ChatSession>();
    appChatSessions.set(application, sessions);
    const entry = sessions.get(sessionKey) ?? { input: "" };
    sessions.set(sessionKey, entry);
    return entry;
  }, [application, sessionKey]);
  const [input, setInputState] = useState(session.input);
  const setInput = (value: string) => { session.input = value; if (!value) session.previousTurnId = undefined; setInputState(value); };
  const [turns, setTurns] = useState<KnowledgeRevisionTurn[]>([]);
  const [diagnostics, setDiagnostics] = useState<KnowledgeRevisionDiagnostic[]>([]);
  const [error, setError] = useState(session.error ?? "");
  const [pending, setPending] = useState(Boolean(session.pending));
  const [deciding, setDeciding] = useState(false);
  const [reviewId, setReviewId] = useState<string>();
  const requestSequence = useRef(0);
  const [loading, setLoading] = useState(true);
  const [requestId, setRequestId] = useState<string | undefined>(session.requestId);
  const transcriptRef = useRef<HTMLDivElement>(null);
  const alive = useRef(true);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const load = useCallback(async () => {
    const result = await taskClient.knowledgeRevisionConversation(taskId, version.revision);
    if (!alive.current) return;
    setTurns(Array.isArray(result.turns) ? result.turns : []);
    setDiagnostics(Array.isArray(result.diagnostics) ? result.diagnostics : []);
  }, [taskId, version.revision]);
  useEffect(() => { alive.current = true; void load().catch(cause => { if (alive.current) setError(String(cause instanceof Error ? cause.message : cause)); }).finally(() => { if (alive.current) setLoading(false); }); return () => { alive.current = false; }; }, [load]);
  useEffect(() => {
    let mounted = true;
    const active = session.pending;
    if (active) void active.finally(() => {
      if (!mounted) return;
      setPending(false); setRequestId(undefined); setInputState(session.input); setError(session.error ?? "");
      void load().catch(cause => setError(String(cause)));
    });
    return () => { mounted = false; };
  }, [session, load]);
  useEffect(() => { transcriptRef.current?.scrollTo?.({ top: transcriptRef.current.scrollHeight }); }, [turns, pending]);
  useEffect(() => { onStateChange(Boolean(input.trim()) || pending || deciding, pending || deciding); return () => onStateChange(false, false); }, [input, pending, deciding, onStateChange]);
  const send = async () => {
    if (disabled || session.pending || pending || deciding || !input.trim()) return;
    const sequence = ++requestSequence.current;
    const message = input;
    const id = crypto.randomUUID();
    setRequestId(id); setPending(true); setError("");
    session.requestId = id; session.error = "";
    const request = taskClient.knowledgeRevisionSend(taskId, version.revision, version.contentHash, message, id, session.previousTurnId)
      .then(() => { if (session.requestId === id) { session.input = ""; session.previousTurnId = undefined; } })
      .catch(cause => { if (session.requestId === id) session.error = `${t.failed} ${String(cause instanceof Error ? cause.message : cause)}`; })
      .finally(() => { if (session.requestId === id) { session.pending = undefined; session.requestId = undefined; } });
    session.pending = request;
    try {
      await request;
      if (!alive.current || sequence !== requestSequence.current) return;
      setInputState(session.input); setError(session.error ?? ""); setReviewId(undefined); await load();
    } catch (cause) { if (alive.current && sequence === requestSequence.current) { setError(`${t.failed} ${String(cause instanceof Error ? cause.message : cause)}`); try { await load(); } catch { /* The composer remains intact. */ } } }
    finally { if (alive.current && sequence === requestSequence.current) { setPending(false); setRequestId(undefined); } }
  };
  const decide = async (id: string, decision: "apply" | "cancel") => {
    if (decision === "apply" && (disabled || pending || deciding || Boolean(input.trim()))) return;
    if (deciding) return;
    setDeciding(true); setError("");
    try {
      const result = await taskClient.knowledgeRevisionDecide(taskId, id, decision);
      if (!alive.current) return;
      if (decision === "apply" && result.draftRevision) { setInput(""); onStateChange(false, false); await onApplied(result.draftRevision); }
      else { if (id === requestId) { ++requestSequence.current; session.pending = undefined; session.requestId = undefined; setPending(false); setRequestId(undefined); } await load(); }
    } catch (cause) { if (alive.current) { const detail=String(cause instanceof Error ? cause.message : cause); setError(/source_snapshot|generation_snapshot|current_private_conflict|content_hash_conflict/.test(detail) ? t.staleProposal : `${t.failed} ${detail}`); } }
    finally { if (alive.current) setDeciding(false); }
  };
  const changedLines = (before: string, after: string) => {
    const left = before.split("\n").filter(line => line.trim()); const right = after.split("\n").filter(line => line.trim());
    const difference = (lines: string[], other: string[]) => {
      const counts = new Map<string, number>(); other.forEach(line => counts.set(line, (counts.get(line) ?? 0) + 1));
      return lines.filter(line => { const count = counts.get(line) ?? 0; if (count) { counts.set(line, count - 1); return false; } return true; });
    };
    return { removed: difference(left, right), added: difference(right, left) };
  };
  const visibleIssues = (items: unknown): KnowledgeRevisionDiagnostic[] => (Array.isArray(items) ? items : [])
    .filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === "object")
    .map(item => ({
      kind: typeof item.kind === "string" ? item.kind : "legacy",
      locator: typeof item.locator === "string" ? item.locator : "article",
      statement: typeof item.statement === "string" && item.statement.trim() ? item.statement : t.savedReviewNote,
      reason: typeof item.reason === "string" && item.reason.trim() ? item.reason : t.legacyReason,
      direction: typeof item.direction === "string" ? item.direction : "",
      evidence: (Array.isArray(item.evidence) ? item.evidence : []).filter(source => source && typeof source.quote === "string" && source.quote.trim() && [source.type, source.id, source.revision, source.locator].every(value => typeof value === "string")),
    }));
  const shownDiagnostics = visibleIssues(diagnostics);
  const issueList = (items: KnowledgeRevisionDiagnostic[]) => <ol className="knowledge-revision-issues">{visibleIssues(items).map((issue, index) => <li key={`${issue.locator}:${index}`}>
    <p><strong>{t.sentence}</strong> {issue.statement || issue.locator}</p>
    {issue.evidence.map((source, i) => <blockquote key={i}><p>{source.quote}</p><cite>{source.type.replaceAll("_", " ")} · {source.locator} · v{source.revision}</cite></blockquote>)}
    <p><strong>{t.reason}</strong> {issue.reason}</p><p>{issue.direction}</p>
    <button type="button" data-control="knowledge-chat-use-issue" data-record-id={`${issue.locator}:${index}`} disabled={disabled || pending || deciding} onClick={() => { setInput(`${t.fixThis}\n${issue.statement}\n${issue.reason}\n${issue.direction}`); inputRef.current?.focus(); }}>{t.fixThis}</button>
  </li>)}</ol>;
  const proposals = turns.filter(turn => turn.reply?.candidate?.article && ["proposed", "failed"].includes(turn.state));
  const selectedProposal = proposals.find(turn => turn.id === reviewId) ?? proposals.at(-1);
  const renderProposal = (turn: KnowledgeRevisionTurn) => {
    if (!turn.reply?.candidate?.article) return null;
    const baseVersion = turn.reply.baseRevision === version.revision ? version : versions?.find(v => v.revision === turn.reply!.baseRevision);
    const content = <section className="knowledge-revision-proposal">
          <h5>{t.compare}</h5><small>v{turn.reply!.baseRevision}</small>
          {!baseVersion && <p role="note">{t.baseUnavailable}</p>}
          {turn.reply!.applicationState === "stale" && <p role="note">{t.staleProposal}</p>}
          {turn.reply!.preparationError && <p role="alert">{turn.reply!.preparationError}</p>}
          <div className="knowledge-revision-actions"><button type="button" data-control="knowledge-chat-apply" data-record-id={turn.id} disabled={disabled || pending || deciding || Boolean(input.trim()) || turn.state !== "proposed" || turn.reply!.applicable === false || !baseVersion} onClick={() => void decide(turn.id, "apply")}>{t.apply}</button><button type="button" data-control="knowledge-chat-cancel" disabled={pending} onClick={() => void decide(turn.id, "cancel")}>{t.cancel}</button><button type="button" data-control="knowledge-chat-update-proposal" disabled={disabled || pending || deciding} onClick={() => { const request = t.updateRequest; setInput(input.trim() ? `${input}\n\n${request}` : request); session.previousTurnId = turn.id; inputRef.current?.focus(); }}>{t.updateProposal}</button></div>
          <div className="knowledge-revision-diff" aria-label={t.compare}>
            {changedLines((baseVersion?.bodyMarkdown ?? ""), turn.reply.candidate.article.bodyMarkdown).removed.map((line, i) => <p key={`removed:${i}`}><span>{t.before}</span><del>{line}</del></p>)}
            {changedLines((baseVersion?.bodyMarkdown ?? ""), turn.reply.candidate.article.bodyMarkdown).added.map((line, i) => <p key={`added:${i}`}><span>{t.after}</span><ins>{line}</ins></p>)}
            {(baseVersion?.bodyMarkdown ?? "") === turn.reply.candidate.article.bodyMarkdown && <p>{t.bindingOnly}</p>}
            {(baseVersion?.bodyMarkdown ?? "") !== turn.reply.candidate.article.bodyMarkdown && changedLines((baseVersion?.bodyMarkdown ?? ""), turn.reply.candidate.article.bodyMarkdown).removed.length === 0 && changedLines((baseVersion?.bodyMarkdown ?? ""), turn.reply.candidate.article.bodyMarkdown).added.length === 0 && <p>{t.orderOnly}</p>}
          </div>
          <details open={Boolean(reviewContainer)} data-control="knowledge-chat-full-compare"><summary>{t.fullComparison}</summary><div className="knowledge-revision-comparison"><section><h6>{t.before}</h6><KnowledgeMarkdown>{(baseVersion?.bodyMarkdown ?? "")}</KnowledgeMarkdown></section><section><h6>{t.after}</h6><KnowledgeMarkdown>{turn.reply.candidate.article.bodyMarkdown}</KnowledgeMarkdown></section></div></details>
          <details data-control="knowledge-chat-bindings"><summary>{t.bindings}</summary><pre>{JSON.stringify({ before: baseVersion?.result.article, after: turn.reply.candidate.article }, null, 2)}</pre></details>
          {turn.reply.qualityState !== "valid" && <p role="alert">{t.blocked}</p>}

        </section>;
    return reviewContainer ? createPortal(content, reviewContainer) : content;
  };
  return <section className="knowledge-revision-chat" aria-label={t.title}>
    <header><h4>{t.title}</h4><p>{t.hint}</p><small>{publishedRevision === version.revision ? t.publishedBase : t.basedOn} v{version.revision}</small>
      {publishedRevision && publishedRevision !== version.revision && onSelectPublished && <button type="button" data-control="knowledge-chat-published" disabled={disabled || pending || deciding || Boolean(input.trim())} onClick={onSelectPublished}>{t.selectPublished} · v{publishedRevision}</button>}</header>
    <div ref={transcriptRef} className="knowledge-revision-transcript" aria-label={t.conversation} aria-live="polite">
    <details className="knowledge-revision-diagnostics" data-control="knowledge-chat-diagnostics"><summary>{t.diagnostics} · {shownDiagnostics.length}</summary>
      <div className="knowledge-revision-diagnostic-content">
      {loading ? <p role="status">{t.loading}</p> : shownDiagnostics.length ? issueList(shownDiagnostics) : <p>{version.qualityState === "valid" ? t.noBlockers : t.detailsUnavailable}</p>}
      </div>
    </details>
      {turns.length === 0 && <p>{t.empty}</p>}
      {turns.map(turn => <article key={turn.id} data-record-id={turn.id} className="knowledge-revision-turn">
        <p className="knowledge-revision-user"><strong>{t.you}</strong><br />{turn.userMessage}</p>
        {turn.reply && <><KnowledgeMarkdown>{turn.reply.message}</KnowledgeMarkdown>
          {turn.reply.missingEvidence?.length > 0 && <section className="knowledge-revision-missing"><strong>{t.missing}</strong><ul>{turn.reply.missingEvidence.map((item, i) => <li key={i}>{item}</li>)}</ul></section>}
          {turn.reply.diagnostics?.length > 0 && issueList(turn.reply.diagnostics)}
        </>}
        {turn.error && <p role="alert">{turn.error}</p>}
        {turn.state === "applied" && <p role="status">{t.applied} v{turn.appliedRevision}</p>}
        {turn.state === "cancelled" && <p>{t.cancelled}</p>}
        {turn.state === "running" && <button type="button" data-control="knowledge-chat-cancel" onClick={() => void decide(turn.id, "cancel")}>{t.cancelPending}</button>}
        {turn.reply?.candidate?.article && ["proposed","failed"].includes(turn.state) && <button type="button" data-control="knowledge-chat-review-proposal" data-record-id={turn.id} onClick={() => setReviewId(turn.id)}>{t.reviewProposal} · v{turn.reply.baseRevision}</button>}

      </article>)}
    </div>
    {selectedProposal && renderProposal(selectedProposal)}
    {error && <p role="alert" className="knowledge-draft-error">{error}</p>}
    <label>{t.input}<textarea ref={inputRef} data-control="knowledge-chat-input" value={input} disabled={disabled || pending || deciding} onChange={event => setInput(event.target.value)} onKeyDown={event => {
      if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing && event.keyCode !== 229) {
        event.preventDefault(); void send();
      }
    }} placeholder={t.placeholder} /></label>
    <div className="knowledge-revision-actions"><button type="button" data-control="knowledge-chat-send" disabled={disabled || pending || deciding || !input.trim()} onClick={() => void send()}>{pending ? t.sending : t.send}</button>
      {pending && requestId ? <button type="button" data-control="knowledge-chat-cancel" onClick={() => void decide(requestId, "cancel")}>{t.cancelPending}</button> : <button type="button" data-control="knowledge-chat-discard-input" disabled={!input} onClick={() => setInput("")}>{t.discardInput}</button>}
    </div>
    <p className="knowledge-revision-footnote">{t.notesSeparate}</p>
  </section>;
}
