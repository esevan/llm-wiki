import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useModalInteraction } from "./useModalInteraction";
import "./task-journey-graph.css";

type TaskJourneyDetail = {
  title?: string;
  summary?: string;
  report?: string;
  result?: string;
  status?: string;
  currentStatus?: string;
  sources?: Array<{ type: string; id: string; revision: string; locator: string; quote?: string }>;
  changes?: Array<{ field: string; before?: string; after?: string }>;
  payload?: unknown;
  taskRevision?: number;
  source?: string;
  decisionKind?: string;
  definition?: Partial<Record<"title" | "detail" | "outcome" | "scope" | "nonGoals" | "validationCriteria", string>>;
  [key: string]: unknown;
};
export interface TaskJourneyEvent {
  id: string;
  type: string;
  occurredAt?: string;
  detail?: TaskJourneyDetail;
}
export interface TaskJourneyRelationship {
  from: string;
  to: string;
  kind: "supersedes" | "derived_from" | "depends_on" | "supports" | "verifies" | "contradicts" | "merged_into";
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
  topicStates?: Array<{ topicKey: string; status: string; currentNodeId?: string; replacementNodeId?: string }>;
  freshness?: string;
}

const copy = {
  en: { heading: "Current decisions and evidence", trace: "Recorded event trace", activity: "Activity", noEvents: "No supported journey nodes are available yet.", notCausal: "Links appear only when saved evidence supports them.", close: "Close activity details", semantic: "Evidence-backed relationships", inferred: "AI interpretation — select a connected step for evidence", aiInterpretation: "AI interpretation", recordedRelationship: "Recorded relationship", supersedes: "Supersedes", derived_from: "Derived from", depends_on: "Requires", supports: "Supports", verifies: "Verifies", contradicts: "Contradicts", merged_into: "Merged into", currentTopics: "Current topics", current: "Current", pending: "Updating", stale: "Source changed", retryable_failure: "Update failed", repair_required: "Repair required", unavailable: "Unavailable", adopted: "Adopted", superseded: "Superseded", withdrawn: "Withdrawn", unresolved: "Unresolved", verified: "Verified", suggested: "Suggested", observed: "Observed", performed: "Performed", attempted: "Attempted", gap: "Time passes", coreContent: "Recorded content", definition: "Task definition", changes: "Before and after", result: "Result", context: "Context and evidence", previous: "Previous activity", next: "Next activity", related: "Linked activity", step: "Step", of: "of", empty: "No additional recorded content.", before: "Before", after: "After" },
  ko: { heading: "현재 결정과 근거", trace: "기록된 이벤트 흐름", activity: "활동", noEvents: "근거가 확인된 과정 노드가 아직 없습니다.", notCausal: "저장된 근거가 뒷받침하는 연결만 표시합니다.", close: "활동 상세 닫기", semantic: "근거가 있는 관계", inferred: "AI 해석 — 연결된 단계를 선택하면 근거를 볼 수 있습니다", aiInterpretation: "AI 해석", recordedRelationship: "기록된 관계", supersedes: "대체", derived_from: "파생", depends_on: "선행 필요", supports: "뒷받침", verifies: "검증", contradicts: "충돌", merged_into: "병합", currentTopics: "현재 주제", current: "최신", pending: "업데이트 중", stale: "출처 변경됨", retryable_failure: "업데이트 실패", repair_required: "복구 필요", unavailable: "사용 불가", adopted: "채택됨", superseded: "대체됨", withdrawn: "철회됨", unresolved: "미해결", verified: "검증됨", suggested: "제안됨", observed: "관찰됨", performed: "수행됨", attempted: "시도됨", gap: "시간 경과", coreContent: "기록 내용", definition: "Task 정의", changes: "변경 전후", result: "결과", context: "맥락과 근거", previous: "이전 활동", next: "다음 활동", related: "연결된 활동", step: "단계", of: "/", empty: "추가로 기록된 내용이 없습니다.", before: "변경 전", after: "변경 후" },
} as const;
const labels = {
  en: { decision: "Decision", result: "Result", evidence: "Evidence", idea: "Idea", attempt: "Attempt", completion: "Completion", goal: "Goal", scope: "Scope", criterion: "Criterion", origin_capture: "Original idea captured", task_created: "Task created", refinement_input: "Refinement note", refinement_applied: "Refinement applied", task_updated: "Task updated", execution_started: "Work started", work_recorded: "Work recorded", decision_recorded: "Decision recorded", task_completed: "Task completed" },
  ko: { decision: "결정", result: "결과", evidence: "근거", idea: "아이디어", attempt: "시도", completion: "완료", goal: "목표", scope: "범위", criterion: "기준", origin_capture: "처음 아이디어 기록", task_created: "Task 생성", refinement_input: "정제 메모", refinement_applied: "정제 적용", task_updated: "Task 수정", execution_started: "작업 시작", work_recorded: "작업 기록 추가", decision_recorded: "결정 기록", task_completed: "Task 완료" },
} as const;
const fields = { en: { criterion: "Criterion", decision: "Decision", title: "title", detail: "detail", outcome: "outcome", scope: "scope", nonGoals: "non-goals", validationCriteria: "validation criteria" }, ko: { criterion: "기준", decision: "결정", title: "제목", detail: "상세", outcome: "결과", scope: "범위", nonGoals: "제외 범위", validationCriteria: "검증 기준" } } as const;
const payloadFields = {
  en: { summary: "Summary", rationale: "Rationale", body: "Body", note: "Note", reason: "Reason", action: "Action", evidence: "Evidence", result: "Result", outcome: "Outcome" },
  ko: { summary: "요약", rationale: "근거", body: "내용", note: "메모", reason: "이유", action: "조치", evidence: "근거 자료", result: "결과", outcome: "결과" },
} as const;
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
const displayValue = (value: unknown) => {
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value) && value.every(item => ["string", "number", "boolean"].includes(typeof item))) return value.join(", ");
  return "";
};
const fieldLabel = (value: string) => value.replace(/([a-z])([A-Z])/g, "$1 $2").replaceAll("_", " ").replace(/^./, letter => letter.toUpperCase());
const payloadEntries = (payload: unknown) => payload && typeof payload === "object" && !Array.isArray(payload)
  ? Object.entries(payload as Record<string, unknown>).map(([key, value]) => [key, displayValue(value)] as const)
    .filter(([key, value]) => value && !/(^id$|id$|revision$)/i.test(key))
  : [];

