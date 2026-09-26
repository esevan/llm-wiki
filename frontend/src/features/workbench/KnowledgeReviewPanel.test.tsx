import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { KnowledgeReviewPanel } from "./KnowledgeReviewPanel";

const response = (body: unknown) => ({ ok: true, status: 200, json: async <T,>() => body as T, text: async () => "", body: null });
const source = { type: "task_distillation", id: "claim-1", revision: "2", locator: "claims/claim-1", quote: "The verified result is available offline." };
const version = (revision = 2, bodyMarkdown = "# Offline use\n\nThe verified result is available offline.") => ({
  revision,
  parentRevision: revision - 1 || undefined,
  derivationKind: revision === 1 ? "generated" : "edited",
  articleType: "guide",
  title: "Offline use",
  bodyMarkdown,
  contentHash: `content-${revision}`,
  generationSnapshotHash: "snapshot-current",
  freshness: "current",
  storedFreshness: "current",
  qualityState: "valid",
  modelStatus: "enhanced",
  modelError: "",
  createdAt: `2026-09-2${revision}T12:00:00Z`,
  applicability: {
    summary: "Use this when work must remain local.",
    representativeQuestions: ["Can this run offline?"],
    helpsWith: ["Local review"],
    conditions: ["The local index is current"],
    exclusions: ["It does not prove remote availability"],
  },
  result: {
    article: {
      type: "guide",
      title: "Offline use",
      finalOutcomes: [{ topicKey: "storage", statement: "Keep final Knowledge local.", claimIds: ["claim-1"] }],
      bodyMarkdown,
      applicability: { summary: "Use this when work must remain local.", representativeQuestions: ["Can this run offline?"], helpsWith: ["Local review"], conditions: ["The local index is current"], exclusions: ["It does not prove remote availability"], sourceRefs: [source] },
      claimBindings: [{ claimId: "claim-1", statement: "The verified result is available offline.", epistemicState: "verified", sourceRefs: [source] }],
      assumptionBindings: [],
    },
    ideas: [],
    qualityFindings: [{ kind: "source_note", severity: "info", locator: "claims/claim-1", message: "Exact source retained", sourceRefs: [source] }],
  },
});
const idea = {
  id: "idea-reminder", revision: 1, knowledgeRevision: 2, title: "Reminder automation",
  bodyMarkdown: "Try a reminder automation later.", disposition: "unverified", reconsiderationConditions: ["A scheduler is connected"],
  sourceRefs: [source], relatedTopicKeys: ["storage"], contentHash: "idea-hash", publicationState: "private",
};
const projection = (versions = [version(1), version(2)]) => ({
  taskId: "task-1",
  pointers: { currentPrivateRevision: versions.at(-1)?.revision, publishedRevision: 1, publicationDocumentId: "knowledge-1", publicationPath: "knowledge/offline.md", publicationContentHash: "published-hash" },
  versions,
  ideas: [idea],
});

