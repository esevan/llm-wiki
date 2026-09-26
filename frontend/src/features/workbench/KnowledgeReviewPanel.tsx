import { useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState, type Ref } from "react";
import { DraftVersionControls } from "../../components/DraftVersionControls";
import { ContentRevisionTransition, type ContentTransitionCause } from "../../components/ContentRevisionTransition";
import { KnowledgeMarkdown } from "../../components/KnowledgeMarkdown";
import { ReferenceViewer, type ReferenceDocument } from "../../components/ReferenceViewer";
import { taskClient } from "../../services/taskClient";
import type {
  ExactReferenceBinding,
  KnowledgeArchiveProposal,
  KnowledgeIdea,
  KnowledgeReviewProjection,
  KnowledgeSourceRef,
  KnowledgeVersion,
  PreviewComparison,
  WorkPreviewFields,
  WorkPreviewVersion,
} from "../../types/taskWorkbench";
import { useTaskWorkbenchText } from "./taskWorkbenchText";
import "./knowledge-review.css";

export interface KnowledgeArchivePrepareRequest {
  operationId: string;
  taskId: string;
  knowledgeRevision: number;
  expectedKnowledgeContentHash: string;
  expectedGenerationSnapshotHash: string;
  selectedIdeaRevisionIds: Array<{ id: string; revision: number }>;
  locale: "en" | "ko";
}
export interface KnowledgeArchivePublishRequest {
  operationId: string;
  proposalId: string;
  proposalVersion: number;
  proposalHash: string;
}
export interface KnowledgeArchiveOperation {
  operationId?: string;
  state: "writing" | "index_pending" | "index_failed" | "complete" | "conflict" | "repair_required" | "compensated";
  error?: string;
  [key: string]: unknown;
}
export interface KnowledgeArchiveOrganizeRequest {
  operationId: string;
  documentId: string;
  expectedRevision: string;
  intent: "move" | "rename" | "withdraw" | "repair";
  requestedPath?: string;
}
export interface KnowledgeArchiveRecoverRequest { operationId: string; choice: "finish" | "compensate" }

interface Props {
  taskId: string;
  completed: boolean;
  generationQueued?: boolean;
  refreshKey?: string | number;
  preferredRevision?: number;
  onGenerate: () => Promise<void>;
  onDirtyChange?: (dirty: boolean) => void;
  onPrepareArchive?: (request: KnowledgeArchivePrepareRequest) => Promise<KnowledgeArchiveProposal>;
  onOrganizeArchive?: (request: KnowledgeArchiveOrganizeRequest) => Promise<KnowledgeArchiveProposal>;
  onPublishArchive?: (request: KnowledgeArchivePublishRequest) => Promise<KnowledgeArchiveOperation>;
  onArchiveStatus?: (operationId: string) => Promise<KnowledgeArchiveOperation>;
  onRetryArchive?: (operationId: string) => Promise<KnowledgeArchiveOperation>;
  onRecoverArchive?: (request: KnowledgeArchiveRecoverRequest) => Promise<KnowledgeArchiveOperation>;
  ref?: Ref<KnowledgeReviewPanelHandle>;
}
export interface KnowledgeReviewPanelHandle { save: () => Promise<boolean>; discard: () => void; focus: () => void }

