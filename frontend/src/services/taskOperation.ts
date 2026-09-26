import type { HttpMethod } from "../types/application";

type Operation = { name: string; input: Record<string, unknown> };

/** URL identifiers always override request bodies at the native trust boundary. */
export function taskOperation(
  method: HttpMethod,
  path: string,
  body: Record<string, unknown>,
  locale: string,
): Operation | undefined {
  const p = path.split("/").slice(1).map(segment => decodeURIComponent(segment.split("?", 1)[0]));
  const op = (name: string, ids: Record<string, unknown> = {}): Operation => ({
    name,
    input: { ...body, ...ids, locale },
  });
  if (path === "/workbench" && method === "GET") return op("workbench.get");
  if (path === "/tasks" && method === "POST") return op("task.create");
  if (path === "/problems" && method === "POST") return op("problem.create");
  if (path === "/knowledge/archive/prepare" && method === "POST") return op("task-knowledge.archive-prepare");
  if (path === "/knowledge/archive/organize" && method === "POST") return op("task-knowledge.archive-organize");
  if (path === "/knowledge/archive/publish" && method === "POST") return op("task-knowledge.archive-publish");
  if (p[0] === "knowledge" && p[1] === "archive" && p[2] === "operations" && p[3]) {
    if (p.length === 4 && method === "GET") return op("task-knowledge.archive-status", { operationId: p[3] });
    if (p.length === 5 && p[4] === "retry" && method === "POST") return op("task-knowledge.archive-retry", { operationId: p[3] });
    if (p.length === 5 && p[4] === "recover" && method === "POST") return op("task-knowledge.archive-recover", { operationId: p[3] });
  }
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
  if (p[0] === "refinement" && p[2] === "reference-workspace" && method === "GET")
    return op("task-refinement.reference-workspace", { sessionId: p[1] });
  if (p[0] === "refinement" && p[2] === "preview-generations" && method === "POST")
    return op("task-refinement.reference-generate", { sessionId: p[1] });
  if (p[0] === "refinement" && p[2] === "investigations" && method === "POST")
    return op("task-refinement.reference-investigate", { sessionId: p[1] });
  if (p[0] === "refinement" && p[2] === "references" && p.length === 3 && method === "GET")
    return op("task-refinement.reference-list", { sessionId: p[1], query: new URL(path, "http://local").searchParams.get("query") ?? "" });
  if (p[0] === "refinement" && p[2] === "references" && p.length === 4 && p[3] === "open" && method === "POST")
    return op("task-refinement.reference-open", { sessionId: p[1] });
  if (p[0] === "refinement" && p[2] === "mention-draft" && method === "PUT")
    return op("task-refinement.mention-draft", { sessionId: p[1] });
  if (p[0] === "refinement" && p[2] === "preview-versions" && p.length === 6 && p[4] === "compare" && method === "GET")
    return op("task-refinement.reference-compare", { sessionId: p[1], left: Number(p[3]), right: Number(p[5]) });
  if (p[0] === "refinement" && p[2] === "preview-versions" && p.length === 4 && method === "GET")
    return op("task-refinement.reference-version", { sessionId: p[1], version: Number(p[3]) });
  if (p[0] === "refinement" && p[2] === "preview-versions" && p.length === 5 && p[4] === "edits" && method === "POST")
    return op("task-refinement.reference-edit", { sessionId: p[1], version: Number(p[3]) });
  if (p[0] === "refinement" && p[2] === "preview-versions" && p.length === 5 && p[4] === "restore" && method === "POST")
    return op("task-refinement.reference-restore", { sessionId: p[1], version: Number(p[3]) });
  if (p[0] === "refinement" && p[2] === "preview-versions" && p.length === 5 && p[4] === "apply" && method === "POST")
    return op("task-refinement.reference-apply", { sessionId: p[1], version: Number(p[3]) });
  if (p[0] === "refinement" && p[2] === "references" && p.length === 5 && p[4] === "usage" && method === "POST")
    return op("task-refinement.reference-usage", { sessionId: p[1], referenceId: p[3] });
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
        "POST distillation-repair": "distillation.repair",
        "GET work-sessions": "work-session.list",
        "POST work-sessions": "work-session.create",
      };
      const name = names[`${method} ${p[2]}`];
      if (name) return op(`task.${name}`, ids);
      if (p[2] === "knowledge" && method === "GET")
        return op("task-knowledge.get", ids);
    }
    if (
      p.length === 6 &&
      p[2] === "knowledge" &&
      p[3] === "versions" &&
      p[5] === "restore" &&
      method === "POST"
    ) {
      return op("task-knowledge.restore", {
        ...ids,
        revision: Number(p[4]),
      });
    }
    if (p.length === 4) {
      if (p[2] === "work-sessions" && method === "GET")
        return op("task.work-session.get", { ...ids, sessionId: p[3] });
      if (p[2] === "work-sessions" && method === "PUT")
        return op("task.work-session.update", { ...ids, sessionId: p[3] });
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
    if (p.length === 5 && p[2] === "work-sessions" && p[4] === "entries" && method === "POST")
      return op("task.work-session.entry.create", { ...ids, sessionId: p[3] });
    if (p.length === 5 && p[2] === "work-sessions" && p[4] === "runs") {
      if (method === "GET")
        return op("task.execution.list", { ...ids, sessionId: p[3] });
      if (method === "POST")
        return op("task.execution.start", { ...ids, sessionId: p[3] });
    }
    if (p.length === 6 && p[2] === "work-sessions" && p[4] === "runs") {
      if (method === "GET")
        return op("task.execution.get", { ...ids, sessionId: p[3], runId: p[5] });
    }
    if (p.length === 7 && p[2] === "work-sessions" && p[4] === "runs" && method === "POST") {
      if (p[6] === "interrupt")
        return op("task.execution.interrupt", { ...ids, sessionId: p[3], runId: p[5] });
      if (p[6] === "work-log-sync")
        return op("task.execution.work-log-sync", { ...ids, sessionId: p[3], runId: p[5] });
    }
    if (p.length === 9 && p[2] === "work-sessions" && p[4] === "runs" && p[6] === "formal-requests" && p[8] === "response" && method === "POST")
      return op("task.execution.formal-response", { ...ids, sessionId: p[3], runId: p[5], requestId: p[7] });
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
  if (p[0] === "work-log" && p.length === 3 && p[2] === "image-summary" && method === "POST")
    return { name: "jobs.enqueue", input: { taskKind: "image_summary", entityType: "task_work_log_entries", entityId: p[1], locale } };
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
