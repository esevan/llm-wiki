import { useCallback, useEffect, useImperativeHandle, useLayoutEffect, useMemo, useRef, useState, type Ref } from "react";
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
    ...(article?.applicability?.sourceRefs ?? []),
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
  type SavedCorrection = { taskId: string; revision: number; contentHash: string; bodyMarkdown: string };
  const [savedCorrection, setSavedCorrection] = useState<SavedCorrection>();
  const savedCorrectionRef = useRef<SavedCorrection | undefined>(undefined);
  const savingCorrection = useRef(false);
  const correctionSequence = useRef(0);
  const currentTaskId = useRef(taskId);
  currentTaskId.current = taskId;


  const [comparison, setComparison] = useState<PreviewComparison>();
  const [body, setBody] = useState("");
  const [savedBody, setSavedBody] = useState("");
  const [editing, setEditing] = useState(false);
  const [noteDrafts, setNoteDrafts] = useState<Record<string, string>>({});
  const [savingNote, setSavingNote] = useState(false);
  const [selectedIdeas, setSelectedIdeas] = useState<Set<string>>(new Set());
  const [source, setSource] = useState<ExactReferenceBinding>();
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<"generate" | "edit" | "restore" | "prepare" | "publish">();
  const [error, setError] = useState("");
  const [publicationError, setPublicationError] = useState("");
  const [canRegenerate, setCanRegenerate] = useState(false);
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
  const bodyDirty = editing && body !== savedBody;
  const noteDirty = Object.entries(noteDrafts).some(([key, value]) => key.startsWith(`${taskId}:`) && Boolean(value.trim()));
  const dirty = bodyDirty || noteDirty;
  const dirtyRef = useRef(dirty);
  dirtyRef.current = dirty;
  useEffect(() => { onDirtyChange?.(dirty); return () => onDirtyChange?.(false); }, [dirty, onDirtyChange]);

  useEffect(() => {
    ++correctionSequence.current;
    savingCorrection.current = false;
    savedCorrectionRef.current = undefined;
    setSavedCorrection(undefined);
    setBusy(undefined);
    setStatus("");
    setError("");
    setPublicationError("");
    setSource(undefined);
    setProjection(undefined);
    setSelected(undefined);
    setEditing(false);
    setBody("");
    setSavedBody("");
  }, [taskId]);

  const load = useCallback(async (preferredRevision?: number) => {
    const sequence = ++loadSequence.current;
    setLoading(true);
    try {
      const next = await taskClient.knowledge(taskId);
      if (sequence !== loadSequence.current || currentTaskId.current !== taskId) return;
      if (!isKnowledgeProjection(next) || next.taskId !== taskId) throw new Error(text.invalidResponse);
      const committed = savedCorrectionRef.current;
      if (committed?.taskId === taskId) {
        const canonical = next.versions.find(v => v.revision === committed.revision);
        if (!canonical || canonical.contentHash !== committed.contentHash || canonical.bodyMarkdown !== committed.bodyMarkdown || (next.pointers.currentPrivateRevision ?? 0) < committed.revision) {
          throw new Error(text.invalidResponse);
        }
        savedCorrectionRef.current = undefined;
        setSavedCorrection(undefined);
      }
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
        const preferred = preferredRevision ?? (committed?.taskId === taskId ? committed.revision : undefined);
        if (preferred && next.versions.some(version => version.revision === preferred)) return preferred;
        if (generatedRevision !== undefined) return generatedRevision;
        if (current && next.versions.some(version => version.revision === current)) return current;
        return next.pointers.currentPrivateRevision ?? next.versions.at(-1)?.revision;
      });
    } catch (cause) {
      if (sequence === loadSequence.current && currentTaskId.current === taskId) setError(String(cause instanceof Error ? cause.message : cause));
    } finally {
      if (sequence === loadSequence.current && currentTaskId.current === taskId) setLoading(false);
    }
  }, [taskId, text.invalidResponse]);

  useEffect(() => {
    const sequences = loadSequence;
    void load(preferredRevision);
    return () => { sequences.current += 1; };
  }, [load, preferredRevision, refreshKey]);
  const version = projection?.versions.find(candidate => candidate.revision === selected);
  const currentRevision = projection?.pointers.currentPrivateRevision;
  useEffect(() => {
    if (pendingGenerationRevision.current === undefined) return;
    if (generationQueued) { observedGenerationQueue.current = true; return; }
    if (observedGenerationQueue.current && currentRevision === pendingGenerationRevision.current) {
      pendingGenerationRevision.current = undefined;
      observedGenerationQueue.current = false;
    }
  }, [currentRevision, generationQueued]);
  useLayoutEffect(() => {
    if (!version || dirty || savedCorrection?.taskId === taskId) return;
    setBody(version.bodyMarkdown);
    setSavedBody(version.bodyMarkdown);
    setEditing(false);
    setProposal(undefined);
    setPublicationOperation(undefined);
    setProposalOperationId(undefined);
    setPublicationOperationId(undefined);
    setSelectedIdeas(new Set());
  }, [dirty, savedCorrection, taskId, version]);

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
    if (savingCorrection.current || savedCorrectionRef.current?.taskId === taskId) return false;
    if (noteDirty) { setStatus(text.unsavedReviewNote); return false; }
    if (!version || version.revision !== currentRevision || !version.generationSnapshotHash || busy || !dirty) return !dirty;
    const correction = ++correctionSequence.current;
    savingCorrection.current = true;
    setBusy("edit"); setError(""); setStatus(text.saving);
    pendingGenerationRevision.current = undefined;
    observedGenerationQueue.current = false;
    try {
      const edited = await taskClient.correctKnowledge(taskId, version.revision, version.contentHash, version.generationSnapshotHash, body);
      if (currentTaskId.current !== taskId || correction !== correctionSequence.current) return true;
      if (!Number.isInteger(edited.draftRevision) || edited.draftRevision <= version.revision || typeof edited.contentHash !== "string" || !edited.contentHash || edited.bodyMarkdown !== body) {
        throw new Error(text.invalidResponse);
      }
      const committed = { taskId, revision: edited.draftRevision, contentHash: edited.contentHash, bodyMarkdown: edited.bodyMarkdown };
      savedCorrectionRef.current = committed;
      setSavedCorrection(committed);
      ++loadSequence.current;
      dirtyRef.current = false;
      setSavedBody(edited.bodyMarkdown); setEditing(false);
      await load(edited.draftRevision);
      if (currentTaskId.current !== taskId || correction !== correctionSequence.current) return true;
      setStatus(text.edited);
      return true;
    } catch (cause) { if (currentTaskId.current === taskId && correction === correctionSequence.current) setError(`${text.saveFailed} ${String(cause instanceof Error ? cause.message : cause)}`); }
    finally { if (correction === correctionSequence.current) { savingCorrection.current = false; setBusy(undefined); } }
    return false;
  }, [body, busy, currentRevision, dirty, load, noteDirty, taskId, text.edited, text.invalidResponse, text.saveFailed, text.saving, text.unsavedReviewNote, version]);
  useImperativeHandle(ref, () => ({
    save,
    discard: () => { setNoteDrafts({}); setBody(savedBody); setEditing(false); setStatus(""); setError(""); },
    focus: () => root.current?.focus({ preventScroll: true }),
  }), [savedBody, save]);
  const generate = async () => {
    if (busy || !completed) return;
    setBusy("generate"); setError(""); setStatus(text.generating);
    pendingGenerationRevision.current = currentRevision;
    observedGenerationQueue.current = false;
    try { await onGenerate(); }
    catch { pendingGenerationRevision.current = undefined; setError(text.generationUnavailable); setStatus(""); }
    finally { setBusy(undefined); }
  };
  const saveReviewNote = async () => {
    if (!version || savingNote) return;
    const key = `${taskId}:${version.revision}:${version.contentHash}`;
    const note = (noteDrafts[key] ?? "").trim();
    if (!note) return;
    setSavingNote(true); setError("");
    try {
      await taskClient.addKnowledgeReviewNote(taskId, { knowledgeRevision: version.revision, expectedContentHash: version.contentHash, body: note, operationId: operationId() });
      setNoteDrafts(current => ({ ...current, [key]: "" }));
      await load(version.revision);
      setStatus(text.reviewNoteSaved);
    } catch { setError(text.noteSaveFailed); }
    finally { setSavingNote(false); }
  };
  const publicationFailure = (cause: unknown) => {
    const message = String(cause instanceof Error ? cause.message : cause);
    if (/knowledge_unverified|missing_reference|invalid_reference|invalid_knowledge|unsupported|finality|quality/i.test(message)) return text.invalidPublication;
    if (/snapshot|content_hash|revision|stale/i.test(message)) return text.stalePublication;
    if (/conflict|changed|collision|repair|external.*edit/i.test(message)) return text.fileConflict;
    if (/provider|model|api.?key|credential|connection/i.test(message)) return text.generationUnavailable;
    return text.publicationFailed;
  };
  const preparePublication = async () => {
    if (!version || busy || savingNote) return;
    const reason = !onPrepareArchive ? text.publicationUnavailable : dirty ? text.saveBeforePublish : !version.generationSnapshotHash ? text.legacyPublication : version.freshness !== "current" ? text.stalePublication : version.qualityState !== "valid" ? text.invalidPublication : "";
    if (reason) { setPublicationError(reason); setCanRegenerate(Boolean(onPrepareArchive && !dirty && (!version.generationSnapshotHash || version.freshness !== "current" || version.qualityState !== "valid"))); return; }
    if (!onPrepareArchive || !version.generationSnapshotHash) return;
    setCanRegenerate(false); setBusy("prepare"); setPublicationError(""); setStatus(text.preparingPublication);
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
    } catch (cause) { setPublicationError(publicationFailure(cause)); setStatus(""); }
    finally { setBusy(undefined); }
  };
  const prepareOrganization = async () => {
    const documentId = projection?.pointers.publicationDocumentId;
    const expectedRevision = projection?.pointers.publicationContentHash;
    if (!documentId || !expectedRevision || !onOrganizeArchive || busy || dirty) return;
    setCanRegenerate(false); setBusy("prepare"); setPublicationError(""); setStatus(text.preparingPublication);
    try {
      const nextOperationId = operationId();
      const next = await onOrganizeArchive({
        operationId: nextOperationId, documentId, expectedRevision, intent: organizeIntent,
        requestedPath: organizeIntent === "withdraw" ? undefined : requestedPath.trim() || undefined,
      });
      setProposal(next); setProposalOperationId(nextOperationId); setPublicationOperation(undefined); setPublicationOperationId(undefined); setStatus("");
    } catch (cause) { setPublicationError(publicationFailure(cause)); setStatus(""); }
    finally { setBusy(undefined); }
  };
  const publish = async () => {
    if (!proposal || !proposalOperationId || !onPublishArchive || busy || savingNote) return;
    const reason = dirty ? text.saveBeforePublish : proposal.outcome === "conflict" ? text.fileConflict : version?.freshness !== "current" ? text.stalePublication : version?.qualityState !== "valid" ? text.invalidPublication : "";
    if (reason) { setPublicationError(reason); setCanRegenerate(!dirty && proposal.outcome !== "conflict"); return; }
    setBusy("publish"); setPublicationError(""); setStatus(text.publishing);
    try {
      const publishOperationId = operationId();
      const operation = await onPublishArchive({ operationId: publishOperationId, proposalId: proposal.proposalId, proposalVersion: proposal.proposalVersion, proposalHash: proposal.proposalHash });
      setPublicationOperation(operation); setPublicationOperationId(operation.operationId ?? publishOperationId); setStatus("");
      if (operation.state === "complete") await load(version?.revision);
    } catch (cause) { setPublicationError(publicationFailure(cause)); setStatus(""); }
    finally { setBusy(undefined); }
  };
  const retryPublication = async () => {
    if (!publicationOperationId || !onRetryArchive || busy) return;
    setBusy("publish"); setPublicationError("");
    try {
      const operation = await onRetryArchive(publicationOperationId);
      setPublicationOperation(operation);
      setPublicationOperationId(operation.operationId ?? publicationOperationId);
      if (operation.state === "complete") await load(version?.revision);
    }
    catch (cause) { setPublicationError(publicationFailure(cause)); }
    finally { setBusy(undefined); }
  };
  const recoverPublication = async (choice: "finish" | "compensate") => {
    if (!publicationOperationId || !onRecoverArchive || busy) return;
    setBusy("publish"); setPublicationError("");
    try {
      const operation = await onRecoverArchive({ operationId: publicationOperationId, choice });
      setPublicationOperation(operation);
      setPublicationOperationId(operation.operationId ?? publicationOperationId);
      if (operation.state === "complete" || operation.state === "compensated") await load(version?.revision);
    }
    catch (cause) { setPublicationError(publicationFailure(cause)); }
    finally { setBusy(undefined); }
  };

  if (savedCorrection?.taskId === taskId) return <section ref={root} className="knowledge-review" aria-label={text.title} tabIndex={-1} data-saved-revision={savedCorrection.revision}>
    <p role="status">{text.savedRefreshPending}</p>
    <KnowledgeMarkdown>{savedCorrection.bodyMarkdown}</KnowledgeMarkdown>
    {error && <p role="alert">{error}</p>}
    <button type="button" data-control="knowledge-review-retry" disabled={loading} onClick={() => void load(savedCorrection.revision)}>{text.retry}</button>
  </section>;

  if (loading && !projection) return <section ref={root} className="knowledge-review" aria-label={text.title} tabIndex={-1}><p role="status">{text.loading}</p></section>;
  if (!projection) return <section ref={root} className="knowledge-review" aria-label={text.title} tabIndex={-1}><p role="alert">{text.loadFailed} {error}</p><button type="button" data-control="knowledge-review-retry" onClick={() => void load()}>{text.retry}</button></section>;
  if (!version) return <section ref={root} className="knowledge-review knowledge-review-empty" aria-label={text.title} tabIndex={-1}><header><div><p className="knowledge-review-kicker">{text.privateDraft}</p><h3>{text.title}</h3></div><span className="knowledge-state">{text.notPublished}</span></header><p>{text.empty}</p><button type="button" data-control="task-knowledge-draft" disabled={!completed || generationQueued || busy === "generate"} onClick={() => void generate()}>{busy === "generate" || generationQueued ? text.generating : text.generate}</button>{error && <p role="alert">{error}</p>}</section>;

  const noteKey = `${taskId}:${version.revision}:${version.contentHash}`;
  const reviewNotes = (projection.reviewNotes ?? []).filter(note => note.knowledgeRevision === version.revision && note.contentHash === version.contentHash);
  const article = version.result.article;
  const outcomes = article?.finalOutcomes ?? [];
  const publicationLocked = Boolean(publicationOperation && ["writing", "index_pending", "complete", "compensated"].includes(publicationOperation.state));
  const proposalConflict = proposal?.outcome === "conflict";
  return <section ref={root} className="knowledge-review" aria-label={text.title} data-content-hash={version.contentHash} tabIndex={-1}>
    <div className="knowledge-reading">
    <header className="knowledge-review-header"><p className="knowledge-review-kicker">{text.privateDraft} · v{version.revision}</p>
      <ContentRevisionTransition as="h3" entityKey={`knowledge:${taskId}`} revision={version.revision} cause={appliedTransition?.revision === version.revision ? appliedTransition.cause : undefined} historical={version.revision !== currentRevision} variant="title">{article?.title ?? version.title}</ContentRevisionTransition>
    </header>
    {status && <p role="status" className="knowledge-draft-status">{status}</p>}
    {error && <p role="alert" className="knowledge-draft-error">{error}</p>}

    <section className="knowledge-article" aria-label={text.articleBody}>
      {editing ? <label className="knowledge-draft-editor"><span>{text.edit}</span><textarea data-control="task-knowledge-draft-body" value={body} onChange={event => setBody(event.target.value)} /></label> : <ContentRevisionTransition as="div" entityKey={`knowledge:${taskId}:body`} revision={version.revision} cause={appliedTransition?.revision === version.revision ? appliedTransition.cause : undefined} historical={version.revision !== currentRevision} variant="body" renderContent={content => <KnowledgeMarkdown>{content}</KnowledgeMarkdown>}>{version.bodyMarkdown}</ContentRevisionTransition>}
      {!version.generationSnapshotHash && <p role="note">{document.documentElement.lang.startsWith("ko") ? "이전 초안은 근거 스냅샷이 없어 편집할 수 없습니다. 새 초안을 생성해 주세요." : "This legacy draft has no evidence snapshot. Generate a new draft to edit."}</p>}

    </section>

    {outcomes.length > 0 && <section className="knowledge-outcomes" aria-labelledby="knowledge-outcomes-title"><h4 id="knowledge-outcomes-title">{text.finalOutcomes}</h4>{outcomes.length ? <ol>{outcomes.map(outcome => <li key={outcome.topicKey}><p>{outcome.statement}</p></li>)}</ol> : <p>{text.noFinalOutcomes}</p>}</section>}
    <section className="knowledge-applicability"><h4>{text.applicability}</h4><p className="knowledge-applicability-summary">{version.applicability.summary}</p><details className="knowledge-applicability-details" data-control="knowledge-applicability-details"><summary>{text.applicabilityDetails}</summary><div><ApplicabilityList title={text.questions} values={version.applicability.representativeQuestions} /><ApplicabilityList title={text.helpsWith} values={version.applicability.helpsWith} /><ApplicabilityList title={text.conditions} values={version.applicability.conditions} /><ApplicabilityList title={text.exclusions} values={version.applicability.exclusions} /></div></details></section>
    {references.length ? <details className="knowledge-provenance" data-control="knowledge-provenance"><summary>{text.provenance} · {references.length}</summary><div className="knowledge-source-list">{references.map(reference => <button type="button" data-control="knowledge-source-open" data-record-id={`${reference.documentId}:${reference.documentVersion}`} key={`${reference.documentId}:${reference.documentVersion}:${reference.section}:${reference.excerpt}`} onClick={() => setSource(reference)}><strong>{reference.title}</strong><span>{reference.section}</span><q>{reference.excerpt}</q><small>{text.openSource}</small></button>)}</div></details> : null}
    </div>
    <aside className="knowledge-controls">
    <section className="knowledge-review-notes" aria-label={text.reviewNotes}>
      <h4>{text.reviewNotes}</h4><p>{text.reviewNotesHint}</p>
      {reviewNotes.length ? <ol>{reviewNotes.map(note => <li key={note.id}><time dateTime={note.createdAt}>{note.createdAt}</time><p>{note.body}</p></li>)}</ol> : null}
      <label className="knowledge-draft-editor"><span>{text.reviewNoteBody}</span><textarea data-control="knowledge-review-note-body" value={noteDrafts[noteKey] ?? ""} disabled={savingNote} onChange={event => setNoteDrafts(current => ({ ...current, [noteKey]: event.target.value }))} /></label>
      <div className="knowledge-note-actions"><button type="button" data-control="knowledge-review-note-cancel" disabled={savingNote || !(noteDrafts[noteKey] ?? "")} onClick={() => setNoteDrafts(current => ({ ...current, [noteKey]: "" }))}>{text.cancelEdit}</button>
      <button type="button" data-control="knowledge-review-note-save" disabled={savingNote || Boolean(busy) || !(noteDrafts[noteKey] ?? "").trim()} onClick={() => void saveReviewNote()}>{savingNote ? text.savingReviewNote : text.saveReviewNote}</button></div>
    </section>


    <section className="knowledge-publication" aria-label={text.publicationReview}>
      <header><div><h4>{text.publicationReview}</h4>{!onPrepareArchive && <p>{text.publicationUnavailable}</p>}</div><button type="button" data-control="knowledge-archive-prepare" disabled={savingNote || Boolean(busy)} onClick={() => void preparePublication()}>{busy === "prepare" ? text.preparingPublication : text.preparePublication}</button></header>
      <p className="knowledge-publication-hint">{text.publicationHint}</p>
      {publicationError && <div className="knowledge-publication-error"><p className="knowledge-draft-error" role="alert">{publicationError}</p>{canRegenerate && <button type="button" data-control="knowledge-publication-regenerate" disabled={!completed || Boolean(busy) || generationQueued} onClick={() => { setPublicationError(""); setCanRegenerate(false); void generate(); }}>{generationQueued || busy === "generate" ? text.generating : text.regenerate}</button>}</div>}
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
        <p>{text.confirmPublication}</p>
        <h5>{text.proposedFiles}</h5><ul>{proposal.artifacts.map(artifact => {
          const binding = [...artifactDocuments.values()].find(document => document.documentId === artifact.documentId && document.path === (artifact.path ?? undefined));

          return <li key={`${artifact.documentId}:${artifact.path}`}><button type="button" data-control="knowledge-archive-artifact-open" data-record-id={`${artifact.documentId}:${artifact.path ?? "withdrawn"}`} disabled={!binding} onClick={() => binding && setSource(binding)}><code>{artifact.path ?? text.withdrawnPath}</code></button></li>;
        })}</ul>
        {(proposal.unresolvedConflicts?.length ?? 0) > 0 && <section className="knowledge-publication-conflicts" role="alert"><h5>{text.unresolvedRepairs}</h5><ul>{proposal.unresolvedConflicts!.map(conflict => <li key={`${conflict.path}:${conflict.reason}`}><code>{conflict.path}</code><span>{conflict.reason}</span></li>)}</ul></section>}
        <details data-control="knowledge-archive-proposal-details"><summary>{text.proposedChanges}</summary><p>{proposal.rationale}</p>
          {proposal.mocPatches.map(patch => <article className="knowledge-moc-patch" key={patch.path}><h6><code>{patch.path}</code></h6><div><section><strong>{text.before}</strong><pre>{patch.before || text.emptyFile}</pre></section><section><strong>{text.after}</strong><pre>{patch.after || text.emptyFile}</pre></section></div></article>)}
          {proposal.referenceLinks.length > 0 && <section className="knowledge-reference-links"><h6>{text.sourceLinks}</h6>{proposal.referenceLinks.map(link => { const binding = [...linkDocuments.values()].find(document => viewerKey(document) === viewerKey({ documentId: link.documentId, documentVersion: link.documentVersion, section: link.section, title: link.path })); return <button type="button" data-control="knowledge-archive-reference-open" data-record-id={`${link.documentId}:${link.documentVersion}:${link.section ?? ""}`} key={`${link.documentId}:${link.documentVersion}:${link.section ?? ""}`} onClick={() => binding && setSource(binding)}><strong>{link.path}{link.section ? `#${link.section}` : ""}</strong><span>{link.rationale}</span><small>{link.role} · {link.documentVersion}</small></button>; })}</section>}
        </details>
        {proposalConflict && <p role="alert">{text.proposalConflict}</p>}
        {!onPublishArchive && onPrepareArchive && <p role="note">{text.publicationUnavailable}</p>}
        <div className="knowledge-publication-actions"><button type="button" data-control="knowledge-archive-cancel" disabled={Boolean(busy) || publicationLocked} onClick={() => { setProposal(undefined); setProposalOperationId(undefined); setPublicationError(""); setStatus(""); }}>{text.cancelPublication}</button>
        <button type="button" data-control="knowledge-archive-publish" disabled={!onPublishArchive || savingNote || Boolean(busy) || proposalConflict || publicationLocked} onClick={() => void publish()}>{busy === "publish" ? text.publishing : text.publishExact}</button></div>
      </div>}
      {publicationOperation && <div className={`knowledge-publication-state is-${publicationOperation.state}`} role={publicationOperation.state === "conflict" || publicationOperation.state === "repair_required" || publicationOperation.state === "index_failed" ? "alert" : "status"}>
        <strong>{text.publicationState}: {publicationOperation.state}</strong>{publicationOperation.error && <p>{publicationFailure(publicationOperation.error)}</p>}
        {publicationOperation.state === "index_pending" && <p>{text.indexPending}</p>}
        {publicationOperation.state === "index_failed" && <button type="button" data-control="knowledge-archive-retry-index" disabled={!onRetryArchive || Boolean(busy)} onClick={() => void retryPublication()}>{text.retryIndex}</button>}
        {(publicationOperation.state === "index_failed" || publicationOperation.state === "repair_required") && <div className="knowledge-recovery-actions"><button type="button" data-control="knowledge-archive-recover-finish" disabled={!onRecoverArchive || Boolean(busy)} onClick={() => void recoverPublication("finish")}>{text.finishRecovery}</button><button type="button" data-control="knowledge-archive-recover-compensate" disabled={!onRecoverArchive || Boolean(busy)} onClick={() => void recoverPublication("compensate")}>{text.compensate}</button></div>}
      </div>}
    </section>

