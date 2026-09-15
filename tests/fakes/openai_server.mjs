import http from "node:http";

/** Visible only to deterministic test clients; never used by the application. */
const requestHistory = [];
const modelAttempts = new Map();

function messageText(content) {
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return String(content ?? "");
  return content
    .map((part) => {
      if (typeof part === "string") return part;
      return part?.type === "text" ? String(part.text ?? "") : "";
    })
    .filter(Boolean)
    .join("\n");
}

function deterministicResult(prompt) {
  if (prompt.includes('Return JSON only as {"message":string}')) return { message: "I will use those details to update the preview in the background." };
  if (prompt.includes('"task_patch|new_task|problem_snapshot|task_problem_link"')) {
    return {
      message: "I prepared one reviewable Task proposal from the saved conversation.",
      proposals: [{
        id: "deterministic-task-proposal",
        type: "new_task",
        payload: { title: "Refined deterministic Task", outcome: "The recorded request is handled." },
      }],
    };
  }
  if (prompt.includes('"status":"clear|findings|insufficient_evidence"')) {
    const path = prompt.match(/"path"\s*:\s*"([^"]+\.md)"/)?.[1];
    if (!path) return { status: "insufficient_evidence", citations: [], findings: [] };
    return {
      status: "findings",
      citations: [{ path }],
      findings: [{ id: "deterministic-finding", path, summary: "The cited local evidence needs a user decision." }],
    };
  }
  if (prompt.includes("Improve readability using only this evidence-bound Task document")) {
    const taskId = prompt.match(/exact Task id `([^`]+)`/)?.[1] ?? "unknown-task";
    return { markdown: `# Evidence-backed Task result\n\nTask \`${taskId}\` preserves its completion evidence and lineage.` };
  }
  if (prompt.includes("executive_summary_markdown")) {
    return {
      executive_summary_markdown: "# Deterministic completion summary",
      report_body_markdown: "Verified from command-path evidence.",
    };
  }
  if (prompt.includes("problem_recommendation")) {
    return {
      resolution: "complete",
      executive_summary: "The recorded evidence meets the criterion.",
      what_changed: ["Command behavior was verified."],
      criteria_review: [{ criterion: "Verified", status: "met", evidence: "Command evidence" }],
      remaining_checklist: [],
      decision_rationale: "All supplied evidence is complete.",
      problem_recommendation: "complete",
      capture_recommendation: "complete",
    };
  }
  if (prompt.includes('"markdown":string')) return { markdown: "# 기존 맥락\n\n재사용 가능한 증거." };
  if (prompt.includes('"ko":string,"en":string')) return { ko: "결정론적 번역", en: "Deterministic translation" };
  if (prompt.includes('"ko":{"summary":string}')) {
    return { ko: { summary: "결정론적 이미지 요약" }, en: { summary: "Deterministic image summary" } };
  }
  if (prompt.includes('"validation_criteria":string')) {
    return {
      ko: { title: "결정론적 솔루션", outcome: "명령 경로가 동작합니다", non_goals: "없음", validation_criteria: "- [ ] 검증됨" },
      en: { title: "Deterministic solution", outcome: "The command path works", non_goals: "None", validation_criteria: "- [ ] Verified" },
    };
  }
  if (prompt.includes("clear problem statement")) {
    return { ko: { title: "명확한 문제", detail: "구조화된 문제 세부 정보" }, en: { title: "Clear problem", detail: "Structured problem detail" } };
  }
  if (prompt.includes('"ko":{"title":string,"detail":string}')) {
    return { ko: { title: "정제된 문제", detail: "정제된 세부 정보" }, en: { title: "Refined problem", detail: "Refined detail" } };
  }
  if (prompt.includes('"title":"refined capture"')) return { title: "Refined deterministic Capture" };
  if (prompt.includes('"entries"') && prompt.includes("attention_rank")) return { entries: [] };
  if (prompt.includes('"claims"') && prompt.includes("evidence_ids")) return { claims: [] };
  if (prompt.includes("Review exactly one Vault passage")) {
    const evidenceId = prompt.match(/"evidence"\s*:\s*\{.*?"id"\s*:\s*"([^"]+)"/s)?.[1] ?? "";
    return {
      conflict: true,
      evidence_id: evidenceId,
      severity: "medium",
      category: "Command compatibility",
      summary: "The deterministic evidence requires a user decision.",
      current_claim: "The command path is covered.",
      existing_claim: "Reusable evidence must remain authoritative.",
      impact: "A transport migration could otherwise alter behavior.",
      recommendation: "Preserve the existing evidence contract.",
      explanation: "The supplied claims require explicit reconciliation.",
    };
  }
  if (prompt.includes("Screen all evidence candidates")) return { decisions: [] };
  if (prompt.includes('"conflicts"') || prompt.includes('"findings"')) return { conflicts: [], summary: "No deterministic conflicts." };
  return {};
}