describe("Knowledge review", () => {
  it("reports independent projection and historical version selection timings", async () => {
    const revisions = Array.from({ length: 40 }, (_, index) =>
      version(index + 1, `# Revision ${index + 1}\n\n${"A recorded outcome retains its conditions and evidence.\n\n".repeat(40)}`));
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(projection(revisions))) };
    const projectionSamples: number[] = [];
    const selectionSamples: number[] = [];
    for (let sample = 0; sample < 25; sample += 1) {
      let mounted!: ReturnType<typeof render>;
      const started = performance.now();
      await act(async () => {
        mounted = render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={vi.fn()} />);
      });
      projectionSamples.push(performance.now() - started);
      const selector = within(mounted.container).getByRole("combobox", { name: /Version/ });
      expect(selector).toHaveValue("40");
      const selecting = performance.now();
      await act(async () => { fireEvent.change(selector, { target: { value: "1" } }); });
      selectionSamples.push(performance.now() - selecting);
      expect(within(mounted.container).getByRole("heading", { name: "Revision 1", level: 1 })).toBeVisible();
      mounted.unmount();
      mounted.container.remove();
    }
    const p95 = (samples: number[]) => [...samples].sort((a, b) => a - b)[Math.ceil(samples.length * 0.95) - 1];
    console.info(`KnowledgeReview samples=25 revisions=40 projection-p95=${p95(projectionSamples).toFixed(2)}ms selection-p95=${p95(selectionSamples).toFixed(2)}ms`);
    expect(projectionSamples.every(value => Number.isFinite(value) && value >= 0)).toBe(true);
    expect(selectionSamples.every(value => Number.isFinite(value) && value >= 0)).toBe(true);
  });

  it("keeps loading, failure, and empty states actionable without enqueuing work", async () => {
    let recover = false;
    const request = vi.fn(() => recover
      ? Promise.resolve(response({ taskId: "task-1", pointers: {}, versions: [], ideas: [] }))
      : Promise.reject(new Error("Local read unavailable")));
    window.llmWikiApplication = { request };
    const generate = vi.fn().mockResolvedValue(undefined);
    render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={generate} />);
    expect(screen.getByRole("status")).toHaveTextContent("Loading Knowledge");
    expect(await screen.findByRole("alert")).toHaveTextContent("Local read unavailable");
    recover = true;
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByText("No private Knowledge draft yet.")).toBeVisible();
    expect(generate).not.toHaveBeenCalled();
  });

  it("reads the stored projection without generation and keeps ideas outside the final article", async () => {
    const request = vi.fn().mockResolvedValue(response(projection()));
    window.llmWikiApplication = { request };
    render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={vi.fn()} />);

    expect(await screen.findByRole("heading", { name: "Offline use", level: 3 })).toBeVisible();
    expect(screen.getByText("Keep final Knowledge local.")).toBeVisible();
    expect(screen.getByText("It does not prove remote availability")).toBeVisible();
    expect(within(document.querySelector(".knowledge-article")!).queryByText("Try a reminder automation later.")).not.toBeInTheDocument();
    fireEvent.click(screen.getByText(/Ideas to revisit/));
    expect(screen.getByText("Try a reminder automation later.")).toBeVisible();
    expect(request).toHaveBeenCalledTimes(1);
    expect(request).toHaveBeenCalledWith(expect.objectContaining({ path: "/tasks/task-1/knowledge", method: "GET" }));
  });

  it("preserves an unsaved edit during refresh and appends it with exact hash guards", async () => {
    let current = projection();
    const request = vi.fn(({ path, method, body }: { path: string; method?: string; body?: string }) => {
      if (path.endsWith("/correction") && method === "POST") {
        expect(JSON.parse(body ?? "{}")).toMatchObject({ expectedContentHash: "content-2", expectedSourceHash: "snapshot-current", bodyMarkdown: "# My retained edit" });
        current = projection([version(1), version(2), version(3, "# My retained edit")]);
        return Promise.resolve(response({ draftRevision: 3, bodyMarkdown: "# My retained edit", contentHash: "content-3", state: "draft" }));
      }
      return Promise.resolve(response(current));
    });
    window.llmWikiApplication = { request };
    const dirty = vi.fn();
    const rendered = render(<KnowledgeReviewPanel taskId="task-1" completed refreshKey={1} onGenerate={vi.fn()} onDirtyChange={dirty} />);
    await screen.findByRole("heading", { name: "Offline use", level: 3 });
    fireEvent.click(screen.getByRole("button", { name: "Edit current draft" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Edit current draft" }), { target: { value: "# My retained edit" } });
    rendered.rerender(<KnowledgeReviewPanel taskId="task-1" completed refreshKey={2} onGenerate={vi.fn()} onDirtyChange={dirty} />);
    await waitFor(() => expect(screen.getByRole("textbox", { name: "Edit current draft" })).toHaveValue("# My retained edit"));
    expect(dirty).toHaveBeenCalledWith(true);
    fireEvent.click(screen.getByRole("button", { name: "Save as new version" }));
    await waitFor(() => expect(screen.getByText("Saved as a new private version.")).toBeVisible());
    expect(screen.getByRole("combobox", { name: /Version/ })).toHaveValue("3");
  });

  it("selects historical bytes, restores as new, and compares stored versions without generation", async () => {
    const historical = { ...version(1, "# Historical body"), title: "Historical Knowledge", result: { ...version(1).result, article: { ...version(1).result.article, title: "Historical Knowledge", bodyMarkdown: "# Historical body" } } };
    let current = projection([historical, version(2, "# Current body")]);
    const request = vi.fn(({ path, method, body }: { path: string; method?: string; body?: string }) => {
      if (path.endsWith("/knowledge/versions/1/restore") && method === "POST") {
        expect(JSON.parse(body ?? "{}")).toMatchObject({ expectedCurrentPrivateRevision: 2 });
        current = projection([historical, version(2, "# Current body"), { ...historical, revision: 3, parentRevision: 2, contentHash: "content-3", derivationKind: "restored" }]);
        return Promise.resolve(response({ taskId: "task-1", draftRevision: 3, derivedFromRevision: 1, contentHash: "content-3", bodyMarkdown: historical.bodyMarkdown, state: "draft" }));
      }
      return Promise.resolve(response(current));
    });
    window.llmWikiApplication = { request };
    render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={vi.fn()} />);
    await screen.findByRole("heading", { name: "Offline use", level: 3 });
    const selector = document.querySelector<HTMLSelectElement>('[data-control="draft-version-select"]')!;
    fireEvent.change(selector, { target: { value: "1" } });
    expect(document.querySelector("[data-transitioning]")).not.toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "Restore as new" }));
    await waitFor(() => expect(selector).toHaveValue("3"));
    fireEvent.click(screen.getByRole("button", { name: "Compare versions" }));
    expect(screen.getByRole("region", { name: "Version comparison" })).toBeVisible();
    expect(request.mock.calls.filter(([input]) => input.path.endsWith("/knowledge/drafts")).length).toBe(0);
  });

  it("reviews exact archive artifacts and publishes only through callbacks", async () => {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(projection())) };
    const exactArtifact = "---\ntitle: Offline use\nllm_wiki:\n  schema: 1\n---\n# Offline use\n\nThe verified result is available offline.";
    const prepare = vi.fn().mockResolvedValue({
      proposalId: "proposal-1", proposalVersion: 1, proposalHash: "proposal-hash", state: "review", outcome: "ready", rationale: "Exact bytes prepared",
      artifacts: [{ documentId: "knowledge-1", path: "knowledge/offline.md", bytes: exactArtifact, sha256: "sha256:file", expectedHash: "content-2", kind: "knowledge" }],
      mocPatches: [{ path: "MOC.md", before: "# MOC", after: "# MOC\n\n[[offline]]" }],
      referenceLinks: [{ documentId: "source-1", documentVersion: "source-revision", section: "Result", path: "sources/run.md", role: "evidence", rationale: "Supports the offline result" }],
    });
    const publish = vi.fn().mockResolvedValue({ operationId: "archive-operation", state: "index_pending" });
    const status = vi.fn().mockResolvedValue({ operationId: "archive-operation", state: "complete" });
    render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={vi.fn()} onPrepareArchive={prepare} onPublishArchive={publish} onArchiveStatus={status} />);
    await screen.findByRole("heading", { name: "Offline use", level: 3 });
    fireEvent.click(screen.getByText(/Ideas to revisit/));
    fireEvent.click(screen.getByRole("checkbox", { name: "Include in publication proposal" }));
    fireEvent.click(screen.getByRole("button", { name: "Prepare publication review" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "knowledge/offline.md" })).toBeVisible());
    expect(prepare).toHaveBeenCalledWith(expect.objectContaining({ knowledgeRevision: 2, expectedKnowledgeContentHash: "content-2", expectedGenerationSnapshotHash: "snapshot-current", selectedIdeaRevisionIds: [{ id: "idea-reminder", revision: 1 }] }));
    expect(screen.queryByText(exactArtifact)).not.toBeInTheDocument();
    expect(screen.getByText(`${new TextEncoder().encode(exactArtifact).byteLength} bytes`, { exact: false })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "knowledge/offline.md" }));
    expect(await screen.findByRole("dialog", { name: "knowledge/offline.md" })).toHaveTextContent("The verified result is available offline.");
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Close reference" }));
    fireEvent.click(screen.getByText("MOC and reference changes"));
    expect(screen.getByText("[[offline]]", { exact: false })).toBeVisible();
    expect(screen.getByText("Supports the offline result")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Publish these exact files" }));
    await waitFor(() => expect(publish).toHaveBeenCalledWith(expect.objectContaining({ proposalId: "proposal-1", proposalVersion: 1, proposalHash: "proposal-hash" })));
    expect(publish.mock.calls[0][0].operationId).not.toBe(prepare.mock.calls[0][0].operationId);
    expect(await screen.findByText("Publication state: index_pending")).toBeVisible();
    expect(await screen.findByText("Publication state: complete", {}, { timeout: 2000 })).toBeVisible();
    expect(status).toHaveBeenCalledWith("archive-operation");
  });

  it("regenerates an existing stale draft, preserves dirty edits, and allows retry after failure", async () => {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(projection([{ ...version(2), freshness: "stale" }]))) };
    const generate = vi.fn().mockRejectedValueOnce(new Error("Provider unavailable")).mockResolvedValueOnce(undefined);
    render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={generate} />);
    const regenerate = await screen.findByRole("button", { name: "Regenerate private draft" });
    fireEvent.click(regenerate);
    expect(await screen.findByRole("alert")).toHaveTextContent("Provider unavailable");
    fireEvent.click(regenerate);
    await waitFor(() => expect(generate).toHaveBeenCalledTimes(2));
    fireEvent.click(screen.getByRole("button", { name: "Edit current draft" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Edit current draft" }), { target: { value: "# Unsaved revision" } });
    expect(regenerate).toBeDisabled();
    expect(screen.getByRole("textbox", { name: "Edit current draft" })).toHaveValue("# Unsaved revision");
  });

  it("uses the shared semantic transition only when generated bytes become current", async () => {
    let current = projection();
    window.llmWikiApplication = { request: vi.fn().mockImplementation(() => Promise.resolve(response(current))) };
    const generated = {
      ...version(3, "# Regenerated body"), title: "Regenerated Knowledge",
      result: { ...version(3).result, article: { ...version(3).result.article, title: "Regenerated Knowledge", bodyMarkdown: "# Regenerated body" } },
    };
    const generate = vi.fn().mockImplementation(async () => { current = projection([version(1), version(2), generated]); });
    const rendered = render(<KnowledgeReviewPanel taskId="task-1" completed refreshKey={1} onGenerate={generate} />);
    await screen.findByRole("heading", { name: "Offline use", level: 3 });
    fireEvent.click(screen.getByRole("button", { name: "Regenerate private draft" }));
    await waitFor(() => expect(generate).toHaveBeenCalledOnce());
    rendered.rerender(<KnowledgeReviewPanel taskId="task-1" completed refreshKey={2} onGenerate={generate} />);
    expect(await screen.findByRole("heading", { name: "Regenerated Knowledge", level: 3 })).toHaveAttribute("data-transitioning", "title");
    expect(document.querySelector('[data-transitioning="body"]')).toBeInTheDocument();
  });

  it("reviews published organization changes and exposes exact recovery choices", async () => {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(projection())) };
    const organize = vi.fn().mockResolvedValue({
      proposalId: "organize-1", proposalVersion: 1, proposalHash: "organize-hash", state: "review_needed", outcome: "rename", rationale: "Managed links need review",
      target: { documentId: "knowledge-1", path: "Knowledge/renamed.md" },
      artifacts: [{ documentId: "knowledge-1", path: "Knowledge/renamed.md", bytes: null, sha256: null, expectedHash: "published-hash", kind: "knowledge" }],
      mocPatches: [], referenceLinks: [], unresolvedConflicts: [{ path: "Notes/manual.md", reason: "Unmanaged link must be reviewed" }],
    });
    const publish = vi.fn().mockResolvedValue({ operationId: "organize-operation", state: "repair_required", error: "External bytes changed" });
    const recover = vi.fn().mockResolvedValue({ operationId: "organize-operation", state: "complete" });
    render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={vi.fn()} onOrganizeArchive={organize} onPublishArchive={publish} onRecoverArchive={recover} />);
    await screen.findByRole("heading", { name: "Offline use", level: 3 });
    fireEvent.click(screen.getByText("Manage published Knowledge"));
    fireEvent.change(screen.getByLabelText("Requested path"), { target: { value: "Knowledge/renamed.md" } });
    fireEvent.click(screen.getByRole("button", { name: "Review organization change" }));
    expect(await screen.findByText("Unmanaged link must be reviewed")).toBeVisible();
    expect(organize).toHaveBeenCalledWith(expect.objectContaining({ documentId: "knowledge-1", expectedRevision: "published-hash", intent: "rename", requestedPath: "Knowledge/renamed.md" }));
    fireEvent.click(screen.getByRole("button", { name: "Publish these exact files" }));
    expect(await screen.findByText("External bytes changed")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Finish exact remaining steps" }));
    await waitFor(() => expect(recover).toHaveBeenCalledWith({ operationId: "organize-operation", choice: "finish" }));
  });

  it("does not publish a proposal that still reports a conflict", async () => {
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(projection())) };
    const prepare = vi.fn().mockResolvedValue({
      proposalId: "conflict-proposal", proposalVersion: 1, proposalHash: "conflict-hash", state: "review_needed", outcome: "conflict", rationale: "Resolve the target collision first",
      artifacts: [], mocPatches: [], referenceLinks: [], unresolvedConflicts: [{ path: "knowledge/offline.md", reason: "Path is owned by another document" }],
    });
    const publish = vi.fn();
    render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={vi.fn()} onPrepareArchive={prepare} onPublishArchive={publish} />);
    await screen.findByRole("heading", { name: "Offline use", level: 3 });
    fireEvent.click(screen.getByRole("button", { name: "Prepare publication review" }));
    expect(await screen.findByText("This proposal has unresolved conflicts and cannot be published.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Publish these exact files" })).toBeDisabled();
    expect(publish).not.toHaveBeenCalled();
  });

  it("restores the newest failed operation with its exact proposal and reloads after recovery", async () => {
    const oldProposal = {
      proposalId: "old-proposal", proposalVersion: 1, proposalHash: "old-hash", state: "applied", outcome: "ready", rationale: "Older applied proposal",
      input: { knowledgeRevision: 2 }, artifacts: [], mocPatches: [], referenceLinks: [],
    };
    const savedProposal = {
      proposalId: "saved-proposal", proposalVersion: 1, proposalHash: "saved-hash", state: "review_needed", outcome: "repair", rationale: "Resume newest exact recovery",
      input: { knowledgeRevision: 2 }, artifacts: [], mocPatches: [], referenceLinks: [],
    };
    const request = vi.fn().mockResolvedValue(response({
      ...projection(), archive: {
        proposals: [savedProposal, oldProposal],
        operations: [
          { operationId: "saved-operation", state: "repair_required", error: "External edit retained", proposalId: "saved-proposal", proposalVersion: 1 },
          { operationId: "old-operation", state: "complete", proposalId: "old-proposal", proposalVersion: 1 },
        ],
      },
    }));
    window.llmWikiApplication = { request };
    const recover = vi.fn().mockResolvedValue({ operationId: "saved-operation", state: "complete" });
    render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={vi.fn()} onRecoverArchive={recover} onPublishArchive={vi.fn()} />);

    expect(await screen.findByText("External edit retained")).toBeVisible();
    expect(screen.getByText("Resume newest exact recovery")).toBeVisible();
    expect(screen.queryByText("Older applied proposal")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Finish exact remaining steps" }));
    await waitFor(() => expect(recover).toHaveBeenCalledWith({ operationId: "saved-operation", choice: "finish" }));
    await waitFor(() => expect(request).toHaveBeenCalledTimes(2));
  });

  it("resumes status polling after retry and reloads the published pointer on completion", async () => {
    let operationState = "index_failed";
    const savedProposal = {
      proposalId: "saved-proposal", proposalVersion: 1, proposalHash: "saved-hash", state: "review_needed", outcome: "ready", rationale: "Resume indexing",
      input: { knowledgeRevision: 2 }, artifacts: [], mocPatches: [], referenceLinks: [],
    };
    const request = vi.fn().mockImplementation(() => Promise.resolve(response({
      ...projection(),
      pointers: { ...projection().pointers, publishedRevision: operationState === "complete" ? 2 : 1 },
      archive: { proposals: [savedProposal], operations: [{ operationId: "saved-operation", state: operationState, error: operationState === "index_failed" ? "Index unavailable" : undefined, proposalId: "saved-proposal", proposalVersion: 1 }] },
    })));
    window.llmWikiApplication = { request };
    const retry = vi.fn().mockImplementation(async () => { operationState = "index_pending"; return { operationId: "saved-operation", state: "index_pending" }; });
    const archiveStatus = vi.fn().mockImplementation(async () => { operationState = "complete"; return { operationId: "saved-operation", state: "complete" }; });
    render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={vi.fn()} onRetryArchive={retry} onArchiveStatus={archiveStatus} />);

    expect(await screen.findByText("Index unavailable")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Retry indexing" }));
    expect(await screen.findByText("Publication state: index_pending")).toBeVisible();
    await waitFor(() => expect(archiveStatus).toHaveBeenCalledWith("saved-operation"), { timeout: 2000 });
    await waitFor(() => expect(request.mock.calls.length).toBeGreaterThan(1));
    await waitFor(() => expect(screen.getByText("Published version")).toBeVisible());
  });

  it("shows Korean freshness, limits, and exact idea dispositions", async () => {
    const original = document.documentElement.lang;
    document.documentElement.lang = "ko";
    const stale = {
      ...projection([{ ...version(2), freshness: "stale" }]),
      ideas: (["unverified", "deferred", "rejected", "out_of_scope"] as const).map((disposition, index) => ({
        ...idea, id: `idea-${disposition}`, title: `Idea ${index + 1}`, disposition,
      })),
    };
    window.llmWikiApplication = { request: vi.fn().mockResolvedValue(response(stale)) };
    try {
      render(<KnowledgeReviewPanel taskId="task-1" completed onGenerate={vi.fn()} />);
      expect(await screen.findByText("출처 변경됨")).toBeVisible();
      expect(screen.getByText("한계와 제외 범위")).toBeVisible();
      fireEvent.click(screen.getByText(/다시 살펴볼 아이디어/));
      expect(screen.getByText("미검증")).toBeVisible();
      expect(screen.getByText("보류")).toBeVisible();
      expect(screen.getByText("기각")).toBeVisible();
      expect(screen.getByText("범위 밖")).toBeVisible();
      expect(screen.getAllByText("다시 살펴볼 조건")).toHaveLength(4);
    } finally { document.documentElement.lang = original; }
  });
});
