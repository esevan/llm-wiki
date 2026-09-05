import fs from "node:fs";

const forbidden = [
  /rusqlite/,
  /native::database/,
  /db_path\s*\(/,
  /adapters::(?:sqlite|vault)/,
  /native::(?:vault|semantic)/,
  /\b(?:SELECT|INSERT|UPDATE|DELETE|CREATE TABLE)\b/,
];

for (const file of [
  "src-tauri/src/mcp.rs",
  "src-tauri/src/mcp_ipc.rs",
  "src-tauri/src/native/work_tracking.rs",
]) {
  if (!fs.existsSync(file)) continue;
  const source = fs.readFileSync(file, "utf8");
  for (const pattern of forbidden) {
    if (pattern.test(source)) {
      throw new Error(`${file} crosses the work-tracking application boundary (${pattern})`);
    }
  }
}

const library = fs.readFileSync("src-tauri/src/lib.rs", "utf8");
const bridge = library.match(/pub async fn run_mcp[\s\S]*?\n}/)?.[0] ?? "";
if (!bridge.includes("mcp_ipc::run_stdio_bridge")) {
  throw new Error("run_mcp must forward to the active GUI process");
}
for (const pattern of [/application_paths/, /SqliteWorkTrackingStore/, /MarkdownVaultAdapter/]) {
  if (pattern.test(bridge)) {
    throw new Error(`the MCP bridge must not compose persistence (${pattern})`);
  }
}

for (const file of ["src-tauri/src/mcp.rs", "src-tauri/src/application/work_tracking_service.rs"]) {
  const source = fs.readFileSync(file, "utf8");
  if (/reqwest|chat\/completions|provider_credentials|completion_request/.test(source)) {
    throw new Error(`${file} must not run a hidden conflict-review model`);
  }
}

console.log("verified work-tracking application boundary");
