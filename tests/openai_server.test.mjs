import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { test } from "node:test";

async function startFakeProvider() {
  const child = spawn(process.execPath, ["tests/fakes/openai_server.mjs", "--port", "0"], {
    cwd: process.cwd(),
    stdio: ["ignore", "pipe", "pipe"],
  });
  const [chunk] = await once(child.stdout, "data");
  return { child, port: Number.parseInt(String(chunk).trim(), 10) };
}

test("current refinement protocol preserves Task identity and only splits on an explicit request", async (context) => {
  const { child, port } = await startFakeProvider();
  context.after(() => child.kill("SIGTERM"));
  const preview = async (session) => {
    const response = await fetch(`http://127.0.0.1:${port}/v1/chat/completions`, {
      method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify({ model: "deterministic-test-model", messages: [{ role: "user", content:
        `Return JSON with type "task_patch|new_task|subtask|problem_snapshot|task_problem_link".\n\n${JSON.stringify(session)}` }] }),
    });
    return JSON.parse((await response.json()).choices[0].message.content).proposals[0];
  };
  assert.equal((await preview({ messages: [] })).type, "new_task");
  const taskSnapshot = { id: "parent", taskRevision: 7 };
  const patch = await preview({ taskSnapshot, messages: [] });
  assert.equal(patch.type, "task_patch");
  assert.equal(patch.payload.expectedTaskRevision, 7);
  const split = await preview({ taskSnapshot, messages: [{ body: "Split a deterministic Subtask" }] });
  assert.equal(split.type, "subtask");
  assert.equal(split.payload.parentTaskId, "parent");
  assert.equal(split.payload.expectedTaskRevision, 7);
});

test("multimodal image-summary prompts return both localized summaries", async (context) => {
  const { child, port } = await startFakeProvider();
  context.after(() => child.kill("SIGTERM"));

  const response = await fetch(`http://127.0.0.1:${port}/v1/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      model: "deterministic-test-model",
      messages: [{
        role: "user",
        content: [
          { type: "text", text: 'Return JSON matching {"ko":{"summary":string},"en":{"summary":string}}.' },
          { type: "image_url", image_url: { url: "data:image/png;base64,iVBORw0KGgo=" } },
        ],
      }],
    }),
  });
  assert.equal(response.status, 200);
  const payload = await response.json();
  const result = JSON.parse(payload.choices[0].message.content);
  assert.equal(result.ko.summary, "결정론적 이미지 요약");
  assert.equal(result.en.summary, "Deterministic image summary");
});

test("fake provider retains complete ordered request history and has deterministic retry failures", async (context) => {
  const { child, port } = await startFakeProvider();
  context.after(() => child.kill("SIGTERM"));
  const request = (model, messages) => fetch(`http://127.0.0.1:${port}/v1/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ model, messages }),
  });

  const turns = [
    { role: "user", content: "Keep the existing Solution." },
    { role: "assistant", content: "Which evidence is missing?" },
    { role: "user", content: "Correction: retain the validation criteria." },
  ];
  assert.equal((await request("deterministic-test-model", turns)).status, 200);
  const failed = await request("deterministic-failure-once", turns);
  assert.equal(failed.status, 503);
  assert.equal((await request("deterministic-failure-once", turns)).status, 200);
  const malformed = await request("deterministic-malformed", turns);
  assert.equal(malformed.status, 200);
  const malformedPayload = await malformed.json();
  assert.throws(() => JSON.parse(malformedPayload.choices[0].message.content));
  const history = await (await fetch(`http://127.0.0.1:${port}/__test/requests`)).json();
  assert.equal(history.requests.length, 4);
  assert.deepEqual(history.requests[0].messages, turns);
  assert.deepEqual(history.requests[3].messages, turns);
});
