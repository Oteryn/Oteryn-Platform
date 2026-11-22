import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { login, submit } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { actor, copy, labels, mutation, open, submitForm } from '../support-helpers.mjs';

const staffTickets = '/admin/support/tickets';
const staffReports = '/admin/moderation/reports';
const enforcement = '/admin/moderation/enforcement';
const players = '/admin/players';
const detail = page => new URL(page.url()).pathname;
const recordId = page => detail(page).split('/').at(-1);
const staffForm = (page, action) => page.locator(`main form:has(input[name=action][value="${action}"])`);
const playerForm = (page, action) => page.locator(`main form:has(button[name=action][value="${action}"])`);
const ticketForm = (page, action) => page.locator(`main form[action$="/${action}"]`);

async function openAccountLink(page, lang, path, name) {
  await open(page, `/${lang}/`);
  const menu = page.locator('#portal-menu-toggle');
  if (await menu.isVisible()) await menu.click();
  await page.locator('[data-account-link]').click();
  const account = page.getByRole('dialog');
  await expect(account).toBeVisible();
  const link = account.getByRole('link', { name, exact: true });
  await expect(link).toHaveAttribute('href', path);
  await link.click();
  await expect(page).toHaveURL(new RegExp(`${path}$`));
}

async function newTicket(page, lang, subject) {
  const base = `/${lang}/support/tickets`;
  const t = copy('tickets', lang);
  await open(page, base);
  await page.getByRole('link', { name: t.create }).click();
  await expect(page).toHaveURL(new RegExp(`${base}/create$`));
  const form = page.locator('main form');
  await labels(page, [t.title, t.create, t.category, t.subject, t.body, t.send]);
  for (const category of ['account', 'character', 'technical', 'payment', 'other']) {
    await expect(form.locator(`option[value=${category}]`)).toHaveText(t[category]);
  }
  await submitForm(form, { category: 'account', subject, body: `Details ${subject}` }, {
    bad: { subject: ' ' },
  });
  await expect(page).toHaveURL(new RegExp(`${base}/[a-f0-9-]+$`));
  await expect(page.getByRole('heading', { name: subject })).toBeVisible();
  return detail(page);
}

