import type { TaskAggregate } from "../../types/taskWorkbench";

export const definitionFields = ["title", "detail", "outcome", "scope", "nonGoals", "validationCriteria"] as const;
export type DefinitionField = typeof definitionFields[number];
export type TaskDefinition = Record<DefinitionField, string>;
export type TaskRevisionResult = TaskDefinition & { id: string; taskRevision: number };
export type DetailState = {
  persisted: TaskAggregate;
  draft: TaskDefinition;
  baseSnapshot: TaskDefinition;
  baseRevision: number;
  dirty: Set<DefinitionField>;
  conflicts: DefinitionField[];
};
export const definitionOf = (task: Partial<TaskDefinition>): TaskDefinition =>
  Object.fromEntries(definitionFields.map((field) => [field, task[field] ?? ""])) as TaskDefinition;
const dirtyFields = (draft: TaskDefinition, base: TaskDefinition) =>
  new Set(definitionFields.filter((field) => draft[field] !== base[field]));
export const baseline = (task: TaskAggregate): DetailState => {
  const definition = definitionOf(task);
  return { persisted: task, draft: definition, baseSnapshot: definition, baseRevision: task.taskRevision, dirty: new Set(), conflicts: [] };
};
/** Explicitly approve keeping local edits against the displayed saved version. */
export function keepEdits(current: DetailState): DetailState {
  const baseSnapshot = definitionOf(current.persisted);
  const draft = { ...baseSnapshot };
  for (const field of current.dirty) draft[field] = current.draft[field];
  return { ...current, draft, baseSnapshot, baseRevision: current.persisted.taskRevision, dirty: dirtyFields(draft, baseSnapshot), conflicts: [] };
}
export function mergeSnapshot(current: DetailState, next: TaskAggregate): DetailState {
  if (next.id !== current.persisted.id || next.taskRevision < current.persisted.taskRevision) return current;
  const incoming = definitionOf(next);
  const conflicts = [...current.dirty].filter((field) => incoming[field] !== current.baseSnapshot[field]);
  if (conflicts.length) return { ...current, persisted: next, conflicts };
  return keepEdits({ ...current, persisted: next });
}
export function editDraft(current: DetailState, field: DefinitionField, value: string): DetailState {
  const draft = { ...current.draft, [field]: value };
  // Re-evaluate against the last full snapshot; editing one field must not clear
  // another field's conflict or silently authorize an overlapping rebase.
  return mergeSnapshot({ ...current, draft, dirty: dirtyFields(draft, current.baseSnapshot) }, current.persisted);
}
export function acknowledgeSave(current: DetailState, saved: TaskRevisionResult, latestPersisted = current.persisted): DetailState {
  // Revision responses contain all six normalized fields, but no aggregate data.
  // Validate before treating anything as durable, rather than trusting a cast.
  if (saved.id !== current.persisted.id || saved.taskRevision !== current.baseRevision + 1 ||
      !definitionFields.every((field) => typeof saved[field] === "string")) {
    throw new Error("Invalid Task revision response");
  }
  const acknowledged = baseline({ ...latestPersisted, ...saved });
  // A GET may have completed during the save. Preserve that newer knowledge.
  return latestPersisted.taskRevision > saved.taskRevision
    ? mergeSnapshot(acknowledged, latestPersisted)
    : acknowledged;
}
