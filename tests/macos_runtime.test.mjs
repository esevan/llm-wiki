import assert from "node:assert/strict";
import test from "node:test";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { stageRuntime, verifyBundledRuntime } from "../scripts/macos_runtime.mjs";

const execute = promisify(execFile);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

test("bundles transitive Mach-O libraries, preserves originals, and rejects external dependencies", { skip: process.platform !== "darwin" }, async () => {
  const temporaryRoot = path.join(root, ".tmp");
  await mkdir(temporaryRoot, { recursive: true });
  const state = await mkdtemp(path.join(temporaryRoot, "runtime-test-"));
  try {
    const source = path.join(state, "source");
    const app = path.join(state, "Test.app");
    const macOS = path.join(app, "Contents/MacOS");
    await mkdir(source);
    await mkdir(macOS, { recursive: true });
    const run = (command, args) => execute(command, args, { env: { ...process.env, TMPDIR: state } });
    await writeFile(path.join(source, "leaf.c"), "int leaf(void) { return 7; }\n");
    await writeFile(path.join(source, "parent.c"), "extern int leaf(void); int parent(void) { return leaf(); }\n");
    await writeFile(path.join(source, "main.c"), "extern int parent(void); int main(void) { return parent() == 7 ? 0 : 1; }\n");
    const leaf = path.join(source, "libleaf.dylib"), parent = path.join(source, "libparent.dylib");
    const executable = path.join(macOS, "llm-wiki-desktop");
    await run("cc", ["-dynamiclib", path.join(source, "leaf.c"), "-Wl,-headerpad_max_install_names", "-o", leaf]);
    await run("cc", ["-dynamiclib", path.join(source, "parent.c"), leaf, "-Wl,-headerpad_max_install_names", "-o", parent]);
    await run("cc", [path.join(source, "main.c"), parent, "-Wl,-headerpad_max_install_names", "-o", executable]);
    await assert.rejects(verifyBundledRuntime(app), /External runtime dependency/);
    const originals = await Promise.all([readFile(leaf), readFile(parent)]);
    const libraries = await stageRuntime(executable, path.join(app, "Contents/Frameworks"));
    assert.equal(libraries.length, 2);
    assert.deepEqual(await Promise.all([readFile(leaf), readFile(parent)]), originals);
    assert.equal(await verifyBundledRuntime(app), 2);
    // Launch after deleting the build-time libraries: no accidental local fallback.
    await rm(source, { recursive: true });
    for (const binary of [...libraries, executable]) await run("codesign", ["--force", "--sign", "-", binary]);
    await run(executable, []);
    await rm(libraries[0]);
    await assert.rejects(verifyBundledRuntime(app), /Missing runtime dependency/);
  } finally { await rm(state, { recursive: true, force: true }); }
});
