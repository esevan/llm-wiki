// A suspended WebView can resume after the deadline. Inspect the condition
// once on resume before timing out; strict latency budgets are checked separately.
const pause = (ms: number) => new Promise(resolve => window.setTimeout(resolve, ms));
export async function waitFor(check: () => boolean, label: string) {
  const deadline = performance.now() + 10_000;
  while (true) {
    if (check()) return;
    if (performance.now() >= deadline) break;
    await pause(50);
  }
  throw new Error(`Timed out waiting for ${label}`);
}
export async function waitForAsync(check: () => Promise<boolean>, label: string) {
  const deadline = performance.now() + 10_000;
  while (true) {
    if (await check()) return;
    if (performance.now() >= deadline) break;
    await pause(50);
  }
  throw new Error(`Timed out waiting for ${label}`);
}
