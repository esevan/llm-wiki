import { createPortal } from "react-dom";
import { taskTitleLength, validTaskTitle } from "../../services/taskTitle";
import { useEffect, useId, useMemo, useRef, useState } from "react";
import { DraftVersionControls } from "../../components/DraftVersionControls";
import {
  ReferenceViewer,
  type ReferenceDocument,
} from "../../components/ReferenceViewer";
import { referenceKey } from "../../components/referenceIdentity";
import {
  ContentRevisionTransition,
  type ContentTransitionCause,
} from "../../components/ContentRevisionTransition";
import type {
  ExactReferenceBinding,
  PreviewComparison,
  ReferenceWorkspace,
  WorkPreviewFields,
  WorkPreviewVersion,
} from "../../types/taskWorkbench";
import { ReferenceList } from "./ReferenceList";
import { useReferenceWorkbenchText } from "./referenceWorkbenchText";

const fields = [
  "title",
  "description",
  "background",
  "goal",
  "scope",
  "nonGoals",
  "constraints",
  "completionCriteria",
  "initialApproach",
] as const;
const arrayFields = new Set<string>([
  "constraints",
  "completionCriteria",
  "initialApproach",
]);
const busy = (status?: string) =>
  ["queued", "running", "retryable"].includes(status ?? "");
