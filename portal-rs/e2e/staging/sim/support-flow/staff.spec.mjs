import { mkdir, rmdir } from 'node:fs/promises';
import { setTimeout as pause } from 'node:timers/promises';
import { test, expect } from '@playwright/test';
import {
  adminForm, captureForm, checkMailbox, click, formFor, MailUnavailable,
  newestMail, onceAfterRedeploy, pathOf, post, ready, register, runId,
  secondContext, SharedBudget, staffSignIn, attachAccessRouting, visit,
} from './helpers.mjs';

const ticketQueue = '/admin/support/tickets';
const reportQueue = '/admin/moderation/reports';
const enforcement = '/admin/moderation/enforcement';
const copy = {
  en: {
    reset: 'Reset password', staffTitle: 'Support and moderation', internal: 'Internal staff note',
    unread: 'New reply', read: 'Mark reply as read', waiting: 'Waiting for you', closed: 'Closed',
    reportStatus: 'Action taken', active: 'Active', revoked: 'Revoked', mailSubject: 'Oteryn support',
  },
  pl: {
    reset: 'Zresetuj hasło', staffTitle: 'Support i moderacja', internal: 'Wewnętrzna notatka zespołu',
    unread: 'Nowa odpowiedź', read: 'Oznacz odpowiedź jako przeczytaną', waiting: 'Oczekuje na Ciebie', closed: 'Zamknięte',
    reportStatus: 'Podjęto działanie', active: 'Aktywna', revoked: 'Cofnięta', mailSubject: 'Pomoc Oteryn',
  },
};

async function withStaffAccounts(run) {
  // The projects share two seeded staff identities. Keep password resets from
  // invalidating a different project's session while it is using either role.
  const lock = '/tmp/oteryn-support-flow-staff.lock';
  const deadline = Date.now() + 240_000;
  for (;;) {
    try {
      await mkdir(lock);
      break;
    } catch (error) {
      if (error.code !== 'EEXIST' || Date.now() >= deadline) throw error;
      await pause(1500);
    }
  }
  try {
    return await run();
  } finally {
    await rmdir(lock);
  }
}

async function openQueueItem(page, queue, id) {
  await visit(page, queue);
  for (let count = 0; count < 10; count++) {
    const match = page.locator(`main a[href="${queue}/${id}"]`);
    if (await match.count()) {
      await match.click();
      await expect(page).toHaveURL(`${queue}/${id}`);
      return;
    }
    const next = page.locator(`main a[href^="${queue}?page="]`);
    if (!(await next.count())) break;
    await next.click();
  }
  throw new Error('New item is missing from the first ten pages of the staff queue.');
}

async function triage(player, support, language, run) {
  const text = copy[language];
  await visit(player, `/${language}/support/tickets/create`);
  const subject = `Simulation ${run} staff triage`;
  await click(formFor(player, '/support/tickets'), {
    category: 'technical', subject, body: `Simulation ${run} question`,
  });
  await expect(player).toHaveURL(/support\/tickets\/[a-f0-9-]{36}$/);
  const ticket = pathOf(player);
  const id = ticket.split('/').at(-1);
  await openQueueItem(support, ticketQueue, id);
  await expect(support.getByRole('heading', { level: 1 })).toHaveText(text.staffTitle);
  await expect(support.getByRole('heading', { name: subject, exact: true })).toBeVisible();

  const reply = adminForm(support, 'reply');
  const empty = await post(support, await captureForm(reply), { body: ' ' });
  expect(empty.status(), 'empty staff reply is refused').toBe(422);
  const note = `Simulation ${run} internal note`;
  await click(reply, { body: note, internal: true });
  const privateArticle = support.locator('main article').filter({ hasText: note });
  await expect(privateArticle.getByRole('heading')).toHaveText(text.internal);
  await visit(player, ticket);
  await expect(player.getByText(note, { exact: true })).toHaveCount(0);
  await expect(formFor(player, '/read')).toHaveCount(0);

  const message = `Simulation ${run} public response`;
  await click(reply, { body: message, internal: false });
  await expect(support.getByText(message, { exact: true })).toBeVisible();
  await visit(player, `/${language}/support/tickets`);
  const row = player.locator('main article').filter({ hasText: subject });
  await expect(row).toContainText(text.unread);
  await row.getByRole('link', { name: subject }).click();
  await expect(player.locator('main > p').first()).toContainText(`Status: ${text.waiting}`);
  await expect(player.getByText(message, { exact: true })).toBeVisible();
  await expect(player.getByText(note, { exact: true })).toHaveCount(0);
  await formFor(player, '/read').getByRole('button', { name: text.read }).click();
  await expect(formFor(player, '/read')).toHaveCount(0);
  await visit(player, `/${language}/support/tickets`);
  await expect(row).not.toContainText(text.unread);

  await click(adminForm(support, 'ticket_status'), { status: 'closed' });
  await visit(support, `${ticketQueue}/${id}`);
  await expect(support.locator('main > p').first()).toHaveText(text.closed);
  await visit(player, ticket);
  await expect(player.locator('main > p').first()).toContainText(`Status: ${text.closed}`);
  await expect(formFor(player, '/reply')).toHaveCount(0);
  await expect(player.getByText(note, { exact: true })).toHaveCount(0);
}

