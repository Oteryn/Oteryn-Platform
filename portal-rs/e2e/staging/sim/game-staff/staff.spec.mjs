import { randomUUID } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import {
  baseURL, capture, expect, formFor, get, local, mailLink, mailReady, mailValue,
  password, ready, submit, submitPost, test, totp, withStagingLock,
} from './helpers.mjs';

// The operation grants below come from 20261105231403_adm-p2a_game_roles.sql.
// 20261122110000_game_staff_admin_access.sql adds admin.access to every role here.
const viewerGrants = [
  'game.ops.read', 'game.world.read', 'game.node.read', 'game.db.read',
  'game.analytics.read', 'game.audit.read',
];
const playerGrants = ['game.player.read'];
const roles = {
  'game-admin': viewerGrants,
  'game-analyst': ['game.analytics.read'],
  'game-viewer': viewerGrants,
  'game-support': [...viewerGrants, ...playerGrants],
  'game-operator': viewerGrants,
  'game-master-senior': [...viewerGrants, ...playerGrants],
  'game-release': [],
  'game-security': ['game.analytics.read', 'game.audit.read'],
  'game-dba': ['game.db.read'],
};

const ops = [
  { path: '/console/dashboard', grant: 'game.ops.read', title: ['Pulpit', 'Dashboard'] },
  { path: '/console/worlds', grant: 'game.world.read', title: ['Światy', 'Worlds'], detail: ['Szczegóły świata', 'World details'] },
  { path: '/console/nodes', grant: 'game.node.read', title: ['Węzły i hosty', 'Nodes and hosts'], detail: ['Szczegóły węzła', 'Node details'] },
  { path: '/console/database', grant: 'game.db.read', title: ['Baza danych', 'Database'] },
];
const analytics = [
  ['', 'characters_created_daily', 'Przegląd analityki administratora', 'Admin analytics overview'],
  ['/creatures', 'boss_kills_daily', 'Zgony stworzeń i bossów według świata', 'Creature and boss deaths by world'],
  ['/activity', 'world_activity_hourly', 'Aktywność godzinowa — dokładna liczba ONLINE', 'Hourly activity — exact ONLINE concurrency'],
  ['/levels', 'level_distribution_daily', 'Dzienny rozkład poziomów', 'Daily level distribution'],
  ['/health', 'player_deaths_daily', 'Stan źródła analitycznego', 'Analytics feed health'],
  ['/economy', null, 'Analityka gospodarki', 'Economy analytics'],
];
const protectedPages = [
  '/admin/roles', '/admin/players', '/admin/console/access', '/admin/console/approvals',
  '/admin/support/tickets', '/admin/moderation/reports', '/admin/moderation/enforcement',
  '/admin/moderation/enforcement/create', '/admin/news', '/admin/pages', '/admin/media',
  '/admin/wiki', '/admin/announcements', '/admin/events', '/admin/events/create',
  '/admin/commerce',
];
const controlPlayers = '/admin/console/players';
const controlAudit = '/admin/console/audit';

test.beforeEach(async ({ request }) => ready(request));

async function signIn(page, lang, email, pass) {
  const login = `/${lang}/login`;
  expect((await page.goto(login)).status(), 'Login page opens').toBe(200);
  expect(await submitPost(page, login, { email, password: pass }), 'Login form redirects').toBe(302);
  await page.waitForURL(url => [`/${lang}/`, `/${lang}/mfa/challenge`].includes(url.pathname));
}