test('ticket creation, private and public replies, acknowledgment, close and reopen', async ({ browser }, info) => {
  test.setTimeout(420_000);
  const lang = info.project.metadata.locale;
  const t = copy('tickets', lang);
  const a = copy('moderation', lang);
  await actor(browser, info, null, async (player, account) => {
    await open(player, `/${lang}/account/notifications`);
    await submit(player, 'main form', { support_email: true, support_portal: true });
    const subject = `Ticket ${randomUUID()}`;
    const path = await newTicket(player, lang, subject);
    const id = recordId(player);
    const base = `/${lang}/support/tickets`;
    await labels(player, [t.open, t.status, t.version, t.user, t.reply]);
    await submitForm(ticketForm(player, 'reply'), { body: 'Player follow-up' }, { bad: { body: ' ' } });
    await labels(player, [t.waiting_staff, 'Player follow-up']);
    await expect(ticketForm(player, 'reply')).toHaveCount(0);

    await actor(browser, info, 'support', async staff => {
      await open(staff, staffTickets);
      await staff.locator(`a[href="${staffTickets}/${id}"]`).click();
      await expect(staff).toHaveURL(new RegExp(`${staffTickets}/${id}$`));
      await labels(staff, [a.title, a.public, a.waiting_staff, a.body, a.reply, a.internal, a.ticket_status]);
      const reply = staffForm(staff, 'reply');
      await submitForm(reply, { body: 'Staff-only ticket note', internal: true }, { bad: { body: ' ' } });
      await expect(staff.locator('article').filter({ hasText: 'Staff-only ticket note' })).toContainText(a.internal);
      await submitForm(reply, { body: 'Visible staff answer', internal: false });
      await labels(staff, [a.waiting_user, 'Visible staff answer']);
      const notifications = await query(
        "SELECT body FROM notifications WHERE account_id=$1 AND category='support'",
        [account.id],
      );
      expect(notifications).toHaveLength(1);
      expect(notifications[0].body).toContain(`${t['notification-reply']} ${path}`);
      await expect.poll(async () => {
        const response = await player.request.get('/__mail');
        return (await response.json()).some(message =>
          message.to.includes(account.email) && message.raw.replace(/=\r?\n/g, '').includes(path));
      }, { timeout: 90_000, message: 'Mail catcher receives the ticket link' }).toBe(true);

      await player.goto(base);
      const row = player.locator('main article').filter({ hasText: subject });
      await expect(row).toContainText(t['new-reply']);
      await row.getByRole('link').click();
      await labels(player, [t.waiting_user, t.staff, 'Visible staff answer', t['mark-read']]);
      await expect(player.locator('main')).not.toContainText('Staff-only ticket note');
      await submitForm(ticketForm(player, 'read'), {}, { bad: { reply: 'invalid' } });
      await expect(ticketForm(player, 'read')).toHaveCount(0);
      await player.locator(`main a[href="${base}"]`).click();
      await expect(row).not.toContainText(t['new-reply']);
      await player.getByRole('link', { name: t.previous }).click();
      await expect(player).toHaveURL(new RegExp(`${base}\\?page=0$`));
      await row.getByRole('link').click();
      await expect(player).toHaveURL(new RegExp(`${path}$`));
      const status = staffForm(staff, 'ticket_status');
      for (const state of ['open', 'waiting_staff', 'waiting_user', 'resolved', 'closed']) {
        await expect(status.locator(`option[value=${state}]`)).toHaveText(a[state]);
      }
      for (const state of ['open', 'waiting_staff', 'waiting_user', 'resolved', 'closed']) {
        await submitForm(status, { status: state }, { bad: { status: 'invalid' } });
        await expect(staff.locator('main > p').first()).toHaveText(a[state]);
      }
      await submitForm(status, { status: 'closed' }, { bad: { lock_version: '1', expected: 409 } });
      const statusNotices = await query(
        "SELECT body FROM notifications WHERE account_id=$1 AND category='support' AND body LIKE $2",
        [account.id, `%${t['notification-status']} ${path}%`],
      );
      expect(statusNotices).toHaveLength(6);
    });
    await open(player, path);
    await labels(player, [t.closed]);
    const status = ticketForm(player, 'status');
    await submitForm(status, { status: 'open' }, { bad: { status: 'waiting_staff' } });
    await labels(player, [t.open]);
    await submitForm(status, { status: 'closed' }, { method: 'PUT', bad: { status: 'invalid' } });
    await player.reload();
    await labels(player, [t.closed]);
    await submitForm(status, { status: 'open' });
    await submitForm(status, { status: 'closed' });
    await labels(player, [t.closed]);

    await actor(browser, info, null, async outsider => {
      await open(outsider, path, 404);
      await open(outsider, base + '/create');
      const token = await outsider.locator('main [name=_csrf]').inputValue();
      await mutation(outsider, path + '/reply', { _csrf: token, lock_version: 1, body: 'Foreign' }, 404);
      await mutation(outsider, path + '/status', { _csrf: token, lock_version: 1, status: 'closed' }, 404);
      await mutation(outsider, path + '/read', { _csrf: token, reply: randomUUID() }, 404);
      await open(outsider, staffTickets, 403);
    });
  });
});

