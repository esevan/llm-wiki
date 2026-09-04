import { spawn } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, unlink, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import process from "node:process";

const root = path.resolve(import.meta.dirname, "..");

function executablePath() {
  if (process.platform === "darwin") return path.join(root, "src-tauri/target/release/bundle/macos/LLM Wiki.app/Contents/MacOS/llm-wiki-desktop");
  if (process.platform === "win32") return path.join(root, "src-tauri/target/release/llm-wiki-desktop.exe");
  return path.join(root, "src-tauri/target/release/llm-wiki-desktop");
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

async function waitForResult(result, child, launch) {
  const deadline = Date.now() + 180_000;
  while (Date.now() < deadline) {
    try {
      return JSON.parse(await readFile(result, "utf8"));
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
    if (child.exitCode !== null) break;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  let completedSteps = [];
  try { completedSteps = JSON.parse(await readFile(result.replace(/\.json$/, ".progress"), "utf8")); } catch {}
  throw new Error(`Desktop scenario did not report a result (launch=${launch}, exit=${child.exitCode}, completed_steps=${JSON.stringify(completedSteps)}).`);
}

async function terminate(child) {
  if (!child || child.exitCode !== null) return;
  child.kill("SIGTERM");
  await Promise.race([
    new Promise((resolve) => child.once("exit", resolve)),
    new Promise((resolve) => setTimeout(resolve, 10_000)),
  ]);
  if (child.exitCode === null) child.kill("SIGKILL");
}

await mkdir(path.join(root, ".tmp"), { recursive: true });
const state = await mkdtemp(path.join(root, ".tmp", "desktop-e2e-"));
const vault = path.join(state, "vault");
const result = path.join(state, "result.json");
await mkdir(vault);
await writeFile(path.join(vault, "startup.md"), "# Startup indexing\n\nThe bundled embedding model indexes this note after launch.\n", "utf8");

const providerPort = await availablePort();
const provider = spawn(process.execPath, [path.join(root, "tests/fakes/openai_server.mjs"), "--port", String(providerPort)], { cwd: root, stdio: ["ignore", "pipe", "inherit"] });
await new Promise((resolve, reject) => {
  provider.stdout.once("data", resolve);
  provider.once("exit", (code) => reject(new Error(`Deterministic provider exited during startup (${code}).`)));
});

const environment = {
  ...process.env,
  LLM_WIKI_VAULT: vault,
  LLM_WIKI_DB: path.join(state, "state.sqlite3"),
  LLM_WORKBENCH_HOME: path.join(state, ".llm-workbench"),
  LLM_WIKI_E2E_RESULT: result,
  LLM_WIKI_E2E_PROVIDER_URL: `http://127.0.0.1:${providerPort}/v1`,
  LLM_WIKI_TEST_MODE: "1",
  LLM_WIKI_TEST_API_KEY: "desktop-e2e-key",
};

let application;
try {
  let payload;
  for (let launch = 1; launch <= 2; launch += 1) {
    application = spawn(executablePath(), [], { cwd: root, env: environment, stdio: "inherit" });
    payload = await waitForResult(result, application, launch);
    if (payload.status !== "relaunch") break;
    if (application.exitCode === null) {
      await new Promise((resolve) => application.once("exit", resolve));
    }
    environment.LLM_WIKI_E2E_RESTORE_CAPTURE = payload.capture;
    environment.LLM_WIKI_E2E_RESTORE_STEPS = JSON.stringify(payload.steps);
    await unlink(result);
  }
  if (payload?.status !== "passed") throw new Error(`Desktop scenario failed: ${JSON.stringify(payload)}`);
  const settingsText = await readFile(path.join(environment.LLM_WORKBENCH_HOME, "settings.json"), "utf8");
  const settings = JSON.parse(settingsText);
  if (settings.provider?.baseUrl !== environment.LLM_WIKI_E2E_PROVIDER_URL) {
    throw new Error("Desktop settings did not persist in the isolated home directory.");
  }
  if (settingsText.includes(environment.LLM_WIKI_TEST_API_KEY)) {
    throw new Error("Desktop settings exposed the provider API key.");
  }
  payload.steps.push("non-secret settings persisted in the isolated home file without an API key");
  console.log("desktop E2E passed");
  for (const step of payload.steps) console.log(`- ${step}`);
} finally {
  await terminate(application);
  await terminate(provider);
  await rm(state, { recursive: true, force: true });
}
