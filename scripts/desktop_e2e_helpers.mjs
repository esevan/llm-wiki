import { cp, mkdir, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const DEFAULT_TIMEOUT_MS = 180_000;
const MIN_TIMEOUT_MS = 1_000;

export const DESKTOP_E2E_SCENARIO_NAMES = Object.freeze(("task-capture,task-worklog,task-refinement,task-relationships,task-review,task-publication,task-problem-resolution,task-persistence,task-localization,task-bilingual-refinement,task-controls,task-workbench-retry,task-legacy-refinement,task-legacy-chat-controls,task-legacy-preview-retry,task-chat-controls,task-refinement-provider-recovery,task-refinement-close-retry,task-refinement-close-pending,task-refinement-relaunch,task-mcp-continuation,global-shell,global-search,global-compass,global-provider,global-mcp,global-queue-notifications,global-notice,global-intro-navigation,global-intro-retry,global-vault-choose,global-vault-retry,global-migration-restore,global-migration-retry").split(","));

export function desktopE2eHelp() {
  return `Usage: npm run test:desktop -- [options]\n\nOptions:\n  --list                         list the ${DESKTOP_E2E_SCENARIO_NAMES.length} registered scenarios\n  --full                         run every scenario and enforce registered control coverage\n  --scenario NAME                run one scenario (repeat for a focused subset)\n  --artifact-dir PATH            retain results and diagnostics at PATH\n  --keep-state                   retain isolated runtime state after each scenario\n  --help                         show this help\n\nThe default executable is the release bundle. Set LLM_WIKI_E2E_EXECUTABLE to an existing signed build only when intentionally reusing it.`;
}

function parseTimeout(value, fallback = DEFAULT_TIMEOUT_MS) {
  if (value === undefined || value === "") return fallback;
  const parsed = Number.parseInt(value, 10);
  if (!Number.isSafeInteger(parsed) || parsed < MIN_TIMEOUT_MS) throw new Error(`Desktop E2E timeout must be an integer of at least ${MIN_TIMEOUT_MS}ms.`);
  return parsed;
}

function caseEnvironmentKey(name) { return name.replace(/[^A-Za-z0-9]/g, "_").toUpperCase(); }

export function parseDesktopE2eCases({ argv = [], env = process.env, validateRegistry = false } = {}) {
  const requested = [];
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--scenario") {
      const value = argv[index + 1];
      if (!value) throw new Error("--scenario requires a scenario name.");
      requested.push(value); index += 1;
    } else if (argument.startsWith("--scenario=")) requested.push(argument.slice("--scenario=".length));
    else throw new Error(`Unknown desktop E2E runner option: ${argument}`);
  }
  // Each group receives an independent home, database, and Vault.  `legacy`
  // exercised removed board routes and is intentionally no longer the default.
  const selectors = requested.length > 0 ? requested : (env.LLM_WIKI_E2E_SCENARIOS ? String(env.LLM_WIKI_E2E_SCENARIOS).split(",") : DESKTOP_E2E_SCENARIO_NAMES);
  const names = selectors.flatMap((value) => value.split(",")).map((value) => value.trim()).filter(Boolean);
  if (names.length === 0) throw new Error("At least one desktop E2E scenario is required.");
  const unique = [...new Set(names)];
  for (const name of unique) if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(name)) throw new Error(`Invalid desktop E2E scenario name: ${name}`);
  if (validateRegistry) for (const name of unique) if (!DESKTOP_E2E_SCENARIO_NAMES.includes(name)) throw new Error(`Unknown desktop E2E scenario "${name}". Use --list to see registered scenarios.`);
  const defaultTimeout = parseTimeout(env.LLM_WIKI_E2E_TIMEOUT_MS);
  return unique.map((name) => ({ name, timeoutMs: parseTimeout(env[`LLM_WIKI_E2E_TIMEOUT_MS_${caseEnvironmentKey(name)}`], defaultTimeout) }));
}

export function redactedText(value) {
  return String(value).replace(/(api[_ -]?key|authorization|token|password)\s*[:=]\s*[^\s,;)}\]]+/gi, "$1=[REDACTED]").replace(/desktop-e2e-key/gi, "[REDACTED]");
}

export function resultSummary(payload) {
  if (!payload || typeof payload !== "object") return "no result payload";
  const status = typeof payload.status === "string" ? payload.status : "unknown";
  const error = typeof payload.error === "string" && payload.error ? `: ${redactedText(payload.error)}` : "";
  return `${status}${error}`;
}

export async function retainDiagnosticFiles({ artifactRoot, scenario, state, logs, error }) {
  const destination = path.join(artifactRoot, scenario);
  await mkdir(destination, { recursive: true });
  for (const [name, source] of Object.entries({ state, logs })) {
    try { await cp(source, path.join(destination, name), { recursive: true, force: true }); } catch {}
  }
  await writeFile(path.join(destination, "failure.txt"), `${redactedText(error.stack ?? error)}${os.EOL}`, "utf8");
  return destination;
}