async function findPlayer(support, email) {
  await visit(support, '/admin/players');
  await click(support.locator('main form[method=get]'), { q: email });
  const account = support.locator('main ul a[href^="/admin/players/"]');
  await expect(account).toHaveCount(1);
  const id = await account.textContent();
  expect(id).toMatch(/^[a-f0-9-]{36}$/);
  return id;
}

async function reviewReport(player, support, language, target, run) {
  await visit(player, `/${language}/support/reports/create`);
  const evidence = `Simulation ${run} evidence`;
  await click(formFor(player, '/support/reports'), {
    report_type: 'player', category: 'conduct', target_reference: target, evidence_summary: evidence,
  });
  await expect(player).toHaveURL(/support\/reports\/[a-f0-9-]{36}$/);
  const report = pathOf(player);
  await openQueueItem(support, reportQueue, report.split('/').at(-1));
  await expect(support.getByText(evidence, { exact: true })).toBeVisible();
  const outcome = language === 'pl' ? `Zgłoszenie ${run} rozpatrzone` : `Report ${run} reviewed`;
  const note = `Simulation ${run} review note`;
  await click(adminForm(support, 'moderate'), { status: 'actioned', public_outcome: outcome, moderator_notes: note });
  await expect(support.getByText(note, { exact: true })).toBeVisible();
  await visit(player, report);
  await expect(player.locator('main article')).toContainText(copy[language].reportStatus);
  await expect(player.getByText(outcome, { exact: true })).toBeVisible();
  await expect(player.getByText(note, { exact: true })).toHaveCount(0);
}

