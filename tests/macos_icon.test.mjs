import assert from "node:assert/strict";
import test from "node:test";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { validateIcns, verifyBundledIcon } from "../scripts/macos_icon.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
test("bundle configuration explicitly includes the existing cross-platform icons", async () => {
  const config = JSON.parse(await readFile(path.join(root, "src-tauri/tauri.conf.json"), "utf8"));
  for (const icon of ["icons/icon.icns", "icons/icon.ico", "icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png"]) {
    assert.ok(config.bundle.icon.includes(icon), `Missing ${icon}`);
    assert.ok((await readFile(path.join(root, "src-tauri", icon))).length > 0);
  }
  validateIcns(await readFile(path.join(root, "src-tauri/icons/icon.icns")));
  assert.throws(() => validateIcns(Buffer.from("not an icon")), /Invalid ICNS/);
});

test("macOS bundle icon points to a real decodable ICNS and fails closed on missing or corrupt assets", { skip: process.platform !== "darwin" }, async () => {
  await mkdir(path.join(root, ".tmp"), { recursive: true });
  const state = await mkdtemp(path.join(root, ".tmp/icon-test-"));
  const app = path.join(state, "Test.app");
  const resources = path.join(app, "Contents/Resources");
  const plist = path.join(app, "Contents/Info.plist");
  const info = icon => `<?xml version="1.0"?><plist version="1.0"><dict>${icon === null ? "" : `<key>CFBundleIconFile</key><string>${icon}</string>`}</dict></plist>`;
  try {
    await mkdir(resources, { recursive: true });
    await writeFile(plist, info(null));
    await assert.rejects(verifyBundledIcon(app));
    await writeFile(plist, info("icon.icns"));
    await assert.rejects(verifyBundledIcon(app), /ENOENT/);
    await writeFile(path.join(resources, "icon.icns"), "corrupt");
    await assert.rejects(verifyBundledIcon(app), /Invalid ICNS/);
    await copyFile(path.join(root, "src-tauri/icons/icon.icns"), path.join(resources, "icon.icns"));
    assert.equal(await verifyBundledIcon(app), "icon.icns");
    await writeFile(plist, info("../icon.icns"));
    await assert.rejects(verifyBundledIcon(app), /CFBundleIconFile/);
  } finally { await rm(state, { recursive: true, force: true }); }
});
