import { useId } from "react";
import type { DistillationProjection, TaskExecutionWorkLogLink } from "../../types/taskWorkbench";

const copy = {
  en: { heading: "Distilled Run result", generated: "AI-generated working view", current: "Current", pending: "Updating", stale: "Source changed — showing the last good view", retryable_failure: "Update failed — the last good view is retained", repair_required: "Repair required", unavailable: "Distillation is unavailable", empty: "No supported result is available yet.", sources: "Sources", original: "Inspect original Run", warnings: "Limitations", evidence: "Evidence", completion: "Completion", user: "User", ai: "AI", tool: "Tool", system: "System", unknown: "Unknown", retry: "Retry update", repair: "Repair Distillation", outcome: "Outcome", decisions: "Decisions", performed: "Work performed", checks: "Checks", failed_approaches: "Failed approaches", unresolved: "Unresolved", suggested: "Suggested", decided: "Decided", attempted: "Attempted", observed: "Observed", verified: "Verified" },
  ko: { heading: "Run 결과 정리", generated: "AI가 생성한 작업용 보기", current: "최신", pending: "업데이트 중", stale: "출처가 변경되어 마지막 정상 보기를 표시합니다", retryable_failure: "업데이트에 실패하여 마지막 정상 보기를 유지합니다", repair_required: "복구가 필요합니다", unavailable: "정리 결과를 사용할 수 없습니다", empty: "근거가 확인된 결과가 아직 없습니다.", sources: "출처", original: "원본 Run 확인", warnings: "제한 사항", evidence: "근거", completion: "완료", user: "사용자", ai: "AI", tool: "도구", system: "시스템", unknown: "알 수 없음", retry: "업데이트 재시도", repair: "정리 복구", outcome: "결과", decisions: "결정", performed: "수행한 작업", checks: "확인", failed_approaches: "실패한 접근", unresolved: "미해결", suggested: "제안", decided: "결정됨", attempted: "시도", observed: "관찰", verified: "검증됨" },
} as const;

export function WorkLogDistillation({ projection, execution, busy, onOpenOriginal, onRetry, onRepair }: { projection?: DistillationProjection; execution?: TaskExecutionWorkLogLink; busy?: boolean; onOpenOriginal: () => void; onRetry?: () => void; onRepair?: () => void }) {
  const headingId = useId();
  const locale = document.documentElement.lang.toLowerCase().startsWith("ko") ? "ko" : "en";
  const text = copy[locale];
  if (!projection) return null;
  const claims = new Map((projection.result?.claims ?? []).map((claim) => [claim.id, claim]));
  const sections = projection.result?.workLogView?.sections ?? [];
  const freshness = text[projection.freshness];
  return <section className={`work-log-distillation is-${projection.freshness}`} aria-labelledby={headingId}>
    <header>
      <div><small>{text.generated}</small><h4 id={headingId}>{text.heading}</h4></div>
      <span role="status">{freshness}</span>
    </header>
    {sections.length ? <div className="work-log-distillation-sections">{sections.map((section) => {
      const sectionClaims = section.claimIds.map((id) => claims.get(id)).filter(Boolean);
      if (!sectionClaims.length) return null;
      return <section key={section.kind}><h5>{text[section.kind as keyof typeof text] ?? section.kind}</h5><ul>{sectionClaims.map((claim) => <li key={claim!.id} className={claim!.status === "contradicted" ? "is-contradicted" : ""}>
        <p>{claim!.statement}</p><small>{text[claim!.epistemicState as keyof typeof text] ?? claim!.epistemicState} · {text[claim!.actor]}</small>
        <details data-control="task-worklog-distillation-sources"><summary>{text.sources} ({claim!.sources.length})</summary><ul>{claim!.sources.map((source) => <li key={`${source.type}:${source.id}:${source.revision}:${source.locator}`}><code>{source.type}</code> · {source.id} · {source.locator} · {source.revision}{source.quote ? <blockquote>{source.quote}</blockquote> : null}</li>)}</ul></details>
      </li>)}</ul></section>;
    })}</div> : <p>{text.empty}</p>}
    {(projection.result?.warnings?.length ?? 0) > 0 && <details data-control="task-worklog-distillation-warnings"><summary>{text.warnings}</summary><ul>{projection.result.warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul></details>}
    <footer>{execution && <button type="button" data-control="task-worklog-distillation-original" onClick={onOpenOriginal}>{text.original}</button>}
    {projection.freshness === "retryable_failure" && onRetry && <button type="button" disabled={busy} data-control="task-worklog-distillation-retry" onClick={onRetry}>{text.retry}</button>}
    {projection.freshness === "repair_required" && onRepair && <button type="button" disabled={busy} data-control="task-worklog-distillation-repair" onClick={onRepair}>{text.repair}</button>}</footer>
  </section>;
}
