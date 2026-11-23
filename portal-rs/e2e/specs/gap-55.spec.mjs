import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { pool, submit } from '../helpers.mjs';
import { action, seededAccount, seededLogin, seededStaff } from '../staff-rbac-helpers.mjs';

test('i18n-parity for recruitment labels', () => {
  const keys = (lang) => {
    const locale = readFileSync(new URL(`../../templates/community/party_${lang}.ftl`, import.meta.url), 'utf8');
    return [...locale.matchAll(/^([a-z][a-z-]*) = \S/gm)].map((match) => match[1]).sort();
  };
  const english = keys('en');
  expect(english).toContain('next');
  expect(english).toContain('previous');
  expect(new Set(english).size).toBe(english.length);
  expect(keys('pl')).toEqual(english);
});

test('players manage recruitment listings and staff resolve recruitment reports', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const board = `/${lang}/party`;
  const queue = `/admin/${lang}/party`;
  const owner = await seededAccount(lang);
  const reporter = await seededAccount(lang);
  const db = pool();
  const staffContext = await browser.newContext({
    baseURL: info.project.use.baseURL,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  });
  const ids = [];
  const reportReason = `Recruitment review fixture ${'x'.repeat(360)}`;

  try {
    // Seed the Platform post history only. The browser stack has no qualified
    // character-profile producer. The Rust authority flow covers live publication.
    for (const [note, starts, expires] of [
      ['Owner withdrawal fixture', '-2 minutes', '1 hour'],
      ['Expired listing fixture', '-2 hours', '-1 hour'],
      ['Moderation fixture', '-2 minutes', '1 hour'],
    ]) {
      const result = await db.query(
        `INSERT INTO pd_party_posts
           (account_id, character_id, character_name, world_id, purpose, min_level,
            max_level, profession, note, starts_at, expires_at)
         VALUES ($1, uuidv7(), 'Recruitment fixture', uuidv7(), 'hunt', 1, 100, 'any',
                 $2, now()+$3::interval, now()+$4::interval)
         RETURNING id`,
        [owner.id, note, starts, expires],
      );
      ids.push(result.rows[0].id);
    }
    const [active, expired, reported] = ids;
    await db.query(
      'INSERT INTO pd_party_reports (post_id, account_id, reason) VALUES ($1, $2, $3)',
      [reported, reporter.id, reportReason],
    );

    await seededLogin(page, lang, owner);
    await page.goto(`/${lang}/news`);
    if (info.project.use.viewport.width <= 960) {
      await page.locator('#portal-header .om-fallback-menu > summary').click();
    }
    const link = page.locator(`#portal-community-panel a[href="${board}"]`);
    await link.locator('xpath=ancestor::details[contains(@class, "om-section")]')
      .locator('summary').click();
    await link.click();
    await expect(page).toHaveURL(new RegExp(`${board}$`));

    const filter = page.locator('main form[method=get]');
    await filter.locator('[name=purpose]').selectOption('hunt');
    await filter.locator('[name=level]').fill('50');
    await filter.getByRole('button').click();
    await expect(filter.locator('[name=purpose]')).toHaveValue('hunt');
    await expect(filter.locator('[name=level]')).toHaveValue('50');
    expect((await page.request.get(`${board}?level=-1`)).status()).toBe(422);
    expect((await page.request.get(`${board}?page=101`)).status()).toBe(422);

    const unavailable = page.locator('section[aria-labelledby=party-create] [role=status]');
    await expect(unavailable).toHaveText(lang === 'pl'
      ? 'Publikacja jest niedostępna do czasu potwierdzenia konta i własności postaci.'
      : 'Posting is unavailable until your account and character ownership can be verified.');
    await expect(page.locator(`main form[action="${board}"][method=post]`)).toHaveCount(0);

    const withdrawal = page.locator(`[data-listing="${active}"]`);
    const expiredListing = page.locator(`[data-listing="${expired}"]`);
    await expect(expiredListing.locator('strong')).toHaveText(lang === 'pl' ? 'Wygasło' : 'Expired');
    await expect(expiredListing.locator('form')).toHaveCount(0);
    const remove = `${board}/${active}/remove`;
    await submit(page, `form[action="${remove}"]`);
    await expect(withdrawal.locator('strong')).toHaveText(lang === 'pl' ? 'Usunięte' : 'Removed');

    const decision = `${queue}/${reported}/moderate`;
    const options = {
      headers: { origin: new URL(page.url()).origin },
      maxRedirects: 0,
    };
    const csrf = await page.locator(`form[action="${board}/${reported}/remove"] [name=_csrf]`).inputValue();
    expect((await page.request.post(decision, {
      ...options, form: { _csrf: csrf, action: 'remove', reason: 'Review' },
    })).status()).toBe(403);

    const staff = await staffContext.newPage();
    await seededStaff(staff, lang, 'platform_admin');
    await staff.goto(board);
    await staff.locator(`main a[href$="${queue}"]`).click();
    await expect(staff).toHaveURL(new RegExp(`${queue}$`));
    await expect(staff.locator('#party-feed-status')).toHaveText(lang === 'pl'
      ? 'Stan danych z gry' : 'Game feed status');
    const moderation = `form[action="${decision}"]`;
    await expect(staff.locator(moderation).locator('..')).toContainText(reportReason);
    for (const screen of [page, staff]) {
      expect(await screen.evaluate(() => document.documentElement.scrollWidth))
        .toBeLessThanOrEqual(info.project.use.viewport.width);
    }

    const decisionFields = await staff.locator(moderation)
      .evaluate((form) => Object.fromEntries(new FormData(form)));
    const submitDecision = (reason) => staff.request.post(decision, {
      ...options,
      headers: { ...options.headers, accept: 'text/html' },
      form: { ...decisionFields, action: 'remove', reason },
    });
    const invalid = await submitDecision(' ');
    expect(invalid.status()).toBe(422);
    expect(await invalid.text()).toContain(lang === 'pl'
      ? 'Sprawdź wprowadzone dane'
      : 'Please check your input');

    await action(staff, moderation, { reason: 'Review complete' }, 'remove');
    await expect(staff.locator(moderation)).toHaveCount(0);
    expect((await submitDecision('Review complete')).status()).toBe(404);
    await page.goto(board);
    await expect(page.locator(`[data-listing="${reported}"] strong`)).toHaveText(lang === 'pl'
      ? 'Usunięte' : 'Removed');
    const decisions = await db.query(
      "SELECT count(*)::int AS count FROM audit_log WHERE action='party.moderate' AND target=$1",
      [reported],
    );
    expect(decisions.rows[0].count).toBe(1);
  } finally {
    await db.query('DELETE FROM pd_party_reports WHERE post_id=ANY($1::uuid[])', [ids]);
    await db.query('DELETE FROM pd_party_posts WHERE id=ANY($1::uuid[])', [ids]);
    await db.end();
    await staffContext.close();
  }
});
