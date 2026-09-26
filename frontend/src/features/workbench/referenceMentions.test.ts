import { describe, expect, it } from "vitest";
import type { DocumentMention } from "../../types/taskWorkbench";
import {
  deleteMentionRange,
  positionMentions,
  reconcileMentionEdit,
  removeMention,
} from "./referenceMentions";
const mention = (id: string): DocumentMention => ({
  mentionId: id,
  displayLabel: "Same",
  documentId: id,
  documentVersion: "v1",
  title: "Same",
});
describe("Exact mention editing", () => {
  it("removes the selected duplicate binding atomically and preserves the other identity", () => {
    const before = "Use @Same and @Same.";
    const mentions = positionMentions(before, [
      mention("first"),
      mention("second"),
    ]);
    const removed = removeMention(before, mentions, "second");
    expect(removed.value).toBe("Use @Same and .");
    expect(removed.mentions.map((item) => item.documentId)).toEqual(["first"]);
    const backwards = deleteMentionRange(before, mentions, 9, 9, "Backspace")!;
    expect(backwards.value).toBe("Use  and @Same.");
    expect(backwards.mentions[0]).toMatchObject({
      documentId: "second",
      start: 9,
      end: 14,
    });
    const forward = deleteMentionRange(before, mentions, 4, 4, "Delete")!;
    expect(forward.value).toBe(backwards.value);
  });
  it("keeps pasted labels plain and drops bindings changed by free text edits", () => {
    const before = "Use @Same.";
    const mentions = positionMentions(before, [mention("original")]);
    expect(
      reconcileMentionEdit(before, `${before} @Same`, mentions),
    ).toHaveLength(1);
    expect(reconcileMentionEdit(before, "Use @Sme.", mentions)).toEqual([]);
    expect(
      reconcileMentionEdit(before, `First: ${before}`, mentions)[0],
    ).toMatchObject({ start: 11, end: 16 });
  });
  it("expands a partial selection across whole tokens while preserving surrounding text", () => {
    const text = "before @Same between @Same after";
    const result = deleteMentionRange(
      text,
      [mention("one"), mention("two")],
      9,
      24,
      "Delete",
    )!;
    expect(result.value).toBe("before  after");
    expect(result.mentions).toEqual([]);
    expect(result.caret).toBe(7);
  });
});
