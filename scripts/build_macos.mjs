import { spawn } from "node:child_process";
import { access, copyFile, mkdir, mkdtemp, rm } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { assertStableSignature, configuredIdentity, identityIsAvailable, signingIdentities } from "./macos_signing.mjs";
import { configureOnnxEnvironment, stageRuntime, verifyBundledRuntime } from "./macos_runtime.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const appPath = path.join(root, "src-tauri", "target", "release", "bundle", "macos", "LLM Wiki.app");
const argumentsForTauri = process.argv.slice(2);
const tauriCli = path.join(root, "node_modules", "@tauri-apps", "cli", "tauri.js");
const temporaryRoot = path.join(root, ".tmp");
await mkdir(temporaryRoot, { recursive: true });
process.env.TMPDIR = temporaryRoot;

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
await configureOnnxEnvironment();
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

const macOS = { signingIdentity: identity };
const signingName = signingIdentities(availableIdentities).find(item => item.fingerprint === identity)?.name;
// Local certificates have no Team ID. Keep hardened runtime, permitting only
// this app to load its signed libraries without Apple's Team-ID requirement.
if (!signingName?.startsWith("Developer ID Application:") && !signingName?.startsWith("Apple Distribution:")) {
  macOS.entitlements = "Entitlements.local.plist";
}
await run(process.execPath, [tauriCli, "build", "--no-bundle", "--config", JSON.stringify({ bundle: { macOS } }), ...argumentsForTauri]);
const staging = await mkdtemp(path.join(temporaryRoot, "macos-runtime-"));
const executable = path.join(root, "src-tauri/target/release/llm-wiki-desktop");
const backup = path.join(staging, "unbundled-executable");
await copyFile(executable, backup);
try {
  macOS.frameworks = await stageRuntime(executable, path.join(staging, "Frameworks"));
  // Build flags (features, verbosity, bundle selection) supported by both commands.
  // Cargo-only arguments remain on the compile invocation above.
  const bundleArgs = [];
  for (let index = 0; index < argumentsForTauri.length; index += 1) {
    const arg = argumentsForTauri[index];
    if (["--bundles", "-b", "--features", "-f"].includes(arg)) {
      bundleArgs.push(arg);
      while (argumentsForTauri[index + 1] && !argumentsForTauri[index + 1].startsWith("-")) bundleArgs.push(argumentsForTauri[++index]);
    } else if (/^--(?:bundles|features)=/.test(arg) || ["--ci", "--skip-stapling", "--verbose", "-v"].includes(arg)) bundleArgs.push(arg);
  }
  await run(process.execPath, [tauriCli, "bundle", "--config", JSON.stringify({ bundle: { macOS } }), ...bundleArgs]);
  const count = await verifyBundledRuntime(appPath);
  console.log(`Verified ${count} bundled runtime libraries without external dependencies.`);
} finally {
  // Preserve Cargo's unmodified incremental output even if bundling fails.
  await copyFile(backup, executable);
  await rm(staging, { recursive: true, force: true });
}
await access(appPath);
await run("codesign", ["--verify", "--deep", "--strict", "--verbose=2", appPath]);
assertStableSignature(await run("codesign", ["-dvvv", "-r-", appPath]));
console.log(`macOS package has a stable non-ad-hoc signature: ${appPath}`);
