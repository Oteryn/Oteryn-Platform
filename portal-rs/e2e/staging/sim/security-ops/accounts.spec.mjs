import {
  expect, freshPlayer, get, localized, login, navigate, register, routeStaging, status, submit, test,
} from './helpers.mjs';

test('a player signs out their other browser and a stale tab cannot repeat it', async ({ page, browser }, info) => {
  const lang = info.project.metadata.language;
  const sessions = `/${lang}/account/sessions`;
  const account = freshPlayer();
  await register(page, lang, account);
  expect(await login(page, lang, account)).toBe(`/${lang}/`);

  const secondContext = await browser.newContext();
  try {
    await routeStaging(secondContext);
    const second = await secondContext.newPage();
    expect(await login(second, lang, account)).toBe(`/${lang}/`);
    await status(page, await navigate(page, sessions), 200, 'first browser sessions');
    await status(second, await navigate(second, sessions), 200, 'second browser sessions');
    const heading = localized(lang, 'Sesje web', 'Web sessions');
    const entries = 'main > ul:first-of-type form[method="post"]';
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(heading);
    await expect(second.getByRole('heading', { level: 1 })).toHaveText(heading);
    await expect(page.locator(entries)).toHaveCount(2);
    await expect(second.locator(entries)).toHaveCount(2);

    const current = localized(lang, 'Ta sesja', 'This session');
    const staleTab = await page.context().newPage();
    await status(staleTab, await navigate(staleTab, sessions), 200, 'second tab sessions');
    const staleForm = 'main > ul:first-of-type li'
      + `:not(:has-text("${current}")) form[method="post"]`;
    await expect(staleTab.locator(staleForm)).toHaveCount(1);

    const allOthers = 'main form[action$="/sessions/others"]';
    await status(page, (await submit(page, allOthers, {})).status(), 303, 'sign out other sessions');
    await expect(page.locator('main [role="status"]')).toHaveText(
      localized(lang, 'Sesje zostały wylogowane.', 'Sessions signed out.'));
    const denied = await get(second.request, sessions);
    expect(denied.status(), 'revoked browser is denied on its next request').toBe(303);
    expect(denied.headers().location).toBe(`/${lang}/login`);
    await navigate(second, sessions);
    await expect(second).toHaveURL(`/${lang}/login`);

    await status(staleTab, (await submit(staleTab, staleForm, {})).status(), 303, 'stale session form');
    await expect(staleTab.locator('main [role="status"]')).toHaveText(localized(lang,
      'Wybrana sesja jest nieaktywna lub niedostępna.', 'The selected session is inactive or unavailable.'));
    await status(page, (await page.reload()).status(), 200, 'remaining session after reload');
    await expect(page.locator(entries)).toHaveCount(1);
    await expect(page.getByRole('heading', { name: localized(lang,
      'Wylogowano pozostałe sesje', 'Other sessions signed out'), exact: true })).toBeVisible();

    const staff = await get(page.request, '/admin/roles');
    if (staff.status() === 404) {
      test.info().annotations.push({ type: 'staging limitation', description: 'Staff panel unavailable' });
    } else {
      expect(staff.status(), 'authenticated player cannot open role administration').toBe(403);
    }
  } finally {
    await secondContext.close();
  }
});
