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
  it("shows a request while execution is still running without implying completion", () => {
    render(<WorkLogDistillation execution={{runId:"live", sessionId:"session", instruction:"Check the export", status:"running", provider:"codex", model:"model", evidence:[],artifacts:[],limitations:[],syncState:"synced"}} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("Check the export")).toBeVisible();
    expect(screen.getByText("Execution is in progress. No final result is available yet.")).toBeVisible();
  });
  it("shows each exact Run request and keeps technical structure collapsed", () => {
    const execution = {runId:"one", sessionId:"session", instruction:"Draft a reply, do not send it", status:"succeeded" as const, provider:"codex" as const, model:"model", evidence:[], artifacts:[], limitations:[], syncState:"synced" as const};
    const {container, rerender} = render(<WorkLogDistillation execution={execution} projection={projection()} onOpenOriginal={() => undefined} />);
    expect(screen.getByText(execution.instruction)).toBeVisible();
    expect(screen.getByRole("heading", {name:"What Codex did"})).toBeVisible();
    expect(screen.getByRole("heading", {name:"Result"})).toBeVisible();
    expect(container.querySelector('[data-control="task-worklog-summary-details"]')).not.toHaveAttribute("open");
    expect(screen.getByText(/sha256:one/)).not.toBeVisible();
    rerender(<WorkLogDistillation execution={{...execution, runId:"two", instruction:"Review only the second draft"}} projection={projection()} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("Review only the second draft")).toBeVisible();
    expect(screen.queryByText(execution.instruction)).not.toBeInTheDocument();
    rerender(<WorkLogDistillation execution={{...execution, instruction:undefined}} projection={projection()} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("Original request unavailable for this Run.")).toBeVisible();
  });

  it("makes conflicting evidence explicit instead of asserting verification", () => {
    const saved = projection();
    saved.result.claims[0].status = "contradicted";
    render(<WorkLogDistillation projection={saved} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("Conflicting evidence: Export check passed")).toBeVisible();
  });

  it("keeps unfinished claims visible beyond the concise default", () => {
    const saved = projection();
    saved.result.claims = Array.from({length:5}, (_, i) => ({...saved.result.claims[0], id:`c${i}`, statement:`Check ${i}`, epistemicState: i === 4 ? "unresolved" as const : "observed" as const}));
    saved.result.workLogView.sections = [{kind:"checks", claimIds:saved.result.claims.map(claim => claim.id)}];
    render(<WorkLogDistillation projection={saved} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("Unresolved: Check 4")).toBeVisible();
    expect(screen.getByText("Check 3")).not.toBeVisible();
  });
  it("allows an explicit retry without hiding the saved result", () => {
    const retry = vi.fn();
    render(<WorkLogDistillation projection={projection("retryable_failure")} onOpenOriginal={() => undefined} onRetry={retry} />);
    expect(screen.getAllByText(/Export check passed/)[0]).toBeVisible();
    fireEvent.click(screen.getByRole("button", {name: "Retry update"}));
    expect(retry).toHaveBeenCalledOnce();
  });

  it("renders readable claims and inspectable exact sources without invoking a model", () => {
    const open = vi.fn();
    render(<WorkLogDistillation projection={projection()} execution={{runId:"run", sessionId:"session", status:"succeeded", provider:"codex", model:"model", evidence:[], artifacts:[], limitations:[], syncState:"synced"}} onOpenOriginal={open} />);
    expect(screen.getAllByText(/Export check passed/)[0]).toBeInTheDocument();
    fireEvent.click(screen.getByText("Summary details and sources"));
    fireEvent.click(screen.getByText("Sources (1)"));
    expect(screen.getByText(/sha256:one/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Inspect original Run" }));
    expect(open).toHaveBeenCalledTimes(1);
  });

  it("keeps the last good content visible with explicit stale status and warning", () => {
    render(<WorkLogDistillation projection={projection("stale")} onOpenOriginal={() => undefined} />);
    expect(screen.getByRole("status")).toHaveTextContent("Source changed");
    expect(screen.getAllByText(/Export check passed/)[0]).toBeInTheDocument();
    fireEvent.click(screen.getByText("Limitations"));
    expect(screen.getByText("The saved source changed.")).toBeInTheDocument();
  });

  it.each(["pending", "stale", "repair_required"] as const)("retains saved content and visible %s state", freshness => {
    const repair = vi.fn();
    render(<WorkLogDistillation projection={projection(freshness)} onOpenOriginal={() => undefined} onRepair={repair} />);
    expect(screen.getAllByText(/Export check passed/)[0]).toBeVisible();
    expect(screen.getByRole("status")).not.toBeEmptyDOMElement();
    if (freshness === "repair_required") {
      fireEvent.click(screen.getByRole("button", {name: "Repair Distillation"}));
      expect(repair).toHaveBeenCalledOnce();
    }
  });

  it("renders the factual unavailable state without invented claims", () => {
    render(<WorkLogDistillation projection={{ projectionRevision: 0, freshness: "unavailable", result: { claims: [], workLogView: { sections: [] }, warnings: ["No final report is available"] } }} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("No execution summary is available. Inspect the original record.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Inspect original Run" })).not.toBeInTheDocument();
  });

  it.each(["en", "ko"])("handles empty legacy projections and preserves original access in %s", locale => {
    document.documentElement.lang = locale;
    const open = vi.fn();
    const retry = vi.fn();
    render(<WorkLogDistillation projection={{...projection(), result: {claims: [], workLogView: {sections: [{kind: "outcome", claimIds: ["missing"]}]}, warnings: []}}} execution={{runId:"draft-run", sessionId:"session", status:"succeeded", provider:"codex", model:"model", reportExcerpt:"Draft: production transition is complete", evidence:[], artifacts:[], limitations:[], syncState:"synced"}} onOpenOriginal={open} onRetry={retry} />);
    expect(screen.getByRole("heading", {name: locale === "ko" ? "실행 요약" : "Execution summary"})).toBeVisible();
    expect(screen.getByText(locale === "ko" ? "저장된 실행 요약이 아직 없습니다. 원본 기록을 확인하거나 요약을 다시 생성하세요." : "No execution summary has been saved yet. Inspect the original record or retry the summary.")).toBeVisible();
    expect(screen.queryByText(/Draft:/)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", {name: locale === "ko" ? "원본 Run 확인" : "Inspect original Run"}));
    fireEvent.click(screen.getByRole("button", {name: locale === "ko" ? "업데이트 재시도" : "Retry update"}));
    expect(open).toHaveBeenCalledOnce();
    expect(retry).toHaveBeenCalledOnce();
    document.documentElement.lang = "en";
  });

  it("distinguishes missing report, generation failure, pending summary and failed execution", () => {
    const execution = {runId:"run", sessionId:"session", status:"failed" as const, provider:"codex" as const, model:"model", evidence:[], artifacts:[], limitations:["Delivery was not confirmed"], syncState:"synced" as const};
    const {rerender} = render(<WorkLogDistillation execution={execution} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("This Run failed.")).toBeVisible();
    expect(screen.getByText("No final report was saved. Inspect the retained execution evidence.")).toBeVisible();
    expect(screen.getByText("Delivery was not confirmed")).toBeVisible();
    const empty = {...projection(), result: {claims: [], workLogView: {sections: []}, warnings: []}};
    rerender(<WorkLogDistillation projection={{...empty, freshness:"retryable_failure"}} execution={{...execution, reportExcerpt:"An email draft"}} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("Could not generate an execution summary. Inspect the original record.")).toBeVisible();
    rerender(<WorkLogDistillation projection={{...empty, freshness:"pending"}} execution={execution} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("Generating the execution summary. The original record is available.")).toBeVisible();
  });

  it("labels report-only statements as reported rather than verified or performed", () => {
    const result = projection();
    result.result.claims[0] = {...result.result.claims[0], statement:"The model prepared an email draft", actor:"ai", epistemicState:"performed", sources:[{type:"run_final_report",id:"run",revision:"1",locator:"final_report",quote:"Email draft"}]};
    render(<WorkLogDistillation projection={result} onOpenOriginal={() => undefined} />);
    expect(screen.getByText("Codex reported: The model prepared an email draft")).toBeVisible();
    expect(screen.queryByText("Verified · AI")).not.toBeInTheDocument();
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