function ActivityDialog({ active, activeIndex, events, edges, relationships, titles, locale, titleFor, opener, onClose, onSelect }: {
  active: TaskJourneyEvent;
  activeIndex: number;
  events: TaskJourneyEvent[];
  edges: NonNullable<TaskJourney["edges"]>;
  relationships: TaskJourneyRelationship[];
  titles: Map<string, string>;
  locale: "en" | "ko";
  titleFor: (event: TaskJourneyEvent) => string;
  opener?: HTMLElement;
  onClose: () => void;
  onSelect: (id: string) => void;
}) {
  const text = copy[locale];
  const rootRef = useRef<HTMLDivElement>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const headingId = useId();
  useModalInteraction(rootRef, 15, onClose);
  useEffect(() => {
    const owner = opener?.closest<HTMLElement>("[data-task-detail-modal='true']");
    const wasInert = owner?.inert;
    if (owner) owner.inert = true;
    const bodyOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      if (owner) owner.inert = Boolean(wasInert);
      document.body.style.overflow = bodyOverflow;
      requestAnimationFrame(() => { if (opener?.isConnected) opener.focus({ preventScroll: true }); });
    };
  }, [opener]);
  useEffect(() => {
    if (contentRef.current) contentRef.current.scrollTop = 0;
    headingRef.current?.focus({ preventScroll: true });
  }, [active.id]);

  const detail = active.detail ?? {};
  const definition = Object.entries(detail.definition ?? {}).filter((entry): entry is [keyof typeof fields.en, string] => typeof entry[1] === "string" && Boolean(entry[1].trim()));
  const core = [definition.length ? undefined : detail.title, detail.summary].filter((value, index, values): value is string => typeof value === "string" && Boolean(value.trim()) && values.indexOf(value) === index);
  const payload = payloadEntries(detail.payload);
  const activeRelationships = relationships.filter(link => link.from === active.id || link.to === active.id);
  const contextual = edges.filter(edge => edge.kind === "refinement_context" && (edge.from === active.id || edge.to === active.id));
  const linkedIds = [...new Set([
    ...activeRelationships.map(link => link.from === active.id ? link.to : link.from),
    ...contextual.map(edge => edge.from === active.id ? edge.to : edge.from),
  ])];
  const linked = linkedIds.map(id => ({
    event: events.find(event => event.id === id),
    contextual: contextual.some(edge => (edge.from === active.id ? edge.to : edge.from) === id),
  })).filter((item): item is { event: TaskJourneyEvent; contextual: boolean } => Boolean(item.event));
  const date = validDate(active.occurredAt);

  return createPortal(<div ref={rootRef} className="task-journey-modal-layer" tabIndex={-1}>
    <button type="button" className="task-journey-modal-backdrop" data-control="task-journey-activity-close" aria-label={text.close} onClick={onClose} />
    <section className="task-journey-dialog" role="dialog" aria-modal="true" aria-labelledby={headingId}>
      <header className="task-journey-dialog-header">
        <div>
          <small>{labels[locale][active.type as keyof typeof labels.en] ?? fieldLabel(active.type)}</small>
          <h2 id={headingId} ref={headingRef} tabIndex={-1}>{titleFor(active)}</h2>
          <p>{text.step} {activeIndex + 1} {text.of} {events.length}{date && <> · <time dateTime={active.occurredAt}>{new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(date)}</time></>}</p>
        </div>
        <button type="button" className="task-journey-dialog-close" data-control="task-journey-activity-close" aria-label={text.close} onClick={onClose}><span aria-hidden="true">×</span></button>
      </header>
      <div className="task-journey-dialog-content" ref={contentRef}>
        {core.length > 0 && <section className="task-journey-detail-section"><h3>{text.coreContent}</h3>{core.map(value => <p className="task-journey-full-text" key={value}>{value}</p>)}</section>}
        {definition.length > 0 && <section className="task-journey-detail-section"><h3>{text.definition}</h3><dl className="task-journey-payload">{definition.map(([key, value]) => <div key={key}><dt>{fields[locale][key]}</dt><dd>{value}</dd></div>)}</dl></section>}
        {detail.changes?.length ? <section className="task-journey-detail-section"><h3>{text.changes}</h3><div className="task-journey-changes">{detail.changes.map(change => <article key={change.field}><h4>{fields[locale][change.field as keyof typeof fields.en] ?? (locale === "ko" ? change.field : fieldLabel(change.field))}</h4><div><span><small>{text.before}</small>{change.before || "—"}</span><i aria-hidden="true">→</i><span><small>{text.after}</small>{change.after || "—"}</span></div></article>)}</div></section> : null}
        {payload.length > 0 && <section className="task-journey-detail-section"><h3>{text.context}</h3><dl className="task-journey-payload">{payload.map(([key, value]) => <div key={key}><dt>{payloadFields[locale][key as keyof typeof payloadFields.en] ?? (locale === "ko" ? key : fieldLabel(key))}</dt><dd>{value}</dd></div>)}</dl></section>}
        {(detail.report?.trim() || detail.result?.trim()) && <section className="task-journey-detail-section is-result"><h3>{text.result}</h3><p className="task-journey-full-text">{detail.report || detail.result}</p></section>}
        {detail.currentStatus && <p role="status">{text[detail.currentStatus as keyof typeof text] ?? detail.currentStatus}</p>}
        {(detail.sources?.length ?? 0) > 0 && <section className="task-journey-detail-section"><h3>{text.context}</h3>{detail.sources!.map((source, index) => <blockquote key={`${source.id}-${source.locator}-${index}`}>{source.quote || source.locator}<cite>{source.type} · {source.id} · {source.revision} · {source.locator}</cite></blockquote>)}</section>}
        {activeRelationships.length > 0 && <section className="task-journey-detail-section"><h3>{text.semantic}</h3><div className="task-journey-dialog-relationships">{activeRelationships.map((link, index) => {
          const source = events.find(event => event.id === link.from)!;
          const target = events.find(event => event.id === link.to)!;
          return <article className={`task-journey-semantic-link is-${link.kind}`} key={`${link.from}-${link.to}-${link.kind}-${index}`}>
            <span className="task-journey-interpretation-label">{link.provenance === "recorded" ? text.recordedRelationship : text.aiInterpretation}</span>
            <strong>{text[link.kind]}</strong>
            <div><button type="button" data-control="task-journey-activity-open" onClick={() => onSelect(source.id)}>{titleFor(source)}</button><span aria-hidden="true">→</span><button type="button" data-control="task-journey-activity-open" onClick={() => onSelect(target.id)}>{titleFor(target)}</button></div>
            {link.rationale && <p>{link.rationale}</p>}{link.evidence?.map(item => <blockquote key={`${item.eventId}-${item.quote}`}><span aria-hidden="true">“</span>{item.quote}<cite>{titles.get(item.eventId) ?? events.find(event => event.id === item.eventId)?.type ?? item.eventId}</cite></blockquote>)}
          </article>;
        })}</div></section>}
        {!core.length && !definition.length && !detail.changes?.length && !payload.length && !detail.report?.trim() && !detail.result?.trim() && !activeRelationships.length && <p className="task-journey-empty">{text.empty}</p>}
        {linked.length > 0 && <nav className="task-journey-linked" aria-label={text.related}><strong>{text.related}</strong>{linked.map(({ event, contextual }) => contextual
          ? <button type="button" data-control="task-journey-context-open" key={event.id} onClick={() => onSelect(event.id)}><span>↳</span>{titleFor(event)}</button>
          : <button type="button" data-control="task-journey-activity-open" key={event.id} onClick={() => onSelect(event.id)}><span>↳</span>{titleFor(event)}</button>)}</nav>}
      </div>
      <footer className="task-journey-dialog-nav">
        <button type="button" data-control="task-journey-activity-open" disabled={activeIndex === 0} onClick={() => onSelect(events[activeIndex - 1]?.id)}>← <span>{text.previous}</span></button>
        <span>{activeIndex + 1} / {events.length}</span>
        <button type="button" data-control="task-journey-activity-open" disabled={activeIndex === events.length - 1} onClick={() => onSelect(events[activeIndex + 1]?.id)}><span>{text.next}</span> →</button>
      </footer>
    </section>
  </div>, document.body);
}
type Layout = { width: number; height: number; boxes: Record<string, DOMRect> };

