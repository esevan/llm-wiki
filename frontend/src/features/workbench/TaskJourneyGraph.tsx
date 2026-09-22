import type { ReactNode } from "react";
import "./task-journey-graph.css";

export interface TaskJourneyEvent {
  id: string;
  type: "origin_capture" | "task_created" | "refinement_input" | "refinement_applied" | "task_updated" | "execution_started" | "work_recorded" | "decision_recorded" | "task_completed" | string;
  occurredAt?: string;
  detail?: {
    title?: string;
    summary?: string;
    report?: string;
    changes?: Array<{ field: string; before?: string; after?: string }>;
    decisionKind?: string;
    payload?: unknown;
  };
}

export interface TaskJourney {
  events?: TaskJourneyEvent[];
  edges?: Array<{ from: string; to: string; kind: "followed_by" | string }>;
  semantics?: { causalInference?: boolean };
}

const copy = {
  en: {
    heading: "How this Task took shape",
    trace: "Recorded event trace",
    followed: "Then",
    refinementContext: "Recorded in this refinement",
    leadsTo: "Linked refinement",
    noEvents: "No recorded journey events are available yet.",
    notCausal: "The connectors show recorded order, not inferred cause.",
    origin_capture: "Original idea captured",
    task_created: "Task created",
    refinement_input: "Refinement note",
    refinement_applied: "Refinement applied",
    task_updated: "Task updated",
    execution_started: "Work started",
    work_recorded: "Work recorded",
    decision_recorded: "Decision recorded",
    task_completed: "Task completed",
    changed: "Changed",
  },
  ko: {
    heading: "이 Task가 다듬어지고 실행된 과정",
    trace: "기록된 이벤트 흐름",
    followed: "이후",
    refinementContext: "이 정제에서 기록됨",
    leadsTo: "연결된 정제",
    noEvents: "아직 표시할 작업 과정 기록이 없습니다.",
    notCausal: "연결선은 기록된 순서만 나타내며 원인 관계를 추론하지 않습니다.",
    origin_capture: "처음 아이디어 기록",
    task_created: "Task 생성",
    refinement_input: "정제 메모",
    refinement_applied: "정제 적용",
    task_updated: "Task 수정",
    execution_started: "작업 시작",
    work_recorded: "작업 기록 추가",
    decision_recorded: "결정 기록",
    task_completed: "Task 완료",
    changed: "변경",
  },
} as const;

const fieldLabels = {
  en: { title: "title", detail: "detail", outcome: "outcome", scope: "scope", nonGoals: "non-goals", validationCriteria: "validation criteria" },
  ko: { title: "제목", detail: "상세", outcome: "결과", scope: "범위", nonGoals: "제외 범위", validationCriteria: "검증 기준" },
} as const;

function localeFor(language?: string) {
  return language?.toLowerCase().startsWith("ko") ? "ko" : "en";
}

function firstRecordedText(payload: unknown): string | undefined {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) return undefined;
  const values = payload as Record<string, unknown>;
  for (const key of ["summary", "rationale", "body", "note", "reason", "action"]) {
    const value = values[key];
    if (typeof value === "string" && value.trim()) return value.trim();
  }
  return undefined;
}

function detailFor(event: TaskJourneyEvent, language: "en" | "ko"): ReactNode {
  const detail = event.detail ?? {};
  if (event.type === "refinement_input" || event.type === "decision_recorded") {
    const recorded = event.type === "refinement_input" ? detail.summary : firstRecordedText(detail.payload);
    const remainder = recorded?.split(/\r?\n/).slice(1).join("\n").trim();
    return remainder ? <span>{remainder}</span> : null;
  }
  if (event.type === "refinement_applied" || event.type === "task_updated") {
    return <>{detail.changes?.map((change) => {
      const field = fieldLabels[language][change.field as keyof typeof fieldLabels.en] ?? change.field;
      return <span key={change.field}>{copy[language].changed} {field}: {change.before || "—"} → {change.after || "—"}</span>;
    })}</>;
  }
  if (event.type === "origin_capture" || event.type === "work_recorded" || event.type === "task_completed") return <>{detail.summary?.includes("\n") && <span>{detail.summary}</span>}{detail.report?.trim() && <span>{detail.report}</span>}</>;
  const text = detail.summary?.trim() || firstRecordedText(detail.payload) || detail.report?.trim();
  return text ? <span>{text}</span> : null;
}

function firstLine(value?: string) {
  return value?.split(/\r?\n/, 1)[0]?.trim();
}

function headlineFor(event: TaskJourneyEvent, language: "en" | "ko") {
  if (event.type === "refinement_input") return firstLine(event.detail?.summary) || copy[language].refinement_input;
  if (event.type === "decision_recorded") return firstLine(firstRecordedText(event.detail?.payload)) || copy[language].decision_recorded;
  if (event.type === "task_created" || event.type === "refinement_applied" || event.type === "task_updated") return firstLine(event.detail?.title) || copy[language][event.type as keyof typeof copy.en];
  if (event.type === "origin_capture" || event.type === "work_recorded" || event.type === "task_completed") return firstLine(event.detail?.summary) || copy[language][event.type as keyof typeof copy.en];
  return copy[language][event.type as keyof typeof copy.en] ?? event.type.replaceAll("_", " ");
}

export function TaskJourneyGraph({ journey, language }: { journey?: TaskJourney; language?: string }) {
  const locale = localeFor(language ?? document.documentElement.lang ?? navigator.language);
  const text = copy[locale];
  const events = journey?.events ?? [];
  const edges = journey?.edges ?? [];
  if (!events.length) return <p className="task-journey-empty">{text.noEvents}</p>;
  return (
    <section className="task-journey" aria-labelledby="task-journey-heading">
      <header>
        <div>
          <h3 id="task-journey-heading">{text.heading}</h3>
          <p>{text.notCausal}</p>
        </div>
        <span className="task-journey-count">{text.trace} · {events.length}</span>
      </header>
      <ol className="task-journey-graph" aria-label={text.trace}>
        {events.map((event, index) => {
          const previous = events[index - 1];
          const follows = previous && edges.some((edge) => edge.kind === "followed_by" && edge.from === previous.id && edge.to === event.id);
          const contextualTargets = edges.filter((edge) => edge.kind === "refinement_context" && edge.from === event.id).map((edge) => events.find((candidate) => candidate.id === edge.to)).filter((target): target is TaskJourneyEvent => Boolean(target));
          return (
          <li key={event.id} id={`journey-event-${event.id}`} className={`task-journey-event is-${event.type}`}>
            {follows && <span className="task-journey-connector" aria-label={text.followed}>→</span>}
            <article>
              <span className="task-journey-marker" aria-hidden="true" />
              <div className="task-journey-content">
                <small>{text[event.type as keyof typeof text] ?? event.type.replaceAll("_", " ")}</small>
                <strong>{headlineFor(event, locale)}</strong>
                <div className="task-journey-evidence">{detailFor(event, locale)}</div>
                {event.occurredAt && <time dateTime={event.occurredAt}>{new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(event.occurredAt))}</time>}
              </div>
            </article>
            {contextualTargets.map((target) => <a className="task-journey-context" data-control="task-journey-context-open" key={target.id} href={`#journey-event-${encodeURIComponent(target.id)}`}>↳ {text.leadsTo}: {headlineFor(target, locale)}</a>)}
          </li>
        )})}
      </ol>
    </section>
  );
}
