import assert from "node:assert/strict";
import test from "node:test";
import { BUNDLE_IDENTIFIER, assertStableSignature, identityIsAvailable, requirementsMatch, signatureDetails } from "../scripts/macos_signing.mjs";

const certificateIdentity = "0123456789ABCDEF0123456789ABCDEF01234567";
const signedOutput = `Executable=/tmp/LLM Wiki.app/Contents/MacOS/llm-wiki-desktop\nIdentifier=${BUNDLE_IDENTIFIER}\nAuthority=LLM Wiki Local Signing\nSignature size=8999\nTeamIdentifier=not set\n`;
const selfSignedRequirement = `designated => anchor rootCert and identifier \"${BUNDLE_IDENTIFIER}\"`;

test("recognizes only valid code-signing identity fingerprints", () => {
  const identities = `  1) ${certificateIdentity} \"LLM Wiki Local Signing\"\n     1 valid identities found\n`;
  assert.equal(identityIsAvailable(identities, certificateIdentity.toLowerCase()), true);
  assert.equal(identityIsAvailable(identities, "ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD"), false);
});

test("rejects an ad-hoc bundle even when its identifier is correct", () => {
  assert.throws(() => assertStableSignature(`${signedOutput}Signature=adhoc\n${selfSignedRequirement}`), /ad-hoc signature/);
});

test("requires the configured bundle identifier and a designated requirement", () => {
  assert.throws(() => assertStableSignature("Identifier=com.example.other\nSignature size=99\n"), /Expected bundle identifier/);
  assert.throws(() => assertStableSignature(`${signedOutput}Signature size=99\n`), /no designated requirement/);
});

test("compares the entire designated requirement before replacement", () => {
  const candidate = signatureDetails(`${signedOutput}${selfSignedRequirement}`);
  const same = signatureDetails(`${signedOutput}${selfSignedRequirement}`);
  const different = signatureDetails(`${signedOutput}designated => anchor apple generic and identifier \"${BUNDLE_IDENTIFIER}\"`);
  assert.equal(requirementsMatch(candidate, same), true);
  assert.equal(requirementsMatch(candidate, different), false);
});
