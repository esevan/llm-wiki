import { useId, useLayoutEffect, useMemo, useRef, useState } from "react";
import "./task-journey-graph.css";

export interface TaskJourneyEvent {
  id: string;
  type: string;
  occurredAt?: string;
  detail?: { title?: string; summary?: string; report?: string; changes?: Array<{ field: string; before?: string; after?: string }>; payload?: unknown };
}
export interface TaskJourneyRelationship {
  from: string;
  to: string;
  kind: "supersedes" | "derived_from" | "depends_on";
  provenance?: "inferred" | "recorded";
  rationale?: string;
  evidence?: Array<{ eventId: string; quote: string }>;
}
export interface TaskJourney {
  events?: TaskJourneyEvent[];
  edges?: Array<{ from: string; to: string; kind: string }>;
  relationships?: TaskJourneyRelationship[];
  titles?: Array<{ id: string; title: string }>;
  semantics?: { causalInference?: boolean };
}

const copy = {
  en: { heading: "How this Task took shape", trace: "Recorded event trace", activity: "Activity", noEvents: "No recorded journey events are available yet.", notCausal: "Pale arrows show recorded order, not inferred cause.", changed: "Changed", close: "Close activity", leadsTo: "Linked refinement", semantic: "Interpreted relationships", inferred: "AI interpretation — select a connected step for evidence", supersedes: "Supersedes", derived_from: "Derived from", depends_on: "Requires", gap: "Time passes" },
  ko: { heading: "이 Task가 다듬어지고 실행된 과정", trace: "기록된 이벤트 흐름", activity: "활동", noEvents: "아직 표시할 작업 과정 기록이 없습니다.", notCausal: "옅은 화살표는 기록된 순서만 나타내며 원인 관계를 추론하지 않습니다.", changed: "변경", close: "활동 닫기", leadsTo: "연결된 정제", semantic: "해석된 관계", inferred: "AI 해석 — 연결된 단계를 선택하면 근거를 볼 수 있습니다", supersedes: "대체", derived_from: "파생", depends_on: "선행 필요", gap: "시간 경과" },
} as const;
const labels = {
  en: { origin_capture: "Original idea captured", task_created: "Task created", refinement_input: "Refinement note", refinement_applied: "Refinement applied", task_updated: "Task updated", execution_started: "Work started", work_recorded: "Work recorded", decision_recorded: "Decision recorded", task_completed: "Task completed" },
  ko: { origin_capture: "처음 아이디어 기록", task_created: "Task 생성", refinement_input: "정제 메모", refinement_applied: "정제 적용", task_updated: "Task 수정", execution_started: "작업 시작", work_recorded: "작업 기록 추가", decision_recorded: "결정 기록", task_completed: "Task 완료" },
} as const;
const fields = { en: { title: "title", detail: "detail", outcome: "outcome", scope: "scope", nonGoals: "non-goals", validationCriteria: "validation criteria" }, ko: { title: "제목", detail: "상세", outcome: "결과", scope: "범위", nonGoals: "제외 범위", validationCriteria: "검증 기준" } } as const;
const localeFor = (language?: string) => language?.toLowerCase().startsWith("ko") ? "ko" : "en";
const validDate = (value?: string) => {
  if (!value) return undefined;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? undefined : date;
};
const gapLabel = (from?: string, to?: string, locale?: "en" | "ko") => {
  const start = validDate(from), end = validDate(to);
  if (!start || !end) return undefined;
  const days = Math.floor((end.getTime() - start.getTime()) / 86_400_000);
  if (days < 1) return undefined;
  return locale === "ko" ? `${days}일 후` : `${days} ${days === 1 ? "day" : "days"} later`;
};
const payloadText = (payload: unknown): string => {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) return "";
  const record = payload as Record<string, unknown>;
  return ["summary", "rationale", "body", "note", "reason", "action"].map(key => record[key]).filter((value): value is string => typeof value === "string" && Boolean(value.trim())).join("\n");
};
function fullDetail(event: TaskJourneyEvent, locale: "en" | "ko") {
  const detail = event.detail ?? {};
  if (event.type === "refinement_applied" || event.type === "task_updated") return <>{detail.changes?.map(change => <p className="task-journey-full-text" key={change.field}>{copy[locale].changed} {fields[locale][change.field as keyof typeof fields.en] ?? change.field}: {change.before || "—"} → {change.after || "—"}</p>)}</>;
  const value = [detail.title, detail.summary, detail.report, payloadText(detail.payload)].filter(Boolean).join("\n\n");
  return value ? <p className="task-journey-full-text">{value}</p> : <p className="task-journey-empty">—</p>;
}
type Layout = { width: number; height: number; boxes: Record<string, DOMRect> };