<section className="knowledge-version-control">      {(version.revision === currentRevision || editing) && <div className="knowledge-review-actions">{editing ? <><button type="button" data-control="knowledge-version-save" disabled={Boolean(busy) || !bodyDirty || version.revision !== currentRevision || !version.generationSnapshotHash} onClick={() => void save()}>{text.save}</button><button type="button" data-control="knowledge-version-cancel" disabled={Boolean(busy)} onClick={() => { setBody(savedBody); setEditing(false); setStatus(""); }}>{text.cancelEdit}</button></> : <button type="button" data-control="knowledge-version-edit" title={!version.generationSnapshotHash ? (document.documentElement.lang.startsWith("ko") ? "이전 초안은 근거 스냅샷이 없어 편집할 수 없습니다. 새 초안을 생성해 주세요." : "This legacy draft has no evidence snapshot. Generate a new draft to edit.") : undefined} disabled={Boolean(busy) || !version.generationSnapshotHash} onClick={() => setEditing(true)}>{text.edit}</button>}</div>}    <DraftVersionControls versions={previews.map(item => ({ version: item.version, derivationKind: item.derivationKind, createdAt: item.createdAt, derivedFromVersion: item.derivedFromVersion, current: item.current, contentHash: item.contentHash }))} selected={version.revision} currentVersion={currentRevision} comparison={comparison} disabled={Boolean(busy) || dirty} restoreDisabled={version.revision === currentRevision} onSelect={selectVersion} onCompare={(left: number, right: number) => { const a = previews.find(item => item.version === left); const b = previews.find(item => item.version === right); if (a && b) setComparison(compare(a, b)); }} onRestore={(revision: number) => void restore(revision)} /></section>
    <details className="knowledge-secondary" data-control="knowledge-history"><summary>{text.historyTools}</summary>


    {ideas.length > 0 && <details className="knowledge-ideas" data-control="knowledge-ideas"><summary>{text.ideas} · {ideas.length}</summary><p>{text.ideasHint}</p>{ideas.length ? <div>{ideas.map(idea => <IdeaCard key={`${idea.id}:${idea.revision}`} idea={idea} selected={selectedIdeas.has(`${idea.id}:${idea.revision}`)} disabled={Boolean(proposal)} onToggle={() => setSelectedIdeas(current => { const next = new Set(current); const key = `${idea.id}:${idea.revision}`; if (next.has(key)) next.delete(key); else next.add(key); return next; })} onSource={item => setSource(toBinding(item))} text={text} />)}</div> : <p>{text.noIdeas}</p>}</details>}


    <p className="knowledge-compare-hint">{text.compareHint}</p>
    <div className="knowledge-regenerate"><button type="button" data-control="knowledge-version-regenerate" disabled={!completed || generationQueued || Boolean(busy) || dirty} onClick={() => void generate()}>{busy === "generate" || generationQueued ? text.generating : text.regenerate}</button></div>
    </details>
    </aside>
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
