import { spawn } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, unlink, writeFile } from "node:fs/promises";
import { createWriteStream } from "node:fs";
import net from "node:net";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { DESKTOP_E2E_SCENARIO_NAMES, desktopE2eHelp, parseDesktopE2eCases, redactedText, resultSummary, retainDiagnosticFiles } from "./desktop_e2e_helpers.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function executablePath() {
  if (process.env.LLM_WIKI_E2E_EXECUTABLE) return path.resolve(process.env.LLM_WIKI_E2E_EXECUTABLE);
  if (process.platform === "darwin") return path.join(root, "src-tauri/target/release/bundle/macos/LLM Wiki.app/Contents/MacOS/llm-wiki-desktop");
  if (process.platform === "win32") return path.join(root, "src-tauri/target/release/llm-wiki-desktop.exe");
  return path.join(root, "src-tauri/target/release/llm-wiki-desktop");
}

function applicationArguments() { return process.platform === "darwin" ? ["-ApplePersistenceIgnoreState", "YES"] : []; }

function attachLog(stream, destination) {
  if (stream) stream.pipe(createWriteStream(destination, { flags: "a" }));
}

async function availablePort() {
  return await new Promise((resolve, reject) => {
    const listener = net.createServer();
    listener.once("error", reject);
    listener.listen(0, "127.0.0.1", () => {
      const address = listener.address();
      listener.close(() => resolve(address.port));
    });
  });
}

async function waitForResult(result, child, { launch, timeoutMs }) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try { return JSON.parse(await readFile(result, "utf8")); } catch (error) { if (error.code !== "ENOENT") throw error; }
    if (child.exitCode !== null || child.signalCode !== null) break;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  let completedSteps = [];
  try { completedSteps = JSON.parse(await readFile(result.replace(/\.json$/, ".progress"), "utf8")); } catch {}
  throw new Error(`Desktop scenario did not report a result before its ${timeoutMs}ms timeout (launch=${launch}, exit=${child.exitCode}, completed_steps=${JSON.stringify(completedSteps)}).`);
}

async function waitForExit(child, timeoutMs, label) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  await Promise.race([
    new Promise((resolve) => child.once("exit", resolve)),
    new Promise((_, reject) => setTimeout(() => reject(new Error(`${label} did not exit within ${timeoutMs}ms.`)), timeoutMs)),
  ]);
}

async function terminate(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  child.kill("SIGTERM");
  await Promise.race([new Promise((resolve) => child.once("exit", resolve)), new Promise((resolve) => setTimeout(resolve, 10_000))]);
  if (child.exitCode === null) child.kill("SIGKILL");
}

async function startProvider({ providerPort, logs }) {
  const provider = spawn(process.execPath, [path.join(root, "tests/fakes/openai_server.mjs"), "--port", String(providerPort)], { cwd: root, stdio: ["ignore", "pipe", "pipe"] });
  attachLog(provider.stdout, path.join(logs, "provider.stdout.log"));
  attachLog(provider.stderr, path.join(logs, "provider.stderr.log"));
  await Promise.race([new Promise((resolve, reject) => {
    provider.stdout.once("data", resolve);
    provider.once("error", reject);
    provider.once("exit", (code) => reject(new Error(`Deterministic provider exited during startup (${code}).`)));
  }), new Promise((_, reject) => setTimeout(() => reject(new Error("Deterministic provider did not become ready within 10 seconds.")), 10_000))]);
  return provider;
}

