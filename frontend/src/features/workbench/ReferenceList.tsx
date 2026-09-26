import { useMemo, useState } from "react";
import { referenceKey } from "../../components/referenceIdentity";
import type {
  ExactReferenceBinding,
  ReferenceUsageFact,
} from "../../types/taskWorkbench";
import { useReferenceWorkbenchText } from "./referenceWorkbenchText";

export interface ReferenceListProps {
  references: ExactReferenceBinding[];
  interactions?: ReferenceUsageFact[];
  disabled?: boolean;
  onOpen: (
    reference: ExactReferenceBinding,
    visible: ExactReferenceBinding[],
  ) => void;
  onUsage?: (
    reference: ExactReferenceBinding,
    kind: "used" | "excluded",
  ) => void;
}
const pageSize = 20;
const documentKey = (reference: ExactReferenceBinding) =>
  JSON.stringify([reference.documentId, reference.documentVersion]);

export function ReferenceList({
  references,
  interactions = [],
  disabled,
  onOpen,
  onUsage,
}: ReferenceListProps) {
  const text = useReferenceWorkbenchText();
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("all");
  const [aspect, setAspect] = useState("all");
  const [type, setType] = useState("all");
  const [usage, setUsage] = useState("all");
  const [sort, setSort] = useState("reading");
  const [page, setPage] = useState(0);
  const [accepted, setAccepted] = useState(() => references.map(referenceKey));
  // Freeze row membership during reading. A new source or section is admitted
  // only by an explicit acceptance, filter or sort action.
  const acceptedSet = useMemo(() => new Set(accepted), [accepted]);
  const pending = references.filter(
    (reference) => !acceptedSet.has(referenceKey(reference)),
  );
  const usageByKey = useMemo(() => {
    const facts = new Map<string, Set<string>>();
    for (const reference of references) {
      const kinds = new Set<string>();
      if (reference.claimIds?.length) kinds.add("used");
      facts.set(referenceKey(reference), kinds);
    }
    for (const fact of interactions)
      facts.get(referenceKey(fact))?.add(fact.kind);
    return facts;
  }, [interactions, references]);
  const groups = useMemo(() => {
    const order = new Map(accepted.map((key, index) => [key, index]));
    const filtered = references
      .filter((reference) => {
        const key = referenceKey(reference);
        const kinds = usageByKey.get(key);
        return (
          acceptedSet.has(key) &&
          `${reference.title} ${reference.documentId} ${reference.path ?? ""} ${reference.section ?? ""} ${reference.excerpt ?? ""}`
            .toLocaleLowerCase()
            .includes(query.toLocaleLowerCase()) &&
          (status === "all" || reference.status === status) &&
          (aspect === "all" || reference.aspect === aspect) &&
          (type === "all" || reference.informationType === type) &&
          (usage === "all" ||
            (usage === "discovered" ? !kinds?.size : kinds?.has(usage)))
        );
      })
      .sort(
        (left, right) =>
          (order.get(referenceKey(left)) ?? 0) -
          (order.get(referenceKey(right)) ?? 0),
      );
    const documents = new Map<string, ExactReferenceBinding[]>();
    for (const reference of filtered) {
      const key = documentKey(reference);
      const sections = documents.get(key) ?? [];
      if (
        !sections.some(
          (section) => referenceKey(section) === referenceKey(reference),
        )
      )
        sections.push(reference);
      documents.set(key, sections);
    }
    const result = [...documents.entries()];
    if (sort === "title")
      result.sort(
        ([leftKey, left], [rightKey, right]) =>
          left[0].title.localeCompare(right[0].title) ||
          leftKey.localeCompare(rightKey),
      );
    return result;
  }, [
    accepted,
    acceptedSet,
    references,
    query,
    status,
    aspect,
    type,
    usage,
    sort,
    usageByKey,
  ]);
  const totalPages = Math.max(1, Math.ceil(groups.length / pageSize));
  const currentPage = Math.min(page, totalPages - 1);
  const visibleGroups = groups.slice(
    currentPage * pageSize,
    (currentPage + 1) * pageSize,
  );
  const visibleReferences = visibleGroups.flatMap(([, sections]) => sections);
  const acceptArrivals = () =>
    setAccepted((previous) => {
      const known = new Set(previous);
      return [
        ...previous,
        ...references.map(referenceKey).filter((key) => !known.has(key)),
      ];
    });
  const changeFilter = (change: () => void) => {
    acceptArrivals();
    setPage(0);
    change();
  };
  const label = (value: string) => {
    const labels: Record<string, string> = {
      content: text.aspectContent,
      applicability: text.aspectApplicability,
      decision: text.aspectDecision,
      exploration: text.aspectExploration,
      knowledge: text.typeKnowledge,
      idea: text.typeIdea,
      reference: text.typeReference,
      raw: text.typeRaw,
      moc: text.typeMoc,
      current: text.currentStatus,
      historical: text.historicalStatus,
      unverified: text.unverified,
      deferred: text.deferred,
      rejected: text.rejected,
      viewed: text.usageViewed,
      mentioned: text.usageMentioned,
      used: text.usageUsed,
      adopted: text.usageAdopted,
      excluded: text.usageExcluded,
      discovered: text.usageDiscovered,
    };
    return labels[value] ?? value;
  };
  const options = (key: "status" | "aspect" | "informationType") =>
    [
      ...new Set(
        references
          .map((reference) => reference[key])
          .filter((value): value is string => Boolean(value)),
      ),
    ].sort();
  return (
    <section
      className="reference-aware-reference-list"
      aria-label={text.references}
    >
      <h4>{text.references}</h4>
      <div className="reference-list-controls">
        <label>
          {text.search}
          <input
            data-control="reference-list-search"
            value={query}
            onChange={(event) =>
              changeFilter(() => setQuery(event.target.value))
            }
          />
        </label>
        <label>
          {text.filter}
          <select
            data-control="reference-list-filter"
            value={status}
            onChange={(event) =>
              changeFilter(() => setStatus(event.target.value))
            }
          >
            <option value="all">{text.all}</option>
            {options("status").map((value) => (
              <option key={value} value={value}>
                {label(value)}
              </option>
            ))}
          </select>
        </label>
        <label>
          {text.aspectFilter}
          <select
            data-control="reference-list-aspect"
            value={aspect}
            onChange={(event) =>
              changeFilter(() => setAspect(event.target.value))
            }
          >
            <option value="all">{text.allAspects}</option>
            {options("aspect").map((value) => (
              <option key={value} value={value}>
                {label(value)}
              </option>
            ))}
          </select>
        </label>
        <label>
          {text.typeFilter}
          <select
            data-control="reference-list-type"
            value={type}
            onChange={(event) =>
              changeFilter(() => setType(event.target.value))
            }
          >
            <option value="all">{text.allTypes}</option>
            {options("informationType").map((value) => (
              <option key={value} value={value}>
                {label(value)}
              </option>
            ))}
          </select>
        </label>
        <label>
          {text.usageFilter}
          <select
            data-control="reference-list-usage"
            value={usage}
            onChange={(event) =>
              changeFilter(() => setUsage(event.target.value))
            }
          >
            <option value="all">{text.allUsage}</option>
            {[
              "discovered",
              "viewed",
              "mentioned",
              "used",
              "adopted",
              "excluded",
            ].map((value) => (
              <option key={value} value={value}>
                {label(value)}
              </option>
            ))}
          </select>
        </label>
        <label>
          {text.sort}
          <select
            data-control="reference-list-sort"
            value={sort}
            onChange={(event) =>
              changeFilter(() => setSort(event.target.value))
            }
          >
            <option value="reading">{text.stable}</option>
            <option value="title">{text.titleSort}</option>
          </select>
        </label>
      </div>
      {pending.length > 0 && (
        <div className="reference-arrivals" role="status">
          <span>
            {text.newReferences} ({pending.length})
          </span>
          <button
            type="button"
            data-control="reference-list-accept"
            onClick={acceptArrivals}
          >
            {text.showNewReferences}
          </button>
        </div>
      )}
      {!groups.length && <p role="status">{text.emptyReferences}</p>}
      {visibleGroups.map(([key, sections]) => (
        <article className="reference-document-group" key={key}>
          <header>
            <h5>{sections[0].title || sections[0].documentId}</h5>
            <small>
              {sections[0].documentId} · {text.sourceVersion}{" "}
              {sections[0].documentVersion}
            </small>
          </header>
          {sections.map((reference) => (
            <div className="reference-row" key={referenceKey(reference)}>
              <button
                data-control="reference-row-open"
                type="button"
                aria-label={`${reference.title || reference.documentId} ${reference.section || reference.documentVersion}`}
                onClick={() => onOpen(reference, visibleReferences)}
              >
                {reference.section || text.wholeDocument}
                <small>
                  {[
                    reference.aspect,
                    reference.informationType,
                    reference.status,
                  ]
                    .filter((value): value is string => Boolean(value))
                    .map(label)
                    .join(" · ")}
                </small>
                <small>
                  {[...(usageByKey.get(referenceKey(reference)) ?? [])]
                    .map(label)
                    .join(" · ")}
                </small>
              </button>
              {onUsage && (
                <span>
                  <button
                    data-control="reference-mark-used"
                    type="button"
                    disabled={disabled}
                    aria-label={`${text.use}: ${reference.title} ${reference.section ?? ""}`}
                    onClick={() => onUsage(reference, "used")}
                  >
                    {text.use}
                  </button>
                  <button
                    data-control="reference-mark-excluded"
                    type="button"
                    disabled={disabled}
                    aria-label={`${text.exclude}: ${reference.title} ${reference.section ?? ""}`}
                    onClick={() => onUsage(reference, "excluded")}
                  >
                    {text.exclude}
                  </button>
                </span>
              )}
            </div>
          ))}
        </article>
      ))}
      {groups.length > 0 && (
        <nav className="reference-pagination" aria-label={text.referencePages}>
          <button
            type="button"
            data-control="reference-list-previous"
            disabled={currentPage === 0}
            onClick={() => setPage(currentPage - 1)}
          >
            {text.previousPage}
          </button>
          <span role="status">
            {text.page} {currentPage + 1} / {totalPages} · {groups.length}{" "}
            {text.documents}
          </span>
          <button
            type="button"
            data-control="reference-list-next"
            disabled={currentPage + 1 === totalPages}
            onClick={() => setPage(currentPage + 1)}
          >
            {text.nextPage}
          </button>
        </nav>
      )}
    </section>
  );
}
