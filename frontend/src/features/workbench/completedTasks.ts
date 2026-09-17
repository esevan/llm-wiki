import type { TaskCard, WorkbenchItem } from "../../types/taskWorkbench";

export const completedTasksNewestFirst = (items: WorkbenchItem[]): TaskCard[] =>
  items.filter((item): item is TaskCard => item.kind === "task" && item.state === "completed")
    .sort((a, b) => (Date.parse(b.completedAt ?? "") || 0) - (Date.parse(a.completedAt ?? "") || 0) || a.id.localeCompare(b.id));