const operationId = () => globalThis.crypto?.randomUUID?.() ?? `knowledge-${Date.now()}-${Math.random().toString(16).slice(2)}`;
const quoteMarkdown = (reference: ExactReferenceBinding) => [
  `# ${reference.title}`,
  "",
  reference.section ? `**${reference.section}**` : "",
  "",
  ...(reference.excerpt ?? "").split("\n").map((line: string) => `> ${line}`),
].filter(Boolean).join("\n");
const bindingKey = (source: KnowledgeSourceRef) => `${source.type}\u0000${source.id}\u0000${source.revision}\u0000${source.locator}\u0000${source.quote}`;
const viewerKey = (binding: ExactReferenceBinding) => `${binding.documentId}\u0000${binding.documentVersion}\u0000${binding.section ?? ""}`;
const toBinding = (source: KnowledgeSourceRef): ExactReferenceBinding => ({
  documentId: source.id,
  documentVersion: source.revision,
  section: source.locator,
  title: source.type.replaceAll("_", " "),
  excerpt: source.quote,
  role: source.type,
});
const versionSources = (version: KnowledgeVersion) => {
  const article = version.result.article;
  const sources = [
    ...(article?.claimBindings ?? []).flatMap(binding => binding.sourceRefs),
    ...(article?.assumptionBindings ?? []).flatMap(binding => binding.sourceRefs),
    ...(article?.applicability.sourceRefs ?? []),
    ...(version.result.qualityFindings ?? []).flatMap(finding => finding.sourceRefs),
  ];
  return [...new Map(sources.map(source => [bindingKey(source), source])).values()];
};
const toPreview = (version: KnowledgeVersion, current?: number): WorkPreviewVersion => {
  const article = version.result.article;
  const applicability = version.applicability;
  return {
    previewId: `knowledge:${version.revision}`,
    version: version.revision,
    current: version.revision === current,
    derivationKind: version.derivationKind,
    derivedFromVersion: version.derivedFromRevision,
    fields: {
      description: version.bodyMarkdown,
      background: applicability.summary,
      goal: (article?.finalOutcomes ?? []).map(outcome => outcome.statement).join("\n"),
      scope: applicability.helpsWith.join("\n"),
      nonGoals: applicability.exclusions.join("\n"),
      constraints: applicability.conditions,
      completionCriteria: applicability.representativeQuestions,
      initialApproach: [],
      assumptions: [],
    },
    references: versionSources(version).map(toBinding),
    createdAt: version.createdAt,
    contentHash: version.contentHash,
  };
};
const compare = (left: WorkPreviewVersion, right: WorkPreviewVersion): PreviewComparison => {
  const keys = Object.keys(left.fields) as Array<keyof WorkPreviewFields>;
  return {
    left,
    right,
    fields: keys.map(key => ({
      key,
      state: JSON.stringify(left.fields[key]) === JSON.stringify(right.fields[key]) ? "unchanged" : "changed",
    })),
  };
};

const isKnowledgeProjection = (value: unknown): value is KnowledgeReviewProjection => {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<KnowledgeReviewProjection>;
  return Boolean(candidate.pointers && typeof candidate.pointers === "object")
    && Array.isArray(candidate.versions)
    && Array.isArray(candidate.ideas);
};

