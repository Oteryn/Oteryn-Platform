/* Run with external test dependencies; no package manifest or bundler in the portal. */
const assert = require("node:assert/strict");
const path = require("node:path");
const { createRequire } = require("node:module");
const modules = createRequire(path.resolve(process.env.DESIGN_BROWSER_MODULES || "/tmp/portal-t39-browser", "package.json"));
const { chromium } = modules("playwright-core");
const AxeBuilder = modules("@axe-core/playwright").default;
const url = process.env.DESIGN_GALLERY_URL || "http://127.0.0.1:8099/design/index.html";

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
  try {
    for (const width of [360, 1280]) {
      const context = await browser.newContext({ viewport: { width, height: 900 }, reducedMotion: "reduce" });
      const page = await context.newPage();
      const errors = [];
      page.on("pageerror", error => errors.push(error.message));
      await page.goto(url);
      assert.equal(await page.locator("dialog").count(), 4);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, `page overflow at ${width}px`);
      for (const theme of ["dark", "light"]) {
        const section = page.locator(`#${theme}`);
        for (const type of ["modal", "drawer"]) {
          const trigger = section.locator(`[aria-controls="${theme}-${type}-dialog"]`);
          const dialog = page.locator(`#${theme}-${type}-dialog`);
          await trigger.focus();
          await page.keyboard.press("Enter");
          assert.equal(await dialog.evaluate(node => node.open), true);
          assert.equal(await page.evaluate(() => !!document.activeElement.closest("dialog")), true);
          await page.keyboard.press("Tab");
          await page.keyboard.press("Tab");
          assert.equal(await page.evaluate(() => !!document.activeElement.closest("dialog")), true);
          const result = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"]).analyze();
          assert.deepEqual(result.violations.map(v => ({ id: v.id, targets: v.nodes.map(n => n.target) })), [], `${theme} ${type} accessibility`);
          await page.keyboard.press("Escape");
          assert.equal(await dialog.evaluate(node => node.open), false);
          assert.equal(await trigger.evaluate(node => document.activeElement === node), true);
          await trigger.click();
          await dialog.locator("[data-close]").click();
          assert.equal(await trigger.evaluate(node => document.activeElement === node), true);
        }
        const table = section.locator(".op-table-wrap");
        await table.focus();
        if (width === 360) {
          assert.equal(await table.evaluate(node => node.scrollWidth > node.clientWidth), true);
          await page.keyboard.press("ArrowRight");
          await page.waitForFunction(id => document.querySelector(`#${id} .op-table-wrap`).scrollLeft > 0, theme);
        }
        const input = section.locator("input[aria-invalid=true]");
        assert.equal(await input.getAttribute("aria-describedby"), `${theme}-invalid-hint ${theme}-invalid-error`);
        assert.equal(await section.locator(".op-button:disabled").count(), 1);
      }
      const result = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"]).analyze();
      assert.deepEqual(result.violations.map(v => ({ id: v.id, targets: v.nodes.map(n => n.target) })), [], `gallery accessibility at ${width}px`);
      assert.deepEqual(errors, []);
      await page.screenshot({ path: `/tmp/portal-t39-${width}.png`, fullPage: true });
      console.log(`${width}px: both themes, no page overflow, keyboard overlays/table, field/refusal states; axe: 0 violations`);
      await context.close();
    }
    const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 360, height: 900 } });
    const page = await context.newPage();
    await page.goto(url);
    for (const theme of ["dark", "light"]) {
      for (const type of ["modal", "drawer"]) {
        const details = page.locator(`#${theme}-${type}`);
        await details.locator("summary").focus();
        await page.keyboard.press("Enter");
        assert.equal(await details.evaluate(node => node.open), true);
        assert.equal(await details.locator("section").isVisible(), true);
        assert.equal(await details.locator("[data-close]").isVisible(), false);
      }
    }
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    console.log("360px without JavaScript: all four native disclosures accessible; no page overflow");
    await context.close();
    const preference = await browser.newContext();
    const systemPage = await preference.newPage();
    await systemPage.goto(url);
    for (const colorScheme of ["light", "dark"]) {
      await systemPage.emulateMedia({ colorScheme });
      const colors = await systemPage.evaluate(() => [document.body, document.querySelector("#dark"), document.querySelector("#light")].map(node => getComputedStyle(node).backgroundColor));
      assert.equal(colors[0], colorScheme === "dark" ? colors[1] : colors[2]);
      assert.notEqual(colors[1], colors[2]);
    }
    console.log("System color preference: light/dark defaults verified; explicit themes remain stable");
    await preference.close();
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
