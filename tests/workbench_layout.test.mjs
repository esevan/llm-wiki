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
  await test('Task detail groups unequal-length labels and bodies at the top', async () => {
    for (const [width, height] of [[1440,900], [1920,1080], [3440,1440], [640,480], [390,700]]) {
      const page = await browser.newPage({viewport: {width, height}});
      await page.goto(`${origin}layout-review?mode=detail&alignment=1`);
      await page.locator('[data-control="task-detail-tab-details"]').click();
      const fields = await page.locator('.task-definition-read > div').evaluateAll(elements => elements.map(element => {
        const field = element.getBoundingClientRect();
        const label = element.querySelector('dt').getBoundingClientRect();
        const body = element.querySelector('dd').getBoundingClientRect();
        const labelText = document.createRange();
        labelText.selectNodeContents(element.querySelector('dt'));
        const bodyText = document.createRange();
        bodyText.selectNodeContents(element.querySelector('dd'));
        return {label: element.querySelector('dt').textContent, x:field.x, y:field.y, height:field.height,
          labelTop:label.y, gap:body.y-label.bottom, textGap:bodyText.getBoundingClientRect().top-labelText.getBoundingClientRect().bottom, bodyWidth:body.width};
      }));
      console.log(JSON.stringify({alignmentViewport:[width,height], fields}));
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR) {
        await mkdir(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, {recursive:true});
        await page.screenshot({path:path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `${width}-${height}-detail-alignment.png`)});
      }
      assert.equal(fields.length, 5, 'all definition sections remain present');
      for (const field of fields) {
        assert(field.textGap >= 0 && field.textGap <= 12, `${field.label}: visible label/body gap ${field.textGap}px at ${width}`);
        assert(field.gap >= 0 && field.gap <= 4, `${field.label}: label/body gap ${field.gap}px at ${width}`);
        assert(Math.abs(field.labelTop-field.y) <= 1, `${field.label}: label begins at section top`);
        assert(field.bodyWidth < 1000, 'prose remains readable');
      }
      if (width >= 1440) {
        assert(Math.abs(fields[0].labelTop-fields[1].labelTop) <= 1, 'neighbor labels align at top');
        assert(fields[1].height < fields[0].height/2, 'short outcome must not stretch to the long description height');
      } else {
        assert(fields.every(field => Math.abs(field.x-fields[0].x) <= 1), 'narrow layout uses one column');
      }
      const lastBody = page.locator('.task-definition-read > div').last().locator('dd');
      await lastBody.scrollIntoViewIfNeeded();
      const lastBodyBounds = await lastBody.boundingBox();
      assert(lastBodyBounds.y >= 0 && lastBodyBounds.y+lastBodyBounds.height <= height, 'last section remains reachable by scrolling');
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR) await page.screenshot({path:path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `${width}-${height}-detail-alignment-end.png`)});
      const close = await page.locator('[data-control="task-detail-close"]').boundingBox();
      assert(close.y >= 0 && close.y+close.height <= height, 'close remains visible after scrolling');
      assert.equal(await page.locator('.task-detail').evaluate(el => el.scrollWidth > el.clientWidth+1), false, 'no horizontal overflow');
      await page.close();
    }
  });

  await test('cards keep their width and lanes scroll independently', async () => {
    const measured = [];
    for (const [width, height] of sizes) {
      const page = await browser.newPage({viewport: {width, height}});
      await page.goto(`${origin}layout-review?mode=board&crowded=1`);
      const lists = page.locator('.workbench-lane-cards');
      await lists.first().locator('.canonical-card').first().waitFor();
      await lists.first().hover();
      const before = await lists.evaluateAll(elements => elements.map(el => ({top: el.scrollTop, y: el.getBoundingClientRect().y})));
      const card = await lists.first().locator('.canonical-card').first().boundingBox();
      measured.push({width, cardWidth: card.width});
      if (width >= 1440) assert.equal(card.width, 320);
      else assert(card.width <= 320 && card.width > 200);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      await page.mouse.wheel(0, 350);
      await page.waitForFunction(() => document.querySelector('.workbench-lane-cards').scrollTop > 0);
      const after = await lists.evaluateAll(elements => elements.map(el => ({top: el.scrollTop, y: el.getBoundingClientRect().y})));
      assert(after[0].top > 0);
      assert.deepEqual(after.slice(1), before.slice(1), 'neighbor lanes and other categories remain stationary');
      const first = lists.first();
      await first.focus();
      await page.keyboard.press('End');
      await page.waitForFunction(() => { const el = document.querySelector('.workbench-lane-cards'); return el.scrollTop >= el.scrollHeight - el.clientHeight - 1; });
      for (let index = 1; index < await lists.count(); index++) {
        const lane = lists.nth(index);
        const original = await lists.evaluateAll(elements => elements.map(el => el.scrollTop));
        const changed = await lane.evaluate(el => { el.scrollTop = 120; return el.scrollTop; });
        assert(changed > 0, 'every populated lane has an independent scroll range');
        const offsets = await lists.evaluateAll(elements => elements.map(el => el.scrollTop));
        assert.deepEqual(offsets.filter((_, i) => i !== index), original.filter((_, i) => i !== index));
      }
      const header = page.locator('.workbench-lane-header').first();
      assert(await header.isVisible());
      const lastControl = first.locator('button').last();
      await lastControl.focus();
      const control = await lastControl.boundingBox();
      const listBox = await first.boundingBox();
      assert(control.y >= listBox.y && control.y + control.height <= listBox.y + listBox.height + 1, 'last control reachable inside lane');
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR) {
        await mkdir(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, {recursive: true});
        await page.screenshot({path: path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `${width}-${height}-lanes.png`)});
      }
      await page.close();
    }
    console.log(JSON.stringify({cardMeasurements: measured}));
  });
  await test('lane columns grow while card height width previews and actions stay fixed', async () => {
    const measurements=[];
    for (const [width, expectedColumns] of [[1440,1],[2560,2],[3440,2],[3840,3],[5120,4],[390,1],[320,1]]) {
      const page=await browser.newPage({viewport:{width,height:900}});
      await page.goto(`${origin}layout-review?mode=board&crowded=1&integrated=1&previews=1&held=3`);
      await page.locator('[data-category="general"] [data-lane="tasks"] article').first().waitFor();
      const laneLists=page.locator('[data-category="general"] .workbench-lane-cards');
      const counts=await laneLists.evaluateAll(lists=>lists.map(list=>{const cards=[...list.querySelectorAll(':scope > article')];return cards.filter(card=>Math.abs(card.getBoundingClientRect().y-cards[0].getBoundingClientRect().y)<1).length;}));
      assert.deepEqual(counts,[expectedColumns,expectedColumns,expectedColumns]);
      await page.locator('.workbench-held > summary').click();
      const geometry=await page.locator('.workbench-main article.canonical-card').evaluateAll(cards=>cards.map(card=>{const box=card.getBoundingClientRect(),footer=card.querySelector('footer').getBoundingClientRect();return {width:box.width,height:box.height,footerBottom:box.bottom-footer.bottom,buttons:[...card.querySelectorAll(':scope > footer button, :scope > .workbench-subtasks-open')].map(button=>{const r=button.getBoundingClientRect();return {inside:r.top>=box.top && r.bottom<=box.bottom && r.left>=box.left && r.right<=box.right};})};}));
      assert(geometry.every(box=>box.height===240), 'long short empty active and held cards share one height');
      assert(geometry.every(box=>box.buttons.every(button=>button.inside)), 'all actions stay inside fixed cards');
      assert(geometry.every(box=>Math.abs(box.footerBottom-17)<1), 'action baseline is consistent');
      if (width>=1440) assert(geometry.every(box=>box.width===320));
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
      const preview=page.locator('article[data-entity-id="task-0"] .workbench-card-preview');
      assert((await preview.textContent()).length>1000, 'CSS clamp preserves the original text');
      assert.equal(await preview.evaluate(el=>getComputedStyle(el).webkitLineClamp),'2');
      assert((await preview.boundingBox()).height<=36);
      measurements.push({viewport:width,columns:counts,cardHeight:geometry[0].height});
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR) {
        await mkdir(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR,{recursive:true});
        await laneLists.last().scrollIntoViewIfNeeded();
        await page.screenshot({path:path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR,`cards-${width}-columns.png`)});
      }
      if (width===1440 || width===320) {
        await page.locator('article[data-entity-id="task-0"] [data-control="task-card-open"]').click();
        await page.locator('[data-control="task-detail-tab-details"]').click();
        await page.locator('[data-control="task-origin-capture"] > summary').click();
        const full=page.locator('[data-control="task-origin-capture"] > p');
        assert((await full.textContent()).length>1000);
        assert.notEqual(await full.evaluate(el=>getComputedStyle(el).webkitLineClamp),'2', 'full detail is not clamped');
        await page.locator('[data-control="task-detail-close"]').click();
        await page.locator('article[data-entity-id="preview-parent"] [data-control="task-subtasks-expand"]').click();
        await page.locator('[data-control="task-hierarchy-details"] > summary').click();
        await page.locator('[data-control="task-family-open"]').click();
        await page.getByRole('dialog',{name:'Child Task',exact:true}).waitFor();
        await page.locator('[data-control="task-detail-close"]').click();
        assert.equal((await page.locator('article[data-entity-id="preview-parent"]').boundingBox()).height,240);
      }
      await page.close();
    }
    console.log(JSON.stringify({laneCardMeasurements:measurements}));
  });
  await test('held disclosure stays compact and expands into one horizontal card row', async () => {
    const measurements = [];
    for (const width of [1440, 2560, 3440, 5120, 390, 320]) for (const count of [0, 1, 20]) {
      const page = await browser.newPage({viewport:{width,height:900},hasTouch:true});
      await page.goto(`${origin}layout-review?mode=board&held=${count}`);
      const held = page.locator('.workbench-held');
      const summary = held.locator(':scope > summary');
      const row = held.locator('.workbench-held-cards');
      const board = page.locator('.workbench-board');
      await summary.waitFor();
      assert.equal(await held.getAttribute('open'), null);
      assert((await summary.textContent()).includes(`(${count})`));
      const collapsed = await held.boundingBox();
      const initialBoard = await board.boundingBox();
      const initialBoardTop = initialBoard.y + await page.evaluate(() => scrollY);
      assert(collapsed.height <= 48, 'closed hold is a compact title row');
      assert(initialBoard.y >= collapsed.y + collapsed.height - 1, 'board starts below the closed disclosure');
      const appearance = await held.evaluate(el => {const s=getComputedStyle(el);return {bg:s.backgroundColor,border:s.borderTopWidth,padding:s.paddingTop,align:s.alignSelf};});
      assert.equal(appearance.bg, 'rgba(0, 0, 0, 0)');
      assert.equal(appearance.border, '0px');
      assert.equal(appearance.padding, '0px');
      assert.equal(appearance.align, 'start');
      assert.equal(await row.isVisible(), false, 'native closed contents cannot contribute a panel');
      await summary.focus();
      await page.keyboard.press('Enter');
      assert.equal(await held.getAttribute('open'), '');
      if (count) {
        const cards = row.locator(':scope > article');
        await cards.first().waitFor();
        const boxes = await cards.evaluateAll(elements => elements.map(el => {const r=el.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};}));
        assert(boxes.every(box => Math.abs(box.y - boxes[0].y) < 1), 'all cards occupy one row');
        assert(boxes.every(box => box.width === boxes[0].width), 'one consistent card width');
        if (width >= 390) assert.equal(boxes[0].width, 320);
        else assert(boxes[0].width < 320 && boxes[0].width > 200);
        const expanded = await held.boundingBox();
        const lower = await board.boundingBox();
        assert(lower.y >= expanded.y + expanded.height - 1, 'expanded content never overlaps the lower board');
        assert(expanded.height <= collapsed.height + Math.max(...boxes.map(box => box.height)) + 32, 'row height does not grow with card count');
        if (count > 1) {
          const range = await row.evaluate(el => el.scrollWidth - el.clientWidth);
          assert(range > 0, 'many cards overflow horizontally');
          await row.hover();
          const before = await board.boundingBox();
          await page.mouse.wheel(400, 0);
          await page.waitForFunction(() => document.querySelector('.workbench-held-cards').scrollLeft > 0);
          assert.deepEqual(await board.boundingBox(), before, 'horizontal scrolling leaves the board stationary');
          await row.evaluate(el => {el.scrollLeft = 0;});
          await row.focus();
          await page.keyboard.press('ArrowRight');
          await page.waitForFunction(() => document.querySelector('.workbench-held-cards').scrollLeft > 0);
          await cards.last().locator('.canonical-card-title').focus();
          const last = await cards.last().boundingBox(), viewport = await row.boundingBox();
          assert(last.x >= viewport.x - 1 && last.x + last.width <= viewport.x + viewport.width + 1, 'keyboard focus reaches the last card');
          assert.equal(await row.evaluate(el => el.scrollHeight > el.clientHeight + 1), false, 'held cards create no vertical scroller');
          if (width <= 390) {
            await row.scrollIntoViewIfNeeded();
            await row.evaluate(el => {el.scrollLeft = 0;});
            const touchBox = await row.boundingBox();
            const x = touchBox.x + Math.min(260, touchBox.width - 24), y = touchBox.y + touchBox.height / 2;
            const cdp = await page.context().newCDPSession(page);
            await cdp.send('Input.dispatchTouchEvent', {type:'touchStart',touchPoints:[{x,y}]});
            for (let step = 1; step <= 4; step++) {
              await cdp.send('Input.dispatchTouchEvent', {type:'touchMove',touchPoints:[{x:x-step*30,y}]});
              await page.waitForTimeout(20);
            }
            await cdp.send('Input.dispatchTouchEvent', {type:'touchEnd',touchPoints:[]});
            await page.waitForFunction(() => document.querySelector('.workbench-held-cards').scrollLeft > 0);
            await cdp.detach();
          }
        }
        measurements.push({viewport:width,count,cardWidth:boxes[0].width,closedHeight:collapsed.height,expandedHeight:expanded.height});
      } else {
        assert.equal(await row.isVisible(), false);
        assert.equal((await held.boundingBox()).height, collapsed.height, 'empty expansion adds no blank panel');
      }
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR && [1440,390,320].includes(width)) {
        await mkdir(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, {recursive:true});
        await page.screenshot({path:path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `held-${width}-${count}-expanded.png`)});
      }
      await summary.focus();
      await page.keyboard.press('Enter');
      assert.equal(await held.getAttribute('open'), null);
      assert.equal((await held.boundingBox()).height, collapsed.height);
      assert.equal((await board.boundingBox()).y + await page.evaluate(() => scrollY), initialBoardTop, 'collapse restores the lower board document position');
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR && count === 20 && [1440,390].includes(width)) await page.screenshot({path:path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `held-${width}-collapsed.png`)});
      await page.close();
    }
    console.log(JSON.stringify({heldMeasurements:measurements}));
  });
  await test('combined menus hold routing and saved Work Log survive wide and narrow layouts', async () => {
    for (const [width, height] of [[1440,900], [3440,1440], [390,700]]) {
      const page = await browser.newPage({viewport: {width, height}});
      page.setDefaultTimeout(10000);
      await page.goto(`${origin}layout-review?mode=board&crowded=1&integrated=1`);
      const held = page.locator('.workbench-held');
      await held.waitFor();
      assert.equal(await held.getAttribute('open'), null, 'one global hold region begins collapsed');
      assert.equal(await held.count(), 1);
      const card = id => page.locator(`article[data-entity-id="${id}"]`);
      const active = card('integration-active');
      const trigger = active.locator('[data-control="item-actions-menu"]');
      await trigger.focus();
      await page.keyboard.press('ArrowDown');
      const menu = page.getByRole('menu');
      await menu.waitFor();
      const bounds = await menu.boundingBox();
      assert(bounds.x >= 0 && bounds.x + bounds.width <= width && bounds.y >= 0 && bounds.y + bounds.height <= height, 'menu fits viewport');
      assert.equal(await page.getByRole('menuitem', {name:'Put on hold', exact:true}).evaluate(el => el === document.activeElement), true);
      await page.keyboard.press('End');
      assert.equal(await page.getByRole('menuitem', {name:'Delete', exact:true}).evaluate(el => el === document.activeElement), true);
      await page.keyboard.press('Escape');
      assert.equal(await trigger.evaluate(el => el === document.activeElement), true);
      await trigger.click();
      await page.getByRole('menuitem', {name:'Put on hold', exact:true}).click();
      await held.locator('article[data-entity-id="integration-active"]').waitFor({state:'visible'});
      assert.equal(await active.evaluate(el => Boolean(el.closest('.workbench-held'))), true);
      const heldWidth = (await active.boundingBox()).width;
      assert(width >= 1440 ? heldWidth === 320 : heldWidth <= 320 && heldWidth > 200);
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR) {
        await active.scrollIntoViewIfNeeded();
        await page.screenshot({path:path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `integrated-${width}-hold.png`)});
      }
      await active.locator('[data-control="item-actions-menu"]').click();
      await page.getByRole('menuitem', {name:'Resume', exact:true}).click();
      await page.locator('.workbench-active article[data-entity-id="integration-active"]').waitFor();
      await active.locator('[data-control="item-actions-menu"]').click();
      await page.getByRole('menuitem', {name:'Move to execution backlog', exact:true}).click();
      await page.locator('[data-lane="tasks"] article[data-entity-id="integration-active"]').waitFor();
      await card('integration-guarded').locator('[data-control="item-actions-menu"]').click();
      assert(await page.getByRole('menuitem', {name:'Put on hold', exact:true}).isDisabled());
      assert(await page.getByRole('menuitem', {name:'Move to execution backlog', exact:true}).isDisabled());
      await page.keyboard.press('Escape');
      // Open the last card's menu at a scroll boundary: portal must escape lane clipping.
      const lane = page.locator('[data-category="general"] [data-lane="inbox"] .workbench-lane-cards');
      const last = lane.locator('article').last();
      await last.locator('[data-control="item-actions-menu"]').click();
      const edgeMenu = await menu.boundingBox();
      assert(edgeMenu.x >= 0 && edgeMenu.y >= 0 && edgeMenu.x + edgeMenu.width <= width && edgeMenu.y + edgeMenu.height <= height);
      const deleteItem = page.getByRole('menuitem', {name:'Delete', exact:true});
      assert.equal(await deleteItem.evaluate(el => { const r=el.getBoundingClientRect(); return el.contains(document.elementFromPoint(r.x+r.width/2, r.y+r.height/2)); }), true, 'menu action is not clipped or covered');
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR) await page.screenshot({path:path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `integrated-${width}-menu.png`)});
      await deleteItem.click();
      await page.getByRole('alertdialog').waitFor();
      await page.locator('[data-control="task-delete-cancel"]').click();
      assert.equal(await last.count(), 1, 'cancel preserves Capture');
      await last.locator('.canonical-card-title').click();
      await page.locator('.refinement-panel[role="dialog"]').waitFor();
      await page.locator('[data-control="refinement-close"]').click();
      const ready = card('task-0');
      await ready.locator('.canonical-card-title').click();
      const detail = page.locator('.task-detail');
      await detail.waitFor();
      await detail.locator('[data-control="task-detail-tab-work"]').click();
      await page.getByText('Review the first draft only.', {exact:true}).waitFor();
      assert(await page.getByText(/Draft a customer reply/).isVisible());
      assert.equal(await detail.locator('[data-control="task-worklog-summary-details"][open]').count(), 0);
      assert.equal(await detail.locator('[data-control="task-worklog-original-details"][open]').count(), 0);
      assert.equal(await detail.locator('[data-control="task-worklog-original-details"]').count(), 2);
      // The detail modal's focus manager must allow its own portalled action menu.
      await detail.locator('[data-control="item-actions-menu"]').click();
      await page.getByRole('menuitem', {name:'Delete', exact:true}).waitFor();
      assert.equal(await page.getByRole('menuitem', {name:'Delete', exact:true}).evaluate(el => el === document.activeElement), true);
      await page.keyboard.press('Escape');
      assert(await detail.isVisible());
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR) {
        await detail.locator('.work-log-distillation').first().scrollIntoViewIfNeeded();
        await page.screenshot({path:path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `integrated-${width}-worklog.png`)});
      }
      await page.locator('[data-control="task-detail-close"]').click();
      await page.waitForFunction(() => document.activeElement?.closest('article')?.getAttribute('data-entity-id') === 'task-0');
      await page.close();
    }
  });
  await test('short-window menus keep their focused action reachable', async () => {
    const page = await browser.newPage({viewport:{width:390,height:200}});
    await page.goto(`${origin}layout-review?mode=board&integrated=1`);
    await page.locator('article[data-entity-id="integration-guarded"] [data-control="item-actions-menu"]').click();
    const action = page.getByRole('menuitem', {name:'Delete',exact:true});
    await action.waitFor();
    const bounds = await action.boundingBox();
    assert(bounds.y >= 0 && bounds.y + bounds.height <= 200, 'focused action scrolls inside a bounded menu');
    assert.equal(await action.evaluate(el => el === document.activeElement), true);
    await page.keyboard.press('Escape');
    assert.equal(await page.getByRole('menu').count(), 0);
    await page.close();
  });
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
  await test('Work log keeps request action result readable and details collapsed', async () => {
    for (const width of [390, 1440]) for (const lang of ['en', 'ko']) {
      const page = await browser.newPage({viewport:{width,height:900}});
      await page.goto(`${origin}layout-review?mode=worklog&lang=${lang}`);
      const summary = page.locator('.work-log-distillation');
      await summary.waitFor();
      assert.equal(await summary.locator('[data-control="task-worklog-summary-details"]').getAttribute('open'), null);
      assert(await page.getByText(/Draft a customer reply/).isVisible());
      assert(await page.getByText(/Export failed; the reply/).first().isVisible());
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR) {
        await mkdir(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, {recursive:true});
        await page.screenshot({path:path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `worklog-${width}-${lang}-collapsed.png`)});
      }
      await page.locator('[data-control="task-worklog-summary-details"] > summary').focus();
      await page.keyboard.press('Enter');
      assert.equal(await summary.locator('[data-control="task-worklog-summary-details"]').getAttribute('open'), '');
      if (process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR) {
        await mkdir(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, { recursive: true });
        await page.screenshot({path:path.join(process.env.LLM_WIKI_LAYOUT_ARTIFACT_DIR, `worklog-${width}-${lang}.png`)});
      }
      await page.close();
    }
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