export interface ReferenceAwarePreviewProps {
  readOnly?: boolean;
  workspace?: ReferenceWorkspace;
  referenceContainer?: HTMLElement | null;
  onGenerate: () => Promise<void>;
  onInvestigate: () => Promise<void>;
  onRestore: (version: WorkPreviewVersion) => Promise<void>;
  onSelect: (version: number) => Promise<WorkPreviewVersion>;
  onCompare: (left: number, right: number) => Promise<PreviewComparison>;
  onSave: (
    base: WorkPreviewVersion,
    fields: WorkPreviewFields,
  ) => Promise<void>;
  onApply: (version: WorkPreviewVersion) => Promise<void>;
  onOpenReference: (
    binding: ExactReferenceBinding,
    version: number,
  ) => Promise<ReferenceDocument>;
  onUsage?: (
    binding: ExactReferenceBinding,
    kind: "used" | "excluded",
  ) => Promise<void>;
  onDirtyChange?: (dirty: boolean) => void;
}
export function ReferenceAwarePreview(props: ReferenceAwarePreviewProps) {
  const text = useReferenceWorkbenchText();
  const titleErrorId = useId();
  const { workspace, onDirtyChange, onSelect } = props;
  const current = workspace?.preview?.current;
  const [selected, setSelected] = useState<number>();
  const [historical, setHistorical] = useState<WorkPreviewVersion>();
  const [edit, setEdit] = useState<{
    base: WorkPreviewVersion;
    fields: WorkPreviewFields;
  }>();
  const [comparison, setComparison] = useState<PreviewComparison>();
  const [pending, setPending] = useState(false);
  const pendingRef = useRef(false);
  const [loadingVersion, setLoadingVersion] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [cause, setCause] = useState<ContentTransitionCause>();
  const [viewer, setViewer] = useState<{
    binding: ExactReferenceBinding;
    version: number;
    references: ExactReferenceBinding[];
  }>();
  const chosen = edit?.base ?? (selected !== undefined ? historical : current);
  const isHistorical =
    chosen?.version !== workspace?.preview?.currentVersion && !edit;
  const shownFields = edit?.fields ?? chosen?.fields;
  const shownTitle = shownFields?.title ?? shownFields?.description ?? "";
  const titleInvalid = !validTaskTitle(shownTitle);
  const generationBusy = busy(workspace?.generation?.status);
  const investigation = workspace?.investigations?.at(-1);
  const sourceReferences = useMemo(() => {
    const references = new Map<string, ExactReferenceBinding>();
    for (const reference of [
      ...(chosen?.references ?? []),
      ...(!isHistorical
        ? (workspace?.findings?.flatMap((finding) => finding.references) ?? [])
        : []),
    ])
      references.set(referenceKey(reference), reference);
    return [...references.values()];
  }, [chosen?.references, isHistorical, workspace?.findings]);
  useEffect(() => {
    onDirtyChange?.(Boolean(edit) || pending);
    return () => onDirtyChange?.(false);
  }, [edit, pending, onDirtyChange]);
  useEffect(() => {
    if (selected === undefined) return;
    let cancelled = false;
    setLoadingVersion(true);
    setError("");
    void onSelect(selected)
      .then((version) => {
        if (!cancelled) setHistorical(version);
      })
      .catch(() => {
        if (!cancelled) setError(text.actionFailed);
      })
      .finally(() => {
        if (!cancelled) setLoadingVersion(false);
      });
    return () => {
      cancelled = true;
    };
  }, [selected, onSelect, text.actionFailed]);
  const act = async (action: () => Promise<void>, success?: string) => {
    if (pendingRef.current) return;
    pendingRef.current = true;
    setPending(true);
    setError("");
    setNotice("");
    try {
      await action();
      if (success) setNotice(success);
    } catch {
      setError(text.actionFailed);
    } finally {
      pendingRef.current = false;
      setPending(false);
    }
  };
  const compare = (left: number, right: number) =>
    void act(async () => {
      setComparison(await props.onCompare(left, right));
    });
  const renderFields = (keys: ReadonlyArray<(typeof fields)[number]>) => {
    if (!chosen || !shownFields) return null;
    return (
          <dl className="reference-aware-fields">
            {keys.map((key) => (
              <div key={key} className={`preview-field preview-field-${key}`}>
                <dt>{key === "title" ? text.taskTitle : !edit && key === "goal" ? text.goalReading : !edit && key === "completionCriteria" ? text.completionReading : text[key]}</dt>
                <dd>
                  {key === "title" && titleInvalid && !isHistorical && <p id={titleErrorId} role="alert">{text.titleLimit} ({taskTitleLength(shownTitle)}/70)</p>}
                  {key === "title" && edit && !titleInvalid && <small>{taskTitleLength(shownTitle)}/70</small>}
                  {edit ? (
                    <textarea
                      data-control="reference-preview-field"
                      aria-label={`${text.edit}: ${key === "title" ? text.taskTitle : text[key]}`}
                      aria-invalid={key === "title" && titleInvalid}
                      aria-describedby={key === "title" && titleInvalid ? titleErrorId : undefined}
                      value={
                        Array.isArray(shownFields[key])
                          ? shownFields[key].join("\n")
                          : key === "title" ? shownTitle : shownFields[key]
                      }
                      onChange={(event) => {
                        const value = event.target.value;
                        setEdit(
                          (previous) =>
                            previous && {
                              ...previous,
                              fields: {
                                ...previous.fields,
                                [key]: arrayFields.has(key)
                                  ? value.split("\n")
                                  : value,
                              },
                            },
                        );
                      }}
                    />
                  ) : (
                    <ContentRevisionTransition
                      entityKey={`${chosen.previewId}:${key}`}
                      revision={chosen.version}
                      variant="body"
                      cause={cause}
                      historical={isHistorical}
                      renderContent={key === "completionCriteria" ? () => <ul className="preview-completion-list">{shownFields.completionCriteria.map((criterion, index) => <li key={index}>{criterion}</li>)}</ul> : undefined}
                    >
                      {(Array.isArray(shownFields[key])
                        ? shownFields[key].join("\n")
                        : key === "title" ? shownTitle : shownFields[key]) || text.empty}
                    </ContentRevisionTransition>
                  )}
                </dd>
              </div>
            ))}
          </dl>
    );
  };
  const evidence = chosen ? (<section className="preview-evidence-card" aria-label={text.evidence}>
            <h4>{text.evidence} <span>{sourceReferences.length}</span></h4>
            {!sourceReferences.length && <p>{text.emptyReferences}</p>}
            <div className="preview-evidence-summaries">{sourceReferences.slice(0, 4).map((reference) => <button type="button" data-control="preview-evidence-open" data-record-id={referenceKey(reference)} aria-label={`${text.evidence}: ${reference.title || text.references} ${reference.section || ""}`} key={referenceKey(reference)} disabled={pending} onClick={() => setViewer({ binding: reference, references: sourceReferences, version: chosen!.version })}><strong>{reference.title || text.references}</strong>{reference.excerpt && <q>{reference.excerpt}</q>}<small>{text.references} ↗</small></button>)}</div>
            <details className="preview-reference-tools" data-control="preview-reference-tools"><summary>{text.references}</summary>          <ReferenceList
            key={`${chosen.previewId}:${isHistorical ? chosen.version : "current"}`}
            references={sourceReferences}
            interactions={workspace?.interactions?.filter(
              (fact) => fact.contextRevision === chosen.contextRevision,
            )}
            disabled={pending}
            onOpen={(binding, references) =>
              setViewer({ binding, references, version: chosen.version })
            }
            onUsage={
              !props.readOnly && !isHistorical && props.onUsage
                ? (reference, kind) =>
                    void act(() => props.onUsage!(reference, kind))
                : undefined
            }
          />
</details>
        <div className="reference-aware-preview-actions">
          <button
            data-control="reference-preview-generate"
            type="button"
            disabled={Boolean(props.readOnly) || pending || generationBusy}
            onClick={() => void act(props.onGenerate)}
          >
            {text.generate}
          </button>
          {current && (
            <button
              data-control="reference-preview-investigate"
              type="button"
              disabled={Boolean(props.readOnly) || pending || busy(investigation?.state)}
              onClick={() => void act(props.onInvestigate)}
            >
              {text.investigate}
            </button>
          )}
        </div>
      {(workspace?.retrievalOutcome || investigation || Boolean(workspace?.generationDiagnostics?.length)) && <details className="reference-status" data-control="reference-preview-search-details">
        <summary>{text.searchDetails}</summary>
        {workspace?.retrievalOutcome && <p>{workspace.retrievalOutcome === "unverified" ? text.unverifiedRetrieval : workspace.retrievalOutcome === "not_needed" ? text.notNeeded : workspace.retrievalOutcome === "no_suitable_result" ? text.noResults : text.results}</p>}
        {investigation && ["no_suitable_result", "not_needed"].includes(investigation.state) && <p>{investigation.state === "not_needed" ? text.notNeeded : text.noResults}</p>}
        {Boolean(workspace?.generationDiagnostics?.length) && <ul>{workspace!.generationDiagnostics!.map((diagnostic, index) => <li key={index}>{diagnostic}</li>)}</ul>}
      </details>}

          </section>) : null;
  return (
    <section className="reference-aware-preview" aria-label={text.title} aria-busy={pending || generationBusy || loadingVersion}>
      <header className="reference-aware-preview-header">
        <div>
          <small className="preview-document-label">{text.title}</small>
          <h3>{shownTitle || text.title}</h3>
        </div>

      </header>
      {generationBusy && <p role="status">{text.generating}</p>}
      {workspace?.generation?.status === "failed" && (
        <p role="alert">{text.failed}</p>
      )}
      {workspace?.generationStatus === "partial" && <p className="reference-status" data-control="reference-preview-partial">{text.partial}</p>}
      {workspace?.retrievalOutcome === "unverified" && <p className="reference-status" role="alert">{text.unverifiedRetrieval}</p>}
      {investigation && !["no_suitable_result", "not_needed"].includes(investigation.state) && (
        <p role={investigation.state === "failed" ? "alert" : "status"} className="reference-status">
          {busy(investigation.state) ? text.optionalRunning : investigation.state === "failed" ? text.optionalFailed : investigation.state === "changed_context" ? text.changed : text.optionalDone}
        </p>
      )}

      {(error || notice) && (
        <p role={error ? "alert" : "status"}>{error || notice}</p>
      )}
      {loadingVersion && <p role="status">{text.loading}</p>}
      {edit && current?.version !== edit.base.version && (
        <p role="status">{text.queued}</p>
      )}
      {chosen && shownFields && (
        <>
          {isHistorical && (
            <p className="reference-historical">{text.historical}</p>
          )}
          {!isHistorical &&
            workspace?.findings
              ?.filter((finding) => finding.priority === "critical")
              .map((finding, index) => (
                <div
                  role="alert"
                  className="reference-critical"
                  key={finding.id ?? index}
                >
                  <strong>{text.critical}</strong>
                  <p>{finding.summary}</p>
                </div>
              ))}
          {shownFields.title === undefined && <p role="status">{text.legacyTitle}</p>}
          {renderFields(fields.filter(key => ["description", "goal", "completionCriteria"].includes(key) || (key === "title" && Boolean(edit))))}
          <p className="reference-checklist-hint">{text.checklistHint}</p>
          <details className="reference-preview-details" data-control="reference-preview-more" open={edit ? true : undefined}>
            <summary>{text.moreFields}</summary>
            {renderFields(fields.filter(key => !["title", "description", "goal", "completionCriteria"].includes(key)))}
          </details>
          <details data-control="reference-preview-assumptions">
            <summary>
              {text.assumptions} (
              {(chosen.assumptions ?? shownFields.assumptions ?? []).length})
            </summary>
            <ul>
              {(chosen.assumptions ?? shownFields.assumptions ?? []).map(
                (assumption) => (
                  <li key={assumption.id}>
                    {assumption.text}
                    <small className="reference-assumption-status">
                      {
                        text[
                          assumption.status === "confirmed"
                            ? "assumptionConfirmed"
                            : assumption.status === "rejected"
                              ? "assumptionRejected"
                              : assumption.status === "superseded"
                                ? "assumptionSuperseded"
                                : "assumptionOpen"
                        ]
                      }{" "}
                      · {text[assumption.basis]}
                    </small>
                  </li>
                ),
              )}
            </ul>
          </details>
          <div className="reference-edit-actions">
            {edit ? (
              <>
                <button
                  data-control="reference-preview-save"
                  type="button"
                  disabled={Boolean(props.readOnly) || pending || titleInvalid}
                  onClick={() =>
                    void act(async () => {
                      await props.onSave(edit.base, edit.fields);
                      setEdit(undefined);
                      setSelected(undefined);
                      setCause(undefined);
                    }, text.saved)
                  }
                >
                  {text.save}
                </button>
                <button
                  data-control="reference-preview-cancel"
                  type="button"
                  disabled={pending}
                  onClick={() => setEdit(undefined)}
                >
                  {text.cancel}
                </button>
              </>
            ) : (
              !isHistorical && (
                <>
                  <button
                    data-control="reference-preview-edit"
                    type="button"
                    disabled={Boolean(props.readOnly) || pending || loadingVersion}
                    onClick={() => {
                      setEdit({
                        base: chosen,
                        fields: { ...structuredClone(chosen.fields), title: chosen.fields.title ?? chosen.fields.description },
                      });
                      setCause(undefined);
                    }}
                  >
                    {text.edit}
                  </button>
                  <button
                    data-control="reference-preview-apply"
                    className="primary"
                    type="button"
                    disabled={Boolean(props.readOnly) || pending || generationBusy || loadingVersion || titleInvalid}
                    onClick={() =>
                      void act(async () => {
                        await props.onApply(chosen);
                        setCause("preview_adopted");
                      }, text.applied)
                    }
                  >
                    {text.apply}
                  </button>
                </>
              )
            )}
          </div>
          <DraftVersionControls
            versions={workspace?.preview?.versions ?? []}
            restoreDisabled={props.readOnly}
            currentVersion={workspace?.preview?.currentVersion}
            selected={chosen.version}
            comparison={comparison}
            disabled={pending || Boolean(edit) || loadingVersion}
            onSelect={(version) => {
              setSelected(
                version === workspace?.preview?.currentVersion
                  ? undefined
                  : version,
              );
              setCause(undefined);
            }}
            onCompare={compare}
            onRestore={() =>
              void act(async () => {
                await props.onRestore(chosen);
                setSelected(undefined);
                setCause("version_restore");
              })
            }
          />
          {props.referenceContainer ? createPortal(evidence, props.referenceContainer) : evidence}
          <ReferenceViewer
            open={Boolean(viewer)}
            binding={viewer?.binding}
            references={viewer?.references ?? []}
            read={(binding) => props.onOpenReference(binding, viewer!.version)}
            onNavigate={(binding) =>
              setViewer((current) => current && { ...current, binding })
            }
            onClose={() => setViewer(undefined)}
          />
        </>
      )}
    </section>
  );
}