export function KnowledgeReviewPanel({ taskId, completed, generationQueued, refreshKey, preferredRevision, onGenerate, onDirtyChange, onPrepareArchive, onOrganizeArchive, onPublishArchive, onArchiveStatus, onRetryArchive, onRecoverArchive, ref }: Props) {
  const text = useTaskWorkbenchText().knowledgeReview;
  const [projection, setProjection] = useState<KnowledgeReviewProjection>();
  const [selected, setSelected] = useState<number>();
  const [comparison, setComparison] = useState<PreviewComparison>();
  const [body, setBody] = useState("");
  const [savedBody, setSavedBody] = useState("");
  const [editing, setEditing] = useState(false);
  const [selectedIdeas, setSelectedIdeas] = useState<Set<string>>(new Set());
  const [source, setSource] = useState<ExactReferenceBinding>();
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<"generate" | "edit" | "restore" | "prepare" | "publish">();
  const [error, setError] = useState("");
  const [status, setStatus] = useState("");
  const [proposal, setProposal] = useState<KnowledgeArchiveProposal>();
  const [publicationOperation, setPublicationOperation] = useState<KnowledgeArchiveOperation>();
  const [proposalOperationId, setProposalOperationId] = useState<string>();
  const [publicationOperationId, setPublicationOperationId] = useState<string>();
  const [organizeIntent, setOrganizeIntent] = useState<KnowledgeArchiveOrganizeRequest["intent"]>("rename");
  const [requestedPath, setRequestedPath] = useState("");
  const [appliedTransition, setAppliedTransition] = useState<{ revision: number; cause: ContentTransitionCause }>();
  const pendingGenerationRevision = useRef<number | undefined>(undefined);
  const observedGenerationQueue = useRef(false);
  const loadSequence = useRef(0);
  const root = useRef<HTMLElement>(null);
  const dirty = editing && body !== savedBody;
  const dirtyRef = useRef(dirty);
  dirtyRef.current = dirty;
  useEffect(() => { onDirtyChange?.(dirty); return () => onDirtyChange?.(false); }, [dirty, onDirtyChange]);

  const load = useCallback(async (preferredRevision?: number) => {
    const sequence = ++loadSequence.current;
    setLoading(true);
    try {
      const next = await taskClient.knowledge(taskId);
      if (sequence !== loadSequence.current) return;
      if (!isKnowledgeProjection(next)) throw new Error(text.invalidResponse);
      const generatedRevision = pendingGenerationRevision.current !== undefined && next.pointers.currentPrivateRevision !== undefined && next.pointers.currentPrivateRevision !== pendingGenerationRevision.current
        ? next.pointers.currentPrivateRevision
        : undefined;
      if (generatedRevision !== undefined) {
        setAppliedTransition({ revision: generatedRevision, cause: "automatic_apply" });
        pendingGenerationRevision.current = undefined;
      }
      setProjection(next);
      setError("");
      setSelected(current => {
        if (dirtyRef.current && current && next.versions.some(version => version.revision === current)) return current;
        if (preferredRevision && next.versions.some(version => version.revision === preferredRevision)) return preferredRevision;
        if (generatedRevision !== undefined) return generatedRevision;
        if (current && next.versions.some(version => version.revision === current)) return current;
        return next.pointers.currentPrivateRevision ?? next.versions.at(-1)?.revision;
      });
    } catch (cause) {
      if (sequence === loadSequence.current) setError(String(cause instanceof Error ? cause.message : cause));
    } finally {
      if (sequence === loadSequence.current) setLoading(false);
    }
  }, [taskId, text.invalidResponse]);

  useEffect(() => {
    const sequences = loadSequence;
    void load(preferredRevision);
    return () => { sequences.current += 1; };
  }, [load, preferredRevision, refreshKey]);
  const version = projection?.versions.find(candidate => candidate.revision === selected);
  const currentRevision = projection?.pointers.currentPrivateRevision;
  const publishedRevision = projection?.pointers.publishedRevision;
  useEffect(() => {
    if (pendingGenerationRevision.current === undefined) return;
    if (generationQueued) { observedGenerationQueue.current = true; return; }
    if (observedGenerationQueue.current && currentRevision === pendingGenerationRevision.current) {
      pendingGenerationRevision.current = undefined;
      observedGenerationQueue.current = false;
    }
  }, [currentRevision, generationQueued]);
  useEffect(() => {
    if (!version || dirty) return;
    setBody(version.bodyMarkdown);
    setSavedBody(version.bodyMarkdown);
    setEditing(false);
    setProposal(undefined);
    setPublicationOperation(undefined);
    setProposalOperationId(undefined);
    setPublicationOperationId(undefined);
    setSelectedIdeas(new Set());
  }, [dirty, version]);

  useEffect(() => {
    if (!version || !projection?.archive) return;
    const appliesToVersion = (candidate: KnowledgeArchiveProposal) => {
      const input = candidate.input && typeof candidate.input === "object" ? candidate.input as Record<string, unknown> : undefined;
      return input?.knowledgeRevision === undefined
        ? version.revision === currentRevision
        : input.knowledgeRevision === version.revision;
    };
    const recoverableStates = new Set(["writing", "index_pending", "index_failed", "conflict", "repair_required"]);
    const savedOperation = projection.archive.operations.find(candidate => recoverableStates.has(candidate.state));
    const savedProposal = savedOperation?.proposalId
      ? projection.archive.proposals.find(candidate => candidate.proposalId === savedOperation.proposalId)
      : projection.archive.proposals.find(candidate => appliesToVersion(candidate) && candidate.state === "review_needed");
    if (savedProposal) {
      setProposal(savedProposal);
      setProposalOperationId(savedProposal.operationId ?? savedProposal.proposalId);
    }
    if (savedOperation) {
      setPublicationOperation(savedOperation);
      setPublicationOperationId(savedOperation.operationId);
    }
  }, [currentRevision, projection, version]);

  useEffect(() => {
    if (!publicationOperationId || !onArchiveStatus || !publicationOperation || !["writing", "index_pending"].includes(publicationOperation.state)) return;
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void onArchiveStatus(publicationOperationId).then(next => {
        if (cancelled) return;
        setPublicationOperation(next);
        if (next.state === "complete") void load(version?.revision);
      }).catch(cause => { if (!cancelled) setError(`${text.operationFailed} ${String(cause instanceof Error ? cause.message : cause)}`); });
    }, 1000);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [load, onArchiveStatus, publicationOperation, publicationOperationId, text.operationFailed, version?.revision]);

  const previews = useMemo(() => projection?.versions.map(item => toPreview(item, currentRevision)) ?? [], [currentRevision, projection]);
  const references = useMemo(() => version ? versionSources(version).map(toBinding) : [], [version]);
  const artifactDocuments = useMemo(() => new Map((proposal?.artifacts ?? []).filter(artifact => artifact.bytes !== null).map(artifact => {
    const binding: ExactReferenceBinding = {
      documentId: artifact.documentId,
      documentVersion: `${proposal!.proposalId}:${proposal!.proposalVersion}:${artifact.sha256 ?? proposal!.proposalHash}`,
      section: artifact.path ?? artifact.kind,
      title: artifact.path ?? artifact.documentId,
      path: artifact.path ?? undefined,
      excerpt: artifact.bytes!.slice(0, 240),
      role: "publication_artifact",
    };
    return [viewerKey(binding), { ...binding, markdown: artifact.bytes! } as ReferenceDocument];
  })), [proposal]);
  const linkDocuments = useMemo(() => new Map((proposal?.referenceLinks ?? []).map(link => {
    const binding: ExactReferenceBinding = { documentId: link.documentId, documentVersion: link.documentVersion, section: link.section, title: link.path, path: link.path, excerpt: link.rationale, role: link.role };
    const markdown = `# ${link.path}\n\n**${link.role}** · revision \`${link.documentVersion}\`${link.section ? ` · ${link.section}` : ""}\n\n${link.rationale}`;
    return [viewerKey(binding), { ...binding, markdown } as ReferenceDocument];
  })), [proposal]);
  const viewerReferences = useMemo(() => [...references, ...artifactDocuments.values(), ...linkDocuments.values()], [artifactDocuments, linkDocuments, references]);
  const ideas = useMemo(() => projection?.ideas.filter(idea => idea.knowledgeRevision === version?.revision) ?? [], [projection, version]);
  const selectVersion = (revision: number) => {
    if (dirty) { setStatus(text.unsaved); return; }
    setSelected(revision); setComparison(undefined); setStatus("");
  };
  const restore = async (revision: number) => {
    if (!currentRevision || busy || dirty) return;
    pendingGenerationRevision.current = undefined;
    observedGenerationQueue.current = false;
    setBusy("restore"); setError(""); setStatus("");
    try {
      const restored = await taskClient.restoreKnowledge(taskId, revision, currentRevision);
      setAppliedTransition({ revision: restored.draftRevision, cause: "version_restore" });
      await load(restored.draftRevision);
      setStatus(text.restored);
    } catch (cause) { setError(`${text.restoreFailed} ${String(cause instanceof Error ? cause.message : cause)}`); }
    finally { setBusy(undefined); }
  };
  const save = useCallback(async () => {
    if (!version || version.revision !== currentRevision || !version.generationSnapshotHash || busy || !dirty) return !dirty;
    setBusy("edit"); setError(""); setStatus(text.saving);
    pendingGenerationRevision.current = undefined;
    observedGenerationQueue.current = false;
    try {
      const edited = await taskClient.correctKnowledge(taskId, version.revision, version.contentHash, version.generationSnapshotHash, body);
      dirtyRef.current = false;
      setSavedBody(body); setEditing(false);
      await load(edited.draftRevision);
      setStatus(text.edited);
      return true;
    } catch (cause) { setError(`${text.saveFailed} ${String(cause instanceof Error ? cause.message : cause)}`); }
    finally { setBusy(undefined); }
    return false;
  }, [body, busy, currentRevision, dirty, load, taskId, text.edited, text.saveFailed, text.saving, version]);
  useImperativeHandle(ref, () => ({
    save,
    discard: () => { setBody(savedBody); setEditing(false); setStatus(""); setError(""); },
    focus: () => root.current?.focus({ preventScroll: true }),
  }), [savedBody, save]);
  const generate = async () => {
    if (busy || !completed) return;
    setBusy("generate"); setError(""); setStatus(text.generating);
    pendingGenerationRevision.current = currentRevision;
    observedGenerationQueue.current = false;
    try { await onGenerate(); }
    catch (cause) { pendingGenerationRevision.current = undefined; setError(String(cause instanceof Error ? cause.message : cause)); setStatus(""); }
    finally { setBusy(undefined); }
  };
  const preparePublication = async () => {
    if (!version?.generationSnapshotHash || !onPrepareArchive || busy || dirty) return;
    setBusy("prepare"); setError(""); setStatus(text.preparingPublication);
    try {
      const nextOperationId = operationId();
      const next = await onPrepareArchive({
        operationId: nextOperationId, taskId, knowledgeRevision: version.revision,
        expectedKnowledgeContentHash: version.contentHash,
        expectedGenerationSnapshotHash: version.generationSnapshotHash,
        selectedIdeaRevisionIds: ideas.filter(idea => selectedIdeas.has(`${idea.id}:${idea.revision}`)).map(idea => ({ id: idea.id, revision: idea.revision })),
        locale: document.documentElement.lang.startsWith("ko") ? "ko" : "en",
      });
      setProposal(next); setProposalOperationId(nextOperationId); setPublicationOperation(undefined); setPublicationOperationId(undefined); setStatus("");
    } catch (cause) { setError(`${text.operationFailed} ${String(cause instanceof Error ? cause.message : cause)}`); setStatus(""); }
    finally { setBusy(undefined); }
  };
  const prepareOrganization = async () => {
    const documentId = projection?.pointers.publicationDocumentId;
    const expectedRevision = projection?.pointers.publicationContentHash;
    if (!documentId || !expectedRevision || !onOrganizeArchive || busy || dirty) return;
    setBusy("prepare"); setError(""); setStatus(text.preparingPublication);
    try {
      const nextOperationId = operationId();
      const next = await onOrganizeArchive({
        operationId: nextOperationId, documentId, expectedRevision, intent: organizeIntent,
        requestedPath: organizeIntent === "withdraw" ? undefined : requestedPath.trim() || undefined,
      });
      setProposal(next); setProposalOperationId(nextOperationId); setPublicationOperation(undefined); setPublicationOperationId(undefined); setStatus("");
    } catch (cause) { setError(`${text.operationFailed} ${String(cause instanceof Error ? cause.message : cause)}`); setStatus(""); }
    finally { setBusy(undefined); }
  };
  const publish = async () => {
    if (!proposal || !proposalOperationId || !onPublishArchive || busy) return;
    setBusy("publish"); setError(""); setStatus(text.publishing);
    try {
      const publishOperationId = operationId();
      const operation = await onPublishArchive({ operationId: publishOperationId, proposalId: proposal.proposalId, proposalVersion: proposal.proposalVersion, proposalHash: proposal.proposalHash });
      setPublicationOperation(operation); setPublicationOperationId(operation.operationId ?? publishOperationId); setStatus("");
      if (operation.state === "complete") await load(version?.revision);
    } catch (cause) { setError(`${text.operationFailed} ${String(cause instanceof Error ? cause.message : cause)}`); setStatus(""); }
    finally { setBusy(undefined); }
  };
  const retryPublication = async () => {
    if (!publicationOperationId || !onRetryArchive || busy) return;
    setBusy("publish"); setError("");
    try {
      const operation = await onRetryArchive(publicationOperationId);
      setPublicationOperation(operation);
      setPublicationOperationId(operation.operationId ?? publicationOperationId);
      if (operation.state === "complete") await load(version?.revision);
    }
    catch (cause) { setError(`${text.operationFailed} ${String(cause instanceof Error ? cause.message : cause)}`); }
    finally { setBusy(undefined); }
  };
  const recoverPublication = async (choice: "finish" | "compensate") => {
    if (!publicationOperationId || !onRecoverArchive || busy) return;
    setBusy("publish"); setError("");
    try {
      const operation = await onRecoverArchive({ operationId: publicationOperationId, choice });
      setPublicationOperation(operation);
      setPublicationOperationId(operation.operationId ?? publicationOperationId);
      if (operation.state === "complete" || operation.state === "compensated") await load(version?.revision);
    }
    catch (cause) { setError(`${text.operationFailed} ${String(cause instanceof Error ? cause.message : cause)}`); }
    finally { setBusy(undefined); }
  };

  if (loading && !projection) return <section ref={root} className="knowledge-review" aria-label={text.title} tabIndex={-1}><p role="status">{text.loading}</p></section>;
  if (!projection) return <section ref={root} className="knowledge-review" aria-label={text.title} tabIndex={-1}><p role="alert">{text.loadFailed} {error}</p><button type="button" data-control="knowledge-review-retry" onClick={() => void load()}>{text.retry}</button></section>;
  if (!version) return <section ref={root} className="knowledge-review knowledge-review-empty" aria-label={text.title} tabIndex={-1}><header><div><p className="knowledge-review-kicker">{text.privateDraft}</p><h3>{text.title}</h3></div><span className="knowledge-state">{text.notPublished}</span></header><p>{text.empty}</p><button type="button" data-control="task-knowledge-draft" disabled={!completed || generationQueued || busy === "generate"} onClick={() => void generate()}>{busy === "generate" || generationQueued ? text.generating : text.generate}</button>{error && <p role="alert">{error}</p>}</section>;

  const article = version.result.article;
  const outcomes = article?.finalOutcomes ?? [];
  const findings = version.result.qualityFindings ?? [];
  const publicationLocked = Boolean(publicationOperation && ["writing", "index_pending", "complete", "compensated"].includes(publicationOperation.state));
  const proposalConflict = proposal?.outcome === "conflict";
  return <section ref={root} className="knowledge-review" aria-label={text.title} data-content-hash={version.contentHash} tabIndex={-1}>
    <header className="knowledge-review-header">
      <div><p className="knowledge-review-kicker">{text.privateDraft}</p><ContentRevisionTransition as="h3" entityKey={`knowledge:${taskId}`} revision={version.revision} cause={appliedTransition?.revision === version.revision ? appliedTransition.cause : undefined} historical={version.revision !== currentRevision} variant="title">{article?.title ?? version.title}</ContentRevisionTransition><p>{text.articleType}: {article?.type ?? version.articleType}</p></div>
      <div className="knowledge-review-states"><span className={`knowledge-state is-${version.freshness}`}>{version.freshness === "current" ? text.current : text.stale}</span><span className="knowledge-state">{version.revision === currentRevision ? text.currentPrivate : text.historical}</span><span className="knowledge-state">{version.revision === publishedRevision ? text.exactPublished : text.notPublished}</span></div>
    </header>
    <DraftVersionControls versions={previews.map(item => ({ version: item.version, derivationKind: item.derivationKind, createdAt: item.createdAt, derivedFromVersion: item.derivedFromVersion, current: item.current, contentHash: item.contentHash }))} selected={version.revision} currentVersion={currentRevision} comparison={comparison} disabled={Boolean(busy) || dirty} restoreDisabled={version.revision === currentRevision} onSelect={selectVersion} onCompare={(left: number, right: number) => { const a = previews.find(item => item.version === left); const b = previews.find(item => item.version === right); if (a && b) setComparison(compare(a, b)); }} onRestore={(revision: number) => void restore(revision)} />
    <p className="knowledge-compare-hint">{text.compareHint}</p>
    <div className="knowledge-regenerate"><button type="button" data-control="knowledge-version-regenerate" disabled={!completed || generationQueued || Boolean(busy) || dirty} onClick={() => void generate()}>{busy === "generate" || generationQueued ? text.generating : text.regenerate}</button></div>
    {status && <p role="status" className="knowledge-draft-status">{status}</p>}
    {error && <p role="alert" className="knowledge-draft-error">{error}</p>}

    <section className="knowledge-outcomes" aria-labelledby="knowledge-outcomes-title"><h4 id="knowledge-outcomes-title">{text.finalOutcomes}</h4>{outcomes.length ? <ol>{outcomes.map(outcome => <li key={outcome.topicKey}><strong>{outcome.topicKey}</strong><p>{outcome.statement}</p></li>)}</ol> : <p>{text.noFinalOutcomes}</p>}</section>
    <section className="knowledge-article" aria-label={text.articleBody}>
      {editing ? <label className="knowledge-draft-editor"><span>{text.edit}</span><textarea data-control="task-knowledge-draft-body" value={body} onChange={event => setBody(event.target.value)} /></label> : <ContentRevisionTransition as="div" entityKey={`knowledge:${taskId}:body`} revision={version.revision} cause={appliedTransition?.revision === version.revision ? appliedTransition.cause : undefined} historical={version.revision !== currentRevision} variant="body" renderContent={content => <KnowledgeMarkdown>{content}</KnowledgeMarkdown>}>{version.bodyMarkdown}</ContentRevisionTransition>}
      {(version.revision === currentRevision || editing) && <div className="knowledge-review-actions">{editing ? <><button type="button" data-control="knowledge-version-save" disabled={Boolean(busy) || !dirty || version.revision !== currentRevision || !version.generationSnapshotHash} onClick={() => void save()}>{text.save}</button><button type="button" data-control="knowledge-version-cancel" disabled={Boolean(busy)} onClick={() => { setBody(savedBody); setEditing(false); setStatus(""); }}>{text.cancelEdit}</button></> : <button type="button" data-control="knowledge-version-edit" disabled={Boolean(busy)} onClick={() => setEditing(true)}>{text.edit}</button>}</div>}
    </section>

    <section className="knowledge-applicability"><h4>{text.applicability}</h4><p className="knowledge-applicability-summary">{version.applicability.summary}</p><ApplicabilityList title={text.questions} values={version.applicability.representativeQuestions} /><ApplicabilityList title={text.helpsWith} values={version.applicability.helpsWith} /><ApplicabilityList title={text.conditions} values={version.applicability.conditions} /><ApplicabilityList title={text.exclusions} values={version.applicability.exclusions} /></section>

    {(article?.claimBindings.length || article?.assumptionBindings.length) ? <details className="knowledge-provenance" data-control="knowledge-provenance"><summary>{text.provenance} · {references.length}</summary><div className="knowledge-source-list">{references.map(reference => <button type="button" data-control="knowledge-source-open" data-record-id={`${reference.documentId}:${reference.documentVersion}`} key={`${reference.documentId}:${reference.documentVersion}:${reference.section}:${reference.excerpt}`} onClick={() => setSource(reference)}><strong>{reference.title}</strong><span>{reference.section}</span><q>{reference.excerpt}</q><small>{text.openSource}</small></button>)}</div></details> : null}
    {findings.length > 0 && <section className="knowledge-quality"><h4>{text.quality}</h4><ul>{findings.map((finding, index) => <li key={`${finding.kind}:${index}`} className={`is-${finding.severity}`}><strong>{text[finding.severity as "blocking" | "warning" | "info"] ?? finding.severity}</strong><span>{finding.message || finding.kind}</span></li>)}</ul></section>}

    <details className="knowledge-ideas" data-control="knowledge-ideas"><summary>{text.ideas} · {ideas.length}</summary><p>{text.ideasHint}</p>{ideas.length ? <div>{ideas.map(idea => <IdeaCard key={`${idea.id}:${idea.revision}`} idea={idea} selected={selectedIdeas.has(`${idea.id}:${idea.revision}`)} disabled={Boolean(proposal)} onToggle={() => setSelectedIdeas(current => { const next = new Set(current); const key = `${idea.id}:${idea.revision}`; if (next.has(key)) next.delete(key); else next.add(key); return next; })} onSource={item => setSource(toBinding(item))} text={text} />)}</div> : <p>{text.noIdeas}</p>}</details>

    <section className="knowledge-publication">
      <header><div><h4>{text.publicationReview}</h4><p>{onPrepareArchive ? text.preparePublication : text.publicationUnavailable}</p></div><button type="button" data-control="knowledge-archive-prepare" disabled={!onPrepareArchive || Boolean(busy) || dirty || version.freshness !== "current"} onClick={() => void preparePublication()}>{busy === "prepare" ? text.preparingPublication : text.preparePublication}</button></header>
      {projection.pointers.publicationDocumentId && projection.pointers.publicationContentHash && <details className="knowledge-organization" data-control="knowledge-archive-organize">
        <summary>{text.managePublication}</summary>
        <p>{text.currentPublishedPath}: <code>{projection.pointers.publicationPath ?? text.noPath}</code></p>
        <div className="knowledge-organization-controls">
          <label>{text.organizationAction}<select data-control="knowledge-archive-intent" value={organizeIntent} onChange={event => setOrganizeIntent(event.target.value as KnowledgeArchiveOrganizeRequest["intent"])}><option value="move">{text.move}</option><option value="rename">{text.rename}</option><option value="withdraw">{text.withdraw}</option><option value="repair">{text.repair}</option></select></label>
          {organizeIntent !== "withdraw" && <label>{text.requestedPath}<input data-control="knowledge-archive-path" value={requestedPath} onChange={event => setRequestedPath(event.target.value)} /></label>}
          <button type="button" data-control="knowledge-archive-review-organization" disabled={!onOrganizeArchive || Boolean(busy) || dirty} onClick={() => void prepareOrganization()}>{text.reviewOrganization}</button>
        </div>
      </details>}
      {proposal && <div className="knowledge-publication-proposal">
        <p><strong>{proposal.outcome}</strong> {proposal.rationale}</p>
        <h5>{text.proposedFiles}</h5><ul>{proposal.artifacts.map(artifact => {
          const binding = [...artifactDocuments.values()].find(document => document.documentId === artifact.documentId && document.path === (artifact.path ?? undefined));
          const byteCount = artifact.bytes === null ? null : new TextEncoder().encode(artifact.bytes).byteLength;
          return <li key={`${artifact.documentId}:${artifact.path}`}><button type="button" data-control="knowledge-archive-artifact-open" data-record-id={`${artifact.documentId}:${artifact.path ?? "withdrawn"}`} disabled={!binding} onClick={() => binding && setSource(binding)}><code>{artifact.path ?? text.withdrawnPath}</code></button><span>{artifact.kind}{byteCount !== null ? ` · ${byteCount} ${text.bytes}` : ""}</span>{artifact.sha256 && <code>{artifact.sha256}</code>}</li>;
        })}</ul>
        {(proposal.unresolvedConflicts?.length ?? 0) > 0 && <section className="knowledge-publication-conflicts" role="alert"><h5>{text.unresolvedRepairs}</h5><ul>{proposal.unresolvedConflicts!.map(conflict => <li key={`${conflict.path}:${conflict.reason}`}><code>{conflict.path}</code><span>{conflict.reason}</span></li>)}</ul></section>}
        <details data-control="knowledge-archive-proposal-details"><summary>{text.proposedChanges}</summary>
          {proposal.mocPatches.map(patch => <article className="knowledge-moc-patch" key={patch.path}><h6><code>{patch.path}</code></h6><div><section><strong>{text.before}</strong><pre>{patch.before || text.emptyFile}</pre></section><section><strong>{text.after}</strong><pre>{patch.after || text.emptyFile}</pre></section></div></article>)}
          {proposal.referenceLinks.length > 0 && <section className="knowledge-reference-links"><h6>{text.sourceLinks}</h6>{proposal.referenceLinks.map(link => { const binding = [...linkDocuments.values()].find(document => viewerKey(document) === viewerKey({ documentId: link.documentId, documentVersion: link.documentVersion, section: link.section, title: link.path })); return <button type="button" data-control="knowledge-archive-reference-open" data-record-id={`${link.documentId}:${link.documentVersion}:${link.section ?? ""}`} key={`${link.documentId}:${link.documentVersion}:${link.section ?? ""}`} onClick={() => binding && setSource(binding)}><strong>{link.path}{link.section ? `#${link.section}` : ""}</strong><span>{link.rationale}</span><small>{link.role} · {link.documentVersion}</small></button>; })}</section>}
        </details>
        {proposalConflict && <p role="alert">{text.proposalConflict}</p>}
        <button type="button" data-control="knowledge-archive-publish" disabled={!onPublishArchive || Boolean(busy) || dirty || version.freshness !== "current" || proposalConflict || publicationLocked} onClick={() => void publish()}>{busy === "publish" ? text.publishing : text.publishExact}</button>
      </div>}
      {publicationOperation && <div className={`knowledge-publication-state is-${publicationOperation.state}`} role={publicationOperation.state === "conflict" || publicationOperation.state === "repair_required" || publicationOperation.state === "index_failed" ? "alert" : "status"}>
        <strong>{text.publicationState}: {publicationOperation.state}</strong>{publicationOperation.error && <p>{publicationOperation.error}</p>}
        {publicationOperation.state === "index_pending" && <p>{text.indexPending}</p>}
        {publicationOperation.state === "index_failed" && <button type="button" data-control="knowledge-archive-retry-index" disabled={!onRetryArchive || Boolean(busy)} onClick={() => void retryPublication()}>{text.retryIndex}</button>}
        {(publicationOperation.state === "index_failed" || publicationOperation.state === "repair_required") && <div className="knowledge-recovery-actions"><button type="button" data-control="knowledge-archive-recover-finish" disabled={!onRecoverArchive || Boolean(busy)} onClick={() => void recoverPublication("finish")}>{text.finishRecovery}</button><button type="button" data-control="knowledge-archive-recover-compensate" disabled={!onRecoverArchive || Boolean(busy)} onClick={() => void recoverPublication("compensate")}>{text.compensate}</button></div>}
      </div>}
    </section>
    <ReferenceViewer open={Boolean(source)} binding={source} references={viewerReferences} read={async (binding: ExactReferenceBinding) => artifactDocuments.get(viewerKey(binding)) ?? linkDocuments.get(viewerKey(binding)) ?? ({ ...binding, markdown: quoteMarkdown(binding) } as ReferenceDocument)} onClose={() => setSource(undefined)} onNavigate={setSource} />
  </section>;
}

