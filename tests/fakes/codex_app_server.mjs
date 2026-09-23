#!/usr/bin/env node

// Deterministic, local-only app-server used by the opt-in packaged execution
// scenario. It speaks the JSONL protocol the native adapter supervises.
import readline from "node:readline";
import { appendFileSync } from "node:fs";

const externalThread = {
  turns: 1,
  pending: null,
  history: [{
    id: "fixture-external-turn-1", status: "completed", startedAt: 1700000010, completedAt: 1700000020,
    items: [
      { id: "fixture-external-user", type: "userMessage", content: [{ type: "text", text: "Continue imported work" }] },
      { id: "fixture-external-command", type: "commandExecution", status: "completed", command: "fixture-history-check", cwd: process.cwd(), aggregatedOutput: "history ok", exitCode: 0, commandActions: [] },
      { id: "fixture-external-agent", type: "agentMessage", text: "Imported history complete" },
    ], itemsView: "full",
  }],
};
const threads = new Map([["fixture-external-thread", externalThread]]);
let nextThread = 1;
let nextTurn = 1;
let nextRequest = 1;

const write = (value) => process.stdout.write(`${JSON.stringify(value)}\n`);
const response = (id, result) => write({ id, result });

function completeTurn(threadId, turnId, ordinal) {
  write({ method: "item/completed", params: {
    threadId, turnId, completedAtMs: ordinal,
    item: { id: `fixture-item-${threadId}-${turnId}`, type: "agentMessage", status: "completed", phase: "final_answer", text: `Controlled Codex fixture completed turn ${ordinal}.` },
  }});
  write({ method: "item/completed", params: {
    threadId, turnId, completedAtMs: ordinal + 1,
    item: { id: `fixture-evidence-${threadId}-${turnId}`, type: "commandExecution", status: "completed", command: "fixture-check", cwd: process.cwd(), aggregatedOutput: "controlled fixture evidence", exitCode: 0 },
  }});
  write({ method: "turn/completed", params: { threadId, turnId, turn: { id: turnId, status: "completed" } } });
}

function askApproval(threadId, turnId) {
  const id = `fixture-approval-${nextRequest++}`;
  threads.get(threadId).pending = { kind: "approval", id, turnId };
  write({ id, method: "item/commandExecution/requestApproval", params: {
    threadId, turnId, itemId: `fixture-command-${turnId}`, command: "fixture-check", cwd: process.cwd(), reason: "The controlled fixture requests one explicit approval.", availableDecisions: ["accept", "decline"], isBlocking: true,
  }});
}

function askQuestions(threadId, turnId) {
  const id = `fixture-question-${nextRequest++}`;
  threads.get(threadId).pending = { kind: "question", id, turnId };
  write({ id, method: "item/tool/requestUserInput", params: {
    threadId, turnId, itemId: `fixture-question-item-${turnId}`, reason: "The controlled fixture requests structured answers.", isBlocking: true,
    questions: [
      { id: "fixture-choice", header: "Mode", question: "Choose the controlled mode.", options: [{ label: "Proceed", description: "Continue the fixture turn." }], isOther: false, isSecret: false },
      { id: "fixture-text", header: "Note", question: "Add a short note.", options: [], isOther: false, isSecret: false },
    ],
  }});
}

function startTurn(threadId, input) {
  const state = threads.get(threadId);
  const turnId = `fixture-turn-${nextTurn++}`;
  state.turns += 1;
  response(input.id, { turn: { id: turnId, threadId, status: "inProgress" } });
  if (state.turns === 1) askApproval(threadId, turnId);
  else completeTurn(threadId, turnId, state.turns);
}

const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on("line", (line) => {
  let input;
  try { input = JSON.parse(line); } catch { return; }
  if (process.env.LLM_WIKI_FAKE_CODEX_LOG && input.method) appendFileSync(process.env.LLM_WIKI_FAKE_CODEX_LOG, `${input.method}\n`);
  if (input.method === "initialize") {
    response(input.id, { capabilities: { experimentalApi: true } });
  } else if (input.method === "initialized") {
    // notification
  } else if (input.method === "thread/start") {
    const threadId = `fixture-thread-${nextThread++}`;
    threads.set(threadId, { turns: 0, pending: null });
    response(input.id, { thread: { id: threadId }, model: input.params?.model ?? "gpt-5.6-luna", cwd: input.params?.cwd ?? process.cwd(), approvalPolicy: "on-request", approvalsReviewer: input.params?.approvalsReviewer ?? "user", sandbox: { type: "workspaceWrite" } });
  } else if (input.method === "thread/list") {
    response(input.id, { data: [{ id: "fixture-external-thread", name: "Fixture external conversation", preview: "Continue imported work", cwd: process.cwd(), model: "gpt-5.6-sol", modelProvider: "openai", source: "vscode", status: { type: "idle" }, ephemeral: false, createdAt: 1700000000, updatedAt: 1700000020, turns: [] }], nextCursor: null });
  } else if (input.method === "thread/read") {
    const threadId = input.params?.threadId;
    if (threadId !== "fixture-external-thread") response(input.id, null);
    else response(input.id, { thread: { id: threadId, name: "Fixture external conversation", preview: "Continue imported work", cwd: process.cwd(), model: "gpt-5.6-sol", modelProvider: "openai", source: "vscode", status: { type: "idle" }, ephemeral: false, createdAt: 1700000000, updatedAt: 1700000020, turns: input.params?.includeTurns ? externalThread.history : [] } });
  } else if (input.method === "thread/turns/list") {
    const history = input.params?.threadId === "fixture-external-thread" ? [...externalThread.history].reverse() : [];
    response(input.id, { data: history, nextCursor: null });
  } else if (input.method === "thread/resume") {
    const threadId = input.params?.threadId;
    if (!threads.has(threadId)) threads.set(threadId, { turns: 0, pending: null });
    response(input.id, { thread: { id: threadId }, model: input.params?.model ?? "gpt-5.6-luna", cwd: input.params?.cwd ?? process.cwd(), approvalPolicy: "on-request", approvalsReviewer: input.params?.approvalsReviewer ?? "user", sandbox: { type: "workspaceWrite" } });
  } else if (input.method === "turn/start") {
    startTurn(input.params?.threadId, input);
  } else if (input.method === "turn/interrupt") {
    response(input.id, { turn: { id: input.params?.turnId, threadId: input.params?.threadId, status: "interrupted" } });
    write({ method: "turn/completed", params: { threadId: input.params?.threadId, turnId: input.params?.turnId, turn: { id: input.params?.turnId, status: "interrupted" } } });
  } else if (input.id && input.method === undefined) {
    for (const [threadId, state] of threads) {
      if (!state.pending || state.pending.id !== String(input.id)) continue;
      const pending = state.pending;
      state.pending = null;
      response(input.id, {});
      if (pending.kind === "approval") askQuestions(threadId, pending.turnId);
      else completeTurn(threadId, pending.turnId, state.turns);
      break;
    }
  }
});
