import {readFileSync} from 'node:fs';
import {test, expect} from '../fixtures.mjs';
import {submit} from '../helpers.mjs';
import {action, seededAccount, seededLogin, seededStaff, unique} from '../staff-rbac-helpers.mjs';

async function postForm(page, form, overrides) {
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  return page.request.post(await form.getAttribute('action'), {
    form: {...fields, ...overrides},
    headers: {origin: new URL(page.url()).origin},
    maxRedirects: 0,
  });
}

async function openQueueItem(page, queue, id) {
  const item = page.locator(`a[href="${queue}/${id}"]`);
  for (let number = 1; number <= 10000; number++) {
    await expect(page).toHaveURL(url => url.pathname === queue && (url.searchParams.get('page') ?? '1') === String(number));
    if (await item.count()) {
      await item.click();
      await expect(page).toHaveURL(url => url.pathname === `${queue}/${id}`);
      return;
    }
    await page.locator(`a[href="${queue}?page=${number + 1}"]`).click();
  }
  throw new Error('Staff queue pagination exceeded its bound');
}

test('report triage, permanent sanction review and player appeal', async ({page, browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  async function staffPage(role) {
    const context = await browser.newContext({
      baseURL: new URL(page.url()).origin,
      ignoreHTTPSErrors: true,
      locale: lang,
      viewport: info.project.use.viewport,
    });
    contexts.push(context);
    const actor = await context.newPage();
    await seededStaff(actor, lang, role);
    return actor;
  }

  const reports = '/admin/moderation/reports';
  const enforcement = '/admin/moderation/enforcement';
  const history = `/${lang}/account/sanctions`;
  const reportOutcome = unique();
  const reason = unique();
  const appeal = `<appeal> ${unique()}`;
  const decision = unique();
  try {
    const player = await seededAccount(lang);
    await seededLogin(page, lang, player);
    await page.goto(`/${lang}/support/reports/create`);
    await submit(page, 'main form', {target_reference: player.id, evidence_summary: unique()});
    await expect(page).toHaveURL(/support\/reports\/[a-f0-9-]+$/);
    const reportPath = new URL(page.url()).pathname;
    const report = reportPath.split('/').at(-1);

    expect((await page.request.get(reports, {maxRedirects: 0})).status()).toBe(403);
    const support = await staffPage('support');
    const issuer = await staffPage('moderator');
    const reviewer = await staffPage('moderator');

    await support.goto('/admin');
    await expect(support.locator(`a[href="${enforcement}"]`)).toHaveCount(0);
    await support.locator(`a[href="${reports}"]`).click();
    await openQueueItem(support, reports, report);
    const moderate = support.locator('form:has(input[value=moderate])');
    expect((await postForm(support, moderate, {status: 'closed', public_outcome: reportOutcome})).status()).toBe(409);
    expect((await postForm(support, moderate, {status: 'actioned', public_outcome: ''})).status()).toBe(422);
    await action(support, 'form:has(input[value=moderate])', {status: 'reviewing'});
    await action(support, 'form:has(input[value=moderate])', {status: 'actioned', public_outcome: reportOutcome});

    await issuer.goto('/admin');
    await expect(issuer.locator(`a[href="${reports}"]`)).toHaveCount(0);
    await issuer.locator(`a[href="${enforcement}"]`).click();
    await issuer.locator(`a[href="${enforcement}/create"]`).click();
    await action(issuer, 'form:has(input[value=sanction])', {
      target: player.id, category: 'restriction', status: 'active',
      public_reason: reason, moderator_notes: 'Staff-only review context',
      effective_at: String(Math.floor(Date.now() / 1000) - 1), expires_at: '',
    });
    await expect(issuer).toHaveURL(/enforcement\/[a-f0-9-]+$/);
    const sanction = new URL(issuer.url()).pathname.split('/').at(-1);
    const appealPath = `${history}/${sanction}`;
    const approve = 'form:has(input[value=approve_sanction])';
    expect((await postForm(issuer, issuer.locator(approve), {})).status()).toBe(403);
    expect((await page.request.get(appealPath)).status()).toBe(404);
    expect((await support.request.get(`${enforcement}/${sanction}`)).status()).toBe(403);

    await reviewer.goto('/admin');
    await reviewer.locator(`a[href="${enforcement}"]`).click();
    await openQueueItem(reviewer, enforcement, sanction);
    await action(reviewer, approve);
    await expect(reviewer.locator(approve)).toHaveCount(0);
    expect((await support.request.get(appealPath)).status()).toBe(404);

    await page.goto(reportPath);
    await expect(page.getByText(reportOutcome)).toBeVisible();
    await page.locator(`a[href="${history}"]`).click();
    await expect(page.locator('h1')).toHaveText(lang === 'pl' ? 'Historia kar' : 'Sanction history');
    await page.locator(`a[href="${appealPath}"]`).click();
    await expect(page.getByText(reason)).toBeVisible();
    await expect(page.getByText('Staff-only review context')).toHaveCount(0);
    expect(await page.locator('body').evaluate(el => el.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    await action(page, 'form:has(input[value=acknowledge])');
    await expect(page.locator('form:has(input[value=acknowledge])')).toHaveCount(0);
    await action(page, 'form:has(input[value=submit])', {message: appeal});
    await expect(page.getByText(appeal)).toBeVisible();

    await reviewer.reload();
    await expect(reviewer.getByText(appeal)).toBeVisible();
    await action(reviewer, 'form:has(input[value=appeal])', {appeal_status: 'accepted', appeal_outcome: decision});
    await page.reload();
    await expect(page.getByText(decision)).toBeVisible();
    await expect(page.getByText(lang === 'pl' ? 'Cofnięta' : 'Revoked', {exact: true})).toBeVisible();

    await page.goto(`/${lang}/`);
    if (info.project.use.viewport.width <= 960) await page.locator('#portal-menu-toggle').click();
    await page.locator('[data-account-link]').click();
    await expect(page.locator(`#podglad-konta a[href="${history}"]`)).toBeVisible();
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('i18n-parity for Platform sanctions and appeals', () => {
  for (const [english, polish, required] of [
    ['crates/support/src/reports/en.ftl', 'crates/support/src/reports/pl.ftl',
      ['sanctions-title', 'sanctions-handoff', 'sanctions-submit', 'sanctions-conflict', 'revoked']],
    ['crates/admin/src/support_en.ftl', 'crates/admin/src/support_pl.ftl',
      ['enforcement', 'pending', 'approve_sanction', 'expiry_hint']],
    ['crates/ui/i18n/en.ftl', 'crates/ui/i18n/pl.ftl', ['shell-sanctions']],
  ]) {
    const [en, pl] = [english, polish].map(path => {
      const source = readFileSync(new URL(`../../${path}`, import.meta.url), 'utf8');
      return [...source.matchAll(/^([\w-]+) = \S/gm)].map(match => match[1]).sort();
    });
    expect(pl).toEqual(en);
    for (const key of required) expect(en).toContain(key);
  }
});
