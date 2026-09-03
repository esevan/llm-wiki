import http from "node:http";

function deterministicResult(prompt) {
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
  response.writeHead(status, { "content-type": "application/json", "content-length": Buffer.byteLength(body) });
  response.end(body);
}

const requestedPort = Number.parseInt(process.argv[process.argv.indexOf("--port") + 1] ?? "0", 10);
if (!Number.isInteger(requestedPort) || requestedPort < 0) throw new Error("Pass a valid --port value.");

const server = http.createServer((request, response) => {
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
    if (payload.model === "deterministic-failure") {
      sendJson(response, 400, { error: { message: "deterministic provider failure" } });
      return;
    }
    const reply = () => {
      if (payload.stream) {
        response.writeHead(200, { "content-type": "text/event-stream" });
        for (const text of ["Deterministic ", "desktop response"]) {
          response.write(`data: ${JSON.stringify({ choices: [{ delta: { content: text } }] })}\n\n`);
        }
        response.end("data: [DONE]\n\n");
        return;
      }
      const prompt = (payload.messages ?? []).map((message) => String(message.content ?? "")).join("\n");
      sendJson(response, 200, { choices: [{ message: { content: JSON.stringify(deterministicResult(prompt)) } }] });
    };
    if (payload.model === "deterministic-timeout") setTimeout(reply, 600);
    else reply();
  });
});

server.listen(requestedPort, "127.0.0.1", () => {
  const address = server.address();
  process.stdout.write(`${typeof address === "object" ? address.port : requestedPort}\n`);
});

for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => server.close(() => process.exit(0)));
