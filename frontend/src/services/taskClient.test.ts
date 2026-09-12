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