function sendJson(response, status, value) {
  const body = JSON.stringify(value);
  response.writeHead(status, { "access-control-allow-origin": "*", "content-type": "application/json", "content-length": Buffer.byteLength(body) });
  response.end(body);
}

const requestedPort = Number.parseInt(process.argv[process.argv.indexOf("--port") + 1] ?? "0", 10);
if (!Number.isInteger(requestedPort) || requestedPort < 0) throw new Error("Pass a valid --port value.");

const server = http.createServer((request, response) => {
  if (request.method === "OPTIONS") {
    response.writeHead(204, { "access-control-allow-origin": "*", "access-control-allow-methods": "GET,POST,DELETE,OPTIONS", "access-control-allow-headers": "content-type" });
    response.end();
    return;
  }
  if (request.method === "GET" && request.url === "/__test/requests") {
    sendJson(response, 200, { requests: requestHistory });
    return;
  }
  if (request.method === "DELETE" && request.url === "/__test/requests") {
    requestHistory.length = 0;
    modelAttempts.clear();
    sendJson(response, 204, {});
    return;
  }
  if (request.method === "GET" && request.url === "/v1/models") {
    sendJson(response, 200, { data: [{ id: "deterministic-test-model" }] });
    return;
  }
  if (request.method !== "POST" || request.url !== "/v1/chat/completions") {
    sendJson(response, 404, { error: { message: "not found" } });
    return;
  }
  const chunks = [];
  request.on("data", (chunk) => chunks.push(chunk));
  request.on("end", () => {
    const payload = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
    requestHistory.push({
      model: payload.model,
      stream: Boolean(payload.stream),
      messages: payload.messages ?? [],
    });
    const attempts = (modelAttempts.get(payload.model) ?? 0) + 1;
    modelAttempts.set(payload.model, attempts);
    if (payload.model === "deterministic-failure") {
      sendJson(response, 400, { error: { message: "deterministic provider failure" } });
      return;
    }
    if (payload.model === "deterministic-failure-once" && attempts === 1) {
      sendJson(response, 503, { error: { message: "deterministic provider failure once" } });
      return;
    }
    if (payload.model === "deterministic-malformed") {
      sendJson(response, 200, { choices: [{ message: { content: "{not valid JSON" } }] });
      return;
    }
    const reply = () => {
      if (payload.stream) {
        response.writeHead(200, { "access-control-allow-origin": "*", "content-type": "text/event-stream" });
        for (const text of ["Deterministic ", "desktop response"]) {
          response.write(`data: ${JSON.stringify({ choices: [{ delta: { content: text } }] })}\n\n`);
        }
        response.end("data: [DONE]\n\n");
        return;
      }
      const prompt = (payload.messages ?? []).map((message) => messageText(message.content)).join("\n");
      sendJson(response, 200, { choices: [{ message: { content: JSON.stringify(deterministicResult(prompt)) } }] });
    };
    const previewRequest = (payload.messages ?? []).some(message => messageText(message.content).includes('"task_patch|new_task|problem_snapshot|task_problem_link"'));
    if (payload.model === "deterministic-slow-preview" && previewRequest) setTimeout(reply, 3000);
    else if (payload.model === "deterministic-timeout") setTimeout(reply, 600);
    else reply();
  });
});

server.listen(requestedPort, "127.0.0.1", () => {
  const address = server.address();
  process.stdout.write(`${typeof address === "object" ? address.port : requestedPort}\n`);
});

for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => server.close(() => process.exit(0)));
