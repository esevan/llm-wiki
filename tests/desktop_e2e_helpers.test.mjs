import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { DESKTOP_E2E_SCENARIO_NAMES, parseDesktopE2eCases, redactedText, resultSummary, retainDiagnosticFiles } from "../scripts/desktop_e2e_helpers.mjs";

test("desktop E2E cases are isolated selectors with per-case timeouts", () => {
  assert.deepEqual(parseDesktopE2eCases({ env: { LLM_WIKI_E2E_SCENARIOS: "E2E-CAPTURE-01, E2E-REFINE-RESUME-01,E2E-CAPTURE-01", LLM_WIKI_E2E_TIMEOUT_MS: "2500", LLM_WIKI_E2E_TIMEOUT_MS_E2E_REFINE_RESUME_01: "4200" } }), [
    { name: "E2E-CAPTURE-01", timeoutMs: 2500 }, { name: "E2E-REFINE-RESUME-01", timeoutMs: 4200 },
  ]);
});

test("desktop E2E defaults to independent Task-centred scenario groups", () => {
  assert.deepEqual(parseDesktopE2eCases({ env: {} }).map((item) => item.name), [
    "task-capture", "task-worklog", "task-refinement", "task-relationships",
    "task-review", "task-publication", "task-problem-resolution", "task-persistence", "task-localization",
    "task-controls", "task-workbench-retry", "task-legacy-refinement", "task-legacy-chat-controls", "task-legacy-preview-retry", "task-chat-controls",
    "task-refinement-provider-recovery", "task-refinement-close-retry", "task-refinement-close-pending", "task-refinement-relaunch",
    "global-shell", "global-search", "global-compass", "global-provider", "global-mcp", "global-queue-notifications", "global-notice",
    "global-intro-navigation", "global-intro-retry", "global-vault-choose", "global-vault-retry", "global-migration-restore", "global-migration-retry",
  ]);
});

test("desktop E2E runner rejects malformed selectors and unusable timeouts", () => {
  assert.throws(() => parseDesktopE2eCases({ argv: ["--scenario", "../../state"] }), /Invalid desktop E2E scenario name/);
  assert.throws(() => parseDesktopE2eCases({ argv: ["--scenario", "missing-case"], validateRegistry: true }), /Unknown desktop E2E scenario/);
  assert.throws(() => parseDesktopE2eCases({ env: { LLM_WIKI_E2E_TIMEOUT_MS: "999" } }), /at least 1000ms/);
});

test("desktop E2E registry exposes the complete acceptance set", () => {
  assert.equal(DESKTOP_E2E_SCENARIO_NAMES.length, 32);
  assert.equal(new Set(DESKTOP_E2E_SCENARIO_NAMES).size, 32);
});

test("desktop E2E console summaries redact provider credentials", () => {
  assert.equal(redactedText("api_key=desktop-e2e-key"), "api_key=[REDACTED]");
  assert.equal(resultSummary({ status: "failed", error: "authorization: bearer-secret" }), "failed: authorization=[REDACTED]");
});

test("desktop E2E retains isolated state and logs when a case fails", async (context) => {
  const temporaryRoot = path.join(process.cwd(), ".tmp");
  await mkdir(temporaryRoot, { recursive: true });
  const temporary = await mkdtemp(path.join(temporaryRoot, "desktop-e2e-helper-"));
  context.after(() => rm(temporary, { recursive: true, force: true }));
  const state = path.join(temporary, "state");
  const logs = path.join(state, "logs");
  const artifacts = path.join(temporary, "artifacts");
  await writeFile(path.join(temporary, "placeholder"), "unused");
  await mkdir(logs, { recursive: true });
  await writeFile(path.join(state, "state.sqlite3"), "sqlite fixture");
  await writeFile(path.join(logs, "application.stderr.log"), "application failure");
  const destination = await retainDiagnosticFiles({ artifactRoot: artifacts, scenario: "E2E-CAPTURE-01", state, logs, error: new Error("api_key=desktop-e2e-key") });
  assert.equal(await readFile(path.join(destination, "state", "state.sqlite3"), "utf8"), "sqlite fixture");
  assert.equal(await readFile(path.join(destination, "logs", "application.stderr.log"), "utf8"), "application failure");
  assert.doesNotMatch(await readFile(path.join(destination, "failure.txt"), "utf8"), /desktop-e2e-key/);
});