async function openReset(page, lang, email, pass, info) {
  const forgot = `/${lang}/forgot-password`;
  expect((await page.goto(forgot)).status(), 'Password reset request page opens').toBe(200);
  const since = Date.now() - 2_000;
  expect(await submitPost(page, forgot, { email }), 'Reset request redirects').toBe(302);
  await expect(page.getByRole('status')).toHaveText(local(
    lang,
    'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
    'If an account exists for that email, a password reset link has been sent.',
  ));
  const link = await mailValue(page.request, email, [
    'Zresetuj hasło do Oteryn', 'Reset your Oteryn password',
  ], since, body => mailLink(body, /^\/(pl|en)\/reset-password\/[a-f0-9]+$/));
  test.skip(!link, 'No reset mail arrived within 60 seconds for the named staging staff account');
  // The mailer uses the account's saved language to build the reset page URL.
  const resetLang = new URL(link).pathname.split('/')[1];

  let phase = 'reset link';
  let status;
  try {
    status = (await page.goto(link, { waitUntil: 'domcontentloaded' })).status();
    expect(status, 'Reset link opens').toBe(200);
    const reset = `/${resetLang}/reset-password`;
    const fields = { email, password: pass, password_confirmation: pass };
    phase = 'mismatched password confirmation';
    status = await submitPost(page, reset, { ...fields, password_confirmation: `x${pass}` }, { retryPage: link });
    expect(status, 'Mismatched confirmation redirects').toBe(302);
    await expect(page.getByRole('alert')).toHaveText(local(
      resetLang,
      'Link jest nieprawidłowy lub wygasł albo hasła nie są identyczne. W razie potrzeby poproś o nowy link.',
      'This password reset link is invalid or expired, or the passwords do not match. Request a new link if needed.',
    ));
    phase = 'valid password submission';
    status = await submitPost(page, reset, fields, { retryPage: link });
    expect(status, 'Valid password reset redirects').toBe(302);
    phase = 'login redirect';
    await page.waitForURL(`${baseURL}/${resetLang}/login`);
  } catch (error) {
    if (status >= 500) await capture(page, info, 'staff-reset-unavailable');
    await page.goto(forgot);
    if (error.constructor.name.includes('Skip')) throw error;
    throw new Error(`Staff reset failed at ${phase} (HTTP ${status ?? 'response not available'})`);
  }
}

async function authenticateStaff(page, lang, role, info) {
  const email = `staff-${role}@example.invalid`;
  const pass = password();
  await openReset(page, lang, email, pass, info);
  await signIn(page, lang, email, pass);

  const challenge = `/${lang}/mfa/challenge`;
  const alreadyEnrolled = new URL(page.url()).pathname === challenge;
  let secret;
  let enrollmentStep;
  if (!alreadyEnrolled) {
    const mfa = `/${lang}/mfa`;
    const enroll = async () => {
      expect((await page.goto(mfa)).status(), 'MFA settings opens').toBe(200);
      expect(await submitPost(page, `${mfa}/enroll`, { password: pass },
        { retryPage: mfa }), 'Enrollment form returns authenticator setup').toBe(200);
      secret = await page.locator('main .op-prose').innerText();
      enrollmentStep = Math.floor(Date.now() / 30_000);
      return { password: pass, code: totp(secret, enrollmentStep) };
    };
    expect(await submitPost(page, `${mfa}/confirm`, await enroll(), { retryPage: enroll }),
      'Authenticator confirmation returns recovery codes').toBe(200);
    await expect(page.locator('main textarea[readonly]')).toHaveCount(8);
    await signIn(page, lang, email, pass);
    await expect(page).toHaveURL(baseURL + challenge);
  }

  expect(await submitPost(page, challenge, { code: 'invalid' }), 'Invalid MFA code redirects').toBe(302);
  await expect(page).toHaveURL(baseURL + challenge);
  await expect(page.getByRole('alert')).toBeVisible();
  let code;
  if (alreadyEnrolled) {
    const since = Date.now() - 2_000;
    expect(await submitPost(page, `/${lang}/mfa/email`, {}, { retryPage: challenge }),
      'Email fallback redirects to the verification form').toBe(302);
    code = await mailValue(page.request, email, local(lang,
      'Twój kod weryfikacyjny Oteryn', 'Your Oteryn verification code',
    ), since, body => body.match(/\b[0-9a-f]{64}\b/i)?.[0]);
    expect(Boolean(code), 'MFA fallback mail arrived for this seeded account').toBe(true);
  } else {
    const nextStep = (enrollmentStep + 1) * 30_000;
    await delay(Math.max(0, nextStep - Date.now()) + 250);
    code = totp(secret);
  }
  expect(await submitPost(page, challenge, { code }), 'Valid MFA verification redirects').toBe(302);
  await expect(page).toHaveURL(`${baseURL}/${lang}/`);
}

