import { spawn } from "node:child_process";
import { chmod, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { identityIsAvailable, normalizeIdentity, signingIdentityConfigPath } from "./macos_signing.mjs";

if (process.platform !== "darwin") throw new Error("A macOS signing identity can only be registered on macOS.");
const identity = normalizeIdentity(process.argv[2]);
const identities = await new Promise((resolve, reject) => {
  const child = spawn("security", ["find-identity", "-v", "-p", "codesigning"], { stdio: ["ignore", "pipe", "pipe"] });
  let output = "";
  child.stdout.on("data", (chunk) => { output += chunk; });
  child.stderr.on("data", (chunk) => { output += chunk; });
  child.once("error", reject);
  child.once("close", (code) => code === 0 ? resolve(output) : reject(new Error("Could not inspect macOS code-signing identities.")));
});
if (!identityIsAvailable(identities, identity)) throw new Error(`${identity} is not a valid code-signing identity in this login keychain.`);
const configPath = signingIdentityConfigPath();
await mkdir(path.dirname(configPath), { recursive: true, mode: 0o700 });
await chmod(path.dirname(configPath), 0o700);
await writeFile(configPath, `${JSON.stringify({ identity }, null, 2)}\n`, { encoding: "utf8", mode: 0o600 });
await chmod(configPath, 0o600);
console.log(`Registered the macOS signing fingerprint at ${configPath}.`);
