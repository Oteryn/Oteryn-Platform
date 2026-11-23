import {readFileSync} from 'node:fs';
import {test, expect} from '../fixtures.mjs';
import {assertShell} from '../shell.mjs';
import {query, seededAccount, seededLogin, seededStaff, unique} from '../staff-rbac-helpers.mjs';

const ticket = '/{lang}/support/tickets/{id}';
const staffTicket = '/{lang}/admin/support/tickets/{id}';
const definitions = [
  ['POST', '/{lang}/support/tickets', 'owner', 'ticket and initial message created'],
  ['POST', `${ticket}/reply`, 'owner', 'reply visible to support staff'],
  ['POST', `${staffTicket}/reply`, 'support staff', 'reply visible to ticket owner'],
  ['POST', `${ticket}/read`, 'owner', 'unread indicator cleared'],
  ['POST', `${ticket}/attachments`, 'owner', 'screenshot visible to support staff'],
  ['POST', `${staffTicket}/attachments`, 'support staff', 'screenshot visible to ticket owner'],
  ['POST', `${staffTicket}/status`, 'support staff', 'ticket resolved'],
  ['PUT', `${staffTicket}/status`, 'support staff', 'ticket open'],
  ['POST', `${ticket}/status`, 'owner', 'ticket closed'],
  ['PUT', `${ticket}/status`, 'owner', 'ticket reopened'],
].map(([method, route, role, result]) => ({
  method, route, role, result,
  flags: role === 'owner' ? ['OTERYN_SUPPORT_TICKETS'] : ['OTERYN_SUPPORT_TICKETS', 'OTERYN_ADMIN_PANEL'],
  alias: route.includes('{lang}') ? 'pl/en' : null,
  source: 'crates/support/src/tickets/web.rs',
}));

const keyOf = ({method, route, role}) => `${method} ${route} (${role})`;
const sampleEvidence = definitions.map(row => ({...row, status: 302}));
function validate(rows) {
  const expected = new Map(definitions.map(row => [keyOf(row), row]));
  const seen = new Set();
  for (const row of rows) {
    const key = keyOf(row);
    const definition = expected.get(key);
    if (!definition) throw new Error(`Unknown mutation journey: ${key}`);
    if (seen.has(key)) throw new Error(`Duplicate mutation journey: ${key}`);
    seen.add(key);
    if (row.status !== 302 || row.result !== definition.result) {
      throw new Error(`Mutation journey has no verified result: ${key}`);
    }
  }
  const missing = [...expected.keys()].filter(key => !seen.has(key));
  if (missing.length) throw new Error(`Missing mutation journeys: ${missing.join(', ')}`);
}