function query(path, fields) {
  return `${path}?${new URLSearchParams(fields)}`;
}

async function checkStatus(page, path, expected) {
  const response = await get(page.request, path);
  // No request or response headers are attached to findings.
  expect(response.status(), path).toBe(expected);
  return response;
}

async function pageHeading(page, lang, title) {
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(local(lang, ...title));
}

async function navPaths(page, selector = 'nav a[href]') {
  return page.locator(selector).evaluateAll(links => links
    .filter(link => link.origin === location.origin && /^\/(admin|console)(\/|$)/.test(link.pathname))
    .map(link => link.pathname));
}

async function assertOfflineOps(page, lang) {
  const source = page.getByText(local(lang, 'Agent gry', 'Game agent'), { exact: true });
  const gameSections = page.locator('main section').filter({ has: source });
  expect(await gameSections.count()).toBeGreaterThan(0);
  for (const section of await gameSections.all()) {
    await expect(section.locator('.op-badge')).toHaveText(`R0 · ${local(lang, 'Niedostępne', 'Unavailable')}`);
    await expect(section.locator('article')).toHaveCount(0);
  }
  await expect(page.locator('main form[method=post]')).toHaveCount(0);
}

async function opsPage(page, lang, entry, allGrants, navigation) {
  const { path, grant, title, detail } = entry;
  expect((await page.goto(query(path, { lang }))).status()).toBe(200);
  await pageHeading(page, lang, title);
  const visible = await navPaths(page, 'header nav.op-tabs a');
  navigation[path] = visible;
  expect.soft(visible, `operation navigation on ${path}`)
    .toEqual(ops.filter(candidate => allGrants.includes(candidate.grant)).map(candidate => candidate.path));
  await assertOfflineOps(page, lang);
  expect(await page.getByRole('main').getByText(local(lang, 'Metryki Platformy', 'Platform metrics'), { exact: true }).count()).toBe(0);

  const fields = { lang, agent: 'sim-missing-agent', target: detail ? 'sim-missing-target' : '' };
  const selected = query(path, fields);
  await checkStatus(page, query(path, { lang, agent: 'bad agent' }), 400);
  if (!allGrants.includes(grant)) {
    // The console landing shell is visible by address. Selecting an agent requires the operation grant.
    await checkStatus(page, selected, 403);
    return { page: 'unavailable', filter: 'refused' };
  }
  await submit(page.locator('main form[method=get]'), { agent: fields.agent, target: fields.target });
  await expect(page).toHaveURL(baseURL + selected);
  await assertOfflineOps(page, lang);
  if (detail) await expect(page.getByRole('heading', { level: 2 })).toHaveText(local(lang, ...detail));

  await page.locator('main a[href*="agent="]').click();
  await assertOfflineOps(page, lang);
  if (path === '/console/worlds') {
    const anotherTab = await page.context().newPage();
    try {
      expect((await anotherTab.goto(selected)).status()).toBe(200);
      await assertOfflineOps(anotherTab, lang);
    } finally {
      await anotherTab.close();
    }
  }
  const otherLang = local(lang, 'en', 'pl');
  await page.locator(`a[hreflang="${otherLang}"]`).click();
  await pageHeading(page, otherLang, title);
  await page.goBack();
  await assertOfflineOps(page, lang);
  return { page: 'unavailable', filter: 'unavailable' };
}

const unknownWorld = '01900000-0000-7000-8000-000000000001';
const range = now => {
  const day = Math.floor(now / 86_400_000);
  const minute = offset => new Date((day + offset) * 86_400_000).toISOString().slice(0, 16);
  return { from: minute(-3), to: minute(-2) };
};

async function assertOfflineAnalytics(page, lang, economy) {
  const unavailable = page.locator('[data-state=unavailable]');
  await expect(unavailable).toHaveCount(economy ? 5 : 1);
  await expect(unavailable.first()).toHaveText(economy
    ? 'unavailable: accepted resync baseline and fresh Game evidence required'
    : local(lang, 'Niedostępne — wymagana zaakceptowana baza synchronizacji i aktualne dane',
      'Unavailable — accepted resync baseline and fresh evidence required'));
  await expect(page.locator('main tbody tr')).toHaveCount(0);
}

