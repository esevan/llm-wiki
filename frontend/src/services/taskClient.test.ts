import { afterEach, describe, expect, it, vi } from "vitest";
import { taskClient } from "./taskClient";

const response = (body: unknown, ok = true, status = 200) => ({
  ok,
  status,
  json: async () => body,
  text: async () => JSON.stringify(body),
  body: null,
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Task client Problem resolution contract", () => {
  it("sends the exact expected revision and preserves the explicit rationale/evidence", async () => {
    const request = vi.fn().mockResolvedValue(response({ state: "resolved" }));
    window.llmWikiApplication = { request };

    await taskClient.resolveProblem("problem-1", 3, "Evidence was reviewed");

    const call = request.mock.calls[0][0];
    expect(call).toMatchObject({
      path: "/problems/problem-1/resolutions",
      method: "POST",
    });
    expect(JSON.parse(call.body)).toMatchObject({
      expectedProblemRevision: 3,
      rationale: "Evidence was reviewed",
      evidenceRefs: [],
    });
    expect(JSON.parse(call.body)).not.toHaveProperty("problemRevision");
  });

  it("surfaces the backend detail for a rejected stale resolution", async () => {
    window.llmWikiApplication = {
      request: vi.fn().mockResolvedValue(
        response({ detail: "head_conflict: currentRevision=4" }, false, 409),
      ),
    };

    await expect(taskClient.resolveProblem("problem-1", 3, "stale")).rejects.toThrow(
      "head_conflict: currentRevision=4",
    );
  });
});

describe("Task Knowledge hash contract", () => {
  it("sends exact content and source hashes for correction, publication, and withdrawal", async () => {
    const request = vi.fn().mockResolvedValue(response({ state: "queued" }));
    window.llmWikiApplication = { request };

    await taskClient.correctKnowledge("task-1", 2, "body-old", "source-2", "# Corrected");
    await taskClient.publish("task-1", 2, "body-old", "source-2");
    await taskClient.withdrawKnowledge("task-1", 2, "body-old", "source-2");

    expect(request.mock.calls.map(([input]) => JSON.parse(input.body))).toEqual([
      expect.objectContaining({ expectedContentHash: "body-old", expectedSourceHash: "source-2", bodyMarkdown: "# Corrected" }),
      expect.objectContaining({ expectedContentHash: "body-old", expectedSourceHash: "source-2" }),
      expect.objectContaining({ expectedContentHash: "body-old", expectedSourceHash: "source-2" }),
    ]);
  });
});

describe("Knowledge archive application boundary", () => {
  it("preserves exact proposal and publication operation identities through recovery routes", async () => {
    const request = vi.fn().mockResolvedValue(response({ state: "review_needed" }));
    window.llmWikiApplication = { request };

    await taskClient.prepareKnowledgeArchive({ operationId: "prepare-1", taskId: "task-1", knowledgeRevision: 2, expectedKnowledgeContentHash: "content-2", expectedGenerationSnapshotHash: "snapshot-2", selectedIdeaRevisionIds: [{ id: "idea-1", revision: 1 }], locale: "en" });
    await taskClient.publishKnowledgeArchive({ operationId: "publish-1", proposalId: "proposal-1", proposalVersion: 1, proposalHash: "proposal-hash" });
    await taskClient.knowledgeArchiveStatus("publish/1");
    await taskClient.retryKnowledgeArchive("publish/1");
    await taskClient.recoverKnowledgeArchive({ operationId: "publish/1", choice: "compensate" });

    expect(request.mock.calls.map(([input]) => input.path)).toEqual([
      "/knowledge/archive/prepare",
      "/knowledge/archive/publish",
      "/knowledge/archive/operations/publish%2F1",
      "/knowledge/archive/operations/publish%2F1/retry",
      "/knowledge/archive/operations/publish%2F1/recover",
    ]);
    expect(JSON.parse(request.mock.calls[0][0].body)).toMatchObject({ operationId: "prepare-1", expectedKnowledgeContentHash: "content-2", selectedIdeaRevisionIds: [{ id: "idea-1", revision: 1 }] });
    expect(JSON.parse(request.mock.calls[1][0].body)).toMatchObject({ operationId: "publish-1", proposalHash: "proposal-hash" });
    expect(JSON.parse(request.mock.calls[4][0].body)).toMatchObject({ operationId: "publish/1", choice: "compensate" });
  });
});


describe("Capture and refinement image requests", () => {
  it("preserves image bytes on image-only submissions and omits absent images", async () => {
    const request = vi.fn().mockResolvedValue(response({ id: "saved" }));
    window.llmWikiApplication = { request };
    const images = [{ name: "shot.png", mediaType: "image/png", data: "iVBORw0KGgo=" }, { name: "second.png", mediaType: "image/png", data: "iVBORw0KGgo=" }];
    await taskClient.createCapture("", images);
    await taskClient.message("session", "", images);
    await taskClient.createCapture("text only");
    expect(JSON.parse(request.mock.calls[0][0].body)).toMatchObject({ text: "", images });
    expect(JSON.parse(request.mock.calls[1][0].body)).toMatchObject({ message: "", images });
    expect(JSON.parse(request.mock.calls[2][0].body)).not.toHaveProperty("images");
  });
});
