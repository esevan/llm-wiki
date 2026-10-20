import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdir } from 'node:fs/promises';
import path from 'node:path';
import { createServer } from 'vite';

// Use an existing browser and Playwright installation; never install or touch
// the user's profile. Run separately from jsdom tests, which have no layout.
const { chromium } = await import(process.env.LLM_WIKI_PLAYWRIGHT_MODULE || 'playwright-core');
const server = await createServer({
  plugins: [{ name: 'layout-fixture', configureServer(server) {
    server.middlewares.use('/layout-review', async (_req, res) => {
      res.setHeader('Content-Type', 'text/html');
      res.end(await server.transformIndexHtml('/layout-review', '<html><head></head><body><div id="root"></div><script type="module" src="/src/test/workbenchLayout.fixture.tsx"></script></body></html>'));
    });
  } }],
  server: { host: '127.0.0.1', port: 0 },
});
await server.listen();
const origin = server.resolvedUrls.local[0];
const browser = await chromium.launch({ headless: true, ...(process.env.LLM_WIKI_BROWSER_EXECUTABLE ? { executablePath: process.env.LLM_WIKI_BROWSER_EXECUTABLE } : {}) });
const sizes = [[1440,900], [2560,1440], [3440,1440], [5120,1440], [640,480], [390,700]];
try {
  await test('open workspaces grow when the same window is maximized', async () => {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    for (const [mode, selector] of [['board', '.workbench-main'], ['detail', '.task-detail'], ['refinement', '.refinement-panel[role="dialog"]']]) {
      await page.setViewportSize({ width: 1440, height: 900 });
      await page.goto(`${origin}layout-review?mode=${mode}`);
      await page.locator(selector).waitFor();
      const before = await page.locator(selector).boundingBox();
      await page.setViewportSize({ width: 3440, height: 1440 });
      const after = await page.locator(selector).boundingBox();
      assert(after.width > before.width * 2, `${mode}: live resize must increase rendered width`);
    }
    await page.close();
  });
  await test('rendered Workbench and content dialogs use available width', async () => {
    for (const [width, height] of sizes) {
      const page = await browser.newPage({ viewport: { width, height } });
      page.on('pageerror', error => console.error(error.message));
      page.setDefaultTimeout(10000);
      for (const mode of ['board', 'detail', 'refinement', 'confirm']) {
        await page.goto(`${origin}layout-review?mode=${mode}`);
        const selector = mode === 'board' ? '.workbench-main' : mode === 'detail' ? '.task-detail' : mode === 'refinement' ? '.refinement-panel[role="dialog"]' : '.workbench-delete-dialog';
        await page.locator(selector).waitFor();
        if (mode === 'detail') await page.locator('[data-control="task-detail-tab-details"]').click();
        const bounds = await page.locator(selector).boundingBox();
        console.log(JSON.stringify({ viewport: [width,height], mode, bounds }));
        if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR) {
          await mkdir(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, { recursive: true });
          await page.screenshot({ path: path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `${width}-${height}-${mode}.png`) });
        }
        assert(bounds.x >= 0 && bounds.x + bounds.width <= width + 1, `${mode}: horizontal gutters at ${width}`);
        if (mode === 'board') {
          const main = await page.locator('main').boundingBox();
          assert(main.x + main.width >= width - 1, `Workbench shell must use remaining width at ${width}`);
          assert(bounds.width > main.width * .65, 'board must use the main content area');
          assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false, 'board must not overflow horizontally');
        } else if (mode === 'confirm') {
          assert(bounds.width <= 470, 'confirmation must stay compact');
        } else {
          assert(bounds.width >= width - 64, `${mode}: use viewport width at ${width}`);
          assert(bounds.y >= 0 && bounds.y + bounds.height <= height + 1, `${mode}: vertical gutters`);
          const close = page.locator(mode === 'detail' ? '[data-control="task-detail-close"]' : '[data-control="refinement-close"]');
          await close.focus();
          assert(await close.isVisible(), 'close remains reachable');
          const closeBox = await close.boundingBox();
          assert(closeBox.y >= 0 && closeBox.y + closeBox.height <= height, 'close is inside viewport');
          const overflow = await page.locator(selector).evaluate(el => el.scrollWidth > el.clientWidth + 1);
          assert.equal(overflow, false, `${mode}: no horizontal content overflow at ${width}`);
          if (mode === 'detail' && width >= 2560) {
            const prose = await page.locator('.task-definition-read dd').first().boundingBox();
            assert(prose.width < 1000, 'definition prose keeps a readable measure');
          }
          if (mode === 'detail') {
            await page.locator('[data-control="task-definition-edit"]').click();
            const lastField = page.locator('[data-control="task-revision-criteria"]');
            await lastField.focus();
            await lastField.scrollIntoViewIfNeeded();
            const field = await lastField.boundingBox();
            assert(field.y >= 0 && field.y + field.height <= height, 'last edit field reachable');
          } else {
            const composer = page.locator('[data-control="refinement-message"]');
            await composer.focus();
            await composer.scrollIntoViewIfNeeded();
            const input = await composer.boundingBox();
            assert(input.y >= 0 && input.y + input.height <= height, 'composer reachable');
          }
        }
      }
      await page.close();
    }
  });
} finally {
  await browser.close();
  await server.close();
}
