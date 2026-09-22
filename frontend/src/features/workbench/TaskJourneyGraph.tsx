import { useState } from "react";
import "./task-journey-graph.css";

export interface TaskJourneyEvent {
  id: string;
  type: string;
  occurredAt?: string;
  detail?: { title?: string; summary?: string; report?: string; changes?: Array<{ field: string; before?: string; after?: string }>; payload?: unknown };
}
export interface TaskJourney {
  events?: TaskJourneyEvent[];
  edges?: Array<{ from: string; to: string; kind: string }>;
  titles?: Array<{ id: string; title: string }>;
  semantics?: { causalInference?: boolean };
}
const copy = {
  en: { heading: "How this Task took shape", trace: "Recorded event trace", activity: "Activity", noEvents: "No recorded journey events are available yet.", notCausal: "The connectors show recorded order, not inferred cause.", changed: "Changed", close: "Close activity", leadsTo: "Linked refinement" },
  ko: { heading: "이 Task가 다듬어지고 실행된 과정", trace: "기록된 이벤트 흐름", activity: "활동", noEvents: "아직 표시할 작업 과정 기록이 없습니다.", notCausal: "연결선은 기록된 순서만 나타내며 원인 관계를 추론하지 않습니다.", changed: "변경", close: "활동 닫기", leadsTo: "연결된 정제" },
} as const;
const labels = {
  en: { origin_capture: "Original idea captured", task_created: "Task created", refinement_input: "Refinement note", refinement_applied: "Refinement applied", task_updated: "Task updated", execution_started: "Work started", work_recorded: "Work recorded", decision_recorded: "Decision recorded", task_completed: "Task completed" },
  ko: { origin_capture: "처음 아이디어 기록", task_created: "Task 생성", refinement_input: "정제 메모", refinement_applied: "정제 적용", task_updated: "Task 수정", execution_started: "작업 시작", work_recorded: "작업 기록 추가", decision_recorded: "결정 기록", task_completed: "Task 완료" },
} as const;
const fields = { en: { title: "title", detail: "detail", outcome: "outcome", scope: "scope", nonGoals: "non-goals", validationCriteria: "validation criteria" }, ko: { title: "제목", detail: "상세", outcome: "결과", scope: "범위", nonGoals: "제외 범위", validationCriteria: "검증 기준" } } as const;
const localeFor = (language?: string) => language?.toLowerCase().startsWith("ko") ? "ko" : "en";
const compact = (value: string) => {
  const characters = Array.from(value);
  return characters.length <= 15 ? value : `${characters.slice(0, 14).join("")}…`;
};
const recordedPayloadText = (payload: unknown): string => {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) return "";
  const record = payload as Record<string, unknown>;
  return ["summary", "rationale", "body", "note", "reason", "action"]
    .map(key => record[key]).filter((value): value is string => typeof value === "string" && Boolean(value.trim())).join("\n");
};
function fullDetail(event: TaskJourneyEvent, locale: "en" | "ko") {
  const detail = event.detail ?? {};
  if (event.type === "refinement_applied" || event.type === "task_updated") return <>{detail.changes?.map(change => <p className="task-journey-full-text" key={change.field}>{copy[locale].changed} {fields[locale][change.field as keyof typeof fields.en] ?? change.field}: {change.before || "—"} → {change.after || "—"}</p>)}</>;
  const value = [detail.title, detail.summary, detail.report, recordedPayloadText(detail.payload)].filter(Boolean).join("\n\n");
  return value ? <p className="task-journey-full-text">{value}</p> : <p className="task-journey-empty">—</p>;
}
export function TaskJourneyGraph({ journey, language }: { journey?: TaskJourney; language?: string }) {
  const locale = localeFor(language ?? document.documentElement.lang ?? navigator.language);
  const text = copy[locale], events = journey?.events ?? [];
  const titles = new Map((journey?.titles ?? []).map(title => [title.id, title.title]));
  const [activeId, setActiveId] = useState<string>();
  const active = events.find(event => event.id === activeId);
  if (!events.length) return <p className="task-journey-empty">{text.noEvents}</p>;
  const titleFor = (event: TaskJourneyEvent) => compact(titles.get(event.id) ?? labels[locale][event.type as keyof typeof labels.en] ?? text.activity);
  return <section className="task-journey" aria-labelledby="task-journey-heading">
    <header><div><h3 id="task-journey-heading">{text.heading}</h3><p>{text.notCausal}</p></div><span className="task-journey-count">{text.trace} · {events.length}</span></header>
    <ol className="task-journey-graph" aria-label={text.trace}>{events.map((event, index) => {
      const previous = events[index - 1];
      const follows = previous && journey?.edges?.some(edge => edge.kind === "followed_by" && edge.from === previous.id && edge.to === event.id);
      return <li key={event.id} id={`journey-event-${event.id}`} className={`task-journey-event is-${event.type}`}>
        {follows && <span className="task-journey-connector" aria-label={text.notCausal}>→</span>}
        <button type="button" data-control="task-journey-activity-open" className="task-journey-node" aria-pressed={activeId === event.id} onClick={() => setActiveId(event.id)} title={titleFor(event)}>{titleFor(event)}</button>
      </li>;
    })}</ol>
    {active && <aside className="task-journey-activity" aria-label={text.activity}>
      <header><div><small>{labels[locale][active.type as keyof typeof labels.en] ?? active.type.replaceAll("_", " ")}</small><h4>{text.activity}</h4></div><button type="button" data-control="task-journey-activity-close" onClick={() => setActiveId(undefined)}>{text.close}</button></header>
      {active.occurredAt && <time dateTime={active.occurredAt}>{new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(active.occurredAt))}</time>}
      {fullDetail(active, locale)}
      {journey?.edges?.filter(edge => edge.kind === "refinement_context" && edge.from === active.id).map(edge => events.find(event => event.id === edge.to)).filter((event): event is TaskJourneyEvent => Boolean(event)).map(event => <a key={event.id} data-control="task-journey-context-open" className="task-journey-context" href={`#journey-event-${encodeURIComponent(event.id)}`} onClick={() => setActiveId(event.id)}>↳ {text.leadsTo}: {titleFor(event)}</a>)}
    </aside>}
  </section>;
}
