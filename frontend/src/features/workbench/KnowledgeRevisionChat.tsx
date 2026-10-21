import { useCallback, useEffect, useRef, useState } from "react";
import { KnowledgeMarkdown } from "../../components/KnowledgeMarkdown";
import { taskClient } from "../../services/taskClient";
import type { KnowledgeVersion, KnowledgeRevisionDiagnostic, KnowledgeRevisionTurn } from "../../types/taskWorkbench";
import { useKnowledgeRevisionText } from "./knowledgeRevisionText";
import "./knowledge-revision-chat.css";

export function KnowledgeRevisionChat({ taskId, version, disabled, publishedRevision, onSelectPublished, onStateChange, onApplied }: {
  taskId: string; version: KnowledgeVersion; disabled: boolean; publishedRevision?: number; onSelectPublished?: () => void;
  onStateChange: (dirty: boolean, pending: boolean) => void;
  onApplied: (revision: number) => Promise<void>;
}) {
  const t = useKnowledgeRevisionText();
  const [input, setInput] = useState("");
  const [turns, setTurns] = useState<KnowledgeRevisionTurn[]>([]);
  const [diagnostics, setDiagnostics] = useState<KnowledgeRevisionDiagnostic[]>([]);
  const [error, setError] = useState("");
  const [pending, setPending] = useState(false);
  const [deciding, setDeciding] = useState(false);
  const requestSequence = useRef(0);
  const [loading, setLoading] = useState(true);
  const [requestId, setRequestId] = useState<string>();
  const alive = useRef(true);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const load = useCallback(async () => {
    const result = await taskClient.knowledgeRevisionConversation(taskId, version.revision);
    if (!alive.current) return;
    setTurns(Array.isArray(result.turns) ? result.turns : []);
    setDiagnostics(Array.isArray(result.diagnostics) ? result.diagnostics : []);
  }, [taskId, version.revision]);
  useEffect(() => { alive.current = true; void load().catch(cause => { if (alive.current) setError(String(cause instanceof Error ? cause.message : cause)); }).finally(() => { if (alive.current) setLoading(false); }); return () => { alive.current = false; }; }, [load]);
  useEffect(() => { onStateChange(Boolean(input.trim()) || pending || deciding, pending || deciding); return () => onStateChange(false, false); }, [input, pending, deciding, onStateChange]);
  const send = async () => {
    if (disabled || pending || deciding || !input.trim()) return;
    const sequence = ++requestSequence.current;
    const message = input;
    const id = crypto.randomUUID();
    setRequestId(id); setPending(true); setError("");
    try {
      await taskClient.knowledgeRevisionSend(taskId, version.revision, version.contentHash, message, id);
      if (!alive.current || sequence !== requestSequence.current) return;
      setInput(""); await load();
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
      else { if (id === requestId) { ++requestSequence.current; setPending(false); setRequestId(undefined); } await load(); }
    } catch (cause) { if (alive.current) setError(`${t.failed} ${String(cause instanceof Error ? cause.message : cause)}`); }
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
  const issueList = (items: KnowledgeRevisionDiagnostic[]) => <ol className="knowledge-revision-issues">{items.map((issue, index) => <li key={`${issue.locator}:${index}`}>
    <p><strong>{t.sentence}</strong> {issue.statement || issue.locator}</p>
    {issue.evidence.map((source, i) => <blockquote key={i}><p>{source.quote}</p><cite>{source.type.replaceAll("_", " ")} · {source.locator} · v{source.revision}</cite></blockquote>)}
    <p><strong>{t.reason}</strong> {issue.reason}</p><p>{issue.direction}</p>
    <button type="button" data-control="knowledge-chat-use-issue" data-record-id={`${issue.locator}:${index}`} disabled={disabled || pending || deciding} onClick={() => { setInput(`${t.fixThis}\n${issue.statement}\n${issue.reason}\n${issue.direction}`); inputRef.current?.focus(); }}>{t.fixThis}</button>
  </li>)}</ol>;
  return <section className="knowledge-revision-chat" aria-label={t.title}>
    <header><h4>{t.title}</h4><p>{t.hint}</p><small>{publishedRevision === version.revision ? t.publishedBase : t.basedOn} v{version.revision}</small>
      {publishedRevision && publishedRevision !== version.revision && onSelectPublished && <button type="button" data-control="knowledge-chat-published" disabled={disabled || pending || deciding || Boolean(input.trim())} onClick={onSelectPublished}>{t.selectPublished} · v{publishedRevision}</button>}</header>
    <details className="knowledge-revision-diagnostics" open={diagnostics.length > 0} data-control="knowledge-chat-diagnostics"><summary>{t.diagnostics} · {diagnostics.length}</summary>
      {loading ? <p role="status">{t.loading}</p> : diagnostics.length ? issueList(diagnostics) : <p>{version.qualityState === "valid" ? t.noBlockers : t.detailsUnavailable}</p>}
    </details>
    <div className="knowledge-revision-transcript" aria-label={t.conversation} aria-live="polite">
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
        {turn.state === "proposed" && turn.reply?.baseRevision === version.revision && turn.reply?.candidate?.article && <section className="knowledge-revision-proposal">
          <h5>{t.compare}</h5>
          <div className="knowledge-revision-diff" aria-label={t.compare}>
            {changedLines(version.bodyMarkdown, turn.reply.candidate.article.bodyMarkdown).removed.map((line, i) => <p key={`removed:${i}`}><span>{t.before}</span><del>{line}</del></p>)}
            {changedLines(version.bodyMarkdown, turn.reply.candidate.article.bodyMarkdown).added.map((line, i) => <p key={`added:${i}`}><span>{t.after}</span><ins>{line}</ins></p>)}
            {version.bodyMarkdown === turn.reply.candidate.article.bodyMarkdown && <p>{t.bindingOnly}</p>}
            {version.bodyMarkdown !== turn.reply.candidate.article.bodyMarkdown && changedLines(version.bodyMarkdown, turn.reply.candidate.article.bodyMarkdown).removed.length === 0 && changedLines(version.bodyMarkdown, turn.reply.candidate.article.bodyMarkdown).added.length === 0 && <p>{t.orderOnly}</p>}
          </div>
          <details data-control="knowledge-chat-full-compare"><summary>{t.fullComparison}</summary><div className="knowledge-revision-comparison"><section><h6>{t.before}</h6><KnowledgeMarkdown>{version.bodyMarkdown}</KnowledgeMarkdown></section><section><h6>{t.after}</h6><KnowledgeMarkdown>{turn.reply.candidate.article.bodyMarkdown}</KnowledgeMarkdown></section></div></details>
          <details data-control="knowledge-chat-bindings"><summary>{t.bindings}</summary><pre>{JSON.stringify({ before: version.result.article, after: turn.reply.candidate.article }, null, 2)}</pre></details>
          {turn.reply.qualityState !== "valid" && <p role="alert">{t.blocked}</p>}
          <div className="knowledge-revision-actions"><button type="button" data-control="knowledge-chat-apply" data-record-id={turn.id} disabled={disabled || pending || deciding || Boolean(input.trim()) || turn.reply.qualityState !== "valid"} onClick={() => void decide(turn.id, "apply")}>{t.apply}</button><button type="button" data-control="knowledge-chat-cancel" disabled={pending} onClick={() => void decide(turn.id, "cancel")}>{t.cancel}</button></div>
        </section>}
      </article>)}
    </div>
    {error && <p role="alert" className="knowledge-draft-error">{error}</p>}
    <label>{t.input}<textarea ref={inputRef} data-control="knowledge-chat-input" value={input} disabled={disabled || pending || deciding} onChange={event => setInput(event.target.value)} placeholder={t.placeholder} /></label>
    <div className="knowledge-revision-actions"><button type="button" data-control="knowledge-chat-send" disabled={disabled || pending || deciding || !input.trim()} onClick={() => void send()}>{pending ? t.sending : t.send}</button>
      {pending && requestId ? <button type="button" data-control="knowledge-chat-cancel" onClick={() => void decide(requestId, "cancel")}>{t.cancelPending}</button> : <button type="button" data-control="knowledge-chat-discard-input" disabled={!input} onClick={() => setInput("")}>{t.discardInput}</button>}
    </div>
    <p className="knowledge-revision-footnote">{t.notesSeparate}</p>
  </section>;
}
