import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const commandArgs = process.platform === "darwin"
  ? [path.join(root, "scripts", "build_macos.mjs"), ...args]
  : [path.join(root, "node_modules", "@tauri-apps", "cli", "tauri.js"), "build", ...args];
const child = spawn(process.execPath, commandArgs, { cwd: root, stdio: "inherit" });
child.once("error", (error) => { throw error; });
child.once("close", (code) => process.exitCode = code ?? 1);
