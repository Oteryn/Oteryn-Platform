import {test, expect} from '../fixtures.mjs';
import {action, query, seededStaff, unique} from '../staff-rbac-helpers.mjs';

test('staff publish an incident and maintenance for players', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const t = (pl, en) => lang === 'pl' ? pl : en;
  const status = `/${lang}/status`;
  expect((await page.goto(status)).status()).toBe(200);
  const staff = await seededStaff(page, lang, 'platform_admin');
  const suffix = unique();
  const world = (await query('INSERT INTO world_status (id,name) VALUES (uuidv7(),$1) RETURNING id',
    [`Aster ${suffix}`]))[0].id;
  const title = t(`Incydent ${suffix}`, `Incident ${suffix}`);
  const message = t(`Prace ${suffix}`, `Maintenance ${suffix}`);
  const resolution = t('Usługa przywrócona & monitorowana', 'Service restored & monitored');
  const update = t('Trwa weryfikacja', 'Verification in progress');
  const admin = `/admin/status?lang=${lang}`;
  await page.goto('/admin');
  await page.locator('main a[href^="/admin/status"]').click();
  await expect(page).toHaveURL(new RegExp('/admin/status'));
  await page.goto(admin);
  const fitsViewport = async (screen) => {
    expect(await screen.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
  };
  await fitsViewport(page);
  const create = '#create-incident';
  const draft = {
    scope: world, severity: 'major',
    title_pl: `Incydent ${suffix}`, title_en: `Incident ${suffix}`,
    message_pl: 'Analizujemy <protokół> & dane.', message_en: 'Investigating <protocol> & data.',
  };
  const post = async (selector, values) => {
    const form = page.locator(selector);
    const body = await form.evaluate(node => Object.fromEntries(new FormData(node)));
    return page.request.post(await form.getAttribute('action'), {
      form: {...body, ...values}, headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
    });
  };
  expect((await post(create, {...draft, message_pl: ' '})).status()).toBe(422);
  await action(page, create, draft);
  await expect(page.locator('main')).toContainText(title);
  const id = new URL(page.url()).hash.slice('#incident-'.length);
  expect(id).toMatch(/^[a-f0-9-]{36}$/);
  const incident = `#incident-${id}`;

  const minute = offset => new Date(Date.now() + offset * 60_000).toISOString().slice(0, 16);
  await action(page, '#schedule-maintenance', {
    scope: world, starts_at: minute(60), ends_at: minute(120),
    message_pl: `Prace ${suffix}`, message_en: `Maintenance ${suffix}`,
  });
  await expect(page.locator('main')).toContainText(message);
  await action(page, `${incident} form[data-update]`, {
    message_pl: 'Trwa weryfikacja', message_en: 'Verification in progress',
  });

  const player = await browser.newContext({
    baseURL: new URL(page.url()).origin, ignoreHTTPSErrors: true, locale: lang,
    timezoneId: 'America/New_York', viewport: info.project.use.viewport,
  });
  try {
    const viewer = await player.newPage();
    await viewer.goto(`/${lang}/`);
    await expect(viewer.locator(`#portal-navigation a[href="${status}"]`)).toHaveCount(1);
    const banner = viewer.locator('#status-notices');
    await expect(banner).toContainText(title);
    await expect(banner).toContainText(message);
    await banner.getByRole('button').click();
    await expect(banner).toBeHidden();
    const reloadedNotices = viewer.waitForResponse(response =>
      new URL(response.url()).pathname === `${status}/notices`);
    await viewer.reload();
    expect((await reloadedNotices).status()).toBe(204);
    await expect(banner).toBeHidden();

    await viewer.goto(status);
    await expect(viewer.getByRole('link', {name: 'Atom'})).toBeVisible();
    const component = viewer.locator(`[data-component="${world}"]`);
    await expect(component).toContainText(`Aster ${suffix}`);
    await expect(component).toContainText(t('Poważna awaria', 'Major outage'));
    await expect(component).toContainText('%');
    const localTime = viewer.locator('#maintenance article').filter({hasText: message}).locator('time').first();
    await expect(localTime).not.toContainText('UTC');
    await expect(viewer.locator('#active-incidents')).toContainText(update);
    await fitsViewport(viewer);
    await viewer.getByRole('link', {name: title}).click();
    await expect(viewer.locator('main')).toContainText(t('Analizujemy <protokół> & dane.', 'Investigating <protocol> & data.'));
    await expect(viewer.locator('main')).toContainText(update);
    await fitsViewport(viewer);

    await action(page, `${incident} form[data-resolve]`, {
      message_pl: 'Usługa przywrócona & monitorowana', message_en: 'Service restored & monitored',
    });
    await viewer.goto(status);
    await expect(viewer.locator('#active-incidents')).not.toContainText(title);
    await expect(viewer.locator('#incident-history')).toContainText(title);
    await expect(viewer.locator(`[data-component="${world}"]`)).toContainText(t('Działa', 'Operational'));
    const feed = await viewer.request.get(`${status}/incidents.atom`);
    expect(feed.headers()['content-type']).toContain('application/atom+xml');
    const xml = await feed.text();
    const entry = await viewer.evaluate(({source, id}) => {
      const parsed = new DOMParser().parseFromString(source, 'application/xml');
      if (parsed.querySelector('parsererror')) throw new Error('Invalid Atom XML');
      const match = Array.from(parsed.querySelectorAll('entry')).find(entry =>
        entry.querySelector('id').textContent === `urn:uuid:${id}`);
      return match && {
        title: match.querySelector('title').textContent,
        message: match.querySelector('summary').textContent,
      };
    }, {source: xml, id});
    expect(entry).toEqual({title, message: resolution});
  } finally {
    await player.close();
  }

  const audit = await query("SELECT action FROM audit_log WHERE actor=$1 AND action LIKE 'status.%' ORDER BY created_at,id", [staff]);
  expect(audit.map(row => row.action)).toEqual([
    'status.incident.create', 'status.maintenance.schedule', 'status.incident.update', 'status.incident.resolve',
  ]);
  expect((await post(create, {...draft, _csrf: 'invalid'})).status()).toBe(403);
  await query("DELETE FROM account_roles WHERE account_id=$1 AND role_id=(SELECT id FROM roles WHERE key='platform_admin')", [staff]);
  expect((await post(create, draft)).status()).toBe(403);
});

test('i18n-parity for public status and its feed', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  await page.goto(`/${lang}/status`);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Status usług' : 'Service status');
  await expect(page.locator('#active-incidents h2')).toHaveText(lang === 'pl' ? 'Aktywne incydenty' : 'Active incidents');
  await expect(page.locator('#maintenance h2')).toHaveText(lang === 'pl' ? 'Prace techniczne' : 'Maintenance');
  const feed = await page.request.get(`/${lang}/status/incidents.atom`);
  expect(await feed.text()).toContain(`xml:lang="${lang}"`);
});
