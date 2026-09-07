import { expect } from '@playwright/test';

/** Reveal the real public navigation; never force-click a hidden destination. */
export async function revealPublicNavigationLink(page, name) {
  const desktop = page.getByRole('navigation', { name: 'Public navigation' });
  if (await desktop.isVisible()) {
    const link = desktop.getByRole('link', { name, exact: true, includeHidden: true });
    const group = link.locator('xpath=ancestor::details[1]');
    if (await group.count() && !(await group.evaluate((element) => element.open))) {
      await group.locator('summary').click();
    }
    await expect(link).toBeVisible();
    return link;
  }
  const menu = page.locator('.mobile-nav');
  if (!(await menu.evaluate((element) => element.open))) await menu.locator('summary').click();
  const link = menu.getByRole('link', { name, exact: true });
  await expect(link).toBeVisible();
  return link;
}