async function analyticsPage(page, lang, entry, allowed, navigation) {
  const [suffix, family, pl, en] = entry;
  const path = '/admin/analytics' + suffix;
  if (!allowed) {
    await checkStatus(page, path, 403);
    await checkStatus(page, path + '/data', 403);
    return { page: 'refused' };
  }
  const response = await page.goto(path);
  expect(response.status()).toBe(200);
  await pageHeading(page, lang, [pl, en]);
  navigation[path] = await navPaths(page);
  expect.soft(navigation[path], `analytics navigation on ${path}`).toEqual(suffix === '/economy'
    ? ['/admin/analytics']
    : ['/admin', ...analytics.map(([item]) => '/admin/analytics' + item)]);
  await assertOfflineAnalytics(page, lang, suffix === '/economy');
  const fields = { world: unknownWorld, ...(family && { family }), ...range(Date.now()) };
  await submit(page.locator('main form[method=get]'), fields);
  await expect(page).toHaveURL(baseURL + query(path, fields));
  await assertOfflineAnalytics(page, lang, suffix === '/economy');
  await expect(page.locator('input[name=world]')).toHaveValue(unknownWorld);
  const filteredData = await checkStatus(page, query(path + '/data', fields), 200);
  const payload = await filteredData.json();
  expect(payload.state).toBe('unavailable');
  if (suffix === '/economy') {
    expect(payload.groups).toHaveLength(5);
    for (const group of payload.groups) {
      expect(group.state).toBe('unavailable');
      expect(group.cells).toEqual([]);
    }
  } else {
    expect(payload.cells).toEqual([]);
  }
  if (suffix === '/creatures') {
    await submit(page.locator('main form[method=get]'), { family: 'creature_kills_daily' });
    await expect(page.locator('select[name=family]')).toHaveValue('creature_kills_daily');
    await assertOfflineAnalytics(page, lang, false);
  }
  await checkStatus(page, query(path, { ...fields, world: 'invalid' }), 400);
  return { page: 'unavailable', filter: 'unavailable' };
}

async function readNav(page, info, path, navigation, permitted) {
  const visible = await navPaths(page);
  navigation[path] = visible;
  const refused = visible.filter(link => !permitted.includes(link));
  if (refused.length) await capture(page, info, path.split('/').at(-1) + '-navigation');
  expect.soft(refused, `navigation on ${path} should only offer accessible pages`).toEqual([]);
}

async function playerLookup(page, lang, allowed, navigation, info, permitted) {
  const path = query(controlPlayers, { lang });
  const response = await page.goto(path);
  expect(response.status()).toBe(allowed ? 200 : 403);
  if (!allowed) return { page: 'refused' };
  await pageHeading(page, lang, ['Gracze', 'Players']);
  await readNav(page, info, controlPlayers, navigation, permitted);
  await expect(page.locator('main form[method=post]')).toHaveCount(0);
  const character = `sim-${randomUUID()}`;
  await submit(page.locator('main form[method=get]'), { character });
  await expect(page).toHaveURL(baseURL + query(controlPlayers, { lang, character }));
  await expect(page.locator('main section li')).toHaveText(Array(3).fill(local(lang, 'Niedostępne', 'Unavailable')));
  expect((await page.reload()).status()).toBe(200);
  await expect(page.locator('main section li')).toHaveText(Array(3).fill(local(lang, 'Niedostępne', 'Unavailable')));
  expect((await page.goto(query(controlPlayers, { lang, character: 'invalid id' }))).status()).toBe(400);
  return { page: 'unavailable', filter: 'unavailable' };
}

