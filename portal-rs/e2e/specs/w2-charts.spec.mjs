import crypto from 'node:crypto';
import {test, expect} from '../fixtures.mjs';
import {csrfRejected, submit, waitForLoginBudget} from '../helpers.mjs';
import {action, query, seededAccount, seededLogin, seededStaff} from '../staff-rbac-helpers.mjs';
import {assertShell} from '../shell.mjs';

test('staff dashboard charts reflect a player ticket and its support resolution', async ({page, browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const context = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
    viewport: info.project.use.viewport,
    locale: lang,
  });
  try {
    const staff = await context.newPage();
    await seededStaff(staff, lang, 'platform_admin');
    expect((await staff.goto('/admin')).status()).toBe(200);
    await staff.locator('nav a[href^="/admin/charts"]').click({timeout: 5000});
    await expect(staff.locator('html')).toHaveAttribute('lang', lang);
    await assertShell(staff);
    const day = new Date().toISOString().slice(0, 10);
    const filter = staff.locator('#dashboard-filters');
    await filter.locator('[name=range]').selectOption('custom');
    await filter.locator('[name=from]').fill(day);
    await filter.locator('[name=to]').fill(day);
    await filter.locator('button').click();
    const dashboard = staff.url();
    const read = async chart => {
      const response = await staff.request.get(
        await staff.locator(`[data-chart="${chart}"] a[download]`).getAttribute('href'),
      );
      expect(response.status()).toBe(200);
      expect(response.headers()['content-type']).toContain('text/csv');
      return (await response.text()).trim().split('\n').find(row => row.startsWith(day)).split(',').at(-1);
    };
    const registrations = Number(await read('registrations'));
    const verified = Number(await read('verified'));
    const opened = Number(await read('tickets-opened'));
    const resolved = Number(await read('tickets-resolved'));
    const sanctions = Number(await read('sanctions'));
    const failures = Number(await read('logins-failed'));
    const successful = Number(await read('logins-succeeded'));
    const account = await seededAccount(lang);
    await waitForLoginBudget();
    await page.goto(`/${lang}/login`);
    const login = page.locator('main form[method=post]').first();
    await csrfRejected(page, `/${lang}/login`, {email: account.email, password: 'wrong-password'});
    await login.locator('[name=email]').fill(account.email);
    await login.locator('[name=password]').fill('wrong-password');
    await login.locator('button').click();
    await expect(page).toHaveURL(new RegExp(`/${lang}/login$`));
    await seededLogin(page, lang, account);
    expect((await page.request.get('/admin/charts')).status()).toBe(403);
    expect((await page.request.get('/admin/charts/tickets-opened.csv')).status()).toBe(403);
    await page.goto(`/${lang}/support/tickets/create`);
    await submit(page, 'main form', {subject: 'Dashboard qualification', body: 'Please review this ticket.'});
    await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
    const ticket = new URL(page.url()).pathname.split('/').at(-1);
    expect((await staff.goto(`/admin/support/tickets/${ticket}`)).status()).toBe(200);
    await action(staff, 'form:has(input[value=reply])', {body: 'First public response.'});
    await action(staff, 'form:has(input[value=ticket_status])', {status: 'resolved'});
    expect((await query('SELECT status FROM tickets WHERE id=$1', [ticket]))[0].status).toBe('resolved');
    await staff.goto('/admin/moderation/enforcement/create');
    const now = Math.floor(Date.now() / 1000);
    await action(staff, 'form:has(input[value=sanction])', {
      target: account.id, category: 'warning', status: 'active', public_reason: 'Dashboard qualification',
      moderator_notes: '', effective_at: String(now - 1), expires_at: String(now + 3600),
    });
    await expect(staff).toHaveURL(/enforcement\/[a-f0-9-]+$/);

    await staff.goto(dashboard);
    expect(Number(await read('registrations'))).toBeGreaterThan(registrations);
    expect(Number(await read('verified'))).toBeGreaterThan(verified);
    expect(Number(await read('tickets-opened'))).toBeGreaterThan(opened);
    expect(Number(await read('tickets-resolved'))).toBeGreaterThan(resolved);
    expect(Number(await read('sanctions'))).toBeGreaterThan(sanctions);
    expect(Number(await read('logins-failed'))).toBeGreaterThan(failures);
    expect(Number(await read('logins-succeeded'))).toBeGreaterThan(successful);
    expect(Number(await read('web-sessions'))).toBeGreaterThan(0);
    const chart = staff.locator('[data-chart="first-response"]');
    expect(await read('first-response')).toMatch(/^\d+$/);
    await expect(chart.locator('svg title')).toHaveCount(1);
    await expect(chart.locator('svg desc')).toHaveCount(1);
    await chart.locator('summary').click();
    await expect(chart.getByRole('row', {name: new RegExp(day)})).toBeVisible();
    expect(await staff.locator('body').evaluate(el => el.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    expect((await staff.request.get('/admin/charts?range=custom&from=2020-01-01&to=2020-05-01')).status()).toBe(400);
    expect((await staff.request.get('/admin/charts?world=invalid')).status()).toBe(400);
    expect((await staff.request.get('/admin/charts/missing.csv')).status()).toBe(404);

    await test.step('the wallet chart reflects a completed sandbox purchase', async () => {
      const walletChart = staff.locator('[data-chart="wallet"]');
      const series = async () => {
        const csv = await staff.request.get(await walletChart.locator('a[download]').getAttribute('href'));
        expect(csv.status()).toBe(200);
        const row = (await csv.text()).split('\r\n')
          .find(line => line.startsWith(`${day},top_up_sandbox,`));
        return Number(row?.split(',').at(-1) ?? 0);
      };
      const before = await series();
      await page.goto(`/${lang}/shop/coins`);
      const selector = 'main form[action$="/shop/coins"]';
      const operation = await page.locator(`${selector} [name=operation]`).first().inputValue();
      await submit(page, selector);
      const [intent] = await query(`SELECT p.id,p.provider_reference FROM store_web_topups t
        JOIN payment_intents p ON p.id=t.payment_intent_id WHERE t.operation_id=$1`, [operation]);
      const body = JSON.stringify({
        event_id: crypto.randomUUID(), intent_id: intent.id, provider_reference: intent.provider_reference,
        amount: {minor_units: 250, currency: 'PLN'}, state: 'succeeded',
      });
      const signature = crypto.createHmac('sha256', process.env.OTERYN_SHOP_TEST_PAYMENT_KEY).update(body).digest('base64');
      const paid = await page.request.post('/__test/payments/webhook', {
        data: body, headers: {'content-type': 'application/json', 'x-test-payment-signature': signature},
      });
      expect(paid.status()).toBe(204);
      await expect.poll(series, {timeout: 30_000}).toBeGreaterThanOrEqual(before + 1234);
    });

    await test.step('a world with no sample stays unavailable and a recorded zero is exported', async () => {
      const [world] = await query("INSERT INTO world_status (id,name) VALUES (uuidv7(),$1) RETURNING id", ['Charts fixture world']);
      try {
        await staff.goto(dashboard);
        await staff.locator('[name=world]').selectOption(world.id);
        await staff.locator('#dashboard-filters button').click();
        const online = staff.locator('[data-chart="game-online"]');
        await expect(online.locator('[data-state=unavailable]')).toBeVisible();
        await expect(online).toContainText(lang === 'en' ? 'No samples recorded' : 'Brak zapisanych pomiarów');
        await query('INSERT INTO stats_world_samples (world,sampled_at,players_online) VALUES ($1,clock_timestamp(),0)', [world.id]);
        await staff.reload();
        await expect(online.locator('[data-state=unavailable]')).toHaveCount(0);
        expect(await read('game-online')).toBe('0');
        await expect(online.locator('svg title')).toHaveCount(1);
      } finally {
        await query('DELETE FROM stats_world_samples WHERE world=$1', [world.id]);
        await query('DELETE FROM world_status WHERE id=$1', [world.id]);
      }
    });
  } finally {
    await context.close();
  }
});


test('i18n-parity renders the Statistics filters and every metric in the selected language', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  await seededStaff(page, lang, 'platform_admin');
  expect((await page.goto(`/admin/charts?lang=${lang}&range=30`)).status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', {level: 1})).toHaveText(lang === 'en' ? 'Statistics' : 'Statystyki');
  await expect(page.locator('select[name=range] option:checked')).toHaveText(lang === 'en' ? 'Last 30 days' : 'Ostatnie 30 dni');
  const headings = lang === 'en'
    ? ['Registrations', 'Verified accounts', 'Successful web sign-ins', 'Failed web sign-ins',
       'Active web sessions', 'Support tickets opened', 'Support resolutions', 'Median first response (seconds)',
       'Sanctions issued', 'Wallet and Store ledger', 'Game players online']
    : ['Rejestracje', 'Zweryfikowane konta', 'Udane logowania do portalu', 'Nieudane logowania do portalu',
       'Aktywne sesje portalu', 'Otwarte zgłoszenia pomocy', 'Rozwiązania zgłoszeń', 'Mediana czasu pierwszej odpowiedzi (sekundy)',
       'Nałożone sankcje', 'Księga portfela i sklepu', 'Gracze online w grze'];
  await expect(page.locator('[data-chart] h2')).toHaveText(headings);
  await expect(page.locator('[data-chart] a[download]')).toHaveText(
    headings.map(() => lang === 'en' ? 'Download CSV' : 'Pobierz CSV'),
  );
});
