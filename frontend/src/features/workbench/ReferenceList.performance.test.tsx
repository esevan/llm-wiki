import { act, fireEvent, render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ExactReferenceBinding } from "../../types/taskWorkbench";
import { ReferenceList } from "./ReferenceList";

const sampleCount = 25;

const references: ExactReferenceBinding[] = Array.from(
  { length: 1_000 },
  (_, index) => {
    const id = String(index).padStart(4, "0");
    return {
      documentId: `knowledge-${id}`,
      documentVersion: `sha256:${id}`,
      title: `Source ${id}`,
      path: `Knowledge/Source-${id}.md`,
      section: `Decision ${id}`,
      excerpt: `Exact reference excerpt ${id}`,
      status: index % 5 === 0 ? "historical" : "current",
      aspect: index % 3 === 0 ? "decision" : "applicability",
      informationType: index % 4 === 0 ? "reference" : "knowledge",
      claimIds: index % 2 === 0 ? [`claim-${id}`] : [],
    };
  },
);

const percentile95 = (samples: number[]) => {
  const ordered = [...samples].sort((left, right) => left - right);
  return ordered[Math.ceil(ordered.length * 0.95) - 1];
};

describe("ReferenceList performance", () => {
  it("keeps 1,000-reference mount and filter-sort commits within 100 ms at p95", () => {
    const mountSamples: number[] = [];
    const filterSortSamples: number[] = [];

    for (let sample = 0; sample < sampleCount; sample += 1) {
      const mountStartedAt = performance.now();
      const view = render(
        <ReferenceList references={references} onOpen={vi.fn()} />,
      );
      mountSamples.push(performance.now() - mountStartedAt);
      expect(
        view.container.querySelectorAll(".reference-document-group"),
      ).toHaveLength(20);

      const filterSortStartedAt = performance.now();
      act(() => {
        fireEvent.change(view.getByLabelText("Search references"), {
          target: { value: "Source 09" },
        });
        fireEvent.change(view.getByLabelText("Reference order"), {
          target: { value: "title" },
        });
      });
      filterSortSamples.push(performance.now() - filterSortStartedAt);
      expect(
        view.container.querySelectorAll(".reference-document-group"),
      ).toHaveLength(20);
      expect(view.getByText("Page 1 / 5 · 100 documents")).toBeVisible();

      view.unmount();
      view.container.remove();
    }

    const mountP95 = percentile95(mountSamples);
    const filterSortP95 = percentile95(filterSortSamples);
    console.info(
      `ReferenceList samples=${sampleCount} mount-p95=${mountP95.toFixed(2)}ms filter-sort-p95=${filterSortP95.toFixed(2)}ms`,
    );
    expect(mountP95).toBeLessThan(100);
    expect(filterSortP95).toBeLessThan(100);
  });
});
