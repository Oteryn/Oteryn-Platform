import {readFileSync} from 'node:fs';
import {test, expect} from '../fixtures.mjs';
import {action, seededAccount, seededLogin, seededStaff, unique} from '../staff-rbac-helpers.mjs';

test('player acknowledges a restriction and sees the staff appeal decision', async ({page, browser}, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const account = await seededAccount(lang);
  await seededLogin(page, lang, account);
  const history = `/${lang}/account/sanctions`;
  await page.goto(`/${lang}/`);
  await expect(page.locator('.om-mast.is-enhanced')).toBeVisible();
  if (info.project.use.viewport.width === 390) {
    await page.locator('#portal-menu-toggle').click();
  }
  await page.locator('[data-account-link]').click();
  const link = page.locator(`#podglad-konta a[href="${history}"]`);
  await expect(link).toHaveText(lang === 'pl' ? 'Historia kar' : 'Sanction history');
  const [historyResponse] = await Promise.all([
    page.waitForResponse(response => new URL(response.url()).pathname === history),
    link.click(),
  ]);
  expect(historyResponse.status()).toBe(200);
  expect(historyResponse.headers()['cache-control'].split(',').map(value => value.trim())).toContain('no-store');
  await expect(page).toHaveURL(new RegExp(`/${lang}/account/sanctions$`));
  await expect(page.getByRole('heading', {name: lang === 'pl' ? 'Historia kar' : 'Sanction history'})).toBeVisible();

  const moderator = await browser.newContext({
    baseURL: new URL(page.url()).origin,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  });
  const enforcement = '/admin/moderation/enforcement';
  try {
    const staffPage = await moderator.newPage();
    await seededStaff(staffPage, lang, 'moderator');
    expect((await staffPage.goto(`${enforcement}/create`)).status()).toBe(200);
    const reason = unique();
    const notes = unique();
    const now = Math.floor(Date.now() / 1000);
    await action(staffPage, 'form:has(input[value=sanction])', {
      target: account.id, category: 'restriction', status: 'active',
      public_reason: reason, moderator_notes: notes,
      effective_at: String(now - 60), expires_at: String(now + 3600),
    });
    await expect(staffPage).toHaveURL(/enforcement\/[a-f0-9-]+$/);
    const staffPath = new URL(staffPage.url()).pathname;
    const id = staffPath.split('/').at(-1);
    const detail = `${history}/${id}`;

    await page.reload();
    await page.getByRole('link', {name: reason}).click();
    await expect(page).toHaveURL(new RegExp(`/sanctions/${id}$`));
    const main = page.locator('main');
    await expect(main).toContainText(lang === 'pl' ? 'Aktywna' : 'Active');
    const dates = await main.locator('time[datetime]').evaluateAll(nodes =>
      nodes.map(node => Date.parse(node.dateTime)));
    expect(dates).toEqual([(now - 60) * 1000, (now + 3600) * 1000]);
    await expect(main).not.toContainText(notes);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
    expect(overflow).toBe(false);

    await action(page, 'main form[action$="/acknowledge"]');
    await expect(main).toContainText(lang === 'pl' ? 'Potwierdzono zapoznanie się' : 'Acknowledged');
    const message = `Please review ${unique()} <example>`;
    await action(page, 'main form[action$="/appeal"]', {message});
    const requested = lang === 'pl' ? 'Odwołanie złożone' : 'Appeal submitted';
    await expect(main).toContainText(requested);
    await expect(main).toContainText(message);
    await expect(main.locator('example')).toHaveCount(0);
    await expect(main.locator('form[action$="/appeal"]')).toHaveCount(0);

    expect((await staffPage.goto(enforcement)).status()).toBe(200);
    const queueItem = staffPage.locator('li').filter({has: staffPage.locator(`a[href="${staffPath}"]`)});
    await expect(queueItem).toContainText(requested);
    await queueItem.getByRole('link').click();
    await expect(staffPage.locator('main')).toContainText(message);
    const decision = `Restriction lifted ${unique()}`;
    await action(staffPage, 'form:has(input[value=appeal])', {
      appeal_status: 'accepted', appeal_outcome: decision,
    });
    await expect(staffPage.locator('main')).toContainText(decision);

    expect((await page.goto(detail)).status()).toBe(200);
    await expect(main).toContainText(lang === 'pl' ? 'Odwołanie uwzględnione' : 'Appeal accepted');
    await expect(main).toContainText(lang === 'pl' ? 'Uchylona' : 'Revoked');
    await expect(main).toContainText(decision);
    await expect(main.locator('form[action$="/appeal"]')).toHaveCount(0);
  } finally {
    await moderator.close();
  }
});

test('i18n-parity for the sanction history and staff appeal labels', () => {
  const messages = (path, select = () => true) => {
    const ftl = readFileSync(new URL(path, import.meta.url), 'utf8');
    return Object.fromEntries([...ftl.matchAll(/^([a-z][a-z-]*) = (.+)$/gm)]
      .filter(([, key]) => select(key))
      .map(([, key, value]) => [key, value]));
  };
  const player = lang => messages(`../../crates/support/src/appeals/${lang}.ftl`);
  const staffAppeals = lang => messages(`../../crates/admin/src/support_${lang}.ftl`,
    key => key.startsWith('appeal-'));
  const navigation = lang => messages(`../../crates/ui/i18n/${lang}.ftl`,
    key => key === 'shell-sanctions');
  expect(Object.keys(player('pl')).sort()).toEqual(Object.keys(player('en')).sort());
  expect(Object.keys(staffAppeals('pl')).sort()).toEqual(Object.keys(staffAppeals('en')).sort());
  for (const lang of ['pl', 'en']) {
    expect(navigation(lang)['shell-sanctions']).toBe(player(lang).title);
    for (const status of ['requested', 'reviewing', 'accepted', 'rejected']) {
      expect(player(lang)[status]).toBeTruthy();
      expect(staffAppeals(lang)[`appeal-${status}`]).toBe(player(lang)[status]);
    }
  }
});