async function runScenario(scenario, artifactRoot) {
  const state = await mkdtemp(path.join(root, ".tmp", `desktop-e2e-${scenario.name}-`));
  const vault = path.join(state, "vault");
  const result = path.join(state, "result.json");
  const logs = path.join(state, "logs");
  await Promise.all([mkdir(vault), mkdir(logs)]);
  await writeFile(path.join(vault, "startup.md"), "# Startup indexing\n\nThe bundled embedding model indexes this note after launch.\n", "utf8");
  await Promise.all(Array.from({ length: 24 }, (_, index) =>
    writeFile(path.join(vault, `startup-${String(index + 1).padStart(2, "0")}.md`), `# Startup indexing ${index + 1}\n\nDeterministic packaged search coverage document ${index + 1}.\n`, "utf8"),
  ));
  let application;
  let provider;
  let payload;
  const startedAt = Date.now();
  try {
    const providerPort = await availablePort();
    provider = await startProvider({ providerPort, logs });
    const environment = {
      ...process.env,
      ...(/^(global-intro|global-vault)-/.test(scenario.name) ? {} : { LLM_WIKI_VAULT: vault }),
      LLM_WIKI_DB: path.join(state, "state.sqlite3"),
      LLM_WORKBENCH_HOME: path.join(state, ".llm-workbench"), LLM_WIKI_E2E_RESULT: result,
      LLM_WIKI_E2E_SCENARIO: scenario.name, LLM_WIKI_E2E_ARTIFACT_DIR: path.join(artifactRoot, scenario.name),
      LLM_WIKI_E2E_PROVIDER_URL: `http://127.0.0.1:${providerPort}/v1`,
      LLM_WIKI_MCP_ENDPOINT: process.platform === "win32" ? `\\\\.\\pipe\\llm-wiki-e2e-${path.basename(state)}` : path.join(path.relative(root, state), "ipc", "mcp.sock"),
    };
    await mkdir(environment.LLM_WIKI_E2E_ARTIFACT_DIR, { recursive: true });
    for (let launch = 1; launch <= 2; launch += 1) {
      application = spawn(executablePath(), applicationArguments(), { cwd: root, env: environment, stdio: ["ignore", "pipe", "pipe"] });
      let launchError;
      application.once("error", (error) => { launchError = error; });
      attachLog(application.stdout, path.join(logs, `application-${launch}.stdout.log`));
      attachLog(application.stderr, path.join(logs, `application-${launch}.stderr.log`));
      payload = await Promise.race([
        waitForResult(result, application, { launch, timeoutMs: scenario.timeoutMs }),
        new Promise((_, reject) => application.once("error", reject)),
      ]);
      if (launchError) throw launchError;
      if (payload.status !== "relaunch") break;
      await waitForExit(application, 10_000, "Desktop relaunch application");
      environment.LLM_WIKI_E2E_RESTORE_CAPTURE = payload.capture;
      environment.LLM_WIKI_E2E_RESTORE_STEPS = JSON.stringify(payload.steps);
      await unlink(result);
    }
    if (payload?.status !== "passed") throw new Error(`Desktop scenario failed: ${resultSummary(payload)}`);
    if (scenario.name === "legacy") {
      const settings = JSON.parse(await readFile(path.join(environment.LLM_WORKBENCH_HOME, "settings.json"), "utf8"));
      if (settings.provider?.baseUrl !== environment.LLM_WIKI_E2E_PROVIDER_URL || settings.provider?.apiKey !== "desktop-e2e-key") throw new Error("Desktop settings did not persist in the isolated home directory.");
    }
    return { name: scenario.name, status: "passed", durationMs: Date.now() - startedAt, steps: Array.isArray(payload.steps) ? payload.steps : [], coverage: payload.coverage };
  } catch (error) {
    await terminate(application); application = undefined;
    await terminate(provider); provider = undefined;
    const artifacts = await retainDiagnosticFiles({ artifactRoot, scenario: scenario.name, state, logs, error });
    return { name: scenario.name, status: "failed", durationMs: Date.now() - startedAt, steps: Array.isArray(payload?.steps) ? payload.steps : [], error: redactedText(error.message), artifacts, coverage: payload?.coverage };
  } finally {
    await terminate(application); await terminate(provider);
    if (process.env.LLM_WIKI_E2E_KEEP_STATE === "1") console.error(`desktop E2E state retained at ${state}`);
    else await rm(state, { recursive: true, force: true });
  }
}