test('report creation, validation, staff decisions and public history', async ({ browser }, info) => {
  test.setTimeout(420_000);
  const lang = info.project.metadata.locale;
  const r = copy('reports', lang);
  const a = copy('moderation', lang);
  const base = `/${lang}/support/reports`;
  await actor(browser, info, null, async (player, account) => {
    await open(player, base);
    await labels(player, [r.title, r.history, r.new, r.empty]);
    await expect(player.getByRole('navigation', { name: r.navigation })).toBeVisible();
    await player.getByRole('link', { name: r.new }).click();
    await labels(player, [r.type, r.category, r['category-help'], r.target, r.evidence, r.submit]);
    const form = player.locator('main form');
    for (const name of ['player', 'content', 'guild']) {
      await expect(form.locator(`[name=report_type] option[value=${name}]`)).toHaveText(r[name]);
      await expect(form.locator(`optgroup[label="${r[name]}"]`)).toHaveCount(1);
    }
    for (const name of ['harassment', 'cheating', 'name', 'conduct', 'other', 'illegal_content',
      'privacy', 'spam', 'misinformation', 'description', 'impersonation']) {
      const options = form.locator(`option[value=${name}]`);
      expect(await options.count()).toBeGreaterThan(0);
      for (const option of await options.all()) await expect(option).toHaveText(r[name]);
    }
    const reports = [];
    for (const [kind, category] of [['player', 'cheating'], ['content', 'spam'], ['guild', 'description']]) {
      await open(player, base + '/create');
      await submitForm(form, { report_type: kind, category, target_reference: account.id, evidence_summary: kind });
      await expect(player).toHaveURL(new RegExp(`${base}/[a-f0-9-]+$`));
      reports.push(recordId(player));
      await labels(player, [r[kind], r[category], r.status, r.submitted, r.date]);
    }
    const [direct] = await query(
      `INSERT INTO player_reports(reporter_account_id,request_key,report_type,category,target_reference)
       VALUES($1,uuidv7(),'player','harassment','Direct decision fixture') RETURNING id`,
      [account.id],
    );
    reports.push(direct.id);
    await player.getByRole('link', { name: r.history }).click();
    await player.locator(`a[href="${base}/${reports[0]}"]`).click();

    await actor(browser, info, 'support', async staff => {
      await open(staff, staffReports);
      await staff.locator(`a[href="${staffReports}/${reports[0]}"]`).click();
      await labels(staff, [a.title, a.submitted, a.status, a.public_outcome, a.moderator_notes, a.moderate]);
      const form = staffForm(staff, 'moderate');
      for (const state of ['submitted', 'reviewing', 'actioned', 'rejected', 'closed']) {
        await expect(form.locator(`option[value=${state}]`)).toHaveText(a[state]);
      }
      const workflows = [
        ['submitted', 'reviewing', 'reviewing', 'actioned', 'closed'],
        ['rejected', 'closed'],
        ['reviewing', 'rejected', 'closed'],
        ['actioned', 'closed'],
      ];
      for (const [index, workflow] of workflows.entries()) {
        await open(staff, `${staffReports}/${reports[index]}`);
        for (const state of workflow) {
          const outcome = `Public decision ${state}`;
          await submitForm(form, { status: state, public_outcome: outcome, moderator_notes: 'Staff-only report note' }, {
            bad: { status: 'invalid' },
          });
          await expect(staff.locator('main > p').first()).toHaveText(a[state]);
          await open(player, `${base}/${reports[index]}`);
          await labels(player, [r[state], r.outcome, outcome]);
          await expect(player.locator('main')).not.toContainText('Staff-only report note');
        }
        const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
        await mutation(staff, detail(staff), { ...fields, status: 'reviewing' }, 409);
      }
      await open(staff, enforcement, 403);
    });
    await actor(browser, info, null, async outsider => {
      await open(outsider, base + '/create');
      const invalid = outsider.locator('main form');
      await submitForm(invalid, {
        report_type: 'player', category: 'privacy', target_reference: account.id, evidence_summary: 'Report evidence',
      });
      await expect(outsider).toHaveURL(new RegExp(`${base}/create$`));
      await labels(outsider, [r.invalid]);
      await expect(invalid.locator('[name=target_reference]')).toHaveValue(account.id);
      await open(outsider, `${base}/${reports[0]}`, 404);
      await open(outsider, staffReports, 403);
    });
  });
});

