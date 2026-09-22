import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TaskJourneyGraph } from "./TaskJourneyGraph";

describe("TaskJourneyGraph", () => {
  it("keeps long AI titles readable and reveals untruncated recorded Activity on click", () => {
    const long = "Edge case: blank exports need a safe path and preserve every recorded detail";
    render(<TaskJourneyGraph language="en" journey={{ events: [{ id: "idea", type: "refinement_input", occurredAt: "2026-09-20T09:30:00Z", detail: { summary: long } }], titles: [{ id: "idea", title: long }] }} />);
    expect(screen.getByRole("button", { name: long })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: long }));
    expect(screen.getByRole("complementary", { name: "Activity" })).toHaveTextContent(long);
  });

  it("marks a recorded day-scale gap without stretching the flow layout", () => {
    render(<TaskJourneyGraph language="en" journey={{
      events: [
        { id: "start", type: "task_created", occurredAt: "2026-09-20T09:30:00Z" },
        { id: "next", type: "work_recorded", occurredAt: "2026-09-23T09:30:00Z" },
      ],
      edges: [{ from: "start", to: "next", kind: "followed_by" }],
    }} />);
    expect(screen.getByLabelText("Time passes: 3 days later")).toHaveTextContent("3 days later");
  });

  it("reveals only the selected node's semantic relationship with rationale and evidence", () => {
    render(<TaskJourneyGraph language="en" journey={{
      events: [{ id: "old", type: "work_recorded" }, { id: "new", type: "decision_recorded" }],
      titles: [{ id: "old", title: "old" }, { id: "new", title: "new" }],
      relationships: [{ from: "new", to: "old", kind: "supersedes", provenance: "inferred", rationale: "The later decision replaces the earlier record.", evidence: [{ eventId: "new", quote: "Use the revised export path" }] }],
    }} />);
    fireEvent.click(screen.getByRole("button", { name: "new" }));
    expect(screen.getAllByText("Supersedes")).toHaveLength(2);
    expect(screen.getByText("The later decision replaces the earlier record.")).toBeVisible();
    expect(screen.getByText("Use the revised export path")).toBeVisible();
    expect(document.querySelector('path[data-relationship="supersedes"]')).toBeInTheDocument();
  });

  it("uses Korean Activity labels while preserving recorded decision content", () => {
    render(<TaskJourneyGraph language="ko" journey={{ events: [{ id: "decision", type: "decision_recorded", detail: { payload: { rationale: "빈 입력 처리를 추가" } } }], titles: [{ id: "decision", title: "결정" }] }} />);
    fireEvent.click(screen.getByRole("button", { name: "결정" }));
    expect(screen.getByRole("complementary", { name: "활동" })).toHaveTextContent("빈 입력 처리를 추가");
  });

  it("ignores an invalid timestamp when the activity detail opens", () => {
    render(<TaskJourneyGraph language="en" journey={{ events: [{ id: "bad-date", type: "work_recorded", occurredAt: "not-a-date", detail: { summary: "Still readable" } }] }} />);
    fireEvent.click(screen.getByRole("button", { name: "Work recorded" }));
    expect(screen.getByRole("complementary", { name: "Activity" })).toHaveTextContent("Still readable");
    expect(screen.queryByRole("time")).not.toBeInTheDocument();
  });

  it("uses separate SVG marker ids for multiple graphs", () => {
    const journey = { events: [{ id: "one", type: "task_created" }, { id: "two", type: "task_completed" }], edges: [{ from: "one", to: "two", kind: "followed_by" }] };
    render(<><TaskJourneyGraph language="en" journey={journey} /><TaskJourneyGraph language="en" journey={journey} /></>);
    const ids = [...document.querySelectorAll("marker")].map(marker => marker.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
