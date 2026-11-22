import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect } from '@playwright/test';
import { csrfRejected, password } from './helpers.mjs';
import { query } from './staff-rbac-helpers.mjs';

function seed(entity, args) {
  const database = new URL(process.env.PORTAL_E2E_DATABASE_URL || process.env.OTERYN_DATABASE_URL);
  if (!['localhost', '127.0.0.1', '[::1]'].includes(database.hostname)
      || !database.pathname.startsWith('/portal_e2e_')) {
    throw new Error('Admin fixtures require a local portal_e2e_ database');
  }
  const target = process.env.CARGO_TARGET_DIR || fileURLToPath(new URL('../target', import.meta.url));
  const binary = process.env.PORTAL_E2E_BINARY || path.resolve(target, 'debug/oteryn-portal');
  const result = spawnSync(binary, ['seed', entity, JSON.stringify(args)], {
    encoding: 'utf8',
    env: { ...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: database.href },
  });
  if (result.status !== 0) throw new Error(`Seed ${entity} failed with exit code ${result.status}`);
  return JSON.parse(result.stdout);
}

export function seedAdminAccount(locale) {
  const account = { email: `admin-e2e-${randomUUID()}@example.invalid`, password: password() };
  return { ...account, id: seed('account', { ...account, locale }).accountId };
}

export function seedAdminMfa(account) {
  return seed('account-mfa', { accountId: account.id, password: account.password }).recoveryCodes;
}

export function adminContext(browser, info) {
  return browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
    locale: info.project.metadata.locale,
    viewport: info.project.use.viewport,
  });
}

export async function postAdmin(page, path, form) {
  return page.request.post(path, {
    form,
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
}

export async function roleAction(page, operation, fields, status) {
  // Empty HTTP errors can cause Chromium to commit an error document after
  // the response event. Keep those genuine form submissions in their own tab.
  const screen = status >= 400 ? await page.context().newPage() : page;
  try {
    if (screen !== page) await screen.goto(page.url());
    const button = screen.locator(`button[name="action"][value="${operation}"]`);
    const form = screen.locator('main form').filter({ has: button });
    for (const [name, value] of Object.entries(fields)) {
      const field = form.locator(`[name="${name}"]`);
      if (await field.evaluate(element => element.tagName === 'SELECT')) {
        await field.selectOption(value);
      } else {
        await field.fill(value);
      }
    }
    await csrfRejected(screen, '/admin/roles', { ...fields, action: operation });
    const [response] = await Promise.all([
      screen.waitForResponse(response =>
        response.request().method() === 'POST' && new URL(response.url()).pathname === '/admin/roles'),
      button.click(),
    ]);
    expect(response.status(), `${operation} submission`).toBe(status);
    if (status === 303) await expect(screen).toHaveURL(/\/admin\/roles$/);
  } finally {
    if (screen !== page) await screen.close();
  }
}

export async function accountRoles(account) {
  return (await query(
    'SELECT r.key FROM roles r JOIN account_roles ar ON ar.role_id=r.id WHERE ar.account_id=$1 ORDER BY r.key',
    [account],
  )).map(row => row.key);
}

export async function expectAuditEntry({ action, actor, target, before, after }) {
  const [event] = await query(
    'SELECT request_id, before, after FROM audit_log WHERE action=$1 AND actor=$2 AND target=$3 ORDER BY created_at DESC LIMIT 1',
    [action, actor, target],
  );
  expect(event, action).toMatchObject({ before, after });
  return event;
}

export async function roleAudit(page, lang, details) {
  const event = await expectAuditEntry(details);
  const { action, actor, target } = details;
  await page.goto(`/admin/console/audit?lang=${lang}`);
  const form = page.locator('main form[method=get]');
  for (const [name, value] of Object.entries({
    request_id: event.request_id, admin: actor, operation: action, target,
  })) {
    await form.locator(`[name="${name}"]`).fill(value);
  }
  const [response] = await Promise.all([page.waitForNavigation(), form.getByRole('button').click()]);
  expect(response.status()).toBe(200);
  for (const value of [event.request_id, actor, action, target]) {
    await expect(page.locator('main')).toContainText(value);
  }
}

// Defaults pinned from the built-in role catalog and its seeding migrations.
const viewer = `game.ops.read game.ops.logs.read game.world.read game.node.read game.db.read game.economy.read
  game.sanction.read game.analytics.read game.audit.read`;
const operator = `game.world.ops game.world.config game.node.ops game.db.backup game.ops.write`;
const gameSupport = `game.player.read game.player.moderate game.sanction.write`;

export const builtInPermissions = {
  platform_admin: `admin.analytics.view admin.analytics.export admin.access admin.roles.manage audit.view cms.news.manage
    cms.pages.manage media.manage portal.access portal.announcements.manage portal.settings.manage
    downloads.manage events.manage events.publish support.content.manage support.tickets.manage
    support.reports.manage support.enforcement.manage marketplace.manage products.premium.manage
    payments.reconcile game_catalog.access game_catalog.snapshots.view game_catalog.snapshots.import
    game_catalog.snapshots.activate game_catalog.profiles.manage game_catalog.translations.manage
    game_catalog.overrides.manage wiki.access wiki.articles.manage wiki.categories.manage wiki.publish
    account.security.kick payments.coin_hold.release store.catalog.import store.catalog.activate
    store.codes.generate`,
  security_admin: `admin.access admin.roles.manage audit.view`,
  content_editor: `cms.news.manage cms.pages.manage media.manage portal.announcements.manage events.manage events.publish
    support.content.manage wiki.access wiki.articles.manage wiki.categories.manage wiki.publish admin.access
    portal.access`,
  support: `admin.access support.tickets.manage support.reports.manage`,
  moderator: `admin.access support.enforcement.manage marketplace.manage account.security.kick`,
  operations: `admin.access downloads.manage portal.access portal.settings.manage game_catalog.access
    game_catalog.snapshots.view game_catalog.snapshots.import game_catalog.snapshots.activate
    game_catalog.profiles.manage game_catalog.translations.manage game_catalog.overrides.manage`,
  game_viewer: `admin.access ${viewer}`,
  game_support: `admin.access ${viewer} ${gameSupport}`,
  game_master_senior: `admin.access ${viewer} ${gameSupport} game.player.manage game.player.value.write game.sanction.ban game.world.events
    game.economy.moderate`,
  game_operator: `admin.access ${viewer} ${operator}`,
  game_admin: `admin.access ${viewer} ${operator} game.world.admin game.node.admin game.db.admin game.staff.admin`,
  game_dba: `admin.access game.db.read game.db.backup game.db.query game.analytics.sql`,
  game_analyst: `admin.access game.analytics.read game.analytics.player game.analytics.sql game.analytics.export`,
  game_security: `admin.access game.analytics.read game.analytics.player game.analytics.sql game.analytics.export game.analytics.relink
    game.audit.read game.sanction.read`,
  game_release: `admin.access game.node.deploy`,
};

export const permissionCatalog = [...new Set(
  Object.values(builtInPermissions).flatMap(value => value.split(/\s+/)),
)].sort();