test('moderator creates sanctions, revokes them, and adjudicates requested appeals', async ({ browser }, info) => {
  test.setTimeout(420_000);
  const lang = info.project.metadata.locale;
  const a = copy('moderation', lang);
  await actor(browser, info, null, async (player, account) => {
    await actor(browser, info, 'moderator', async staff => {
      await open(staff, staffReports, 403);
      await open(staff, staffTickets, 403);
      const now = Math.floor(Date.now() / 1000);
      const values = {
        target: account.id, category: 'warning', status: 'active', public_reason: '',
        moderator_notes: 'Private enforcement note', effective_at: String(now - 60), expires_at: String(now + 3600),
      };
      const sanction = staffForm(staff, 'sanction');
      const appeal = staffForm(staff, 'appeal');
      for (const decision of ['accepted', 'rejected', 'revoked', 'expired']) {
        if (decision === 'expired') values.expires_at = String(now - 1);
        values.public_reason = `Sanction ${decision} ${randomUUID()}`;
        await open(staff, enforcement);
        await staff.getByRole('link', { name: a.create }).click();
        await expect(staff).toHaveURL(new RegExp(`${enforcement}/create$`));
        await labels(staff, [a.title, a.create, a.active, a.target, a.category, a.status, a.public_reason,
          a.moderator_notes, a.effective_at, a.expires_at]);
        await expect(staff.locator('main h1')).toHaveText(lang === 'pl' ? 'Pomoc i moderacja' : 'Support and moderation');
        await expect(sanction.locator('button')).toHaveText(a.create);
        for (const option of ['warning', 'restriction', 'suspension', 'active', 'expired', 'revoked']) {
          await expect(sanction.locator(`option[value=${option}]`)).toHaveText(a[option]);
        }
        await submitForm(sanction, values, { bad: { expires_at: values.effective_at } });
        await expect(staff).toHaveURL(new RegExp(`${enforcement}/[a-f0-9-]+$`));
        const id = recordId(staff);
        await expect(staff.locator('main > p').first()).toHaveText(a.active);
        await expect(sanction.locator('button')).toHaveText(a.sanction);
        await open(staff, enforcement);
        await staff.getByRole('link', { name: values.public_reason }).click();
        await expect(staff).toHaveURL(new RegExp(`${enforcement}/${id}$`));
        await labels(staff, [a.appeal_status, a.appeal_outcome, a.appeal, a.internal, values.moderator_notes]);
        for (const option of ['reviewing', 'accepted', 'rejected']) {
          await expect(appeal.locator(`option[value=${option}]`)).toHaveText(a[option]);
        }
        if (decision === 'revoked' || decision === 'expired') {
          const bad = decision === 'revoked' ? { status: 'expired', expected: 409 } : { target: randomUUID() };
          await submitForm(sanction, { ...values, status: decision }, { bad });
          await expect(staff.locator('main > p').first()).toHaveText(a[decision]);
        } else {
          // No player-facing appeal router is composed. Seed only the request
          // that would enter the staff workflow, then use its real SSR controls.
          await query(
            "UPDATE support_appeal_workflows SET status='requested',message=$2 WHERE sanction_id=$1",
            [id, 'Review requested by account owner'],
          );
          await staff.reload();
          await labels(staff, [a.public, 'Review requested by account owner']);
          await submitForm(appeal, { appeal_status: 'reviewing', appeal_outcome: 'Review started' });
          await expect(staff.locator('main')).toContainText('Review started');
          await submitForm(appeal, { appeal_status: decision, appeal_outcome: `Appeal ${decision}` }, {
            bad: { appeal_outcome: ' ' },
          });
          await expect(staff.locator('main')).toContainText(`Appeal ${decision}`);
          const [row] = await query('SELECT status FROM support_appeal_workflows WHERE sanction_id=$1', [id]);
          expect(row.status).toBe(decision);
          await expect(staff.locator('main > p').first()).toHaveText(decision === 'accepted' ? a.revoked : a.active);
        }
        await open(player, `${enforcement}/${id}`, 403);
      }
    });
  });
});

