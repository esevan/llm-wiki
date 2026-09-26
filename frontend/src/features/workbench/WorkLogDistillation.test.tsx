import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Profiler } from "react";
import { describe, expect, it, vi } from "vitest";
import { WorkLogDistillation } from "./WorkLogDistillation";
import type { DistillationProjection } from "../../types/taskWorkbench";

const projection = (freshness: DistillationProjection["freshness"] = "current"): DistillationProjection => ({
  projectionRevision: 3, freshness,
  result: {
    claims: [{ id: "claim", kind: "evidence", statement: "Export check passed", actor: "tool", epistemicState: "verified", status: "current", sources: [{ type: "run_completed_item", id: "check", revision: "sha256:one", locator: "content_json", quote: "Export check passed" }] }],
    workLogView: { sections: [{ kind: "checks", claimIds: ["claim"] }] },
    warnings: freshness === "stale" ? ["The saved source changed."] : [],
  },
});

describe("WorkLogDistillation", () => {
  it("allows an explicit retry without hiding the saved result", () => {
    const retry = vi.fn();
    render(<WorkLogDistillation projection={projection("retryable_failure")} onOpenOriginal={() => undefined} onRetry={retry} />);
    expect(screen.getAllByText("Export check passed")[0]).toBeVisible();
    fireEvent.click(screen.getByRole("button", {name: "Retry update"}));
    expect(retry).toHaveBeenCalledOnce();
  });

  it("renders readable claims and inspectable exact sources without invoking a model", () => {
    const open = vi.fn();
    render(<WorkLogDistillation projection={projection()} execution={{runId:"run", sessionId:"session", status:"succeeded", provider:"codex", model:"model", evidence:[], artifacts:[], limitations:[], syncState:"synced"}} onOpenOriginal={open} />);
    expect(screen.getAllByText("Export check passed")[0]).toBeInTheDocument();
    fireEvent.click(screen.getByText("Sources (1)"));
    expect(screen.getByText(/sha256:one/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Inspect original Run" }));
    expect(open).toHaveBeenCalledTimes(1);
  });

  it("keeps the last good content visible with explicit stale status and warning", () => {
    render(<WorkLogDistillation projection={projection("stale")} onOpenOriginal={() => undefined} />);
    expect(screen.getByRole("status")).toHaveTextContent("Source changed");
    expect(screen.getAllByText("Export check passed")[0]).toBeInTheDocument();
    fireEvent.click(screen.getByText("Limitations"));
    expect(screen.getByText("The saved source changed.")).toBeInTheDocument();
  });

  it.each(["pending", "stale", "repair_required"] as const)("retains saved content and visible %s state", freshness => {
    const repair = vi.fn();
    render(<WorkLogDistillation projection={projection(freshness)} onOpenOriginal={() => undefined} onRepair={repair} />);
    expect(screen.getAllByText("Export check passed")[0]).toBeVisible();
    expect(screen.getByRole("status")).not.toBeEmptyDOMElement();
    if (freshness === "repair_required") {
      fireEvent.click(screen.getByRole("button", {name: "Repair Distillation"}));
      expect(repair).toHaveBeenCalledOnce();
    }
  });

  it("renders the factual unavailable state without invented claims", () => {
    render(<WorkLogDistillation projection={{ projectionRevision: 0, freshness: "unavailable", result: { claims: [], workLogView: { sections: [] }, warnings: ["No final report is available"] } }} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("No supported result is available yet.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Inspect original Run" })).not.toBeInTheDocument();
  });

  it("keeps collapsed Work Log projection within the 100 ms p95 budget", () => {
    const durations: number[] = [];
    const longProjection = projection();
    longProjection.result.claims = Array.from({ length: 40 }, (_, index) => ({
      ...longProjection.result.claims[0],
      id: `claim-${index}`,
      statement: `Verified result ${index} with a representative long path and value`,
    }));
    longProjection.result.workLogView.sections = [{
      kind: "checks",
      claimIds: longProjection.result.claims.map(claim => claim.id),
    }];
    for (let cycle = 0; cycle < 20; cycle += 1) {
      render(<Profiler id={`work-log-${cycle}`} onRender={(_id, _phase, duration) => durations.push(duration)}>
        <WorkLogDistillation projection={longProjection} onOpenOriginal={() => undefined} />
      </Profiler>);
      cleanup();
    }
    const sorted = [...durations].sort((a, b) => a - b);
    const p95 = sorted[Math.max(0, Math.ceil(sorted.length * 0.95) - 1)];
    expect(p95).toBeLessThan(100);
  });
});