export function TaskJourneyGraph({ journey, language }: { journey?: TaskJourney; language?: string }) {
  const uid = useId().replaceAll(":", "");
  const locale = localeFor(language ?? document.documentElement.lang ?? navigator.language);
  const text = copy[locale];
  const events = useMemo(() => journey?.events ?? [], [journey?.events]);
  const titles = new Map((journey?.titles ?? []).map(title => [title.id, title.title]));
  const [activeId, setActiveId] = useState<string>();
  const [activityOpener, setActivityOpener] = useState<HTMLElement>();
  const [layout, setLayout] = useState<Layout>({ width: 0, height: 0, boxes: {} });
  const boardRef = useRef<HTMLDivElement>(null);
  const nodeRefs = useRef(new Map<string, HTMLButtonElement>());
  const relationships = useMemo(() => (journey?.relationships ?? []).filter(link => events.some(event => event.id === link.from) && events.some(event => event.id === link.to)), [events, journey?.relationships]);
  const active = events.find(event => event.id === activeId);
  const activeIndex = active ? events.indexOf(active) : -1;

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
    <header><div><h3 id={`${uid}-heading`}>{text.heading}</h3><p>{text.notCausal}</p></div><span className="task-journey-count">{journey?.freshness ? text[journey.freshness as keyof typeof text] ?? journey.freshness : text.trace} · {events.length}</span></header>
    {(journey?.topicStates?.length ?? 0) > 0 && <section className="task-journey-topics" aria-label={text.currentTopics}><h4>{text.currentTopics}</h4><ul>{journey!.topicStates!.map(topic => <li key={topic.topicKey} className={`is-${topic.status}`}><strong>{topic.topicKey}</strong><span>{text[topic.status as keyof typeof text] ?? topic.status}</span></li>)}</ul></section>}
    <div className={`task-journey-board${activeId ? " has-selection" : ""}`} ref={boardRef}>
      <svg className="task-journey-wires" width={layout.width} height={layout.height} viewBox={`0 0 ${layout.width} ${layout.height}`} aria-hidden="true">
        <defs>
          <marker id={`${uid}-chronology-arrow`} viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto"><path d="M 0 0 L 10 5 L 0 10 z" fill="#c9bdc2" /></marker>
          <marker id={`${uid}-supersedes-end`} viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto"><path d="M1 1 L10 6 L1 11" fill="none" stroke="#bb5973" strokeWidth="2" /></marker>
          <marker id={`${uid}-derived_from-end`} viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto"><path d="M1 6 L6 1 L11 6 L6 11 Z" fill="white" stroke="#37866a" strokeWidth="2" /></marker>
          <marker id={`${uid}-depends_on-end`} viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto"><rect x="2" y="2" width="8" height="8" fill="white" stroke="#b67a12" strokeWidth="2" /></marker>
          {(["supports", "verifies", "contradicts", "merged_into"] as const).map(kind => <marker key={kind} id={`${uid}-${kind}-end`} viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto"><circle cx="6" cy="6" r="4" fill="white" stroke="currentColor" strokeWidth="2" /></marker>)}
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
        const nodeStatus = event.detail?.currentStatus || event.detail?.status;
        return <li key={event.id} id={`${uid}-event-${index}`} className={`task-journey-event is-${event.type}${nodeStatus ? ` status-${nodeStatus}` : ""}${keyNode ? " is-key-node" : ""}`} style={{ gridColumn: column, gridRow: row + 1 }}>
          <button ref={node => { if (node) nodeRefs.current.set(event.id, node); else nodeRefs.current.delete(event.id); }} type="button" data-control="task-journey-activity-open" className={`task-journey-node${incident(event.id) ? " is-semantic-linked" : ""}`} aria-label={titleFor(event)} aria-pressed={activeId === event.id} onClick={eventClick => { setActivityOpener(eventClick.currentTarget); setActiveId(event.id); }} title={titleFor(event)}>
            <span className="task-journey-step" aria-hidden="true">{index + 1}</span><span className="task-journey-node-copy">{titleFor(event)}{nodeStatus && <small className="task-journey-node-state">{text[nodeStatus as keyof typeof text] ?? nodeStatus}</small>}{date && <time className="task-journey-node-time" dateTime={event.occurredAt}>{new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(date)}</time>}</span>
          </button>
        </li>;
      })}</ol>
    </div>
    {relationships.length > 0 && <div className="task-journey-legend" aria-label={text.semantic}>
      <span className="task-journey-legend-heading"><strong>{text.semantic}</strong><small>{text.inferred}</small></span>
      {(["supersedes", "derived_from", "depends_on", "supports", "verifies", "contradicts", "merged_into"] as const).filter(kind => relationships.some(link => link.kind === kind)).map(kind => <span className={`task-journey-legend-key is-${kind}`} key={kind}><i aria-hidden="true" />{text[kind]}</span>)}
    </div>}
    {active && <ActivityDialog active={active} activeIndex={activeIndex} events={events} edges={journey?.edges ?? []} relationships={relationships} titles={titles} locale={locale} titleFor={titleFor} opener={activityOpener} onClose={() => setActiveId(undefined)} onSelect={setActiveId} />}
  </section>;
}