test('support searches players, adds notes, revokes sessions and requests a two-person MFA reset', async ({ browser }, info) => {
  test.setTimeout(420_000);
  const lang = info.project.metadata.locale;
  const p = copy('players', lang);
  await actor(browser, info, null, async (player, account) => {
    const ticket = await newTicket(player, lang, `Account recovery ${randomUUID()}`);
    await actor(browser, info, 'support', async staff => {
      const path = `${players}/${account.id}`;
      await open(staff, players);
      await labels(staff, [p.title, p.query, p.search]);
      const search = staff.locator('main form[method=get]');
      await search.locator('[name=q]').fill(account.email);
      await search.getByRole('button', { name: p.search }).click();
      await expect(staff.locator(`main a[href="${path}"]`)).toHaveCount(1);
      await staff.locator(`main a[href="${path}"]`).click();
      await labels(staff, [account.id, p.status, p['status-active'], p.locale, p.verified, p.yes,
        p.sessions, p.characters, p.notes, p['add-note'], p.logout, p.kick, p.reason,
        p.reset, p.approve, p.evidence, p.request]);
      await expect(staff.getByRole('heading', { name: p.characters }).locator('..')).toContainText(p.unavailable);
      await expect(staff.getByRole('heading', { name: p.sessions }).locator('..').getByRole('listitem'))
        .toHaveCount(1);
      await open(staff, players + '?q=bad-search', 400);
      await open(staff, path);
      await submitForm(playerForm(staff, 'note'), { body: 'New internal player note' }, { bad: { body: ' ', expected: 400 } });
      await expect(staff.getByRole('heading', { name: p.notes }).locator('..').getByRole('listitem'))
        .toHaveText('New internal player note');
      if (lang === 'pl') {
        await expect(playerForm(staff, 'request-reset').getByRole('button'))
          .toHaveText('Zleć reset MFA po weryfikacji przez zespół pomocy');
        await expect(playerForm(staff, 'request-reset').locator('label[for=evidence]'))
          .toHaveText('Identyfikator potwierdzonej weryfikacji zespołu pomocy');
      }
      const kick = playerForm(staff, 'kick');
      const csrf = await kick.locator('[name=_csrf]').inputValue();
      await mutation(staff, path, { _csrf: csrf, action: 'kick', reason: 'Support has no kick permission' }, 403);
      await submitForm(playerForm(staff, 'logout'), { reason: 'Verified player requests session revocation' }, {
        bad: { reason: ' ', expected: 400 },
      });
      const revoked = await player.request.get(`/${lang}/account/sessions`, { maxRedirects: 0 });
      expect(revoked.status()).toBe(303);
      expect(revoked.headers().location).toBe(`/${lang}/login`);

      const attest = playerForm(staff, 'verify-identity');
      await expect(attest.locator('button')).toHaveText(lang === 'pl' ? 'Potwierdź weryfikację' : 'Attest verification');
      await expect(attest.locator('label')).toHaveText(lang === 'pl'
        ? 'Zweryfikowano tożsamość właściciela zgłoszenia konta'
        : 'Identity verified for the account ticket owner');
      await submitForm(attest, { ticket: ticket.split('/').at(-1) }, { bad: { ticket: randomUUID(), expected: 404 } });
      await expect(staff).toHaveURL(/\?evidence=[a-f0-9-]+$/);
      const evidence = new URL(staff.url()).searchParams.get('evidence');
      await expect(playerForm(staff, 'request-reset').locator('[name=evidence]')).toHaveValue(evidence);
      await submitForm(playerForm(staff, 'request-reset'), { evidence, reason: 'Account owner verified' }, {
        bad: { evidence: randomUUID(), expected: 403 },
      });
      await expect(staff).toHaveURL(/\?request=[a-f0-9-]+$/);
      const request = new URL(staff.url()).searchParams.get('request');
      await expect(staff.locator('main [role=status]').filter({ hasText: request })).toContainText(p.request);
      await mutation(staff, path, { _csrf: csrf, action: 'approve-reset', request }, 403);

      await actor(browser, info, 'platform_admin', async approver => {
        await open(approver, path);
        await submitForm(playerForm(approver, 'approve-reset'), { request }, { bad: { request: randomUUID(), expected: 403 } });
        await expect(approver).toHaveURL(new RegExp(`${path}$`));
        const [row] = await query('SELECT confirmed FROM account_mfa_totp WHERE account_id=$1', [account.id]);
        expect(row).toBeUndefined();
        await login(player, lang, account);
        await open(player, `/${lang}/account/sessions`);
        await expect(player).toHaveURL(new RegExp(`/${lang}/account/sessions$`));
      });
    });
  }, { mfa: true });
});

test('moderator kick-only view advances Game revocation and game_support is refused', async ({ browser }, info) => {
  test.setTimeout(300_000);
  const p = copy('players', info.project.metadata.locale);
  await actor(browser, info, null, async (player, account) => {
    const path = `${players}/${account.id}`;
    await actor(browser, info, 'moderator', async moderator => {
      await open(moderator, players, 403);
      await open(moderator, path);
      await labels(moderator, [p.title, account.id, p.reason, p.kick]);
      await expect(moderator.locator('main form')).toHaveCount(1);
      await submitForm(playerForm(moderator, 'kick'), { reason: 'Verified Game session disconnect' }, {
        bad: { reason: ' ', expected: 400 },
      });
      await expect(moderator).toHaveURL(new RegExp(`${path}$`));
      const [event] = await query(
        'SELECT native_security_generation FROM identity_security_revocations WHERE account_id=$1 ORDER BY position DESC LIMIT 1',
        [account.id],
      );
      expect(Number(event.native_security_generation)).toBe(2);
    });
    await actor(browser, info, 'game_support', async staff => {
      await open(staff, `/${info.project.metadata.locale}/support/tickets/create`);
      const _csrf = await staff.locator('main [name=_csrf]').inputValue();
      await mutation(staff, path, { _csrf, action: 'kick', reason: 'Game role lacks the portal permission' }, 403);
      await open(staff, players, 403);
      await open(staff, path, 403);
      await open(staff, staffTickets, 403);
      await open(staff, staffReports, 403);
      await open(staff, enforcement, 403);
    });
    await open(player, path, 403);
  });
});

