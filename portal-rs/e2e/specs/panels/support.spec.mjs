import { createHash } from 'node:crypto';
import { expect, test } from '../../fixtures.mjs';
import { login, submit } from '../../helpers.mjs';
import {
  action, enrollMfa, loginWithRecoveryCode, query, unique,
} from '../../staff-rbac-helpers.mjs';
import { otherBrowser, postForm, seedAccount } from '../../security/auth-fixtures.mjs';

const tickets = '/admin/support/tickets';
const reports = '/admin/moderation/reports';
const sanctions = '/admin/moderation/enforcement';
const form = name => `form:has(input[name="action"][value="${name}"])`;
const digest = reason => createHash('sha256').update(reason).digest('hex');

async function staff(browser, info, roles) {
  const context = await otherBrowser(browser, info);
  try {
    const page = await context.newPage();
    const lang = info.project.metadata.locale;
    const account = await seedAccount(lang);
    const grants = await query(
      'INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=ANY($2) RETURNING role_id',
      [account.id, roles],
    );
    expect(grants).toHaveLength(roles.length);
    await login(page, lang, account);
    await enrollMfa(page, lang, account);
    const code = page.locator('textarea[readonly]').first();
    await expect(code).toBeVisible();
    await loginWithRecoveryCode(page, lang, account, await code.inputValue());
    return { context, page, id: account.id };
  } catch (error) {
    await context.close();
    throw error;
  }
}

async function visit(page, path, lang, visible) {
  const response = await page.goto(path);
  expect(response.status(), path).toBe(200);
  expect(response.headers()['cache-control']).toContain('no-store');
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  if (visible) await expect(page.locator('main')).toContainText(visible);
  await fitsViewport(page, path);
}

async function fitsViewport(page, path) {
  const widths = await page.locator('body').evaluate(body => ({
    content: body.scrollWidth, viewport: document.documentElement.clientWidth,
  }));
  expect(widths.content, `${path} fits the viewport`).toBeLessThanOrEqual(widths.viewport);
}

async function mutate(page, name, fields, destination) {
  const selector = form(name);
  const target = await page.locator(selector).getAttribute('action');
  const [response] = await Promise.all([
    page.waitForResponse(response =>
      new URL(response.url()).pathname === target &&
      response.request().method() === 'POST' &&
      response.request().isNavigationRequest()),
    action(page, selector, fields),
  ]);
  expect(response.status(), name).toBe(303);
  expect(response.headers().location).toBe(destination || target);
  await expect(page).toHaveURL(new URL(destination || target, response.url()).href);
}

async function refused(page, name, fields, status, path) {
  const response = await postForm(page, form(name), fields, { action: path });
  expect(response.status(), name).toBe(status);
  expect(response.headers()['cache-control']).toContain('no-store');
}

async function audit(id, action, actor, reason, before, after) {
  const rows = await query(
    'SELECT actor,before,after FROM audit_log WHERE target=$1 AND action=$2 ORDER BY id',
    [id, action],
  );
  expect(rows.filter(row => row.after.reason_sha256 === digest(reason))).toEqual([
    { actor, before, after: { ...after, reason_sha256: digest(reason) } },
  ]);
  expect(JSON.stringify(rows)).not.toContain(reason);
}

function reportAuditState(status, lockVersion, {
  assignee = null, sanction = null, outcome = null, notes = null,
} = {}) {
  return {
    status,
    lock_version: lockVersion,
    assigned_to: assignee,
    sanction_id: sanction,
    public_outcome_sha256: outcome === null ? null : digest(outcome),
    moderator_notes_sha256: notes === null ? null : digest(notes),
  };
}

async function submitReport(page, lang, target, evidence) {
  await visit(page, `/${lang}/support/reports/create`, lang);
  await submit(page, 'main form', { target_reference: target, evidence_summary: evidence });
  await expect(page).toHaveURL(/support\/reports\/[a-f0-9-]+$/);
  const path = new URL(page.url()).pathname;
  return { path, id: path.split('/').at(-1) };
}

