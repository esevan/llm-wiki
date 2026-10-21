import { execFile } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

const execute = promisify(execFile);

export function validateIcns(bytes) {
  if (bytes.length < 16 || bytes.toString("ascii", 0, 4) !== "icns" || bytes.readUInt32BE(4) !== bytes.length) {
    throw new Error("Invalid ICNS header or container length");
  }
  let images = 0;
  for (let offset = 8; offset < bytes.length;) {
    if (offset + 8 > bytes.length) throw new Error("Truncated ICNS entry");
    const kind = bytes.toString("ascii", offset, offset + 4);
    const length = bytes.readUInt32BE(offset + 4);
    if (length <= 8 || offset + length > bytes.length) throw new Error("Invalid ICNS entry length");
    if (/^ic(?:0[4-9]|1[0-4])$/.test(kind) || /^(?:is32|il32|ih32|it32)$/.test(kind)) images++;
    offset += length;
  }
  if (!images) throw new Error("ICNS has no image entries");
}

export async function verifyBundledIcon(app) {
  const plist = path.join(app, "Contents/Info.plist");
  const { stdout } = await execute("/usr/bin/plutil", ["-extract", "CFBundleIconFile", "raw", "-o", "-", plist]);
  const icon = stdout.trim();
  if (!icon || path.basename(icon) !== icon || !icon.endsWith(".icns")) {
    throw new Error("CFBundleIconFile must name an ICNS file in Contents/Resources");
  }
  const iconPath = path.join(app, "Contents/Resources", icon);
  validateIcns(await readFile(iconPath));
  // Let macOS decode the bundled image as well as checking its container.
  await execute("/usr/bin/sips", ["-g", "pixelWidth", "-g", "pixelHeight", iconPath]);
  return icon;
}