test('anonymous request gates for support and moderation', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const tickets = `/${lang}/support/tickets`;
  const reports = `/${lang}/support/reports`;
  const id = randomUUID();
  await open(page, `/${lang}/`);
  for (const [path, location, status] of [
    ...[tickets, reports].flatMap(base => [base, base + '/create', `${base}/${id}`])
      .map(path => [path, `/${lang}/login`, 303]),
    ...[staffTickets, staffReports, enforcement, enforcement + '/create', players,
      ...[staffTickets, staffReports, enforcement, players].map(path => `${path}/${id}`)]
      .map(path => [path, '/login', 303]),
  ]) {
    const response = await page.request.get(path, { maxRedirects: 0 });
    expect(response.status(), path).toBe(status);
    expect(response.headers().location, path).toBe(location);
  }
  for (const path of [
    tickets, reports, ...['reply', 'read', 'status'].map(action => `${tickets}/${id}/${action}`),
    ...[staffTickets, staffReports, enforcement, players].map(base => `${base}/${id}`),
  ]) await mutation(page, path, {}, 403);
  await mutation(page, `${tickets}/${id}/status`, {}, 403, 'PUT');

  await open(page, `/${lang}/login`);
  const _csrf = await page.locator('main form [name=_csrf]').inputValue();
  await mutation(page, tickets, { _csrf }, 403);
  await mutation(page, `${tickets}/${id}/reply`, { _csrf, lock_version: 1, body: 'Anonymous reply' }, 403);
  await mutation(page, `${tickets}/${id}/read`, { _csrf, reply: id }, 403);
  for (const method of ['POST', 'PUT']) {
    await mutation(page, `${tickets}/${id}/status`, { _csrf, lock_version: 1, status: 'closed' }, 403, method);
  }
  const report = await mutation(page, reports, { _csrf }, 302);
  expect(report.headers().location).toBe(`/${lang}/login`);
  for (const base of [staffTickets, staffReports, enforcement, players]) {
    const staff = await mutation(page, `${base}/${id}`, { _csrf }, 303);
    expect(staff.headers().location).toBe('/login');
  }
});

