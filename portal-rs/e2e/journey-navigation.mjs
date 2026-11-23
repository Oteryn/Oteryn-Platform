export async function revealPortalLink(page, selector) {
  const link = page.locator(selector).filter({ visible: true }).first();
  if (await link.count()) return link;

  const fallback = page.locator('#portal-header details.om-fallback-menu');
  const summary = fallback.locator(':scope > summary');
  if (await summary.isVisible() && await fallback.getAttribute('open') === null) {
    await summary.click();
  }

  const menu = page.locator('#portal-menu-toggle');
  if (!await link.count() && await menu.isVisible() && await menu.getAttribute('aria-expanded') === 'false') {
    await menu.click();
  }

  const nav = page.locator('#portal-navigation');
  const disclosure = nav.locator('details').filter({ has: page.locator(selector) }).first();
  const toggle = disclosure.locator(':scope > summary');
  if (!await link.count() && await toggle.isVisible() && await disclosure.getAttribute('open') === null) {
    await toggle.click();
  }

  if (!await link.count()) {
    for (const panel of await nav.locator('.om-panel').all()) {
      if (await panel.locator(selector).count() && !await panel.isVisible()) {
        const id = await panel.getAttribute('id');
        const button = nav.locator(`button[aria-controls="${id}"]`);
        if (await button.isVisible()) await button.click();
        break;
      }
    }
  }
  if (!await link.count() && await page.locator('dialog').locator(selector).count()) {
    await nav.locator('[data-account-link]').click();
  }
  return link;
}
