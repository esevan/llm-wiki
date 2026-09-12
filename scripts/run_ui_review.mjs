import { spawn } from "node:child_process";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const app = path.join(root, "src-tauri", "target", "release", "bundle", "macos", "LLM Wiki.app");
const executable = process.env.LLM_WIKI_UI_REVIEW_EXECUTABLE
  ? path.resolve(process.env.LLM_WIKI_UI_REVIEW_EXECUTABLE)
  : path.join(app, "Contents", "MacOS", "llm-wiki-desktop");
const arguments_ = process.argv.slice(2);
const reuseBuild = arguments_.includes("--reuse-build");
if (arguments_.some((argument) => argument !== "--reuse-build")) throw new Error("Usage: node scripts/run_ui_review.mjs [--reuse-build]");

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: root, stdio: "inherit" });
    child.once("error", reject);
    child.once("close", (code) => code === 0 ? resolve() : reject(new Error(`${command} ${args.join(" ")} failed with exit code ${code}.`)));
  });
}

if (process.platform !== "darwin") throw new Error("The isolated UI review launcher currently supports macOS.");

const temporaryRoot = path.join(root, ".tmp");
await mkdir(temporaryRoot, { recursive: true });
const state = await mkdtemp(path.join(temporaryRoot, "ui-review-"));
const vault = path.join(state, "vault");
await mkdir(vault);
await writeFile(path.join(vault, "review-notes.md"), "# UI review\n\nThis temporary vault belongs only to the review session.\n", "utf8");

if (!reuseBuild) await run(process.execPath, [path.join(root, "scripts", "build_desktop.mjs")]);
const child = spawn(executable, ["-ApplePersistenceIgnoreState", "YES"], {
  cwd: root,
  detached: true,
  stdio: "ignore",
  env: {
    ...process.env,
    LLM_WORKBENCH_HOME: path.join(state, ".llm-workbench"),
    LLM_WIKI_VAULT: vault,
    LLM_WIKI_DB: path.join(state, "state.sqlite3"),
  },
});
child.unref();
console.log(`Started a signed, isolated UI review app${reuseBuild ? " using the existing release artifact" : ""}. Temporary review state: ${state}`);
