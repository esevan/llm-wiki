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