test('ticket queue and moderation preserve owner privacy and refuse invalid writes', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const pl = lang === 'pl';
  const owner = await seedAccount(lang);
  await login(page, lang, owner);
  const subject = unique().repeat(4);
  await visit(page, `/${lang}/support/tickets/create`, lang);
  await submit(page, 'main form', { subject, body: 'Question about my account' });
  await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
  const playerPath = new URL(page.url()).pathname;
  const id = playerPath.split('/').at(-1);
  const staffPath = `${tickets}/${id}`;
  const team = await staff(browser, info, ['support']);
  const outsider = await otherBrowser(browser, info);
  try {
    const outsiderPage = await outsider.newPage();
    const outsiderAccount = await seedAccount(lang);
    await login(outsiderPage, lang, outsiderAccount);
    expect((await outsiderPage.goto(playerPath)).status()).toBe(404);
    expect((await outsiderPage.goto(staffPath)).status()).toBe(403);
    const ownerStatusForm = 'form[action$="/status"]';
    await visit(outsiderPage, `/${lang}/support/tickets/create`, lang);
    await submit(outsiderPage, 'main form', { subject: unique(), body: 'Separate private ticket' });
    await expect(outsiderPage).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
    const foreignWrite = await postForm(outsiderPage, ownerStatusForm, { status: 'closed' }, {
      action: `${playerPath}/status`,
    });
    expect(foreignWrite.status()).toBe(404);
    const noStaff = await outsiderPage.request.post(staffPath, {
      form: {
        action: 'ticket_status', status: 'closed', reason: unique(), lock_version: '1',
        _csrf: await outsiderPage.locator(`${ownerStatusForm} [name=_csrf]`).inputValue(),
      },
      headers: { origin: new URL(outsiderPage.url()).origin },
      maxRedirects: 0,
    });
    expect(noStaff.status()).toBe(403);

    for (const invalid of ['page=bad', 'status=open&status=closed']) {
      const response = await team.page.goto(`${tickets}?${invalid}`);
      expect(response.status()).toBe(400);
      await expect(team.page.locator('html')).toHaveAttribute('lang', lang);
      const message = pl
        ? 'Sprawdź numer strony i filtry spraw.'
        : 'Check the page number and ticket filters.';
      await expect(team.page.locator('main').getByText(message, { exact: true })).toBeVisible();
    }
    await visit(team.page, tickets, lang, pl ? 'Kolejka spraw pomocy' : 'Ticket queue');
    const filters = team.page.locator('form[method=get]');
    await filters.locator('[name=status]').selectOption('closed');
    await filters.getByRole('button').click();
    await expect(team.page).toHaveURL(/status=closed/);
    await expect(team.page.locator(`a[href="${staffPath}"]`)).toHaveCount(0);
    await filters.locator('[name=status]').selectOption('open');
    await filters.locator('[name=category]').selectOption('account');
    await filters.getByRole('button').click();
    await expect(team.page).toHaveURL(/status=open&category=account/);
    const ticket = team.page.getByRole('link', { name: subject, exact: true });
    await expect(ticket).toHaveAttribute('href', staffPath);
    await ticket.click();
    await expect(team.page.getByRole('heading', { name: subject, exact: true })).toBeVisible();
    await fitsViewport(team.page, staffPath);

    const assignReason = unique();
    await refused(team.page, 'assign_ticket', { assignee: 'bad-id', reason: assignReason }, 422);
    await refused(team.page, 'assign_ticket', { assignee: outsiderAccount.id, reason: assignReason }, 403);
    await refused(team.page, 'assign_ticket', { assignee: '', reason: '' }, 422);
    await mutate(team.page, 'assign_ticket', { assignee: '', reason: assignReason });
    await expect(team.page.locator('main')).toContainText(team.id);
    expect(await query('SELECT assigned_to FROM tickets WHERE id=$1', [id]))
      .toEqual([{ assigned_to: team.id }]);
    await refused(team.page, 'assign_ticket', { assignee: team.id, reason: assignReason }, 409);
    await audit(id, 'support.ticket_assigned', team.id, assignReason,
      { assigned_to: null, lock_version: 1 }, { assigned_to: team.id, lock_version: 2 });

    const reply = unique();
    const replyReason = unique();
    for (const [fields, status] of [
      [{ body: ' ', reason: replyReason }, 422],
      [{ body: 'x'.repeat(8001), reason: replyReason }, 422],
      [{ body: reply, reason: ' ' }, 422],
      [{ body: reply, reason: 'x'.repeat(501) }, 422],
      [{ body: reply, reason: replyReason, _csrf: 'invalid' }, 403],
    ]) {
      await refused(team.page, 'reply', fields, status);
    }
    await refused(team.page, 'reply', { body: reply, reason: replyReason }, 400, `${tickets}/invalid-id`);
    const stale = await team.page.locator(`${form('reply')} [name=lock_version]`).inputValue();
    await mutate(team.page, 'reply', { body: reply, reason: replyReason });
    await refused(team.page, 'reply', { body: 'stale reply', reason: replyReason, lock_version: stale }, 409);

    const privateNote = unique();
    const noteReason = unique();
    await team.page.locator(`${form('reply')} [name=internal]`).check();
    await mutate(team.page, 'reply', { body: privateNote, reason: noteReason });
    await expect(team.page.locator('main')).toContainText(privateNote);
    await visit(page, playerPath, lang, reply);
    await expect(page.getByRole('heading', { name: subject, exact: true })).toBeVisible();
    await expect(page.locator('main')).not.toContainText(privateNote);
    await expect(page.locator('main')).not.toContainText(team.id);

    const resolvedReason = unique();
    await mutate(team.page, 'ticket_status', { status: 'resolved', reason: resolvedReason });
    await visit(page, playerPath, lang, pl ? 'Rozwiązane' : 'Resolved');
    const closeReason = unique();
    await mutate(team.page, 'ticket_status', { status: 'closed', reason: closeReason });
    await visit(page, playerPath, lang, pl ? 'Zamknięte' : 'Closed');
    await expect(team.page.locator(form('reply'))).toHaveCount(0);
    const current = await team.page.locator(`${form('ticket_status')} [name=lock_version]`).inputValue();
    const token = await team.page.locator(`${form('ticket_status')} [name=_csrf]`).inputValue();
    const closedReply = await team.page.request.post(staffPath, {
      form: { action: 'reply', body: reply, reason: replyReason, lock_version: current, _csrf: token },
      headers: { origin: new URL(team.page.url()).origin },
      maxRedirects: 0,
    });
    expect(closedReply.status()).toBe(409);
    const reopenReason = unique();
    await mutate(team.page, 'ticket_status', { status: 'open', reason: reopenReason });
    await visit(page, playerPath, lang, pl ? 'Otwarte' : 'Open');

    const [record] = await query('SELECT status,lock_version::int FROM tickets WHERE id=$1', [id]);
    expect(record).toEqual({ status: 'open', lock_version: 7 });
    const messages = await query(
      'SELECT id,internal,body FROM ticket_messages WHERE ticket_id=$1 AND author_account_id=$2',
      [id, team.id],
    );
    expect(messages).toHaveLength(2);
    const publicMessage = messages.find(message => message.body === reply);
    const internalMessage = messages.find(message => message.body === privateNote);
    expect(publicMessage).toMatchObject({ internal: false });
    expect(internalMessage).toMatchObject({ internal: true });
    await audit(id, 'support.ticket_replied', team.id, replyReason,
      { status: 'open', lock_version: 2 },
      { status: 'waiting_user', lock_version: 3, message_id: publicMessage.id });
    await audit(id, 'support.ticket_internal_note_added', team.id, noteReason,
      { status: 'waiting_user', lock_version: 3 },
      { status: 'waiting_user', lock_version: 4, message_id: internalMessage.id });
    await audit(id, 'support.ticket_status_changed', team.id, resolvedReason,
      { status: 'waiting_user', lock_version: 4 }, { status: 'resolved', lock_version: 5 });
    await audit(id, 'support.ticket_status_changed', team.id, closeReason,
      { status: 'resolved', lock_version: 5 }, { status: 'closed', lock_version: 6 });
    await audit(id, 'support.ticket_status_changed', team.id, reopenReason,
      { status: 'closed', lock_version: 6 }, { status: 'open', lock_version: 7 });
  } finally {
    await outsider.close();
    await team.context.close();
  }
});

