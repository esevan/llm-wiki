import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TaskJourneyGraph } from "./TaskJourneyGraph";

describe("TaskJourneyGraph", () => {
  it("keeps compact AI titles in nodes and reveals untruncated recorded Activity on click", () => {
    const long = "Edge case: blank exports need a safe path and preserve every recorded detail";
    render(<TaskJourneyGraph language="en" journey={{ events: [{ id: "idea", type: "refinement_input", occurredAt: "2026-09-20T09:30:00Z", detail: { summary: long } }], titles: [{ id: "idea", title: "Safe export" }] }} />);
    expect(screen.getByRole("button", { name: "Safe export" })).toBeVisible();
    expect(screen.queryByText(long)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Safe export" }));
    expect(screen.getByRole("complementary", { name: "Activity" })).toHaveTextContent(long);
  });

  it("uses Korean Activity labels while preserving recorded decision content", () => {
    render(<TaskJourneyGraph language="ko" journey={{ events: [{ id: "decision", type: "decision_recorded", detail: { payload: { rationale: "빈 입력 처리를 추가" } } }], titles: [{ id: "decision", title: "결정" }] }} />);
    fireEvent.click(screen.getByRole("button", { name: "결정" }));
    expect(screen.getByRole("complementary", { name: "활동" })).toHaveTextContent("빈 입력 처리를 추가");
  });
});
