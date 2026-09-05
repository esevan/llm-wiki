import { spawn } from "node:child_process";
import { access, rename } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { assertStableSignature, requirementsMatch, signatureDetails } from "./macos_signing.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = path.join(root, "src-tauri", "target", "release", "bundle", "macos", "LLM Wiki.app");
const destination = "/Applications/LLM Wiki.app";
const operationId = `${Date.now()}-${process.pid}`;
const backup = `/Applications/LLM Wiki.app.previous-${operationId}`;
const staging = `/Applications/.LLM Wiki.app.staging-${operationId}`;
const failedReplacement = `/Applications/LLM Wiki.app.failed-${operationId}`;
const flags = new Set(process.argv.slice(2));

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ["ignore", "pipe", "pipe"] });
    let output = "";
    child.stdout.on("data", (chunk) => { output += chunk; process.stdout.write(chunk); });
    child.stderr.on("data", (chunk) => { output += chunk; process.stderr.write(chunk); });
    child.once("error", reject);
    child.once("close", (code) => code === 0 ? resolve(output) : reject(new Error(`${command} ${args.join(" ")} failed with exit code ${code}.`)));
  });
}

async function exists(item) {
  try { await access(item); return true; } catch { return false; }
}

if (process.platform !== "darwin") throw new Error("The macOS installer must run on macOS.");
if (!flags.has("--replace")) throw new Error("Refusing to change /Applications without --replace. This installer never resets user settings, Keychain items, or TCC permissions.");

await access(source);
await run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", source]);
const candidate = assertStableSignature(await run("codesign", ["-dvvv", "-r-", source]));

const replacing = await exists(destination);
if (replacing) {
  const current = signatureDetails(await run("codesign", ["-dvvv", "-r-", destination]));
  if (!requirementsMatch(candidate, current) && !flags.has("--accept-designated-requirement-change")) {
    throw new Error("The installed app has a different designated requirement. Inspect both signatures and rerun with --accept-designated-requirement-change only for the one-time migration from an old identity.");
  }
}

let previousMoved = false;
let replacementActivated = false;
try {
  await run("ditto", [source, staging]);
  await run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", staging]);
  const staged = assertStableSignature(await run("codesign", ["-dvvv", "-r-", staging]));
  if (!requirementsMatch(candidate, staged)) throw new Error("The staged app's designated requirement changed during installation.");
  if (replacing) {
    await rename(destination, backup);
    previousMoved = true;
  }
  await rename(staging, destination);
  replacementActivated = true;
  await run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", destination]);
  const installed = assertStableSignature(await run("codesign", ["-dvvv", "-r-", destination]));
  if (!requirementsMatch(candidate, installed)) throw new Error("The copied app's designated requirement changed during installation.");
  console.log(`Installed ${destination} without touching app data, Keychain items, or TCC permissions.${replacing ? ` Previous app retained at ${backup}.` : ""}`);
} catch (error) {
  if (previousMoved) {
    if (replacementActivated && await exists(destination)) await rename(destination, failedReplacement);
    if (await exists(backup) && !(await exists(destination))) await rename(backup, destination);
  }
  throw error;
}
