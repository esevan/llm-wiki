import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ConflictReviewPanel } from "./ConflictReviewPanel";

describe("Conflict review", () => {
  it("shows cited findings without disabling Task work", async () => {
    const request = vi
      .fn()
      .mockImplementation(async ({ method }: { method?: string }) => ({
        ok: true,
        status: method === "POST" ? 202 : 200,
        json: async () => method === "POST" ? { id: "review-1", status: "queued" } : ({
          attempts: [{ id: "review-1", status: "findings", findings: [{ id: "finding-1", path: "Knowledge/guide.md", excerpt: "<mark>Conflicts</mark> with the stated scope." }] }],
          currentResult: { id: "review-1", status: "findings", findings: [{ id: "finding-1", path: "Knowledge/guide.md", excerpt: "<mark>Conflicts</mark> with the stated scope." }] },
        }),
        text: async () => "",
        body: null,
      }));
    window.llmWikiApplication = { request };
    render(<ConflictReviewPanel taskId="task-1" taskRevision={4} />);
    fireEvent.click(screen.getByRole("button", { name: "Run review" }));
    await waitFor(() =>
      expect(screen.getAllByText("Conflicts with the stated scope.")[0]).toBeVisible(),
    );
    expect(
      screen.getByText("Review is advisory. You can continue working."),
    ).toBeVisible();
    expect(screen.getByLabelText("Current result")).toHaveTextContent(
      "Current result Findings",
    );
    expect(screen.getByLabelText("Current result")).not.toHaveTextContent("<mark>");
  });

  it("keeps a previous current result while separately showing a failed retry", async () => {
    const request = vi.fn().mockResolvedValue({
      ok: true, status: 200, text: async () => "", body: null,
      json: async () => ({
        attempts: [
          { id: "retry", status: "failed", safeError: "provider_unavailable" },
          { id: "current", status: "clear" },
        ],
        currentResult: { id: "current", status: "clear" },
      }),
    });
    window.llmWikiApplication = { request };
    render(<ConflictReviewPanel taskId="task-1" taskRevision={4} />);
    await waitFor(() => expect(screen.getByLabelText("Current result")).toHaveTextContent("Clear"));
    expect(screen.getByLabelText("Review attempts")).toHaveTextContent("Failed");
    expect(screen.getByLabelText("Review attempts")).toHaveTextContent("provider_unavailable");
  });

  it("continues polling while an attempt remains active", async () => {
    let historyReads = 0;
    const request = vi.fn().mockImplementation(
      async ({ path, method }: { path: string; method?: string }) => ({
        ok: true,
        status: method === "POST" ? 202 : 200,
        json: async () => {
          if (method === "POST") return { id: "review-poll", status: "queued" };
          historyReads += 1;
          if (historyReads < 3)
            return {
              attempts: [{ id: "review-poll", status: "running" }],
            };
          return {
            attempts: [{ id: "review-poll", status: "cancelled" }],
          };
        },
        text: async () => path,
        body: null,
      }),
    );
    window.llmWikiApplication = { request };
    render(<ConflictReviewPanel taskId="task-1" taskRevision={4} />);

    fireEvent.click(screen.getByRole("button", { name: "Run review" }));

    await waitFor(
      () => expect(screen.getByLabelText("Review attempts")).toHaveTextContent("Cancelled"),
      { timeout: 2_000 },
    );
    expect(historyReads).toBeGreaterThanOrEqual(3);
  });
});