const argv = process.argv.slice(2);
if (argv.includes("--help")) { console.log(desktopE2eHelp()); process.exit(0); }
if (argv.includes("--list")) { console.log(DESKTOP_E2E_SCENARIO_NAMES.join("\n")); process.exit(0); }
const full = argv.includes("--full");
const filteredArgv = argv.filter((argument) => argument !== "--full");
if (full && filteredArgv.some((argument) => argument === "--scenario" || argument.startsWith("--scenario="))) throw new Error("--full cannot be combined with --scenario; choose the complete suite or a focused subset.");
const artifactArgumentIndex = filteredArgv.indexOf("--artifact-dir");
let artifactRoot;
if (artifactArgumentIndex >= 0) {
  const value = filteredArgv[artifactArgumentIndex + 1];
  if (!value || value.startsWith("--")) throw new Error("--artifact-dir requires a directory path.");
  process.env.LLM_WIKI_E2E_ARTIFACT_DIR = value;
  filteredArgv.splice(artifactArgumentIndex, 2);
}
if (argv.includes("--keep-state")) {
  process.env.LLM_WIKI_E2E_KEEP_STATE = "1";
  filteredArgv.splice(filteredArgv.indexOf("--keep-state"), 1);
}
await mkdir(path.join(root, ".tmp"), { recursive: true });
artifactRoot = process.env.LLM_WIKI_E2E_ARTIFACT_DIR ? path.resolve(process.env.LLM_WIKI_E2E_ARTIFACT_DIR) : await mkdtemp(path.join(root, ".tmp", "desktop-e2e-artifacts-"));
const scenarios = parseDesktopE2eCases({ argv: filteredArgv, env: full ? { ...process.env, LLM_WIKI_E2E_SCENARIOS: DESKTOP_E2E_SCENARIO_NAMES.join(",") } : process.env, validateRegistry: true });
const executable = executablePath();
try { await readFile(executable); } catch (error) {
  if (error.code === "ENOENT") throw new Error(`Release executable is missing at ${executable}. Build it first, or set LLM_WIKI_E2E_EXECUTABLE to an existing build when intentionally reusing one.`);
  throw error;
}
const outcomes = [];
for (const scenario of scenarios) outcomes.push(await runScenario(scenario, artifactRoot));
// Failed-case reports are retained for diagnosis but cannot satisfy the release gate.
const coverageReports = outcomes.filter(outcome => outcome.status === "passed").map(outcome => outcome.coverage).filter(Boolean);
const union = (key) => [...new Set(coverageReports.flatMap(report => report[key] ?? []))];
const renderedIds = union("renderedIds"), exercisedIds = union("exercisedIds"), assertedIds = union("assertedIds");
const coverage = coverageReports.length ? {
  inventoryCount: Math.max(...coverageReports.map(report => report.inventoryCount ?? 0)),
  scannedSourceControls: Math.max(...coverageReports.map(report => report.scannedSourceControls ?? 0)),
  rendered: renderedIds.length,
  exercised: exercisedIds.length,
  asserted: assertedIds.length,
  renderedIds,
  exercisedIds,
  assertedIds,
  unknownEnabled: union("unknownEnabled"),
  missingSourceEvidence: union("missingSourceEvidence"),
  undocumentedDisabled: union("undocumentedDisabled"),
  unexercised: union("unexercised").filter(id => renderedIds.includes(id) && !exercisedIds.includes(id)),
  effectMissing: union("effectMissing").filter(id => exercisedIds.includes(id) && !assertedIds.includes(id)),
  notRendered: union("notRendered").filter(id => !renderedIds.includes(id)),
} : null;
await writeFile(path.join(artifactRoot, "results.json"), `${JSON.stringify(outcomes, null, 2)}\n`, "utf8");
if (coverage) await writeFile(path.join(artifactRoot, "interactive-coverage.json"), `${JSON.stringify(coverage, null, 2)}\n`, "utf8");
const requireCompleteCoverage = full || (filteredArgv.length === 0 && !process.env.LLM_WIKI_E2E_SCENARIOS);
const coverageFailures = !requireCompleteCoverage ? [] : !coverage ? ["no scenario coverage reports"] : [
  coverage.unknownEnabled.length && `unknown enabled controls: ${coverage.unknownEnabled.join(", ")}`,
  coverage.missingSourceEvidence.length && `source drift: ${coverage.missingSourceEvidence.join(", ")}`,
  coverage.notRendered.length && `not rendered: ${coverage.notRendered.join(", ")}`,
  coverage.unexercised.length && `unexercised: ${coverage.unexercised.join(", ")}`,
  coverage.effectMissing.length && `effect missing: ${coverage.effectMissing.join(", ")}`,
  coverage.undocumentedDisabled.length && `undocumented disabled: ${coverage.undocumentedDisabled.join(", ")}`,
].filter(Boolean);
if (outcomes.every((outcome) => outcome.status === "passed") && coverageFailures.length === 0) {
  console.log("desktop E2E passed");
  console.log(requireCompleteCoverage ? "- mode: full acceptance (complete 169-control coverage enforced)" : "- mode: focused subset (full coverage not evaluated)");
  console.log(`- results: ${path.join(artifactRoot, "results.json")}`);
  if (coverage) console.log(`- coverage: ${path.join(artifactRoot, "interactive-coverage.json")}`);
  for (const outcome of outcomes) console.log(`- ${outcome.name}: ${outcome.status}`);
} else {
  console.error(`desktop E2E failed; case results are retained at ${path.join(artifactRoot, "results.json")}`);
  for (const outcome of outcomes) console.error(`- ${outcome.name}: ${outcome.status}`);
  for (const failure of coverageFailures) console.error(`- interactive coverage: ${failure}`);
  process.exitCode = 1;
}
