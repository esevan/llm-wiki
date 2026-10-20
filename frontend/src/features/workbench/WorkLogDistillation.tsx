import { useId } from "react";
import type { DistillationProjection, TaskExecutionWorkLogLink } from "../../types/taskWorkbench";

const copy = {
  en: { heading: "Execution summary", generated: "AI-generated working view", current: "Current", pending: "Updating", stale: "Source changed — showing the last good view", retryable_failure: "Update failed — the last good view is retained", repair_required: "Repair required", unavailable: "Summary unavailable", empty: "No execution summary is available. Inspect the original record.", missingReport: "No final report was saved. Inspect the retained execution evidence.", summaryMissing: "No execution summary has been saved yet. Inspect the original record or retry the summary.", reported: "Reported", summaryFailed: "Could not generate an execution summary. Inspect the original record.", summaryPending: "Generating the execution summary. The original record is available.", executionFailed: "This Run failed.", executionCancelled: "This Run was cancelled.", executionInterrupted: "This Run was interrupted.", executionAttention: "This Run needs attention.", performedState: "Performed", sources: "Sources", original: "Inspect original Run", warnings: "Limitations", evidence: "Evidence", completion: "Completion", user: "User", ai: "AI", tool: "Tool", system: "System", unknown: "Unknown", retry: "Retry update", repair: "Repair Distillation", outcome: "Outcome", decisions: "Decisions", performed: "Work performed", checks: "Checks", failed_approaches: "Failed approaches", unresolved: "Unresolved", suggested: "Suggested", decided: "Decided", attempted: "Attempted", observed: "Observed", verified: "Verified" },
  ko: { heading: "실행 요약", generated: "AI가 생성한 작업용 보기", current: "최신", pending: "업데이트 중", stale: "출처가 변경되어 마지막 정상 보기를 표시합니다", retryable_failure: "업데이트에 실패하여 마지막 정상 보기를 유지합니다", repair_required: "복구가 필요합니다", unavailable: "요약을 사용할 수 없습니다", empty: "실행 요약이 없습니다. 원본 기록을 확인하세요.", missingReport: "저장된 최종 보고서가 없습니다. 남아 있는 실행 증거를 확인하세요.", summaryMissing: "저장된 실행 요약이 아직 없습니다. 원본 기록을 확인하거나 요약을 다시 생성하세요.", reported: "모델 보고", summaryFailed: "실행 요약을 생성하지 못했습니다. 원본 기록을 확인하세요.", summaryPending: "실행 요약을 생성 중입니다. 원본 기록을 확인할 수 있습니다.", executionFailed: "이 Run은 실패했습니다.", executionCancelled: "이 Run은 취소되었습니다.", executionInterrupted: "이 Run은 중단되었습니다.", executionAttention: "이 Run은 확인이 필요합니다.", performedState: "수행", sources: "출처", original: "원본 Run 확인", warnings: "제한 사항", evidence: "근거", completion: "완료", user: "사용자", ai: "AI", tool: "도구", system: "시스템", unknown: "알 수 없음", retry: "업데이트 재시도", repair: "정리 복구", outcome: "결과", decisions: "결정", performed: "수행한 작업", checks: "확인", failed_approaches: "실패한 접근", unresolved: "미해결", suggested: "제안", decided: "결정됨", attempted: "시도", observed: "관찰", verified: "검증됨" },
} as const;

export function WorkLogDistillation({ projection, execution, busy, onOpenOriginal, onRetry, onRepair }: { projection?: DistillationProjection; execution?: TaskExecutionWorkLogLink; busy?: boolean; onOpenOriginal: () => void; onRetry?: () => void; onRepair?: () => void }) {
  const headingId = useId();
  const locale = document.documentElement.lang.toLowerCase().startsWith("ko") ? "ko" : "en";
  const text = copy[locale];
  if (!projection && (!execution || ["queued", "running", "awaiting_response"].includes(execution.status))) return null;
  const claims = new Map((projection?.result?.claims ?? []).map((claim) => [claim.id, claim]));
  const sections = (projection?.result?.workLogView?.sections ?? []).filter(section => section.claimIds.some(id => claims.has(id)));
  const hasContent = sections.length > 0;
  const pending = projection?.freshness === "pending";
  const emptyMessage = pending ? text.summaryPending : execution && !execution.reportExcerpt?.trim() ? text.missingReport : projection?.freshness === "retryable_failure" ? text.summaryFailed : execution?.reportExcerpt?.trim() ? text.summaryMissing : text.empty;
  const executionNotice = execution ? { failed: text.executionFailed, cancelled: text.executionCancelled, interrupted: text.executionInterrupted, needs_attention: text.executionAttention }[execution.status as "failed" | "cancelled" | "interrupted" | "needs_attention"] : undefined;
  const freshness = !hasContent && !pending && (projection?.freshness === "current" || projection?.freshness === "retryable_failure") ? text.unavailable : projection ? text[projection.freshness] : text.unavailable;
  return <section className={`work-log-distillation is-${projection?.freshness}`} aria-labelledby={headingId}>
    <header>
      <div><small>{text.generated}</small><h4 id={headingId}>{text.heading}</h4></div>
      <span role="status">{freshness}</span>
    </header>
    {executionNotice && <p role="note">{executionNotice}</p>}
    {hasContent ? <div className="work-log-distillation-sections">{sections.map((section) => {
      const sectionClaims = section.claimIds.map((id) => claims.get(id)).filter(Boolean);
      if (!sectionClaims.length) return null;
      return <section key={section.kind}><h5>{text[section.kind as keyof typeof text] ?? section.kind}</h5><ul>{sectionClaims.map((claim) => <li key={claim!.id} className={claim!.status === "contradicted" ? "is-contradicted" : ""}>
        <p>{claim!.statement}</p><small>{(claim!.sources.length > 0 && claim!.sources.every(source => source.type === "run_final_report") && ["performed", "observed"].includes(claim!.epistemicState) ? text.reported : claim!.epistemicState === "performed" ? text.performedState : text[claim!.epistemicState as keyof typeof text]) ?? claim!.epistemicState} · {text[claim!.actor]}</small>
        <details data-control="task-worklog-distillation-sources"><summary>{text.sources} ({claim!.sources.length})</summary><ul>{claim!.sources.map((source) => <li key={`${source.type}:${source.id}:${source.revision}:${source.locator}`}><code>{source.type}</code> · {source.id} · {source.locator} · {source.revision}{source.quote ? <blockquote>{source.quote}</blockquote> : null}</li>)}</ul></details>
      </li>)}</ul></section>;
    })}</div> : <p>{emptyMessage}</p>}
    {(projection?.result?.warnings?.length ?? 0) > 0 && <details open data-control="task-worklog-distillation-warnings"><summary>{text.warnings}</summary><ul>{projection!.result.warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul></details>}
    {execution?.limitations.length ? <ul>{execution.limitations.map((limitation, index) => <li key={index}>{limitation}</li>)}</ul> : null}
    <footer>{execution && <button type="button" data-control="task-worklog-distillation-original" onClick={onOpenOriginal}>{text.original}</button>}
    {(projection?.freshness === "retryable_failure" || (!hasContent && !pending && projection?.freshness !== "repair_required")) && onRetry && <button type="button" disabled={busy} data-control="task-worklog-distillation-retry" onClick={onRetry}>{text.retry}</button>}
    {projection?.freshness === "repair_required" && onRepair && <button type="button" disabled={busy} data-control="task-worklog-distillation-repair" onClick={onRepair}>{text.repair}</button>}</footer>
  </section>;
}
