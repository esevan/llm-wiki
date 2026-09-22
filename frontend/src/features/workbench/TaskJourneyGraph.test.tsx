import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TaskJourneyGraph } from "./TaskJourneyGraph";

describe("TaskJourneyGraph", () => {
  it("renders refinement and execution evidence as an ordered, non-causal trace", () => {
    render(<TaskJourneyGraph language="en" journey={{
      semantics: { causalInference: false },
      events: [
        { id: "capture", type: "origin_capture", occurredAt: "2026-09-20T09:00:00Z", detail: { summary: "Add an export option" } },
        { id: "idea", type: "refinement_input", occurredAt: "2026-09-20T09:30:00Z", detail: { summary: "Edge case: blank exports need a safe path" } },
        { id: "revision:task:2", type: "refinement_applied", occurredAt: "2026-09-20T10:00:00Z", detail: { title: "Export safely", changes: [{ field: "scope", before: "files", after: "all exports" }, { field: "validationCriteria", before: "", after: "empty export" }] } },
        { id: "work", type: "work_recorded", occurredAt: "2026-09-20T11:00:00Z", detail: { summary: "Covered the empty export case" } },
      ],
      edges: [
        { from: "capture", to: "idea", kind: "followed_by" },
        { from: "idea", to: "revision:task:2", kind: "followed_by" },
        { from: "revision:task:2", to: "work", kind: "followed_by" },
        { from: "idea", to: "revision:task:2", kind: "refinement_context" },
      ],
    }} />);

    expect(screen.getByRole("heading", { name: "How this Task took shape" })).toBeVisible();
    expect(screen.getByText("Export safely")).toBeVisible();
    expect(screen.getByText("Edge case: blank exports need a safe path")).toBeVisible();
    expect(screen.getByRole("link", { name: "↳ Linked refinement: Export safely" })).toHaveAttribute("href", "#journey-event-revision%3Atask%3A2");
    const link = screen.getByRole("link", { name: "↳ Linked refinement: Export safely" });
    expect(document.getElementById(decodeURIComponent(link.getAttribute("href")!.slice(1)))).not.toBeNull();
    expect(screen.getByText("Changed scope: files → all exports")).toBeVisible();
    expect(screen.getByText("Covered the empty export case")).toBeVisible();
    expect(screen.getByText("The connectors show recorded order, not inferred cause.")).toBeVisible();
    expect(screen.getByRole("list", { name: "Recorded event trace" }).children).toHaveLength(4);
  });

  it("uses Korean labels and preserves recorded decision text without classifying it", () => {
    render(<TaskJourneyGraph language="ko" journey={{
      events: [{ id: "decision", type: "decision_recorded", detail: { payload: { rationale: "빈 입력 처리를 추가" } } }],
    }} />);

    expect(screen.getByText("결정 기록")).toBeVisible();
    expect(screen.getByText("빈 입력 처리를 추가")).toBeVisible();
    expect(screen.queryByText("새로운 아이디어 추가")).toBeNull();
  });
});
