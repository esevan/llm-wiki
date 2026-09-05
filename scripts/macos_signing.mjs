const BUNDLE_IDENTIFIER = "com.llm-wiki.desktop";
const AD_HOC_SIGNATURE = /Signature=adhoc\b/i;
const DESIGNATED_REQUIREMENT = /^designated\s*=>\s*(.+)$/im;
const IDENTITY_LINE = /^\s*\d+\)\s+([A-Fa-f0-9]{40})\s+"([^"]+)"\s*$/gm;

export { BUNDLE_IDENTIFIER };

export function normalizeIdentity(value) {
  const identity = value?.trim().toUpperCase();
  if (!identity || !/^[A-F0-9]{40}$/.test(identity)) {
    throw new Error("LLM_WIKI_CODESIGN_IDENTITY must be the 40-character SHA-1 fingerprint shown by `security find-identity -v -p codesigning`.");
  }
  return identity;
}

export function signingIdentityConfigPath() {
  const home = process.env.LLM_WORKBENCH_HOME || path.join(os.homedir(), ".llm-workbench");
  return path.join(home, "macos-signing.json");
}

export async function configuredIdentity() {
  if (process.env.LLM_WIKI_CODESIGN_IDENTITY) return normalizeIdentity(process.env.LLM_WIKI_CODESIGN_IDENTITY);
  try {
    const config = JSON.parse(await readFile(signingIdentityConfigPath(), "utf8"));
    return normalizeIdentity(config.identity);
  } catch (error) {
    if (error.code === "ENOENT") throw new Error("No macOS signing identity is configured. Run `node scripts/register_macos_signing_identity.mjs <fingerprint>` once, or set LLM_WIKI_CODESIGN_IDENTITY.");
    throw error;
  }
}

export function signingIdentities(output) {
  return [...output.matchAll(IDENTITY_LINE)].map((match) => ({ fingerprint: match[1].toUpperCase(), name: match[2] }));
}

export function identityIsAvailable(output, identity) {
  return signingIdentities(output).some((candidate) => candidate.fingerprint === normalizeIdentity(identity));
}

export function designatedRequirement(output) {
  return output.match(DESIGNATED_REQUIREMENT)?.[1]?.trim() ?? null;
}

export function signatureDetails(output) {
  return {
    identifier: output.match(/^Identifier=(.+)$/m)?.[1]?.trim() ?? null,
    designatedRequirement: designatedRequirement(output),
    isAdHoc: AD_HOC_SIGNATURE.test(output),
  };
}

export function requirementsMatch(candidate, installed) {
  return candidate.identifier === installed.identifier && candidate.designatedRequirement !== null && candidate.designatedRequirement === installed.designatedRequirement;
}

export function assertStableSignature(output) {
  const details = signatureDetails(output);
  if (details.identifier !== BUNDLE_IDENTIFIER) throw new Error(`Expected bundle identifier ${BUNDLE_IDENTIFIER}, found ${details.identifier ?? "none"}.`);
  if (details.isAdHoc) throw new Error("Refusing an ad-hoc signature because its designated requirement is tied to this build's cdhash.");
  if (!details.designatedRequirement) throw new Error("The app has no designated requirement; it cannot preserve Keychain and TCC identity across replacements.");
  return details;
}
import { readFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