function ApplicabilityList({ title, values }: { title: string; values: string[] }) {
  if (!values.length) return null;
  return <div><h5>{title}</h5><ul>{values.map(value => <li key={value}>{value}</li>)}</ul></div>;
}

function IdeaCard({ idea, selected, disabled, onToggle, onSource, text }: { idea: KnowledgeIdea; selected: boolean; disabled: boolean; onToggle: () => void; onSource: (source: KnowledgeSourceRef) => void; text: ReturnType<typeof useTaskWorkbenchText>["knowledgeReview"] }) {
  return <article className="knowledge-idea-card"><header><div><span className={`knowledge-idea-disposition is-${idea.disposition}`}>{text[idea.disposition]}</span><h5>{idea.title}</h5></div><label><input type="checkbox" data-control="knowledge-idea-select" data-record-id={`${idea.id}:${idea.revision}`} checked={selected} disabled={disabled} onChange={onToggle} />{text.selectIdea}</label></header><KnowledgeMarkdown>{idea.bodyMarkdown}</KnowledgeMarkdown>{idea.reconsiderationConditions.length > 0 && <div><strong>{text.reconsiderWhen}</strong><ul>{idea.reconsiderationConditions.map(condition => <li key={condition}>{condition}</li>)}</ul></div>}<div className="knowledge-idea-sources">{idea.sourceRefs.map(source => <button type="button" data-control="knowledge-idea-source-open" data-record-id={`${idea.id}:${source.id}:${source.revision}`} key={bindingKey(source)} onClick={() => onSource(source)}>{text.openSource}: {source.type}</button>)}</div></article>;
}
