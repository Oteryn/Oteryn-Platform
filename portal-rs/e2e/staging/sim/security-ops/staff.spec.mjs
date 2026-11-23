import { mkdir, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { expect, get, localized, navigate, requireMail, routeStaging, staffLogin, status, submit, test } from './helpers.mjs';

function unavailable(route) {
  test.info().annotations.push({ type: 'staging limitation', description: `${route} unavailable` });
}

async function roleBoundary(page, path, optional = false) {
  const code = await navigate(page, path);
  if (optional && code === 404) {
    unavailable(path);
    return;
  }
  await status(page, code, 403, path);
}

async function staffPortal(page, request) {
  const response = await get(request, '/admin');
  if (response.status() === 404) {
    await status(page, await navigate(page, '/admin'), 404, 'staff panel unavailable');
    test.skip(true, 'The staging hostname does not serve the staff panel (GET /admin HTTP 404)');
  }
  expect(response.status(), 'signed-out staff entry').toBe(303);
  expect(response.headers().location).toBe('/login');
  await requireMail(request);
}

// The same deploy-seeded staff accounts are shared by all four projects.
async function acquireStaffAccounts() {
  const lock = join(tmpdir(), 'oteryn-security-ops-staff.lock');
  const expires = Date.now() + 300_000;
  while (Date.now() < expires) {
    try {
      await mkdir(lock);
      return () => rmdir(lock);
    } catch (error) {
      if (error.code !== 'EEXIST') throw error;
      await delay(2_000);
    }
  }
  throw new Error('Timed out waiting for the two shared staging staff accounts');
}

test('security and operations stay within their assigned roles', async ({ page, browser, request }, info) => {
  const lang = info.project.metadata.language;
  await staffPortal(page, request);
  const release = await acquireStaffAccounts();
  try {
    await staffLogin(page, lang, 'security');
    const accessPath = `/admin/console/access?lang=${lang}`;
    await status(page, await navigate(page, accessPath), 200, 'security access panel');
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(
      localized(lang, 'Dostęp', 'Access'));
    await expect(page.getByText('security_admin: audit.view', { exact: true })).toBeVisible();
    await expect(page.getByText('operations: downloads.manage', { exact: true })).toBeVisible();

    const auditPath = `/admin/console/audit?lang=${lang}`;
    await status(page, await navigate(page, auditPath), 200, 'security audit panel');
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(localized(lang, 'Audyt', 'Audit'));
    await status(page, (await submit(page, 'main form[method="get"]', { request_id: 'invalid' })).status(),
      400, 'malformed audit request ID');
    const deniedAudit = `${auditPath}&operation=admin.access.denied&target=%2Fadmin%2Froles`;
    await roleBoundary(page, '/admin/players', true);
    await roleBoundary(page, '/admin/moderation/enforcement', true);
    await roleBoundary(page, '/admin/world', true);
    const deniedRows = page.locator('main section li').filter({ hasText: 'admin.access.denied' });
    await status(page, await navigate(page, deniedAudit), 200, 'filtered security audit');
    const beforeDenial = new Set(await deniedRows.allTextContents());

    const opsContext = await browser.newContext();
    try {
      await routeStaging(opsContext);
      const operations = await opsContext.newPage();
      await staffLogin(operations, lang, 'operations');
      await roleBoundary(operations, '/admin/roles');
      await status(page, (await page.reload()).status(), 200, 'security audit reload after operations refusal');
      const afterDenial = await deniedRows.allTextContents();
      expect(afterDenial.some(row => !beforeDenial.has(row)), 'security sees the new role access refusal').toBe(true);
      await roleBoundary(operations, auditPath);
      await roleBoundary(operations, '/admin/players', true);
      await roleBoundary(operations, '/admin/moderation/enforcement', true);
      await roleBoundary(operations, '/admin/analytics/health', true);

      const catalog = '/admin/world';
      const code = await navigate(operations, catalog);
      if (code === 404) {
        unavailable(catalog);
        return;
      }
      await status(operations, code, 200, 'operations catalog');
      const title = localized(lang, 'Świat gry', 'Game world');
      await expect(operations.getByRole('heading', { level: 1 })).toHaveText(title);
      await expect(operations.getByRole('button', { name: localized(lang, 'Sprawdź', 'Inspect') })).toBeVisible();
      const active = operations.locator('main > p').first();
      await expect(active).toContainText(localized(lang, 'Aktywna wersja', 'Active version'));
      const version = (await active.locator('code').innerText()).trim();
      await status(operations, await navigate(operations, `${catalog}?page=0`), 422, 'catalog page outside range');
      await status(operations, await navigate(operations, `${catalog}?version=invalid`), 400, 'malformed catalog version');
      await status(operations, await navigate(operations, catalog), 200, 'catalog after bad input');
      await expect(operations.getByRole('heading', { level: 1 })).toHaveText(title);
      if (version) {
        await status(operations, await navigate(operations, `${catalog}?version=${version}`), 200, 'active version inspection');
        await expect(operations.getByRole('heading', { name: localized(lang, 'Wyniki walidacji', 'Validation results') })).toBeVisible();
        const publish = 'form[method="post"]:has(input[name="action"][value="publish"])';
        if (await operations.locator(publish).count()) {
          await status(operations, (await submit(operations, publish, {})).status(), 409,
            'already active catalog cannot be published again');
        }
      }
      await status(operations, await navigate(operations, catalog), 200, 'catalog remains readable');
      await expect(active.locator('code')).toHaveText(version);
    } finally {
      await opsContext.close();
    }
  } finally {
    await release();
  }
});
