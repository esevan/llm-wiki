#!/usr/bin/env node
// Compare instruction bytes only; do not estimate tokens or model latency.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const baseline = process.argv[2];
if (!baseline || process.argv.length !== 3) {
  throw new Error('Usage: node scripts/measure_generation_prompts.mjs <baseline-ref>');
}

function instruction(source, promptId) {
  const start = source.indexOf(`PromptId::${promptId}`);
  if (start < 0) throw new Error(`Prompt not found: ${promptId}`);
  const match = source.slice(start).match(/r#"([\s\S]*?)"#/);
  if (!match) throw new Error(`Raw prompt not found: ${promptId}`);
  return match[1];
}

const prompts = [
  ['current', 'KnowledgeDraft'],
  ['reference_aware_workbench', 'WorkPreviewFinalizer'],
].map(([module, promptId]) => {
  const path = `src-tauri/src/workflow_foundation/prompts/${module}.rs`;
  const before = instruction(execFileSync('git', ['show', `${baseline}:${path}`], { cwd: root, encoding: 'utf8' }), promptId);
  const after = instruction(readFileSync(resolve(root, path), 'utf8'), promptId);
  return {
    prompt: promptId,
    before_utf8_bytes: Buffer.byteLength(before),
    after_utf8_bytes: Buffer.byteLength(after),
    delta_utf8_bytes: Buffer.byteLength(after) - Buffer.byteLength(before),
    delta_json_string_bytes: Buffer.byteLength(JSON.stringify(after)) - Buffer.byteLength(JSON.stringify(before)),
  };
});
console.log(JSON.stringify({ baseline, measure: 'instruction bytes, not tokens or latency', prompts }, null, 2));
