import { describe, expect, it } from "vitest";
import { acknowledgeSave, baseline, definitionFields, definitionOf, editDraft, keepEdits, mergeSnapshot } from "./taskDraft";
import type { TaskAggregate } from "../../types/taskWorkbench";
const task: TaskAggregate = { id: "one", kind: "task", state: "in_progress", taskRevision: 2, title: "Task", detail: "Original", workLog: [] };
describe("Task definition draft", () => {
  it("preserves all six dirty fields while merging Work Log and normalizing absent fields", () => {
    let current = baseline(task);
    for (const field of definitionFields) current = editDraft(current, field, `Edited ${field}`);
    const next = mergeSnapshot(current, { ...task, workLog: [{ id: "entry", body: "Evidence", comments: [] }] });
    expect(next.draft).toEqual(Object.fromEntries(definitionFields.map((field) => [field, `Edited ${field}`])));
    expect(next.conflicts).toEqual([]);
    expect(next.persisted.workLog).toHaveLength(1);
  });
  it("rebases only disjoint changes and resets reverted fields", () => {
    const edited = editDraft(baseline(task), "detail", "Local");
    const next = mergeSnapshot(edited, { ...task, taskRevision: 3, title: "External title" });
    expect(next.baseRevision).toBe(3);
    expect(next.draft).toMatchObject({ title: "External title", detail: "Local" });
    expect(editDraft(next, "detail", "Original").dirty.size).toBe(0);
  });
  it("preserves base and dirty values on overlap until explicit review approval", () => {
    const edited = editDraft(baseline(task), "detail", "Local");
    const next = mergeSnapshot(edited, { ...task, taskRevision: 3, detail: "External", title: "Latest title" });
    expect(next.baseRevision).toBe(2);
    expect(next.draft.detail).toBe("Local");
    expect(next.conflicts).toEqual(["detail"]);
    const furtherEdit = editDraft(next, "scope", "New scope");
    expect(furtherEdit.conflicts).toEqual(["detail"]);
    expect(furtherEdit.baseRevision).toBe(2);
    const approved = keepEdits(furtherEdit);
    expect(approved.baseRevision).toBe(3);
    expect(approved.draft).toMatchObject({ title: "Latest title", detail: "Local", scope: "New scope" });
    expect(approved.conflicts).toEqual([]);
  });
  it("acknowledges normalized partial saves without dropping aggregate data and merges a later writer", () => {
    const current = editDraft(baseline(task), "detail", "  Local  ");
    const saved = acknowledgeSave(current, { ...definitionOf(task), detail: "Local", id: task.id, taskRevision: 3 });
    expect(saved.draft.detail).toBe("Local");
    expect(saved.persisted.state).toBe("in_progress");
    expect(saved.persisted.workLog).toEqual([]);
    expect(saved.dirty.size).toBe(0);
    const nextEdit = editDraft(saved, "detail", "Next local edit");
    const refreshed = mergeSnapshot(nextEdit, { ...task, taskRevision: 4, detail: "Another writer" });
    expect(refreshed.draft.detail).toBe("Next local edit");
    expect(refreshed.baseRevision).toBe(3);
    expect(refreshed.conflicts).toEqual(["detail"]);
  });
  it("ignores snapshots from an older revision or another Task", () => {
    const current = baseline(task);
    expect(mergeSnapshot(current, { ...task, taskRevision: 1 })).toBe(current);
    expect(mergeSnapshot(current, { ...task, id: "other" })).toBe(current);
  });
});

it("acknowledges the captured save base while retaining a snapshot observed during the request", () => {
  const submitted = editDraft(baseline(task), "detail", "Local");
  const newer = { ...task, taskRevision: 4, detail: "Subsequent edit", workLog: [{ id: "later", body: "Evidence", comments: [] }] };
  const result = acknowledgeSave(submitted, { ...definitionOf(task), detail: "Local", id: task.id, taskRevision: 3 }, newer);
  expect(result.persisted.taskRevision).toBe(4);
  expect(result.persisted.workLog).toHaveLength(1);
  expect(result.draft.detail).toBe("Subsequent edit");
  expect(result.dirty.size).toBe(0);
});
