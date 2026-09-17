import type { TaskContentVersions } from "../../types/taskWorkbench";

/** Display-only projection. Never submit this object as an edited Task definition. */
export function localizedTask<T extends { contentVersions?: TaskContentVersions }>(task: T): T {
  const locale = document.documentElement.lang.startsWith("ko") ? "ko" : "en";
  return { ...task, ...task.contentVersions?.[locale] };
}