test('moderation triages private reports and applies or revokes a bounded Platform sanction', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const pl = lang === 'pl';
  const reporter = await seedAccount(lang);
  const target = await seedAccount(lang);
  await login(page, lang, reporter);
  const ticketTeam = await staff(browser, info, ['support']);
  const moderator = await staff(browser, info, ['moderator']);
  const dualRole = await staff(browser, info, ['support', 'moderator']);
  try {
    const evidence = unique();
    const report = await submitReport(page, lang, target.id, evidence);
    const reportPath = `${reports}/${report.id}`;
    expect((await moderator.page.goto(reportPath)).status()).toBe(403);
    expect((await ticketTeam.page.goto(`${sanctions}/create`)).status()).toBe(403);
    await visit(ticketTeam.page, reportPath, lang, evidence);
    const dismiss = { status: 'rejected', public_outcome: unique(), moderator_notes: unique() };
    await refused(ticketTeam.page, 'moderate', { ...dismiss, moderator_notes: ' ' }, 422);
    await refused(ticketTeam.page, 'moderate', { ...dismiss, public_outcome: 'x'.repeat(4001) }, 422);
    await refused(ticketTeam.page, 'moderate', { ...dismiss, _csrf: 'invalid' }, 403);
    const escalated = unique();
    await mutate(ticketTeam.page, 'moderate', { status: 'reviewing', moderator_notes: escalated });
    await visit(page, report.path, lang, pl ? 'W trakcie weryfikacji' : 'Under review');
    await refused(ticketTeam.page, 'moderate', { ...dismiss, lock_version: '1' }, 409);
    await mutate(ticketTeam.page, 'moderate', dismiss);
    await visit(page, report.path, lang, dismiss.public_outcome);
    await expect(page.locator('main')).not.toContainText(dismiss.moderator_notes);
    await audit(report.id, 'support.report_status_changed', ticketTeam.id, escalated,
      reportAuditState('submitted', 1), reportAuditState('reviewing', 2, {
        assignee: ticketTeam.id, notes: escalated,
      }));
    await audit(report.id, 'support.report_status_changed', ticketTeam.id, dismiss.moderator_notes,
      reportAuditState('reviewing', 2, { assignee: ticketTeam.id, notes: escalated }),
      reportAuditState('rejected', 3, {
        assignee: ticketTeam.id, outcome: dismiss.public_outcome, notes: dismiss.moderator_notes,
      }));

    await visit(moderator.page, `${sanctions}/create`, lang, pl ? 'Kary platformy' : 'Platform sanctions');
    const applied = { target: target.id, category: 'restriction', duration_hours: '3',
      public_reason: unique(), moderator_notes: unique() };
    const sanctionPath = await moderator.page.locator(form('apply_sanction')).getAttribute('action');
    const sanction = sanctionPath.split('/').at(-1);
    for (const [fields, status] of [
      [{ target: 'bad-id' }, 422],
      [{ duration_hours: '0' }, 422],
      [{ duration_hours: '8761' }, 422],
      [{ public_reason: ' ' }, 422],
      [{ public_reason: 'x'.repeat(4001) }, 422],
      [{ _csrf: 'invalid' }, 403],
    ]) {
      await refused(moderator.page, 'apply_sanction', { ...applied, ...fields }, status);
    }
    await mutate(moderator.page, 'apply_sanction', applied, sanctionPath);
    const [created] = await query(
      'SELECT account_id,category,status,public_reason,extract(epoch FROM expires_at-effective_at)::int AS seconds FROM sanctions WHERE id=$1',
      [sanction],
    );
    expect(created).toEqual({ account_id: target.id, category: 'restriction', status: 'active',
      public_reason: applied.public_reason, seconds: 10800 });
    const [createdAudit] = await query(
      "SELECT actor,before,after FROM audit_log WHERE target=$1 AND action='support.enforcement_created'", [sanction],
    );
    expect(createdAudit).toMatchObject({ actor: moderator.id, before: null,
      after: { status: 'active', lock_version: 1, reason_sha256: digest(applied.public_reason) } });
    expect(JSON.stringify(createdAudit)).not.toContain(applied.moderator_notes);

    const linkedReport = await submitReport(page, lang, target.id, unique());
    const linkedPath = `${reports}/${linkedReport.id}`;
    await visit(ticketTeam.page, linkedPath, lang);
    const linking = { sanction, public_outcome: unique(), reason: unique() };
    await expect(ticketTeam.page.locator(form('link_sanction'))).toHaveCount(0);
    const token = await ticketTeam.page.locator(`${form('moderate')} [name=_csrf]`).inputValue();
    const unprivilegedLink = await ticketTeam.page.request.post(linkedPath, {
      form: { ...linking, action: 'link_sanction', lock_version: '1', _csrf: token },
      headers: { origin: new URL(ticketTeam.page.url()).origin },
      maxRedirects: 0,
    });
    expect(unprivilegedLink.status()).toBe(403);
    expect(await query('SELECT status,lock_version::int FROM player_reports WHERE id=$1', [linkedReport.id]))
      .toEqual([{ status: 'submitted', lock_version: 1 }]);
    await visit(dualRole.page, reportPath, lang);
    const closure = { status: 'closed', public_outcome: unique(), moderator_notes: unique() };
    await mutate(dualRole.page, 'moderate', closure);
    await audit(report.id, 'support.report_status_changed', dualRole.id, closure.moderator_notes,
      reportAuditState('rejected', 3, {
        assignee: ticketTeam.id, outcome: dismiss.public_outcome, notes: dismiss.moderator_notes,
      }),
      reportAuditState('closed', 4, {
        assignee: dualRole.id, outcome: closure.public_outcome, notes: closure.moderator_notes,
      }));
    const history = await query(
      "SELECT before,after FROM audit_log WHERE target=$1 AND action='support.report_status_changed'",
      [report.id],
    );
    for (const text of [dismiss.public_outcome, dismiss.moderator_notes, closure.public_outcome, closure.moderator_notes]) {
      expect(JSON.stringify(history)).not.toContain(text);
    }
    const unrelated = await submitReport(page, lang, reporter.id, unique());
    await visit(dualRole.page, `${reports}/${unrelated.id}`, lang);
    await refused(dualRole.page, 'link_sanction', linking, 422);
    await visit(dualRole.page, linkedPath, lang);
    await refused(dualRole.page, 'link_sanction', { ...linking, sanction: 'bad-id' }, 422);
    await refused(dualRole.page, 'link_sanction', { ...linking, reason: ' ' }, 422);
    await refused(dualRole.page, 'link_sanction', { ...linking, lock_version: '2' }, 409);
    await mutate(dualRole.page, 'link_sanction', linking);
    await expect(dualRole.page.locator('main')).toContainText(sanction);
    expect(await query('SELECT sanction_id FROM player_reports WHERE id=$1', [linkedReport.id]))
      .toEqual([{ sanction_id: sanction }]);
    await audit(linkedReport.id, 'support.report_sanction_linked', dualRole.id, linking.reason,
      reportAuditState('submitted', 1),
      reportAuditState('actioned', 2, { assignee: dualRole.id, sanction, outcome: linking.public_outcome }));
    await visit(page, linkedReport.path, lang, linking.public_outcome);
    await expect(page.locator('main')).not.toContainText(sanction);
    await expect(page.locator('main')).not.toContainText(linking.reason);

    await visit(moderator.page, sanctionPath, lang, applied.public_reason);
    const lifted = unique();
    await refused(moderator.page, 'revoke_sanction', { public_reason: lifted, lock_version: '2' }, 409);
    await refused(moderator.page, 'revoke_sanction', { public_reason: ' ' }, 422);
    await mutate(moderator.page, 'revoke_sanction', { public_reason: lifted });
    expect(await query('SELECT status,lock_version::int,public_reason FROM sanctions WHERE id=$1', [sanction])).toEqual([
      { status: 'revoked', lock_version: 2, public_reason: applied.public_reason },
    ]);
    const [liftAudit] = await query(
      "SELECT actor,before,after FROM audit_log WHERE target=$1 AND action='support.enforcement_updated'", [sanction],
    );
    expect(liftAudit).toMatchObject({ actor: moderator.id, before: { status: 'active', lock_version: 1 },
      after: { status: 'revoked', lock_version: 2, reason_sha256: digest(lifted) } });
    expect(JSON.stringify(liftAudit)).not.toContain(lifted);
    await expect(moderator.page.locator(form('revoke_sanction'))).toHaveCount(0);

    await login(page, lang, target);
    expect((await page.goto(report.path)).status()).toBe(404);
    expect((await page.goto(sanctionPath)).status()).toBe(403);
    const namedReport = await submitReport(page, lang, 'Unresolved Player', unique());
    await visit(dualRole.page, `${reports}/${namedReport.id}`, lang);
    await refused(dualRole.page, 'link_sanction', linking, 422);
    expect(await query(
      'SELECT status,lock_version::int,public_outcome FROM player_reports WHERE id=$1', [namedReport.id],
    )).toEqual([{ status: 'submitted', lock_version: 1, public_outcome: null }]);
    expect(await query(
      "SELECT id FROM audit_log WHERE target=$1 AND action='support.report_sanction_linked'", [namedReport.id],
    )).toEqual([]);
    expect((await query('SELECT operation_id FROM support_enforcement_commands WHERE account_id=$1', [target.id]))).toEqual([]);
  } finally {
    await dualRole.context.close();
    await moderator.context.close();
    await ticketTeam.context.close();
  }
});
