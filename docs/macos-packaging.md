# macOS packaging and installation

**English** | [한국어](macos-packaging.ko.md)

LLM Wiki keeps its bundle identifier (`com.llm-wiki.desktop`) and installation path
(`/Applications/LLM Wiki.app`) stable. On macOS, Keychain access and privacy permissions can be
associated with the application's code-signing designated requirement. Rebuilding with an ad-hoc
signature changes that requirement on every build, so release packaging rejects ad-hoc signing.

## Set up a signing identity once

For distribution, use the organization's **Developer ID Application** identity. It supports
Gatekeeper and notarization workflows and is the preferred long-term choice.

For an app used only on this Mac, a stable local self-signed identity is sufficient to keep the
same designated requirement across future replacements. In **Keychain Access**, choose
**Certificate Assistant → Create a Certificate**, give it a durable name such as `LLM Wiki Local
Signing`, choose **Self Signed Root** and **Code Signing**, and keep the generated private key in
the login keychain. Do this once; do not create a new identity for each build. The default
keychain policy is enough for code signing—do not change the certificate to **Always Trust**.

Find the identity's SHA-1 fingerprint and register that fingerprint once, rather than relying on a
mutable display name:

```text
security find-identity -v -p codesigning
node scripts/register_macos_signing_identity.mjs 40_CHARACTER_FINGERPRINT
npm run tauri:build
```

Registration stores only the public fingerprint in `~/.llm-workbench/macos-signing.json` with
user-only permissions; it does not export a certificate or private key. The package command checks
that the exact fingerprint is a valid identity in the login keychain, passes it to Tauri, verifies
the completed bundle with `codesign --verify --deep --strict`, and rejects a missing designated
requirement, the wrong bundle identifier, or `Signature=adhoc`. An explicit
`LLM_WIKI_CODESIGN_IDENTITY` environment value can temporarily override the registered fingerprint.
`npm run tauri:build -- --no-bundle` remains available for unsigned compile-only CI checks.

## Install a verified replacement

After a successful signed build, install only with the guarded installer:

```text
node scripts/install_macos_app.mjs --replace
```

It verifies the candidate before changing `/Applications/LLM Wiki.app`, compares its designated
requirement with the installed app, and copies only after they match. It never deletes or resets
`~/.llm-workbench`, the application database, Keychain entries, or TCC privacy permissions. The
previous app bundle is retained at a uniquely named
`/Applications/LLM Wiki.app.previous-<timestamp>-<process-id>` path for recovery, so a later
replacement never overwrites a recovery copy.

The first replacement of an old ad-hoc build necessarily changes its designated requirement. Read
the displayed signatures, then make that one-time migration explicit:

```text
node scripts/install_macos_app.mjs --replace --accept-designated-requirement-change
```

The old app bundle remains as the recovery copy. A provider key created by the former ad-hoc app
may need to be entered once after this migration because its old Keychain access rule named the
old build's cdhash. Keep the same signing identity thereafter so subsequent replacements retain
the new Keychain and TCC identity. This workflow does not prove that a particular folder permission
will be retained; validate that once by granting access, installing a second signed build, and
opening the same folder.

## Release verification

For every release candidate, run the normal packaged scenario after the signed build:

```text
npm run tauri:build
npm run test:desktop
codesign -dvvv "/Applications/LLM Wiki.app"
codesign -d -r- "/Applications/LLM Wiki.app"
```

The last command must show the same designated requirement before and after a replacement. Keep
the Developer ID certificate or local self-signed identity for the life of the installed product;
losing or replacing it starts a new identity migration.