async function auditPage(page, lang, allowed, navigation, info, permitted) {
  const path = query(controlAudit, { lang });
  const response = await page.goto(path);
  expect(response.status()).toBe(allowed ? 200 : 403);
  if (!allowed) return { page: 'refused' };
  await pageHeading(page, lang, ['Audyt', 'Audit']);
  await readNav(page, info, controlAudit, navigation, permitted);
  const fields = {
    request_id: randomUUID(), admin: randomUUID(),
    operation: 'world.list', target: 'sim-missing-world',
  };
  await submit(page.locator('main form[method=get]'), fields);
  await expect(page).toHaveURL(baseURL + query(controlAudit, { lang, ...fields }));
  await expect(page.locator('main section li')).toHaveText([local(lang, 'Niedostępne', 'Unavailable')]);
  await checkStatus(page, query(controlAudit, { lang, request_id: 'invalid-id' }), 400);
  return { page: 'unavailable', filter: 'unavailable' };
}

async function inspectRole(page, lang, role, grants, info) {
  const panel = await page.goto('/admin');
  expect(panel.status(), 'Staff home status after MFA').toBe(200);
  await pageHeading(page, lang, ['Panel zespołu', 'Staff panel']);
  await expect(page.locator('main form[method=post]')).toHaveCount(0);
  const matrix = { '/admin': { page: 'allowed' } };
  const navigation = { '/admin': await navPaths(page) };
  try {
    for (const path of protectedPages) {
      await checkStatus(page, path, 403);
      matrix[path] = { page: 'refused' };
    }
    for (const path of ['/admin/world', `/admin/${lang}/bazaar`]) {
      // Both routers are unmounted when their feature flags are disabled.
      const status = (await get(page.request, path)).status();
      expect([403, 404], path).toContain(status);
      matrix[path] = { page: status === 403 ? 'refused' : 'unavailable' };
    }
    for (const entry of ops) {
      matrix[entry.path] = await test.step(entry.path, () => opsPage(page, lang, entry, grants, navigation));
    }
    for (const entry of analytics) {
      const path = '/admin/analytics' + entry[0];
      matrix[path] = await test.step(path, () => analyticsPage(page, lang, entry,
        grants.includes('game.analytics.read'), navigation));
    }
    const lookup = grants.includes('game.player.read');
    const audit = grants.includes('game.audit.read');
    const permitted = [lookup && controlPlayers, audit && controlAudit].filter(Boolean);
    matrix[controlPlayers] = await playerLookup(page, lang, lookup, navigation, info, permitted);
    matrix[controlAudit] = await auditPage(page, lang, audit, navigation, info, permitted);
  } catch (error) {
    await capture(page, info, 'staff-page');
    throw error;
  } finally {
    await info.attach('role-page-matrix', {
      body: JSON.stringify({ role, matrix, navigation }, null, 2),
      contentType: 'application/json',
    });
  }
}

for (const [role, grants] of Object.entries(roles)) {
  test(`${role} staff navigation, pages and filters`, async ({ page }, info) => {
    const { lang } = info.project.metadata;
    await mailReady(page.request);
    // Keep password resets serialized across the two Playwright workers.
    await withStagingLock('staff', async () => {
      await authenticateStaffSafely(page, lang, role, info);
      const panel = await get(page.request, '/admin');
      test.skip(panel.status() === 404, 'Staff panel is unavailable on the staging host (HTTP 404)');
      try {
        await inspectRole(page, lang, role, grants, info);
      } catch (error) {
        // A reset of staging storage leaves the previously authenticated session anonymous.
        const response = await get(page.request, '/admin');
        if (panel.status() !== 200 || response.status() !== 303 || response.headers().location !== '/login') throw error;
        info.annotations.push({ type: 'staging redeploy', description: 'Lost the authenticated session mid-test. Ran the role once more.' });
        await ready(page.request);
        await page.context().clearCookies();
        await authenticateStaffSafely(page, lang, role, info);
        await inspectRole(page, lang, role, grants, info);
      }
    });
  });
}

async function authenticateStaffSafely(page, lang, role, info) {
  try {
    await authenticateStaff(page, lang, role, info);
  } catch (error) {
    // Leave enrollment and recovery-code documents before Playwright records failure context.
    await page.goto(`/${lang}/login`).catch(() => page.close());
    throw error;
  }
}
