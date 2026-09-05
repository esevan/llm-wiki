import { spawn } from "node:child_process";
import { access } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { assertStableSignature, configuredIdentity, identityIsAvailable } from "./macos_signing.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const appPath = path.join(root, "src-tauri", "target", "release", "bundle", "macos", "LLM Wiki.app");
const argumentsForTauri = process.argv.slice(2);
const tauriCli = path.join(root, "node_modules", "@tauri-apps", "cli", "tauri.js");

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: root, stdio: ["ignore", "pipe", "pipe"] });
    let output = "";
    child.stdout.on("data", (chunk) => { output += chunk; process.stdout.write(chunk); });
    child.stderr.on("data", (chunk) => { output += chunk; process.stderr.write(chunk); });
    child.once("error", reject);
    child.once("close", (code) => code === 0 ? resolve(output) : reject(new Error(`${command} ${args.join(" ")} failed with exit code ${code}.`)));
  });
}

if (process.platform !== "darwin") throw new Error("The macOS package command must run on macOS.");
if (argumentsForTauri.includes("--no-bundle")) {
  await run(process.execPath, [tauriCli, "build", ...argumentsForTauri]);
  process.exit(0);
}
if (argumentsForTauri.some((argument) => ["--debug", "-d", "--target", "-t", "--config", "-c"].includes(argument))) {
  throw new Error("The signed macOS package command only supports the default release output. Run `tauri build` directly for a non-release or alternate-target artifact.");
}

const identity = await configuredIdentity();
const availableIdentities = await run("security", ["find-identity", "-v", "-p", "codesigning"]);
if (!identityIsAvailable(availableIdentities, identity)) throw new Error(`LLM_WIKI_CODESIGN_IDENTITY=${identity} is not a valid code-signing identity in this login keychain.`);

await run(process.execPath, [tauriCli, "build", "--config", JSON.stringify({ bundle: { macOS: { signingIdentity: identity } } }), ...argumentsForTauri]);
await access(appPath);
await run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", appPath]);
assertStableSignature(await run("codesign", ["-dvvv", "-r-", appPath]));
console.log(`macOS package has a stable non-ad-hoc signature: ${appPath}`);
