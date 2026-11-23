import {confirmStaffPassword} from '../staff-rbac-helpers.mjs';
import { createHmac, randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { pool, register, submit, waitForLoginBudget } from '../helpers.mjs';
import { action, audit, expectLoginRedirect, loginWithRecoveryCode, membership, query, role } from '../staff-rbac-helpers.mjs';
import { actorContext, denied, open, portalCommand, publishEditorial, responseForForm, signInWithMfa, staffActor } from '../journey-staff-helpers.mjs';
import { post as gamePost } from '../game-sim-helpers.mjs';
import { paidTopup, uuid7 } from '../staff-commerce-helpers.mjs';
import { fixtureDatabase, withSoleRoleHolder } from '../staff-fixture-database.mjs';

async function openInteriorNavigation(page) {
  const toggle = page.locator('.om-fallback-menu:not([open]) > summary');
  if (await toggle.isVisible()) await toggle.click();
}

async function openAccountMenu(page) {
  if (await page.locator('#portal-header.om-interior').count()) {
    await openInteriorNavigation(page);
    await page.locator('.om-fallback-account > summary').click();
    return page.locator('.om-fallback-account[open]');
  }
  await expect(page.locator('#portal-header')).toHaveClass(/\bis-enhanced\b/);
  const mobile = await page.evaluate(() => matchMedia('(max-width: 960px)').matches);
  if (mobile) await page.locator('#portal-menu-toggle').click();
  await page.locator('[data-account-link]').click();
  const menu = page.locator('#podglad-konta');
  await expect(menu).toBeVisible();
  return menu;
}

async function followAccountLink(page, path) {
  const menu = await openAccountMenu(page);
  await menu.locator(`a[href="${path}"]`).click();
  await expect(page).toHaveURL(new RegExp(`${path}$`));
}

async function followPartyLink(page, path) {
  await openInteriorNavigation(page);
  const community = page.locator('#portal-navigation details.om-section:not([open])')
    .filter({ has: page.locator('#portal-community-panel') });
  if (await community.count()) await community.locator(':scope > summary').click();
  await page.locator(`#portal-community-panel a[href="${path}"]`).click();
  await expect(page).toHaveURL(new RegExp(`${path}$`));
}

test('staff publish content and handle a player’s support, safety and account requests', async ({ page, browser }, info) => {
  test.setTimeout(1_800_000);
  fixtureDatabase();
  const contexts = [];
  let adminId;
  try {
    const lang = info.project.metadata.locale;
    const pl = lang === 'pl';
    const base = `/${lang}/support`;
    const ticketTitle = pl ? 'Zgłoszenia pomocy' : 'Support tickets';
    const subject = `Client download ${randomUUID()}`;
    const question = 'Where can I find the Linux client?';
    let reportTarget;
    const evidence = 'The player repeatedly sent abusive messages in public chat.';
    const main = page.getByRole('main');
    let account;
    let playerId;
    let ticketPath;
    let reportPath;

    await test.step('The player verifies their address and opens a signed-in session', async () => {
      account = await register(page, lang);
      [{ id: playerId }] = await query('SELECT id FROM accounts WHERE email=$1', [account.email]);
      await expect(main.getByRole('status')).toHaveText(pl
        ? 'Twój adres e-mail został zweryfikowany.'
        : 'Your e-mail has been verified.');
      await openInteriorNavigation(page);
      await page.locator(`#portal-navigation a.om-login[href="/${lang}/login"]`).click();
      await waitForLoginBudget();
      await submit(page, 'main form', { email: account.email, password: account.password });
      await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
      await expect(page.locator('[data-account-link]')).toHaveCount(1);
      await expect(page.locator('html')).toHaveAttribute('lang', lang);
    });

    await test.step('The player asks support about a download and finds the open ticket in their list', async () => {
      const menu = await openAccountMenu(page);
      await menu.locator(`a[href="${base}/tickets"]`).click();
      await expect(main.getByRole('heading', { level: 1 })).toHaveText(ticketTitle);
      await main.getByRole('link', { name: pl ? 'Utwórz zgłoszenie' : 'Create ticket' }).click();
      const form = `main form[action="${base}/tickets"]`;
      await expect(main.getByText(pl ? 'Wiadomość' : 'Message', { exact: true })).toBeVisible();
      await action(page, form, { category: 'technical', subject, body: question });
      await expect(page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
      ticketPath = new URL(page.url()).pathname;
      await expect(main).toContainText(question);
      await expect(main).toContainText(pl ? 'Status: Otwarte' : 'Status: Open');
      await main.getByRole('link', { name: ticketTitle, exact: true }).click();
      await main.getByRole('link', { name: subject, exact: true }).click();
      await expect(page).toHaveURL(new RegExp(`${ticketPath}$`));
      await expect(main).toContainText(question);
    });

    await test.step('The player corrects a mismatched category and sees the submitted report in history', async () => {
      const menu = await openAccountMenu(page);
      await menu.locator(`a[href="${base}/reports"]`).click();
      await expect(main.getByRole('heading', { level: 1 })).toHaveText(pl ? 'Zgłoszenia' : 'Reports');
      await main.getByRole('link', { name: pl ? 'Nowe zgłoszenie' : 'New report' }).click();
      const form = `main form[action="${base}/reports"]`;
      reportTarget = `Player ${playerId}`;
      const fields = { report_type: 'player', target_reference: reportTarget, evidence_summary: evidence };
      await action(page, form, { ...fields, category: 'misinformation' });
      await expect(main.getByRole('alert')).toContainText(pl ? 'Sprawdź rodzaj zgłoszenia' : 'Check the report type');
      await expect(main.locator('[name=target_reference]')).toHaveValue(reportTarget);
      await action(page, form, { ...fields, category: 'harassment' });
      await expect(page).toHaveURL(/\/support\/reports\/[a-f0-9-]+$/);
      reportPath = new URL(page.url()).pathname;
      await expect(main).toContainText(evidence);
      await expect(main).toContainText(pl ? 'Status: Wysłane' : 'Status: Submitted');
      await main.getByRole('link', { name: pl ? 'Historia (ostatnie 30 dni)' : 'History (last 30 days)' }).click();
      await main.getByRole('link', { name: reportTarget, exact: true }).click();
      await expect(page).toHaveURL(new RegExp(`${reportPath}$`));
      await expect(main).toContainText(evidence);
    });

    const anonymous = await actorContext(browser, info);
    contexts.push(anonymous);
    const visitor = await anonymous.newPage();
    await test.step('A signed-out visitor cannot open either private conversation', async () => {
      for (const path of [ticketPath, reportPath]) {
        const response = await visitor.request.get(path, { maxRedirects: 0 });
        expect(response.status()).toBe(303);
        expect(response.headers().location).toBe(`/${lang}/login`);
      }
    });

    const evidenceLog = [];
    const actors = {};
    let admin;
    let roles;

    await test.step('The administrator signs in with MFA and assigns the staff roles', async () => {
      const root = await staffActor(browser, info, null, [], contexts);
      admin = root.page;
      adminId = root.id;
      await signInWithMfa(admin, lang, root.credentials, true);
      await confirmStaffPassword(admin, lang, root.credentials.password);
      await open(admin, '/admin');
      await expect(admin.locator('main h1')).toHaveText(pl ? 'Panel zespołu' : 'Staff panel');
      await admin.locator('main a[href="/admin/roles"]').click();
      roles = Object.fromEntries((await query('SELECT id,key FROM roles WHERE built_in')).map(row => [row.key, row.id]));
      const assignments = [
        ['content_editor', ''],
        ['support', ''],
        ['moderator', 'support.reports.manage'],
        ['game_admin', 'account.security.kick'],
        ['game_support', 'products.premium.manage payments.reconcile'],
      ];
      for (const [key, permissions] of assignments) {
        const assigned = [roles[key]];
        if (permissions) {
          await open(admin, '/admin/roles');
          const form = 'main form:has(button[value=create])';
          expect((await responseForForm(admin, form, {
            key: `invalid_${randomUUID()}`, permissions: 'invalid.permission',
          }, admin, 'create')).status()).toBe(400);
          const id = await role(admin, permissions);
          assigned.push(id);
          evidenceLog.push(['rbac.role.created', id]);
        }
        actors[key] = await staffActor(browser, info, admin, assigned, contexts);
        evidenceLog.push(['rbac.role.granted', actors[key].id]);
        actors[key].recovery = await signInWithMfa(actors[key].page, lang, actors[key].credentials);
        await open(actors[key].page, '/admin');
        await expect(actors[key].page.locator('main a[href="/admin/roles"]')).toHaveCount(0);
        expect((await actors[key].page.request.get('/admin/roles')).status()).toBe(403);
      }
      const membershipForm = 'main form:has(input[name=account])';
      await open(admin, '/admin/roles');
      const self = { account: adminId, role: roles.content_editor, reason: "role_assignment" };
      await denied(admin, membershipForm, self, page, visitor, 'grant');
      expect((await responseForForm(admin, membershipForm, self, admin, 'grant')).status()).toBe(403);
      expect((await responseForForm(admin, membershipForm, { ...self, account: 'bad-id' }, admin, 'grant')).status()).toBe(422);
      await withSoleRoleHolder(roles.platform_admin, adminId, async () => {
        expect((await responseForForm(admin, membershipForm, {
          account: adminId, role: roles.platform_admin,
        }, admin, 'revoke')).status()).toBe(403);
      });
    });

    await test.step('The editor publishes news, an event, legal policies and a wiki guide for both languages', async () => {
      await publishEditorial(actors.content_editor.page, visitor, page, lang, evidenceLog);
    });

    await test.step('Support answers the ticket and the player sees the resolution', async () => {
      const support = actors.support.page;
      const ticket = ticketPath.split('/').at(-1);
      await open(support, '/admin/support/tickets');
      await support.locator(`main a[href="/admin/support/tickets/${ticket}"]`).click();
      await expect(support.locator('main')).toContainText(subject);
      const form = 'main form:has(input[value=reply])';
      const reply = pl ? 'Klient jest dostępny na stronie pobierania.' : 'The client is available on the download page.';
      await denied(support, form, { body: reply }, page, visitor);
      expect((await responseForForm(support, form, { body: ' ' })).status()).toBe(422);
      await action(support, form, { body: reply });
      await expect(support.locator('main')).toContainText(reply);
      await action(support, 'main form:has(input[value=ticket_status])', { status: 'resolved' });
      await followAccountLink(page, `${base}/tickets`);
      await main.getByRole('link', { name: subject, exact: true }).click();
      await expect(main).toContainText(reply);
      await expect(main).toContainText(pl ? 'Rozwiązane' : 'Resolved');
      evidenceLog.push(['support.ticket_replied', ticket], ['support.ticket_status_changed', ticket]);
    });

    await test.step('The moderator reviews the report, issues a sanction and publishes the decision', async () => {
      const moderator = actors.moderator.page;
      const report = reportPath.split('/').at(-1);
      await open(moderator, '/admin/moderation/reports');
      const reportLink = moderator.locator(`main a[href="/admin/moderation/reports/${report}"]`);
      const next = moderator.locator('main a[href^="/admin/moderation/reports?page="]');
      while (!(await reportLink.count())) {
        await expect(next).toBeVisible();
        const destination = new URL(await next.getAttribute('href'), moderator.url()).href;
        await Promise.all([
          moderator.waitForURL(destination, { waitUntil: 'domcontentloaded' }),
          next.click(),
        ]);
      }
      await reportLink.click();
      const reportForm = 'main form:has(input[value=moderate])';
      const notes = `Review evidence for report ${report}`;
      const review = { status: 'reviewing', public_outcome: '', moderator_notes: notes };
      await denied(moderator, reportForm, review, page, visitor);
      expect((await responseForForm(moderator, reportForm, { ...review, status: 'invalid' })).status()).toBe(422);
      await action(moderator, reportForm, review);
      await expect(moderator.locator('main > p').first()).toHaveText(pl ? 'W trakcie rozpatrywania' : 'Under review');

      await open(moderator, '/admin/moderation/enforcement');
      await moderator.locator('main a[href="/admin/moderation/enforcement/create"]').click();
      const sanctionForm = 'main form:has(input[value=sanction])';
      const partyPath = `/${lang}/party`;
      const partyForm = `main form[action="${partyPath}"]`;
      await followPartyLink(page, partyPath);
      await expect(page.locator(partyForm)).toBeVisible();
      const partyToken = await page.locator(`${partyForm} [name=_csrf]`).inputValue();
      const now = Math.floor(Date.now() / 1000);
      const reason = `Restriction following report ${report}`;
      const fields = {
        target: playerId, category: 'restriction', status: 'active', public_reason: reason,
        moderator_notes: notes, effective_at: String(now - 1), expires_at: String(now + 3600),
      };
      await denied(moderator, sanctionForm, fields, page, visitor);
      expect((await responseForForm(moderator, sanctionForm, { ...fields, expires_at: fields.effective_at })).status()).toBe(422);
      await action(moderator, sanctionForm, fields);
      await expect(moderator).toHaveURL(/enforcement\/[a-f0-9-]+$/);
      const sanctionPath = new URL(moderator.url()).pathname;
      const sanction = sanctionPath.split('/').at(-1);
      await expect(moderator.locator('main')).toContainText(reason);
      expect((await page.reload()).status()).toBe(200);
      await expect(page.locator(partyForm)).toHaveCount(0);
      const deniedParty = await page.request.post(partyPath, {
        form: {
          _csrf: partyToken, character: uuid7(), purpose: 'hunt', min_level: '1', max_level: '100',
          profession: 'any', starts_at: String(now + 120), expires_at: String(now + 3600), note: 'Party request',
        },
        headers: { origin: new URL(page.url()).origin }, maxRedirects: 0,
      });
      expect(deniedParty.status()).toBe(403);

      const outcome = pl ? 'Po weryfikacji nałożono ograniczenie.' : 'A restriction was issued after review.';
      await open(moderator, `/admin/moderation/reports/${report}`);
      await action(moderator, reportForm, { ...review, status: 'actioned', public_outcome: outcome });
      await followAccountLink(page, `${base}/reports`);
      await main.getByRole('link', { name: reportTarget, exact: true }).click();
      await expect(main).toContainText(outcome);

      await open(moderator, sanctionPath);
      await action(moderator, sanctionForm, { ...fields, status: 'revoked' });
      await expect(moderator.locator('main > p').first()).toHaveText(pl ? 'Cofnięta' : 'Revoked');
      await followPartyLink(page, partyPath);
      await expect(page.locator(partyForm)).toBeVisible();
      evidenceLog.push(['support.report_status_changed', report],
        ['support.enforcement_created', sanction], ['support.enforcement_updated', sanction]);
    });

    await test.step('Game staff inspect the console and confirm the unavailable Game views', async () => {
      const operator = actors.game_admin.page;
      const support = actors.game_support.page;
      await open(support, '/console/dashboard');
      await expect(support.locator('header a[href^="/console/worlds?"]')).toBeVisible();
      await open(operator, '/console/dashboard');
      for (const [path, title, operation] of [
        ['dashboard', pl ? 'Pulpit' : 'Dashboard', 'ops.overview.get'],
        ['worlds', pl ? 'Światy' : 'Worlds', 'world.list'],
        ['nodes', pl ? 'Węzły i hosty' : 'Nodes and hosts', 'node.list'],
        ['database', pl ? 'Baza danych' : 'Database', 'db.status'],
      ]) {
        await operator.locator(`header a[href="/console/${path}?lang=${lang}"]:not([hreflang])`).click();
        await expect(operator.locator('main h1')).toHaveText(title);
        const form = operator.locator('main form[method=get]');
        await form.locator('[name=agent]').fill('unavailable');
        await form.locator('button').click();
        await expect(operator.locator('main .op-badge').first()).toHaveText(pl ? 'R0 · Niedostępne' : 'R0 · Unavailable');
        await operator.locator('main a[href*="agent=unavailable"]').click();
        await expect(operator.locator('main .op-badge').first()).toHaveText(pl ? 'R0 · Niedostępne' : 'R0 · Unavailable');
        const invalid = await operator.request.get(`/console/${path}?agent=%2F`);
        expect(invalid.status()).toBe(400);
        const guest = await visitor.request.get(`/console/${path}`, { maxRedirects: 0 });
        expect(guest.status()).toBe(303);
        const wrong = await actors.support.page.request.get(`/console/${path}?agent=unavailable`);
        expect(wrong.status()).toBe(403);
        // The browser only renders this form with a fresh agent. Exercise the
        // same endpoint against the harness's disconnected agent boundary.
        const invalidRefresh = await gamePost(operator, `/console/${path}`, { lang, operation, agent: '/' });
        expect(invalidRefresh.status()).toBe(400);
        const refresh = await gamePost(operator, `/console/${path}`, { lang, operation, agent: 'unavailable' });
        expect(refresh.status()).toBe(503);
      }
    });

    await test.step('The operator publishes a native session revocation and a Game route', async () => {
      const operator = actors.game_admin.page;
      await open(operator, `/admin/players/${playerId}`);
      const form = 'main form:has(button[value=kick])';
      const why = 'End the player session after resolving the staff incident';
      await denied(operator, form, { reason: why }, page, visitor, 'kick');
      expect((await responseForForm(operator, form, { reason: ' ' }, operator, 'kick')).status()).toBe(400);
      const [{ native_security_generation: before }] = await query(
        'SELECT native_security_generation FROM accounts WHERE id=$1', [playerId],
      );
      const kicked = operator.waitForResponse(response =>
        response.request().method() === 'POST' && response.status() === 303
        && new URL(response.url()).pathname === `/admin/players/${playerId}`);
      await action(operator, form, { reason: why }, 'kick');
      await kicked;
      const [revocation] = await query(`SELECT native_security_generation,cause
        FROM identity_security_revocations WHERE account_id=$1 ORDER BY position DESC LIMIT 1`, [playerId]);
      expect(revocation).toEqual({ native_security_generation: String(BigInt(before) + 1n), cause: 4 });
      evidenceLog.push(['players.kick', playerId]);

      const route = {
        worldId: uuid7(), channelId: uuid7(), name: 'Staff Journey World', region: 'local',
        host: 'route.example.invalid', tlsServerName: 'route.example.invalid', port: 7172, version: 1,
      };
      const command = (id, input) =>
        portalCommand(['game-route', 'publish', '--actor', id, JSON.stringify(input)]);
      expect(command(actors.support.id, route).status).toBe(4);
      expect(command(actors.game_admin.id, { ...route, port: 0 }).status).toBe(2);
      const published = command(actors.game_admin.id, route);
      expect(published.status, 'operator route publication exits successfully').toBe(0);
      expect(JSON.parse(published.stdout)).toMatchObject({ worldId: route.worldId, channelId: route.channelId });
      evidenceLog.push(['game.route.published', `game_channels:${route.worldId}/${route.channelId}`]);
    });

    await test.step('A second staff member approves premium and dispatches the refund', async () => {
      const reviewer = actors.game_support.page;
      const db = pool();
      try {
        const paid = await paidTopup(page, lang, db);
        await open(admin, '/admin/commerce');
        const refundForm = 'main form[action="/admin/commerce/refund"]';
        expect((await responseForForm(admin, refundForm, {
          target: paid.payment_intent_id, reason: 'Review an already credited wallet top-up',
        })).status()).toBe(409);
        const refundable = uuid7();
        // Match the uncredited test intent in commerce/tests.rs and attach a
        // browser top-up so the signed test provider can settle its refund.
        await db.query(`WITH payment AS (
          INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state)
          VALUES ($1,$2,'test',$3,100,'PLN','succeeded') RETURNING id,account_id
        )
        INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id)
        SELECT $4,account_id,'e2e-refund',100,100,'PLN',id FROM payment`,
        [refundable, playerId, `test:${refundable}`, uuid7()]);
        for (const [operation, target, values] of [
          ['grant', playerId, { days: '7' }],
          ['refund', refundable, {}],
        ]) {
          await open(admin, '/admin/commerce');
          const selector = `main form[action="/admin/commerce/${operation}"]`;
          const requestId = await admin.locator(`${selector} [name=request_id]`).inputValue();
          const fields = { ...values, target, reason: 'Process the player account request with independent review' };
          await denied(admin, selector, fields, page, visitor);
          expect((await responseForForm(admin, selector, { ...fields, reason: 'short' })).status()).toBe(422);
          await submit(admin, selector, fields);
          const approval = `main form[action="/admin/commerce/approvals/${requestId}"]`;
          await expect(admin.locator(approval)).toContainText(pl ? 'Oczekuje na drugą osobę z zespołu' : 'Awaiting a second staff member');
          await expect(admin.locator(`${approval} button`)).toHaveCount(0);
          expect((await responseForForm(admin, approval, {})).status()).toBe(403);
          await open(reviewer, '/admin/commerce');
          await submit(reviewer, approval);
          await expect(reviewer.locator(approval)).toHaveCount(0);
          const [result] = await query('SELECT approved_by, result FROM commerce_operations WHERE id=$1', [requestId]);
          expect(result.approved_by).toBe(actors.game_support.id);
          expect(result.result).toBeTruthy();
          evidenceLog.push(['commerce.requested', target], ['commerce.approved', target]);
          if (operation === 'grant') {
            const [premium] = await query('SELECT effective_until FROM premium_ledger WHERE request_id=$1', [requestId]);
            await followAccountLink(page, `/${lang}/panel/premium`);
            await expect(main).toContainText(premium.effective_until.toISOString().slice(0, 10));
          } else {
            await expect.poll(async () => (await query(
              'SELECT state FROM commerce_refunds WHERE operation_id=$1', [requestId],
            ))[0]?.state, { timeout: 90_000 }).toBe('done');
            await open(admin, `/admin/console/audit?operation=commerce.refund.dispatched&request_id=${requestId}`);
            await expect(admin.locator('main')).toContainText('commerce.refund.dispatched');
            await followAccountLink(page, `/${lang}/panel/payments`);
            const payment = main.locator('article').filter({ hasText: target });
            await expect(payment).toContainText('100 PLN');
            await expect(payment).toContainText(pl ? 'Opłacona' : 'Paid');
            const event = JSON.stringify({
              event_id: randomUUID(), intent_id: target, provider_reference: `test:${target}`,
              amount: { minor_units: 100, currency: 'PLN' }, state: 'refunded',
            });
            const signature = createHmac('sha256', process.env.OTERYN_SHOP_TEST_PAYMENT_KEY)
              .update(event).digest('base64');
            const settlement = await visitor.request.post('/__test/payments/webhook', {
              data: event,
              headers: { 'content-type': 'application/json', 'x-test-payment-signature': signature },
            });
            expect(settlement.status()).toBe(204);
            expect((await page.reload()).status()).toBe(200);
            await expect(payment).toContainText(pl ? 'Zwrócona' : 'Refunded');
          }
        }
      } finally {
        await db.end();
      }
    });

    await test.step('The administrator revokes the editor role and verifies the audit trail', async () => {
      await membership(admin, actors.content_editor.id, roles.content_editor, 'revoke');
      const editor = actors.content_editor;
      await expectLoginRedirect(editor.page, '/admin/news');
      await loginWithRecoveryCode(editor.page, lang, editor.credentials, editor.recovery);
      await expect(editor.page).toHaveURL(new RegExp(`/${lang}/$`));
      expect((await editor.page.request.get('/admin/news')).status()).toBe(403);
      expect((await page.request.get('/admin/console/audit')).status()).toBe(403);
      evidenceLog.push(['rbac.role.revoked', actors.content_editor.id]);
      for (const id of [adminId, ...Object.values(actors).map(actor => actor.id)]) {
        await open(admin, `/admin/console/audit?admin=${id}&target=mfa`);
        const log = admin.locator('main');
        await expect(log).toContainText(id);
        for (const operation of ['mfa.enrollment_started', 'mfa.enrolled', 'mfa.login.recovery']) {
          await expect(log).toContainText(operation);
        }
      }
      for (const [operation, target] of evidenceLog) await audit(admin, operation, target);
    });
  } finally {
    await Promise.all(contexts.map(context => context.close()));
    // The harness grants the fixture admin directly. Remove its fixture membership
    // after the browser assertions so the next project can exercise the last-admin guard.
    if (adminId) await query('DELETE FROM account_roles WHERE account_id=$1', [adminId]);
  }
});