test('ticket and report history pagination, including ticket message cursors', async ({ browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const t = copy('tickets', lang);
  const r = copy('reports', lang);
  const a = copy('moderation', lang);
  const tickets = `/${lang}/support/tickets`;
  const reports = `/${lang}/support/reports`;
  await actor(browser, info, null, async (player, account) => {
    const tag = randomUUID();
    const created = await query(
      `INSERT INTO tickets(account_id,request_key,category,subject,status,closed_at,last_message_at)
       SELECT $1,uuidv7(),'other',$2 || ' ' || i,'closed',now(),now()-make_interval(secs=>i)
       FROM generate_series(1,31) i RETURNING id,subject`,
      [account.id, tag],
    );
    const first = created.find(row => row.subject === `${tag} 1`);
    const path = `${tickets}/${first.id}`;
    await query(
      `INSERT INTO ticket_messages(ticket_id,author_account_id,author_kind,body)
       SELECT $1,$2,'user','Message ' || i FROM generate_series(1,50) i`,
      [first.id, account.id],
    );
    await query(
      "INSERT INTO ticket_messages(ticket_id,author_account_id,author_kind,body) VALUES($1,$2,'staff','Paged staff reply')",
      [first.id, account.id],
    );
    await query('UPDATE tickets SET unread_reply=true WHERE id=$1', [first.id]);
    await query(
      `INSERT INTO player_reports(reporter_account_id,request_key,report_type,category,target_reference,status,public_outcome,created_at)
       SELECT $1,uuidv7(),'player','harassment',$2 || ' ' || i,'closed','Historical outcome',now()-make_interval(secs=>i)
       FROM generate_series(1,31) i`,
      [account.id, tag],
    );

    await openAccountLink(player, lang, tickets, t.title);
    await player.getByRole('link', { name: t.next }).click();
    await expect(player).toHaveURL(/\?page=1$/);
    await expect(player.getByRole('link', { name: `${tag} 21`, exact: true })).toBeVisible();
    await player.getByRole('link', { name: t.previous }).click();
    await player.getByRole('link', { name: first.subject, exact: true }).click();
    await expect(player).toHaveURL(new RegExp(`${path}$`));
    await expect(ticketForm(player, 'read')).toHaveCount(0);
    await player.getByRole('link', { name: t.next }).click();
    await expect(player).toHaveURL(/\?after=[a-f0-9-]+$/);
    await expect(player.getByText('Paged staff reply', { exact: true })).toBeVisible();
    await submitForm(ticketForm(player, 'read'));
    await expect(player).toHaveURL(new RegExp(`${path}$`));
    const [read] = await query('SELECT unread_reply FROM tickets WHERE id=$1', [first.id]);
    expect(read.unread_reply).toBe(false);
    await open(player, path + '?after=invalid', 400);

    await openAccountLink(player, lang, reports, lang === 'pl' ? 'Zgłoszenia naruszeń' : 'Abuse reports');
    await player.getByRole('navigation', { name: r.pagination }).getByRole('link', { name: r.next }).click();
    await expect(player).toHaveURL(/\?page=2$/);
    await expect(player.getByRole('link', { name: `${tag} 21`, exact: true })).toBeVisible();
    await player.getByRole('link', { name: r.previous }).click();
    await expect(player).toHaveURL(/\?page=1$/);

    await actor(browser, info, 'support', async staff => {
      for (const base of [staffTickets, staffReports]) {
        await open(staff, base);
        await staff.getByRole('link', { name: a.next }).click();
        await expect(staff).toHaveURL(new RegExp(`${base}\\?page=2$`));
        await expect(staff.locator(`main a[href^="${base}/"]`).first()).toBeVisible();
        await open(staff, base + '?page=0', 422);
      }
      await open(staff, `${staffTickets}/${first.id}`);
      await staff.getByRole('link', { name: a.next }).click();
      await expect(staff.getByText('Paged staff reply', { exact: true })).toBeVisible();
    });
  });
});

test('report submission explains pending, unjustified and rate limits', async ({ browser }, info) => {
  const lang = info.project.metadata.locale;
  const r = copy('reports', lang);
  const base = `/${lang}/support/reports`;
  await actor(browser, info, null, async (player, account) => {
    await query(
      `INSERT INTO player_reports(reporter_account_id,request_key,report_type,category,target_reference)
       SELECT $1,uuidv7(),'player','harassment','Existing report ' || i FROM generate_series(1,5) i`,
      [account.id],
    );
    await open(player, base + '/create');
    const form = player.locator('main form');
    const submitReport = () => submitForm(form, { target_reference: 'New report' });
    await submitReport();
    await labels(player, [r['pending-limit']]);
    await query(
      "UPDATE player_reports SET status='rejected',public_outcome='Rejected fixture' WHERE reporter_account_id=$1",
      [account.id],
    );
    await submitReport();
    await labels(player, [r['unjustified-limit']]);
    await submitReport();
    await labels(player, [r['unjustified-limit']]);
    const response = await submitForm(form, { target_reference: 'New report' }, { status: 429 });
    expect(response.headers()['retry-after']).toBe('60');
    await expect(player.locator('body')).toHaveText(r['rate-limited']);
  });
});

test('player detail releases a coin hold and retains the unpaid reversal', async ({ browser }, info) => {
  test.setTimeout(300_000);
  const p = copy('players', info.project.metadata.locale);
  await actor(browser, info, null, async (_player, account) => {
    const path = `${players}/${account.id}`;
    await actor(browser, info, 'platform_admin', async staff => {
      await open(staff, path);
      await labels(staff, [p['coin-hold'], p['coin-hold-none'], p['coin-debt']]);
      // Mirror the chargeback setup from oteryn_db::test_support using this account only.
      const [hold] = await query(
        `WITH quote AS (
           INSERT INTO coin_purchase_quotes(id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at,consumed_at)
           VALUES(uuidv7(),$1,uuidv7(),10,100,'EUR',2,'testing',now(),now()) RETURNING id,package_id
         ), purchase AS (
           INSERT INTO coin_purchase_orders(id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,code,review_required)
           SELECT uuidv7(),$1,uuidv7(),id,package_id,10,100,'EUR',2,'testing',uuidv7(),'processing','review',true FROM quote RETURNING id
         ), reversal AS (
           INSERT INTO coin_purchase_reversals(id,order_id,payer_account_id,kind,provider,provider_event_id,coins,debited,outcome)
           SELECT uuidv7(),id,$1,'chargeback','test',id::text,10,0,'held' FROM purchase RETURNING order_id
         ) INSERT INTO coin_purchase_holds(account_id,order_id,reason)
           SELECT $1,order_id,'reversal_insufficient_balance' FROM reversal RETURNING order_id`,
        [account.id],
      );
      await staff.reload();
      await labels(staff, [hold.order_id, p['coin-hold-release']]);
      const form = playerForm(staff, 'release-coin-hold');
      const token = await form.locator('[name=_csrf]').inputValue();
      const holdId = await form.locator('[name=hold]').inputValue();
      await submitForm(form, { reason: 'Staff reviewed chargeback' }, { bad: { hold: randomUUID(), expected: 409 } });
      await expect(staff).toHaveURL(/\?coin_hold=released$/);
      await labels(staff, [p['coin-hold-released']]);
      await expect(form).toHaveCount(0);
      const [reversal] = await query('SELECT outcome FROM coin_purchase_reversals WHERE order_id=$1', [hold.order_id]);
      expect(reversal.outcome).toBe('held');
      const replay = await mutation(staff, path,
        { _csrf: token, action: 'release-coin-hold', hold: holdId, reason: 'Staff reviewed chargeback' }, 303);
      expect(replay.headers().location).toBe(path + '?coin_hold=released');
    });
  });
});

test('ticket form preserves the draft when capacity is full and handles request-key replay', async ({ browser }, info) => {
  const lang = info.project.metadata.locale;
  const t = copy('tickets', lang);
  const base = `/${lang}/support/tickets`;
  await actor(browser, info, null, async (player, account) => {
    const existing = await query(
      `INSERT INTO tickets(account_id,request_key,category,subject)
       SELECT $1,uuidv7(),'account','Capacity fixture ' || i FROM generate_series(1,10) i RETURNING id`,
      [account.id],
    );
    await open(player, base + '/create');
    const form = player.locator('main form');
    const subject = 'Draft preserved after capacity refusal';
    await submitForm(form, { category: 'technical', subject, body: 'Useful draft details' });
    await expect(player).toHaveURL(new RegExp(`${base}/create$`));
    await expect(player.getByRole('alert')).toHaveText(t.error);
    await expect(form.locator('[name=subject]')).toHaveValue(subject);
    await expect(form.locator('[name=body]')).toHaveValue('Useful draft details');
    await expect(form.locator('[name=category]')).toHaveValue('technical');

    await open(player, `${base}/${existing[0].id}`);
    await submitForm(ticketForm(player, 'status'), { status: 'closed' });
    await player.getByRole('link', { name: t.create }).click();
    const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
    await submitForm(form);
    const path = detail(player);
    await expect(player.getByRole('heading', { name: subject })).toBeVisible();
    const replay = await mutation(player, base, fields, 302);
    expect(replay.headers().location).toBe(path);
    const conflict = await mutation(player, base, { ...fields, subject: 'Edited after original submission' }, 302);
    expect(conflict.headers().location).toBe(base + '/create');
    await open(player, base + '/create');
    await expect(player.getByRole('alert')).toHaveText(t.error);
    await expect(form.locator('[name=subject]')).toHaveValue('Edited after original submission');
    await expect(form.locator('[name=request_key]')).not.toHaveValue(fields.request_key);
    await open(player, `${base}/${existing[0].id}`);
    const status = ticketForm(player, 'status');
    const reopen = await status.evaluate(element => Object.fromEntries(new FormData(element)));
    const response = await mutation(player, `${base}/${existing[0].id}/status`, { ...reopen, status: 'open' }, 409);
    expect(await response.text()).toBe(t.error);
  });
});
