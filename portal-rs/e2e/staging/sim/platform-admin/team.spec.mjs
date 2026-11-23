import { randomBytes, randomUUID } from 'node:crypto';
import { mkdir, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import {
  checkMail, enroll, expect, form, founder, language, pathOf, register, signIn,
  stagingRequest, test, visit, waitReady,
} from './helpers.mjs';

const rolePath = '/admin/roles';
const roleOption = 'select[name=role] option';
const membershipForm = 'main form:has(input[name=account])';
const editForm = 'main form:has(#edit-role)';
const createForm = 'main form:has(input[name=key])';
const uuid = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/;
const consolePath = (area, lang, fields = {}) =>
  '/admin/console/' + area + '?' + new URLSearchParams({ lang, ...fields });

async function claimFounder(use) {
  const lock = join(tmpdir(), 'oteryn-platform-admin-' + process.env.OTERYN_SIM_RUN_ID);
  const deadline = Date.now() + 300_000;
  for (;;) {
    try {
      await mkdir(lock);
      break;
    } catch (error) {
      if (error.code !== 'EEXIST' || Date.now() >= deadline) throw error;
      await delay(2_000);
    }
  }
  try {
    await use();
  } finally {
    await rmdir(lock);
  }
}

async function roleByKey(page, key) {
  await visit(page, rolePath);
  for (let index = 0; index < 20; index++) {
    const option = page.locator(membershipForm + ' ' + roleOption)
      .filter({ hasText: new RegExp('^' + key + '$') });
    if (await option.count() === 1) {
      const id = await option.getAttribute('value');
      expect(id).toMatch(uuid);
      return id;
    }
    const next = page.locator('main a[href^="/admin/roles?after="]');
    if (await next.count() === 0) break;
    const href = await next.getAttribute('href');
    expect(new URL(href, page.url()).searchParams.get('after')).toMatch(uuid);
    await visit(page, href);
  }
  throw new Error('Expected role was not found on the first 20 pages');
}

async function accountId(page, email, lang) {
  await visit(page, '/admin/players');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(language(lang, 'Gracze', 'Players'));
  const search = page.locator('main form[method=get]');
  await search.locator('[name=q]').fill(email);
  await search.locator('button').click();
  const result = page.locator('main ul a[href^="/admin/players/"]');
  await expect(result).toHaveCount(1);
  const href = await result.getAttribute('href');
  const id = href.split('/').at(-1);
  expect(id).toMatch(uuid);
  await result.click();
  await expect(page.getByRole('heading', { name: id, exact: true })).toBeVisible();
  expect(pathOf(page)).toBe(href);
  return id;
}

async function changeRole(page, key, account, action, status) {
  const role = await roleByKey(page, key);
  const response = await form(page, membershipForm, { account, role }, 'button[value=' + action + ']');
  expect(response.status(), 'Role assignment response').toBe(status);
  if (status === 303) {
    expect(await response.headerValue('location')).toBe(rolePath);
    await expect(page.locator(createForm)).toBeVisible();
  }
}

async function audit(page, lang, admin, operation, target, count = 1) {
  await visit(page, consolePath('audit', lang));
  const filter = page.locator('main form[method=get]');
  for (const [name, value] of Object.entries({ admin, operation, target })) {
    await filter.locator('[name=' + name + ']').fill(value);
  }
  await filter.locator('button').click();
  const rows = page.locator('main li').filter({ hasText: admin + ' · ' + operation + ' · ' + target });
  await expect(rows).toHaveCount(count);
  if (count) {
    const requestId = (await rows.first().innerText()).split(' · ')[0];
    expect(requestId).toMatch(uuid);
    await visit(page, consolePath('audit', lang, { admin, operation, target, request_id: requestId }));
    await expect(rows).toHaveCount(1);
  }
}

async function navigateConsole(page, lang, player) {
  const labels = {
    players: language(lang, 'Gracze', 'Players'),
    audit: language(lang, 'Audyt', 'Audit'),
    access: language(lang, 'Dostęp', 'Access'),
    approvals: language(lang, 'Zatwierdzenia', 'Approvals'),
  };
  await visit(page, consolePath('access', lang));
  const areas = await page.locator('header nav a').evaluateAll(links =>
    links.map(link => new URL(link.href).pathname));
  expect(areas).toEqual(Object.keys(labels).map(area => '/admin/console/' + area));
  for (const [area, title] of Object.entries(labels)) {
    const link = page.locator('header nav').getByRole('link', { name: title, exact: true });
    await expect(link).toHaveAttribute('href', consolePath(area, lang));
    const [response] = await Promise.all([
      page.waitForNavigation({ waitUntil: 'domcontentloaded' }), link.click(),
    ]);
    expect(response?.status(), 'Console navigation').toBe(200);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(title);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await visit(page, consolePath(area, 'xx'), 400);
    await page.goBack({ waitUntil: 'domcontentloaded' });
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(title);
  }
  await expect(page.getByText(language(lang,
    'Tylko odczyt. Zatwierdzanie i odrzucanie są niedostępne do czasu integracji usługi zatwierdzeń.',
    'Read-only. Approve and reject are unavailable until the approval service is integrated.'))).toBeVisible();
  await visit(page, consolePath('players', lang));
  await expect(page.getByText(language(lang, 'Niedostępne', 'Unavailable'), { exact: true })).toHaveCount(3);
  await visit(page, consolePath('audit', lang, { admin: 'invalid' }), 400);
  await visit(page, '/admin/players/' + player);
  await visit(page, '/admin/players/' + randomUUID(), 404);
  await visit(page, '/admin/players?q=%20', 400);

  const other = lang === 'pl' ? 'en' : 'pl';
  await visit(page, consolePath('access', other));
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(language(other, 'Dostęp', 'Access'));
  await expect(page.locator('html')).toHaveAttribute('lang', other);
}

async function browseSupport(page, lang, ticket) {
  const paths = ['/admin/support/tickets', '/admin/moderation/reports',
    '/admin/moderation/enforcement'];
  for (const path of paths) {
    await visit(page, path + '?page=1');
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(language(lang,
      'Support i moderacja', 'Support and moderation'));
    const next = page.locator('main a[href="' + path + '?page=2"]');
    if (await next.count()) {
      await expect(next).toHaveText(language(lang, 'Następna strona', 'Next page'));
      await next.click();
      await expect.poll(() => new URL(page.url()).searchParams.get('page')).toBe('2');
      await page.goBack({ waitUntil: 'domcontentloaded' });
    } else {
      await visit(page, path + '?page=2');
    }
    await visit(page, path + '?page=0', 422);
    await visit(page, path + '/invalid', 400);
  }

  await visit(page, paths[0] + '/' + ticket.id);
  await expect(page.getByRole('heading', { name: ticket.subject, exact: true })).toBeVisible();
  await expect(page.locator('main article')).toContainText(ticket.body);
  await visit(page, paths[0] + '/' + ticket.id + '?after=invalid', 400);

  await visit(page, paths[2]);
  const create = page.locator('main a[href="/admin/moderation/enforcement/create"]');
  await expect(create).toBeVisible();
  await create.click();
  await expect(page.locator('main form:has(input[value=sanction])')).toBeVisible();
}

async function createTicket(page, lang) {
  await visit(page, '/' + lang + '/support/tickets/create');
  const subject = 'Account onboarding ' + randomBytes(8).toString('hex');
  const body = 'Question about signing in with a new authenticator.';
  const response = await form(page, 'main form:has(input[name=subject])', { subject, body });
  expect(response.status(), 'Colleague ticket created').toBe(302);
  const id = (await response.headerValue('location')).split('/').at(-1);
  expect(id).toMatch(uuid);
  await expect(page.getByRole('heading', { name: subject, exact: true })).toBeVisible();
  return { id, subject, body };
}

async function paginateRoles(page, lang) {
  await visit(page, '/admin');
  const roles = page.getByRole('link', { name: language(lang, 'Role', 'Roles'), exact: true });
  await expect(roles).toHaveAttribute('href', rolePath);
  await roles.click();
  const options = page.locator(membershipForm + ' ' + roleOption);
  await expect(options.first()).toBeAttached();
  const first = await options.first().getAttribute('value');
  const second = await options.nth(1).getAttribute('value');
  await visit(page, rolePath + '?after=' + first);
  await expect(options.first()).toHaveAttribute('value', second);
  await visit(page, rolePath + '?after=invalid', 400);
}

async function editRoles(page, lang, key, actor, target) {
  await visit(page, rolePath);
  await page.getByText(language(lang, 'Dostępne uprawnienia', 'Available permissions'), { exact: true }).click();
  await expect(page.locator('details code').getByText('audit.view', { exact: true })).toBeVisible();
  for (const values of [
    { key: 'Invalid key', permissions: 'admin.access' },
    { key, permissions: 'permission.unknown' },
    { key, permissions: 'admin.access admin.access' },
  ]) {
    await visit(page, rolePath);
    expect((await form(page, createForm, values)).status()).toBe(400);
  }
  await visit(page, rolePath);
  const values = { key, permissions: 'admin.access' };
  expect((await form(page, createForm, values)).status()).toBe(303);
  const role = await roleByKey(page, key);
  // A duplicate key reaches the storage error branch. The original role must survive.
  expect((await form(page, createForm, values)).status()).toBe(503);
  expect(await roleByKey(page, key)).toBe(role);
  const response = await form(page, editForm, { role, permissions: 'admin.access audit.view' });
  expect(response.status()).toBe(303);
  await visit(page, consolePath('access', lang));
  await expect(page.locator('main li').filter({ hasText: new RegExp('^' + key + ': ') }))
    .toHaveText([key + ': admin.access', key + ': audit.view']);
  await changeRole(page, key, actor, 'grant', 403);
  await changeRole(page, 'platform_admin', target, 'grant', 403);
  await roleByKey(page, key);
  const csrf = await page.locator(membershipForm + ' input[name=_csrf]').inputValue();
  const missing = await stagingRequest(page.request, rolePath, {
    method: 'POST',
    form: { _csrf: csrf, account: target, role: randomUUID(), action: 'grant' },
    headers: { origin: new URL(page.url()).origin },
  });
  expect(missing.status(), 'Unknown role response').toBe(400);
  await audit(page, lang, actor, 'rbac.role.permissions_changed', role);
  return role;
}

async function verifyPanels(page, lang, builtIn) {
  await visit(page, '/admin');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(language(lang, 'Panel zespołu', 'Staff panel'));
  await expect(page.locator('main nav a')).toHaveCount(0);
  for (const route of [rolePath, consolePath('access', lang), consolePath('approvals', lang)]) {
    await visit(page, route, 403);
  }
  await visit(page, consolePath('audit', lang), builtIn ? 403 : 200);
  if (!builtIn) await expect(page.getByRole('heading', { level: 1 })).toHaveText(language(lang, 'Audyt', 'Audit'));
  await visit(page, '/admin/players', builtIn ? 200 : 403);
  await visit(page, consolePath('players', lang), builtIn ? 200 : 403);
  await visit(page, '/admin/support/tickets', builtIn ? 200 : 403);
  await visit(page, '/admin/moderation/reports', builtIn ? 200 : 403);
  await visit(page, '/admin/moderation/enforcement', 403);
}

async function onboarding(page, teammate, lang, account, actor, target, key, builtIn) {
  await signIn(teammate, lang, account);
  const ticket = builtIn ? await createTicket(teammate, lang) : undefined;
  const oldTab = await teammate.context().newPage();
  await visit(oldTab, '/' + lang + '/account');
  await changeRole(page, key, target, 'grant', 303);
  try {
    if (!builtIn) await changeRole(page, key, target, 'grant', 303);
    const stale = await stagingRequest(oldTab.request, '/admin');
    expect(stale.status(), 'Old colleague session response').toBe(303);
    expect(await stale.headerValue('location')).toBe('/login');
    await signIn(teammate, lang, account);
    await visit(teammate, '/admin', 403);
    await expect(teammate.getByRole('heading', { level: 1 })).toHaveText(language(lang,
      'Wymagane uwierzytelnianie wieloskładnikowe', 'Multi-factor authentication required'));
    await visit(teammate, rolePath, 403);
    await expect(teammate.locator(editForm)).toHaveCount(0);
    await enroll(teammate, lang, account);
    await verifyPanels(teammate, lang, builtIn);
    if (ticket) {
      await visit(teammate, '/admin/support/tickets/' + ticket.id);
      await expect(teammate.getByRole('heading', { name: ticket.subject, exact: true })).toBeVisible();
    }
  } finally {
    await changeRole(page, key, target, 'revoke', 303);
    await oldTab.close();
  }
  const revoked = await stagingRequest(teammate.request, '/admin');
  expect(revoked.status(), 'Revoked colleague session response').toBe(303);
  expect(await revoked.headerValue('location')).toBe('/login');
  await signIn(teammate, lang, account);
  await visit(teammate, '/admin', 403);
  await audit(page, lang, actor, 'rbac.role.granted', target);
  await audit(page, lang, actor, 'rbac.role.revoked', target);
  return ticket;
}

async function teamJourney(page, actors, lang, request, admin) {
  const actor = await accountId(page, admin.email, lang);
  const auditorPage = await actors();
  const supportPage = await actors();
  const auditor = await register(auditorPage, request, lang);
  const support = await register(supportPage, request, lang);
  const auditorId = await accountId(page, auditor.email, lang);
  const supportId = await accountId(page, support.email, lang);
  const key = 'sim_audit_' + randomBytes(8).toString('hex');
  await visit(page, '/' + lang + '/account/sudo');
  const confirmation = await form(page, 'main form', { current_password: admin.password }, 'button',
    { sensitive: true });
  expect(confirmation.status(), 'Founder password confirmed').toBe(302);
  await paginateRoles(page, lang);
  await editRoles(page, lang, key, actor, auditorId);
  await onboarding(page, auditorPage, lang, auditor, actor, auditorId, key, false);
  const ticket = await onboarding(page, supportPage, lang, support, actor, supportId, 'support', true);
  await navigateConsole(page, lang, supportId);
  await browseSupport(page, lang, ticket);
  await visit(page, '/admin/commerce', 404);
}

test('team onboarding, permission matrix, audit and admin navigation', async ({ actors, request }, info) => {
  test.setTimeout(900_000);
  const { lang } = info.project.metadata;
  await checkMail(request);
  const panel = await stagingRequest(request, '/admin');
  test.skip(panel.status() === 404, 'Staging admin entry point HTTP 404');
  expect(panel.status(), 'Admin entry point prompts sign-in').toBe(303);
  await claimFounder(async () => {
    for (let attempt = 0; attempt < 2; attempt++) {
      const page = await actors();
      const admin = await founder(page, request, lang);
      try {
        await teamJourney(page, actors, lang, request, admin);
        return;
      } catch (error) {
        if (attempt) throw error;
        const session = await stagingRequest(page.request, '/admin');
        if (session.status() !== 303 || await session.headerValue('location') !== '/login') throw error;
        info.annotations.push({
          type: 'staging restart',
          description: 'The established founder session disappeared. Retrying with fresh browser accounts.',
        });
        await waitReady(request);
      }
    }
  });
});