export function TaskJourneyGraph({ journey, language }: { journey?: TaskJourney; language?: string }) {
  const uid = useId().replaceAll(":", "");
  const locale = localeFor(language ?? document.documentElement.lang ?? navigator.language);
  const text = copy[locale];
  const events = useMemo(() => journey?.events ?? [], [journey?.events]);
  const titles = new Map((journey?.titles ?? []).map(title => [title.id, title.title]));
  const [activeId, setActiveId] = useState<string>();
  const [layout, setLayout] = useState<Layout>({ width: 0, height: 0, boxes: {} });
  const boardRef = useRef<HTMLDivElement>(null);
  const nodeRefs = useRef(new Map<string, HTMLButtonElement>());
  const relationships = useMemo(() => (journey?.relationships ?? []).filter(link => events.some(event => event.id === link.from) && events.some(event => event.id === link.to)), [events, journey?.relationships]);
  const active = events.find(event => event.id === activeId);
  const activeRelationships = activeId ? relationships.filter(link => link.from === activeId || link.to === activeId) : [];

  useLayoutEffect(() => {
    const board = boardRef.current;
    if (!board) return;
    const measure = () => {
      const outer = board.getBoundingClientRect();
      const boxes: Record<string, DOMRect> = {};
      nodeRefs.current.forEach((node, id) => {
        const rect = node.getBoundingClientRect();
        boxes[id] = new DOMRect(rect.left - outer.left, rect.top - outer.top, rect.width, rect.height);
      });
      setLayout({ width: outer.width, height: outer.height, boxes });
    };
    measure();
    const observer = typeof ResizeObserver === "undefined" ? undefined : new ResizeObserver(measure);
    observer?.observe(board);
    nodeRefs.current.forEach(node => observer?.observe(node));
    window.addEventListener("resize", measure);
    return () => { observer?.disconnect(); window.removeEventListener("resize", measure); };
  }, [events, language]);

  if (!events.length) return <p className="task-journey-empty">{text.noEvents}</p>;
  const titleFor = (event: TaskJourneyEvent) => titles.get(event.id) ?? labels[locale][event.type as keyof typeof labels.en] ?? text.activity;
  const chronological = events.slice(1).map((event, index) => {
    const previous = events[index];
    const follows = journey?.edges?.some(edge => edge.kind === "followed_by" && edge.from === previous.id && edge.to === event.id);
    return follows ? { previous, event, gap: gapLabel(previous.occurredAt, event.occurredAt, locale) } : undefined;
  }).filter((item): item is NonNullable<typeof item> => Boolean(item));
  const chronologyPath = (from: string, to: string) => {
    const source = layout.boxes[from], target = layout.boxes[to];
    if (!source || !target) return "";
    const sameRow = Math.abs(source.y + source.height / 2 - target.y - target.height / 2) < 20;
    if (sameRow) {
      const forward = target.x > source.x, startX = forward ? source.right : source.left, endX = forward ? target.left : target.right;
      const sourceY = source.y + source.height / 2, targetY = target.y + target.height / 2;
      const middleX = (startX + endX) / 2;
      return `M ${startX} ${sourceY} H ${middleX} V ${targetY} H ${endX}`;
    }
    const x = source.x + source.width / 2, middleY = (source.bottom + target.top) / 2;
    return `M ${x} ${source.bottom} V ${middleY} H ${target.x + target.width / 2} V ${target.top}`;
  };
  const relationshipPath = (link: TaskJourneyRelationship, index: number) => {
    const source = layout.boxes[link.from], target = layout.boxes[link.to];
    if (!source || !target) return "";
    const sourceX = source.x + source.width / 2, targetX = target.x + target.width / 2;
    const targetAbove = target.y + target.height / 2 < source.y + source.height / 2;
    const sameRow = Math.abs(source.y + source.height / 2 - target.y - target.height / 2) < 20;
    const offset = 22 + index * 7;
    if (sameRow) {
      const gutter = Math.min(source.top, target.top) - offset;
      return `M ${sourceX} ${source.top} V ${gutter} H ${targetX} V ${target.top}`;
    }
    const sourcePort = targetAbove ? source.top : source.bottom;
    const targetPort = targetAbove ? target.bottom : target.top;
    const sourceGutter = sourcePort + (targetAbove ? -offset : offset);
    const targetGutter = targetPort + (targetAbove ? offset : -offset);
    const right = (sourceX + targetX) / 2 >= layout.width / 2;
    const lane = right ? layout.width - 18 - index * 7 : 18 + index * 7;
    return `M ${sourceX} ${sourcePort} V ${sourceGutter} H ${lane} V ${targetGutter} H ${targetX} V ${targetPort}`;
  };
  const incident = (eventId: string) => activeId && relationships.some(link => (link.from === activeId || link.to === activeId) && (link.from === eventId || link.to === eventId));

  return <section className="task-journey" aria-labelledby={`${uid}-heading`}>
    <header><div><h3 id={`${uid}-heading`}>{text.heading}</h3><p>{text.notCausal}</p></div><span className="task-journey-count">{text.trace} · {events.length}</span></header>
    <div className={`task-journey-board${activeId ? " has-selection" : ""}`} ref={boardRef}>
      <svg className="task-journey-wires" width={layout.width} height={layout.height} viewBox={`0 0 ${layout.width} ${layout.height}`} aria-hidden="true">
        <defs>
          <marker id={`${uid}-chronology-arrow`} viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto"><path d="M 0 0 L 10 5 L 0 10 z" fill="#c9bdc2" /></marker>
          <marker id={`${uid}-supersedes-end`} viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto"><path d="M1 1 L10 6 L1 11" fill="none" stroke="#bb5973" strokeWidth="2" /></marker>
          <marker id={`${uid}-derived_from-end`} viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto"><path d="M1 6 L6 1 L11 6 L6 11 Z" fill="white" stroke="#37866a" strokeWidth="2" /></marker>
          <marker id={`${uid}-depends_on-end`} viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto"><rect x="2" y="2" width="8" height="8" fill="white" stroke="#b67a12" strokeWidth="2" /></marker>
        </defs>
        <g className="task-journey-chronology">{chronological.map(({ previous, event }) => <path key={`${previous.id}-${event.id}`} d={chronologyPath(previous.id, event.id)} markerEnd={`url(#${uid}-chronology-arrow)`} />)}</g>
        <g className="task-journey-crosslinks">{relationships.map((link, index) => <path data-relationship={link.kind} className={`is-${link.kind}${activeId && (link.from === activeId || link.to === activeId) ? " is-active" : ""}`} key={`${link.from}-${link.to}-${link.kind}-${index}`} d={relationshipPath(link, index)} markerEnd={`url(#${uid}-${link.kind}-end)`} />)}</g>
      </svg>
      {chronological.map(({ previous, event, gap }) => {
        const source = layout.boxes[previous.id], target = layout.boxes[event.id];
        return gap && source && target ? <span className="task-journey-gap" key={`gap-${previous.id}-${event.id}`} style={{ left: `${(source.x + source.width / 2 + target.x + target.width / 2) / 2}px`, top: `${(source.y + source.height / 2 + target.y + target.height / 2) / 2}px` }} aria-label={`${text.gap}: ${gap}`}><i aria-hidden="true">···</i><small>{gap}</small></span> : null;
      })}
      <ol className="task-journey-graph" aria-label={text.trace}>{events.map((event, index) => {
        const row = Math.floor(index / 3), column = row % 2 === 0 ? index % 3 + 1 : 3 - index % 3;
        const degree = relationships.filter(link => link.from === event.id || link.to === event.id).length;
        const keyNode = ["refinement_applied", "task_updated", "decision_recorded", "task_completed"].includes(event.type) || degree >= 2;
        const date = validDate(event.occurredAt);
        return <li key={event.id} id={`${uid}-event-${index}`} className={`task-journey-event is-${event.type}${keyNode ? " is-key-node" : ""}`} style={{ gridColumn: column, gridRow: row + 1 }}>
          <button ref={node => { if (node) nodeRefs.current.set(event.id, node); else nodeRefs.current.delete(event.id); }} type="button" data-control="task-journey-activity-open" className={`task-journey-node${incident(event.id) ? " is-semantic-linked" : ""}`} aria-label={titleFor(event)} aria-pressed={activeId === event.id} onClick={() => setActiveId(event.id)} title={titleFor(event)}>
            <span className="task-journey-step" aria-hidden="true">{index + 1}</span><span className="task-journey-node-copy">{titleFor(event)}{date && <time className="task-journey-node-time" dateTime={event.occurredAt}>{new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(date)}</time>}</span>
          </button>
        </li>;
      })}</ol>
    </div>
    {relationships.length > 0 && <div className="task-journey-legend" aria-label={text.semantic}>
      <span className="task-journey-legend-heading"><strong>{text.semantic}</strong><small>{text.inferred}</small></span>
      {(["supersedes", "derived_from", "depends_on"] as const).filter(kind => relationships.some(link => link.kind === kind)).map(kind => <span className={`task-journey-legend-key is-${kind}`} key={kind}><i aria-hidden="true" />{text[kind]}</span>)}
    </div>}
    {activeRelationships.length > 0 && <section className="task-journey-semantic-links" aria-label={text.semantic}>{activeRelationships.map((link, index) => {
      const source = events.find(event => event.id === link.from)!, target = events.find(event => event.id === link.to)!;
      return <article className={`task-journey-semantic-link is-${link.kind}`} key={`${link.from}-${link.to}-${link.kind}-${index}`}>
        <strong>{text[link.kind]}</strong><div><button type="button" data-control="task-journey-activity-open" onClick={() => setActiveId(link.from)}>{titleFor(source)}</button><span aria-hidden="true">→</span><button type="button" data-control="task-journey-activity-open" onClick={() => setActiveId(link.to)}>{titleFor(target)}</button></div>
        {link.rationale && <p>{link.rationale}</p>}{link.evidence?.map(item => <blockquote key={`${item.eventId}-${item.quote}`}><span aria-hidden="true">“</span>{item.quote}<cite>{titles.get(item.eventId) ?? item.eventId}</cite></blockquote>)}
      </article>;
    })}</section>}
    {active && <aside className="task-journey-activity" aria-label={text.activity}>
      <header><div><small>{labels[locale][active.type as keyof typeof labels.en] ?? active.type.replaceAll("_", " ")}</small><h4>{text.activity}</h4></div><button type="button" data-control="task-journey-activity-close" onClick={() => setActiveId(undefined)}>{text.close}</button></header>
      {validDate(active.occurredAt) && <time dateTime={active.occurredAt}>{new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(validDate(active.occurredAt))}</time>}
      {fullDetail(active, locale)}
      {journey?.edges?.filter(edge => edge.kind === "refinement_context" && edge.from === active.id).map(edge => events.find(event => event.id === edge.to)).filter((event): event is TaskJourneyEvent => Boolean(event)).map(event => <a key={event.id} data-control="task-journey-context-open" className="task-journey-context" href={`#${uid}-event-${events.indexOf(event)}`} onClick={() => setActiveId(event.id)}>↳ {text.leadsTo}: {titleFor(event)}</a>)}
    </aside>}
  </section>;
}