test('ticket mutation methods match their router registrations', () => {
  const source = readFileSync(new URL('../../crates/support/src/tickets/web.rs', import.meta.url), 'utf8');
  const router = source.slice(source.indexOf('pub fn router('), source.indexOf('/// Staff routes'));
  const registrations = [...router.matchAll(/\.route\(\s*"([^"]+)"\s*,\s*([^;]+?)\s*,?\s*\)\s*(?=\.(?:route|layer)|[,;])/g)]
    .flatMap(([, route, handlers]) => [...handlers.matchAll(/\b(post|put|delete|patch)\(/g)]
      .map(([, method]) => `${method.toUpperCase()} ${route}`));
  expect(definitions.map(({method, route}) => `${method} ${route}`).sort()).toEqual(registrations.sort());
});

test('route matrix requires a verified result for each mutation', () => {
  validate(sampleEvidence);
  expect(() => validate([{...sampleEvidence[0], status: 403}, ...sampleEvidence.slice(1)])).toThrow(/no verified result/);
  expect(() => validate([...sampleEvidence, sampleEvidence[0]])).toThrow(/Duplicate/);
  expect(() => validate([...sampleEvidence, {...sampleEvidence[0], route: '/unknown'}])).toThrow(/Unknown/);
  expect(() => validate(sampleEvidence.filter(row => row.method !== 'PUT'))).toThrow(/Missing/);
});

function routeFor(route, lang, id) {
  return route.replace('{lang}', lang).replace('{id}', id);
}

async function postForm(page, method, target, values = {}, upload) {
  const form = page.locator(`main form[action="${target}"]`);
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  return page.request.fetch(target, {
    method,
    ...(upload ? {multipart: {...fields, ...values, image: upload}} : {form: {...fields, ...values}}),
    headers: {origin: new URL(page.url()).origin},
    maxRedirects: 0,
  });
}

async function ticketState(id) {
  const [row] = await query('SELECT status, lock_version, unread_reply FROM tickets WHERE id = $1', [id]);
  return row;
}

async function open(page, url) {
  expect((await page.goto(url)).status(), url).toBe(200);
}

test('ticket owner and support staff complete the ticket mutation matrix', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const base = `/${lang}/support/tickets`;
  const owner = await seededAccount(lang);
  await seededLogin(page, lang, owner);

  const staffContext = await browser.newContext({
    baseURL: info.project.use.baseURL,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const staffPage = await staffContext.newPage();
    await seededStaff(staffPage, lang, 'support');
    const subject = unique();
    const evidence = [];
    let id;
    async function mutation(actorPage, method, route, values, verify, upload) {
      const role = actorPage === page ? 'owner' : 'support staff';
      const key = keyOf({method, route, role});
      const definition = definitions.find(row => keyOf(row) === key);
      expect(definition, key).toBeDefined();
      const target = routeFor(route, lang, id);
      const response = await postForm(actorPage, method, target, values, upload);
      expect(response.status(), key).toBe(302);
      await verify(response.headers().location);
      evidence.push({...definition, status: response.status()});
    }

    await open(page, base);
    await page.locator(`main a[href="${base}/create"]`).click();
    await expect(page).toHaveURL(url => url.pathname === `${base}/create`);
    await assertShell(page);
    await mutation(page, 'POST', '/{lang}/support/tickets', {subject, body: 'Please help with my account.'}, async location => {
      expect(location).toMatch(new RegExp(`^${base}/[a-f0-9-]{36}$`));
      id = location.split('/').at(-1);
      await open(page, location);
      await expect(page.locator('main')).toContainText(subject);
      await expect(page.locator('main')).toContainText('Please help with my account.');
      expect(await ticketState(id)).toMatchObject({status: 'open'});
    });
    const ownerPath = routeFor(ticket, lang, id);
    const staffPath = routeFor(staffTicket, lang, id);
    const playerReply = unique();
    const staffReply = unique();

    const beforeBadInput = await ticketState(id);
    const csrf = await page.locator('main form[action$="/reply"] input[name="_csrf"]').inputValue();
    const forbidden = await page.request.post(`${staffPath}/reply`, {
      form: {_csrf: csrf, body: 'Staff-only action', lock_version: beforeBadInput.lock_version},
      headers: {origin: new URL(page.url()).origin},
      maxRedirects: 0,
    });
    expect(forbidden.status()).toBe(403);
    expect(await ticketState(id)).toEqual(beforeBadInput);
    expect((await postForm(page, 'POST', `${ownerPath}/reply`, {body: ''})).status()).toBe(422);
    expect(await ticketState(id)).toEqual(beforeBadInput);

    await mutation(page, 'POST', `${ticket}/reply`, {body: playerReply}, async location => {
      expect(location).toBe(ownerPath);
      await open(staffPage, staffPath);
      await expect(staffPage.locator('main')).toContainText(playerReply);
      expect(await ticketState(id)).toMatchObject({status: 'waiting_staff'});
    });
    await mutation(staffPage, 'POST', `${staffTicket}/reply`, {body: staffReply}, async location => {
      expect(location).toBe(staffPath);
      expect(await ticketState(id)).toMatchObject({status: 'waiting_user', unread_reply: true});
      await open(page, ownerPath);
      await expect(page.locator('main')).toContainText(staffReply);
    });
    await mutation(page, 'POST', `${ticket}/read`, {}, async location => {
      expect(location).toBe(ownerPath);
      expect(await ticketState(id)).toMatchObject({unread_reply: false});
      await open(page, ownerPath);
      await expect(page.locator('main form[action$="/read"]')).toHaveCount(0);
    });

    const image = {
      name: 'ticket.png',
      mimeType: 'image/png',
      buffer: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAHklEQVR4nGNQzrP5T03MMGrgqIGjBo4aOGrgSDUQANpkzt/P0xIDAAAAAElFTkSuQmCC', 'base64'),
    };
    async function visibleAttachments(actorPage, target, count) {
      await open(actorPage, target);
      const links = actorPage.locator('main #screenshots a');
      await expect(links).toHaveCount(count);
      for (const link of await links.all()) {
        const response = await actorPage.request.get(await link.getAttribute('href'));
        expect(response.status()).toBe(200);
        expect(response.headers()['content-type']).toBe('image/png');
      }
    }
    await mutation(page, 'POST', `${ticket}/attachments`, {}, async location => {
      expect(location).toBe(ownerPath);
      await visibleAttachments(staffPage, staffPath, 1);
    }, image);
    await mutation(staffPage, 'POST', `${staffTicket}/attachments`, {}, async location => {
      expect(location).toBe(staffPath);
      await visibleAttachments(page, ownerPath, 2);
    }, image);

    async function transition(actorPage, method, route, status) {
      const ownPath = actorPage === page ? ownerPath : staffPath;
      await open(actorPage, ownPath);
      await mutation(actorPage, method, `${route}/status`, {status}, async location => {
        expect(location).toBe(ownPath);
        expect(await ticketState(id)).toMatchObject({status});
      });
    }
    await transition(staffPage, 'POST', staffTicket, 'resolved');
    const resolved = await ticketState(id);
    await open(page, ownerPath);
    await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Rozwiązane' : 'Resolved');
    expect((await postForm(page, 'PUT', `${ownerPath}/status`, {status: 'open'})).status()).toBe(409);
    expect(await ticketState(id)).toEqual(resolved);
    await transition(staffPage, 'PUT', staffTicket, 'open');
    await transition(page, 'POST', ticket, 'closed');
    await transition(page, 'PUT', ticket, 'open');
    await open(page, ownerPath);
    await expect(page.locator('main h2')).toHaveText(subject);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

    validate(evidence);
    await info.attach('actor-journeys', {
      body: JSON.stringify(evidence),
      contentType: 'application/json',
    });
  } finally {
    await staffContext.close();
  }
});
