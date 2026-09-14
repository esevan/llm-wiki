import type { HttpMethod } from "../types/application";

type Operation = { name: string; input: Record<string, unknown> };

/** URL identifiers always override request bodies at the native trust boundary. */
export function taskOperation(
  method: HttpMethod,
  path: string,
  body: Record<string, unknown>,
  locale: string,
): Operation | undefined {
  const p = path.split("/").slice(1).map(decodeURIComponent);
  const op = (name: string, ids: Record<string, unknown> = {}): Operation => ({
    name,
    input: { ...body, ...ids, locale },
  });
  if (path === "/workbench" && method === "GET") return op("workbench.get");
  if (path === "/tasks" && method === "POST") return op("task.create");
  if (path === "/problems" && method === "POST") return op("problem.create");
  if (p[0] === "problems" && p.length === 3 && method === "POST") {
    if (p[2] === "revisions")
      return op("problem.revision", {
        operationId: body.operationId ?? globalThis.crypto?.randomUUID?.() ??
          `operation-${Date.now()}-${Math.random().toString(16).slice(2)}`,
        problemId: p[1],
      });
    if (p[2] === "resolutions")
      return op("problem.resolution.create", { problemId: p[1] });
  }
  if (
    (p[0] === "captures" || p[0] === "tasks") &&
    p.length === 3 &&
    p[2] === "refinement"
  ) {
    const subject =
      p[0] === "captures" ? { captureId: p[1] } : { taskId: p[1] };
    if (method === "GET" || method === "POST")
      return op(
        method === "GET" ? "task-refinement.get" : "task-refinement.open",
        subject,
      );
  }
  if (p[0] === "refinement" && p.length === 3) {
    const names: Record<string, string> = {
      "POST messages": "message",
      "PUT workspace": "workspace",
      "GET proposals": "proposals",
      "POST proposal-decisions": "decision",
    };
    const name = names[`${method} ${p[2]}`];
    if (name) return op(`task-refinement.${name}`, { sessionId: p[1] });
  }
  if (p[0] === "tasks" && p[1]) {
    const ids = { taskId: p[1] };
    if (p.length === 2 && method === "GET") return op("task.get", ids);
    if (p.length === 2 && method === "DELETE") return op("task.delete", ids);
    if (p.length === 3) {
      const names: Record<string, string> = {
        "POST revisions": "revision",
        "POST transitions": "transition",
        "POST problem-links": "problem-link.create",
        "POST relationships": "relationship.create",
        "GET readiness": "readiness.get",
        "POST readiness-decisions": "readiness.decision",
        "GET work-log": "work-log.get",
        "POST work-log": "work-log.create",
        "POST checklist": "checklist.create",
        "POST decisions": "decision.create",
        "POST completions": "completion.create",
        "GET lineage": "lineage",
      };
      const name = names[`${method} ${p[2]}`];
      if (name) return op(`task.${name}`, ids);
    }
    if (p.length === 4) {
      if (p[2] === "problem-links" && method === "DELETE")
        return op("task.problem-link.delete", { ...ids, linkId: p[3] });
      if (p[2] === "relationships" && method === "DELETE")
        return op("task.relationship.delete", { ...ids, relationshipId: p[3] });
      if (p[2] === "checklist" && method === "PUT")
        return op("task.checklist.update", { ...ids, itemId: p[3] });
      if (p[2] === "knowledge" && p[3] === "drafts" && method === "POST")
        return {
          name: "jobs.enqueue",
          input: { ...body, ...ids, taskKind: "knowledge_draft", entityType: "tasks", entityId: p[1], locale },
        };
    }
    if (
      p.length === 6 &&
      p[2] === "knowledge" &&
      p[3] === "drafts" &&
      method === "POST" &&
      ["publish", "regenerate", "withdraw", "correction"].includes(p[5])
    ) {
      return op(`task-knowledge.${p[5]}`, {
        ...ids,
        draftRevision: Number(p[4]),
      });
    }
  }
  if (
    p[0] === "work-log" &&
    p.length === 3 &&
    p[2] === "comments" &&
    method === "POST"
  )
    return op("work-log.comment.create", { entryId: p[1] });
  if (path === "/conflict-reviews" && method === "POST")
    return op("task-review.create");
  if (path === "/conflict-reviews" && method === "GET")
    return op("task-review.history");
  if (p[0] === "conflict-reviews" && p.length === 2 && method === "GET")
    return op("task-review.get", { runId: p[1] });
  if (
    p[0] === "conflict-reviews" &&
    p.length === 3 &&
    p[2] === "cancel" &&
    method === "POST"
  )
    return op("task-review.cancel", { runId: p[1] });
  if (
    p[0] === "conflict-findings" &&
    p.length === 3 &&
    p[2] === "decisions" &&
    method === "POST"
  )
    return op("task-review.decision", { findingId: p[1] });
  return undefined;
}