async function sanctions(player, moderator, language, identity, target, run) {
  const text = copy[language];
  await visit(player, `/${language}/account/notifications`);
  const preferences = formFor(player, '/account/notifications');
  await click(preferences, { support_email: true, support_portal: true });
  await player.reload();
  await expect(preferences.locator('[name=support_email]')).toBeChecked();
  for (const category of ['warning', 'restriction', 'suspension']) {
    await visit(moderator, enforcement);
    await moderator.locator(`main a[href="${enforcement}/create"]`).click();
    const form = adminForm(moderator, 'sanction');
    const now = Math.floor(Date.now() / 1000);
    const reason = language === 'pl' ? `Decyzja ${run} (${category})` : `Decision ${run} (${category})`;
    const fields = {
      target, category, status: 'active', public_reason: reason, moderator_notes: `Internal ${run}`,
      effective_at: String(now - 1), expires_at: String(now + 3600),
    };
    if (category === 'warning') {
      const invalid = await post(moderator, await captureForm(form), { ...fields, expires_at: fields.effective_at });
      expect(invalid.status(), 'expiry must follow the effective time').toBe(422);
    }
    const createdAt = Date.now() - 2000;
    await click(form, fields);
    await expect(moderator).toHaveURL(/enforcement\/[a-f0-9-]{36}$/);
    const path = pathOf(moderator);
    await expect(moderator.getByRole('heading', { name: reason, exact: true })).toBeVisible();
    await expect(moderator.locator('main > p').first()).toHaveText(text.active);
    const created = await newestMail(player.request, identity.email, createdAt, { subject: text.mailSubject, includes: reason });
    expect(created.Text?.includes(reason), 'player receives the localized decision').toBe(true);

    // The decision form exists before an appeal is filed.
    if (category === 'warning') {
      const appeal = await post(moderator, await captureForm(adminForm(moderator, 'appeal')), {
        appeal_status: 'accepted', appeal_outcome: 'Attempt before submission',
      });
      expect(appeal.status(), 'no appeal can be decided before it is filed').toBe(409);
    }

    const liftedAt = Date.now() - 2000;
    const liftReason = language === 'pl' ? `Cofnięto ${run} (${category})` : `Revoked ${run} (${category})`;
    await click(form, { ...fields, status: 'revoked', public_reason: liftReason });
    await visit(moderator, path);
    await expect(moderator.locator('main > p').first()).toHaveText(text.revoked);
    const lifted = await newestMail(player.request, identity.email, liftedAt, { subject: text.mailSubject, includes: liftReason });
    expect(lifted.Text?.includes(liftReason), 'player receives the localized revocation').toBe(true);
  }
}

test.afterEach(async ({ context }) => {
  await context.unrouteAll({ behavior: 'wait' });
});

test('staff handles the support queue and moderation decisions', async ({ page, browser, request }, info) => {
  const language = info.project.metadata.language;
  await attachAccessRouting(page.context());
  await onceAfterRedeploy(info, page, async () => {
    await ready(request);
    try {
      await test.step('Open the public password reset form', async () => {
        await visit(page, `/${language}/forgot-password`);
        await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[language].reset);
        const form = formFor(page, '/forgot-password');
        await form.getByRole('button').click();
        await expect(form.locator('[name=email]:invalid')).toBeVisible();
      });
      await checkMailbox(request);
      await withStaffAccounts(async () => {
        const supportContext = await secondContext(browser, info);
        const moderatorContext = await secondContext(browser, info);
        try {
          const support = await supportContext.newPage();
          const moderator = await moderatorContext.newPage();
          const signIns = await Promise.allSettled([
            test.step('Open the support workspace', async () => {
              await staffSignIn(support, language, 'support', info);
              await visit(support, ticketQueue);
              await visit(support, enforcement, 403);
            }),
            test.step('Open the moderation workspace', async () => {
              await staffSignIn(moderator, language, 'moderator', info);
              await visit(moderator, enforcement);
              await visit(moderator, '/admin/roles', 403);
              await visit(moderator, ticketQueue, 403);
            }),
          ]);
          const failures = signIns.filter(result => result.status === 'rejected').map(result => result.reason);
          if (failures.length) {
            const defect = failures.find(error => !(error instanceof MailUnavailable || error instanceof SharedBudget));
            throw defect ?? failures[0];
          }
          const run = runId();
          const identity = await register(page, language, run, 1, info);
          await test.step('Triage a ticket, respond and confirm receipt', () => triage(page, support, language, run));
          const target = await findPlayer(support, identity.email);
          await test.step('Publish a report outcome', () => reviewReport(page, support, language, target, run));
          await test.step('Issue and revoke each temporary decision', () => sanctions(page, moderator, language, identity, target, run));
        } finally {
          await Promise.all([
            moderatorContext.unrouteAll({ behavior: 'wait' }),
            supportContext.unrouteAll({ behavior: 'wait' }),
          ]);
          await Promise.all([moderatorContext.close(), supportContext.close()]);
        }
      });
    } catch (error) {
      if (!(error instanceof MailUnavailable || error instanceof SharedBudget)) throw error;
      test.skip(true, error.message);
    }
  });
});
