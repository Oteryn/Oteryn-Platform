import {createHash, randomUUID} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {test, expect} from '../fixtures.mjs';
import {seededMfaAccount, seededLogin, query} from '../staff-rbac-helpers.mjs';
import {password, submit} from '../helpers.mjs';
import {assertShell} from '../shell.mjs';

const action = 'registration.policy.changed';
const route = '/admin/registration-policy';

test('staff confirm, close and reopen signup with bilingual reasons and change history', async ({page, browser}, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const staff = await seededMfaAccount(lang);
  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'", [staff.id]);
  await seededLogin(page, lang, staff);

  await page.goto(`/admin?lang=${lang}`);
  const link = page.locator(`main nav a[href^="${route}"]`);
  await expect(link).toBeVisible();
  await link.click();
  await assertShell(page);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.locator('[data-corpus-status]')).toBeVisible();

  const [original] = await query("SELECT enabled,reason_pl,reason_en,work_bits,work_ttl_seconds,challenge_limit,attempt_limit,rate_window_seconds FROM registration_abuse_policy WHERE singleton");
  const source = 20 + (info.project.use.viewport.width === 390 ? 1 : 0) + (lang === 'en' ? 10 : 0);
  const publicContext = await browser.newContext({
    baseURL: new URL(page.url()).origin, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
    extraHTTPHeaders: {'x-forwarded-for': `192.0.2.${source + 30}`},
  });
  try {
    await query('UPDATE registration_abuse_policy SET enabled=true WHERE singleton');
    const candidate = {email: `signup-${randomUUID()}@local.test`, password: password()};
    const prefix = createHash('sha1').update(candidate.password).digest('hex').toUpperCase().slice(0, 5);
    await query(`INSERT INTO registration_password_ranges(prefix,suffixes,expires_at,complete,complete_xact)
      VALUES($1,ARRAY[]::text[],now()+interval '1 hour',true,pg_current_xact_id())
      ON CONFLICT(prefix) DO UPDATE SET suffixes=EXCLUDED.suffixes,expires_at=EXCLUDED.expires_at,
      complete=true,complete_xact=pg_current_xact_id()`, [prefix]);
    await page.reload();
    const corpus = page.locator('[data-corpus-status] dd');
    expect(Number(await corpus.nth(0).textContent())).toBeGreaterThan(0);
    await expect(corpus.nth(2)).toHaveText(/\d{4}-\d{2}-\d{2} .* UTC/);
    const publicPage = await publicContext.newPage();
    await publicContext.addCookies([{name: 'portal_language', value: lang, url: new URL(page.url()).origin}]);
    expect((await publicPage.goto(`/${lang}/register`)).status()).toBe(200);
    const signup = publicPage.locator('main form[action$="/register"]');
    const presaveCsrf = await signup.locator('input[name=_csrf]').inputValue();
    const initialForm = page.locator(`form[action^="${route}/confirm"]`);
    const csrf = await initialForm.locator('input[name=_csrf]').inputValue();
    const reason = {en: 'Signup is paused for account maintenance. <Updates soon>', pl: 'Rejestracja wstrzymana na czas prac nad kontami. <Wkrótce więcej>'};
    const limits = {work_bits: '17', work_ttl_seconds: '420', challenge_limit: '1', attempt_limit: '3', rate_window_seconds: '120'};
    const requested = {enabled: 'false', reason_pl: reason.pl, reason_en: reason.en, ...limits};
    const post = (request, path, form) => request.post(path, {
      form, headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
    });
    const [revision] = await query('SELECT revision FROM registration_abuse_policy WHERE singleton');
    const early = await post(page.request, `${route}?lang=${lang}`, {...requested, _csrf: csrf, revision: String(revision.revision)});
    expect(early.status()).toBe(403);

    await initialForm.locator('[name=current_password]').fill(staff.password);
    await initialForm.locator('button[type=submit]').click();
    await expect(page).toHaveURL(new RegExp(`/admin/registration-policy\\?lang=${lang}$`));
    const edit = page.locator(`form[action="${route}?lang=${lang}"]`);
    await expect(edit.locator('select[name=enabled]')).toBeEnabled();
    expect(await edit.locator('input[name=_csrf]').inputValue()).not.toBe(csrf);
    for (const [name, value] of Object.entries(requested)) {
      const field = edit.locator(`[name=${name}]`);
      if (name === 'enabled') await field.selectOption(value);
      else await field.fill(value);
    }
    await edit.locator('button[type=submit]').click();
    await expect(page.locator('[data-policy-history]')).toContainText(reason[lang]);
    const [entry] = await query('SELECT before,after FROM audit_log WHERE actor=$1 AND action=$2 ORDER BY created_at DESC LIMIT 1', [staff.id, action]);
    expect(entry.before.enabled).toBe(true);
    expect(entry.after).toMatchObject({enabled: false, reason_pl: reason.pl, reason_en: reason.en, work_bits: 17, challenge_limit: 1, attempt_limit: 3});
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

    const deniedSignup = await post(publicPage.request, `/${lang}/register`, {_csrf: presaveCsrf});
    expect(deniedSignup.status()).toBe(503);
    const closed = await publicPage.goto(`/${lang}/register`);
    expect(closed.status()).toBe(503);
    expect(closed.headers()['cache-control']).toContain('no-store');
    await expect(publicPage.locator('main')).toContainText(reason[lang]);
    await expect(publicPage.locator('main form[action$="/register"]')).toHaveCount(0);
    await expect(publicPage.locator('main updates, main wkrótce')).toHaveCount(0);
    expect((await publicPage.goto('/register')).status()).toBe(503);
    await expect(publicPage.locator('main')).toContainText(reason[lang]);

    await edit.locator('[name=enabled]').selectOption('true');
    await edit.locator('button[type=submit]').click();
    expect((await publicPage.goto(`/${lang}/register`)).status()).toBe(200);
    await expect(signup).toBeVisible();
    const challengePath = `/${lang}/register/abuse/challenge`;
    const challenge = () => publicPage.request.post(challengePath, {headers: {
      origin: new URL(publicPage.url()).origin,
      'x-csrf-token': signupCsrf,
      'x-forwarded-for': `192.0.2.${source}`,
    }});
    const signupCsrf = await signup.locator('input[name=_csrf]').inputValue();
    const issued = await challenge();
    expect(issued.status()).toBe(200);
    expect((await issued.json()).bits).toBe(17);
    expect((await challenge()).status()).toBe(429);
    await submit(publicPage, 'main form[action$="/register"]', {
      email: candidate.email, password: candidate.password, password_confirmation: candidate.password,
      terms: true, privacy: true,
    });
    await expect(publicPage).toHaveURL(new RegExp(`/${lang}/register/check-inbox$`), {timeout: 90_000});
    const [newAccount] = await query('SELECT locale FROM accounts WHERE email=$1', [candidate.email]);
    expect(newAccount.locale).toBe(lang);

    const reader = await seededMfaAccount(lang);
    await query(`INSERT INTO account_roles(account_id, role_id)
      SELECT $1, id FROM roles WHERE key='support'`, [reader.id]);
    await seededLogin(publicPage, lang, reader);
    expect((await publicPage.goto(`${route}?lang=${lang}`)).status()).toBe(403);
  } finally {
    await publicContext.close();
    await query(`UPDATE registration_abuse_policy SET enabled=$1,reason_pl=$2,reason_en=$3,work_bits=$4,
      work_ttl_seconds=$5,challenge_limit=$6,attempt_limit=$7,rate_window_seconds=$8 WHERE singleton`, Object.values(original));
  }
});

test('i18n-parity for registration policy and public registration refusals', () => {
  const keys = (path, filter) => [...readFileSync(new URL(path, import.meta.url), 'utf8')
    .matchAll(/^([a-z][\w-]*)\s*=/gm)].map(([, key]) => key).filter(filter).sort();
  for (const [file, filter] of [
    [lang => `../../templates/admin/${lang}.ftl`, key => key.startsWith('registration-')],
    [lang => `../../crates/identity/src/abuse/${lang}.ftl`, () => true],
  ]) {
    const en = keys(file('en'), filter);
    expect(en.length).toBeGreaterThan(0);
    expect(new Set(en).size).toBe(en.length);
    expect(keys(file('pl'), filter)).toEqual(en);
  }
});
