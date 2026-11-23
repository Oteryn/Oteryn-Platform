import {confirmStaffPassword} from '../staff-rbac-helpers.mjs';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash, createHmac, randomBytes, randomUUID } from 'node:crypto';
import { TOTP } from 'otpauth';
import { test, expect } from '../fixtures.mjs';
import { login, logout, mailLink, password, pool, seedPasswordRange, submit, waitForLoginBudget } from '../helpers.mjs';
import { action, enrollMfa, query, rolePage } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';
import { paidTopup } from '../staff-commerce-helpers.mjs';

import { prepare, sendForm, useFormMethod, rejected, wrongActor, invalid, signInWithRecoveryCode } from './matrix-helpers.mjs';
import './publisher-cases.mjs';
import './services-cases.mjs';
import './rename-cases.mjs';
import './party-cases.mjs';

const portal = fileURLToPath(new URL('../../', import.meta.url));

async function seededPlayer(page, lang, role) {
  const db = pool();
  await db.end(); // Apply the harness's disposable database guard before invoking seed.
  const binary = process.env.PORTAL_E2E_BINARY
    || resolve(process.env.CARGO_TARGET_DIR || resolve(portal, 'target'), 'debug/oteryn-portal');
  const account = { email: `matrix-${randomUUID()}@example.invalid`, password: password(), locale: lang };
  const { status } = spawnSync(binary, ['seed', 'account', JSON.stringify(account)], {
    cwd: portal,
    env: { ...process.env, OTERYN_DATABASE_URL: process.env.PORTAL_E2E_DATABASE_URL, OTERYN_ENV: 'testing' },
    stdio: 'ignore',
  });
  expect(status, 'seed CLI creates the disposable player').toBe(0);
  const [{ id }] = await query('SELECT id FROM accounts WHERE email=$1', [account.email]);
  if (role) {
    const assignments = await query(
      'INSERT INTO account_roles(account_id, role_id) SELECT $1, id FROM roles WHERE key=$2 RETURNING role_id',
      [id, role],
    );
    expect(assignments).toHaveLength(1);
  }
  await login(page, lang, account);
  await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
  if (role) {
    await enrollMfa(page, lang, account);
    const code = await page.locator('textarea[readonly]').first().inputValue();
    await signInWithRecoveryCode(page, lang, account, code);
  }
  return { ...account, id };
}

const formSelector = 'main form[method=post]';

async function foreignActors(browser, project, lang, contexts, role) {
  const actors = [];
  for (const staffRole of [undefined, role]) {
    const context = await browser.newContext({
      baseURL: project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
      viewport: project.use.viewport,
    });
    contexts.push(context);
    const screen = await context.newPage();
    const account = await seededPlayer(screen, lang, staffRole);
    expect((await screen.goto(`/${lang}/account/privacy`)).status()).toBe(200);
    const { fields } = await prepare(screen, formSelector);
    actors.push({ screen, account, csrf: fields._csrf });
  }
  return actors;
}


for (const method of ['POST', 'PUT']) {
  test(`account privacy preferences (${method})`, async ({ page, browser }, info) => {
    const lang = info.project.metadata.locale;
    const anonymous = await browser.newContext({
      baseURL: info.project.use.baseURL,
      ignoreHTTPSErrors: true,
      locale: lang,
      viewport: info.project.use.viewport,
    });
    try {
      const account = await seededPlayer(page, lang);
      expect((await page.goto(`/${lang}/account/privacy`)).status()).toBe(200);
      await page.locator('[name=locale]').selectOption(lang);
      const snapshot = () => query(
        'SELECT public_account_association, public_status_visible, locale FROM accounts WHERE id=$1',
        [account.id],
      );
      const baseline = await snapshot();
      const { url, fields } = await prepare(page, formSelector, {
        public_account_association: '1', public_status_visible: '1',
      });
      await rejected({ page, url, fields, snapshot, baseline, method });
      await wrongActor({ page: await anonymous.newPage(), lang, url, fields, status: 403, snapshot, baseline, method });
      await invalid({
        page, url, fields, snapshot, baseline, method,
        message: lang === 'pl'
          ? 'Sprawdź ustawienia prywatności i wybierz język polski lub angielski.'
          : 'Check the privacy settings and select Polish or English.',
        samples: [
          { name: 'empty locale', patch: { locale: '' } },
          { name: 'overlong locale', patch: { locale: 'en'.repeat(256) } },
          { name: 'wrong boolean type', patch: { public_status_visible: 'abc' } },
          { name: 'boolean beyond the upper boundary', patch: { public_status_visible: '2' } },
        ],
      });
      await useFormMethod(page, url, method, () => submit(page, formSelector, {
        public_account_association: true, public_status_visible: true,
      }));
      await expect(page.locator('[name=public_account_association]')).toBeChecked();
      await expect(page.locator('[name=public_status_visible]')).toBeChecked();
      expect(await snapshot()).toEqual([{
        public_account_association: true, public_status_visible: true, locale: lang,
      }]);
    } finally {
      await anonymous.close();
    }
  });
}

test('ticket creation and exact request-key replay', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const anonymous = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  const contexts = [anonymous];
  try {
    const owner = await seededPlayer(page, lang);
    expect((await page.goto(`/${lang}/support/tickets/create`)).status()).toBe(200);
    const subject = `Mutation matrix ${randomUUID()}`;
    const body = `Local support question for ${subject}`;
    const { url, fields } = await prepare(page, formSelector, { subject, body });
    const snapshot = () => query(
      `SELECT t.id, t.account_id, t.subject, m.body FROM tickets t
       JOIN ticket_messages m ON m.ticket_id=t.id WHERE t.request_key=$1`,
      [fields.request_key],
    );
    const baseline = await snapshot();
    expect(baseline).toEqual([]);
    await rejected({ page, url, fields, snapshot, baseline });
    await wrongActor({ page: await anonymous.newPage(), lang, url, fields, status: 403, snapshot, baseline });
    await invalid({
      page, url, fields, snapshot, baseline,
      message: lang === 'pl'
        ? 'Nie udało się wykonać żądania. Odśwież zgłoszenie i spróbuj ponownie.'
        : 'The request could not be completed. Reload the ticket and try again.',
      samples: [
        { name: 'empty subject', patch: { subject: '' } },
        { name: 'overlong body', patch: { body: 'x'.repeat(8001) } },
        { name: 'wrong request-key type', patch: { request_key: 'invalid' } },
        { name: 'subject beyond the 160-character limit', patch: { subject: 'x'.repeat(161) } },
      ],
    });
    await submit(page, formSelector, { subject, body });
    await expect(page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
    await expect(page.locator('main')).toContainText(subject);
    await expect(page.locator('main')).toContainText(body);
    const target = new URL(page.url()).pathname;
    const created = [{ id: target.split('/').at(-1), account_id: owner.id, subject, body }];
    expect(await snapshot()).toEqual(created);
    for (const { screen, account } of await foreignActors(browser, info.project, lang, contexts, 'content_editor')) {
      expect((await sendForm(screen, url, fields)).status()).toBe(403);
      expect((await screen.goto(target)).status()).toBe(404);
      expect(await query('SELECT id FROM tickets WHERE account_id=$1', [account.id])).toEqual([]);
      expect(await snapshot()).toEqual(created);
    }
    const replay = await sendForm(page, url, fields);
    expect(replay.status()).toBe(302);
    expect(replay.headers().location).toBe(target);
    expect(await snapshot()).toEqual(created);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

const unavailable = [
  {
    family: 'collection PvP and set purchases',
    target: lang => `/${lang}/collection-pvp`,
    status: 200,
    message: {
      pl: 'Walki kolekcji i zakupy są obecnie niedostępne.',
      en: 'Collection battles and purchases are currently unavailable.',
    },
    routes: ['/{lang}/collection-pvp'],
  },
  {
    family: 'collection decks and battle actions',
    target: lang => `/${lang}/collection/${uuid7()}`,
    status: 503,
    message: {
      pl: 'Kolekcja jest niedostępna. Wymagana jest weryfikacja aktualnego prywatnego postępu i właściciela postaci.',
      en: 'Collection is unavailable. Current private progress and ownership must be verified.',
    },
    routes: [
      '/{lang}/collection/{character}/decks', '/{lang}/collection/decks/{id}/battle',
      '/{lang}/collection/decks/{id}/delete', '/{lang}/collection/battles/{id}',
      '/{lang}/collection/battles/{id}/forfeit',
    ],
  },
  {
    family: 'season-pass buy and claim',
    target: lang => `/${lang}/pass`,
    status: 503,
    message: {
      pl: 'Przepustka sezonowa jest obecnie niedostępna.',
      en: 'The season pass is currently unavailable.',
    },
    routes: ['/{lang}/pass/{season}/buy', '/{lang}/pass/{season}/claim'],
  },
  {
    family: 'daily puzzle answer',
    target: lang => `/${lang}/puzzles`,
    status: 200,
    routes: ['/{lang}/puzzles/{id}/answer'],
    message: { pl: 'Dzisiejsze zagadki są niedostępne.', en: "Today's puzzles are unavailable." },
  },
];

test('unavailable Game-backed browser actions', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const player = await seededPlayer(page, lang);
  for (const surface of unavailable) {
    await test.step(`${surface.family} (${surface.routes.join(', ')})`, async () => {
      const response = await page.goto(surface.target(lang));
      expect(response.status()).toBe(surface.status);
      expect(response.headers()['cache-control']).toContain('no-store');
      await expect(page.locator('main')).toContainText(surface.message[lang]);
      await expect(page.locator(formSelector)).toHaveCount(0);
    });
  }
  const [seasonPass] = await query(`
    SELECT EXISTS (SELECT 1 FROM wallet_entries WHERE account_id = $1) AS charged,
           EXISTS (SELECT 1 FROM pass_progress WHERE account_id = $1 AND paid) AS purchased,
           EXISTS (SELECT 1 FROM pass_claims WHERE account_id = $1) AS claimed
  `, [player.id]);
  expect(seasonPass).toEqual({ charged: false, purchased: false, claimed: false });
});

const packageForm = 'main form[action$="/shop/coins"]';
const storeError = {
  pl: 'Sprawdź przesłane wartości.',
  en: 'Check the submitted values.',
};

test('test-provider top-up creates one checkout intent per operation', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const anonymous = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  const contexts = [anonymous];
  try {
    const owner = await seededPlayer(page, lang);
    expect((await page.goto(`/${lang}/shop/coins`)).status()).toBe(200);
    const { url, fields } = await prepare(page, packageForm);
    const snapshot = () => query(
      `SELECT t.payment_intent_id, t.coins, t.minor_units, t.currency, p.provider
       FROM store_web_topups t LEFT JOIN payment_intents p ON p.id=t.payment_intent_id
       WHERE t.operation_id=$1 OR t.account_id=$2 ORDER BY t.operation_id`,
      [fields.operation, owner.id],
    );
    const baseline = await snapshot();
    expect(baseline).toEqual([]);
    await rejected({ page, url, fields, snapshot, baseline });
    await wrongActor({ page: await anonymous.newPage(), lang, url, fields, status: 303, snapshot, baseline });
    await invalid({
      page, url, fields, snapshot, baseline, message: storeError[lang],
      samples: [
        { name: 'empty package', patch: { package: '' } },
        { name: 'overlong package', patch: { package: 'x'.repeat(512) } },
        { name: 'malformed operation', patch: { operation: '3' } },
        { name: 'operation UUID outside the v7 contract', patch: { operation: randomUUID() } },
      ],
    });
    await submit(page, packageForm);
    await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Bank testowy' : 'Test bank');
    const intent = new URL(page.url()).pathname.split('/').at(-1);
    await expect(page).toHaveURL(new RegExp(`/${lang}/shop/test-checkout/${intent}$`));
    const created = [{
      payment_intent_id: intent, coins: '1234', minor_units: '250', currency: 'PLN', provider: 'test',
    }];
    expect(await snapshot()).toEqual(created);
    for (const { screen, account, csrf } of await foreignActors(browser, info.project, lang, contexts, 'support')) {
      expect((await sendForm(screen, url, { ...fields, _csrf: csrf })).status()).toBe(409);
      expect(await snapshot()).toEqual(created);
      expect(await query('SELECT id FROM payment_intents WHERE account_id=$1', [account.id])).toEqual([]);
    }
    const replay = await sendForm(page, url, fields);
    expect(replay.status()).toBe(303);
    expect(replay.headers().location).toBe(`/${lang}/shop/test-checkout/${intent}`);
    expect(await snapshot()).toEqual(created);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

for (const action of ['pay', 'reject', 'cancel']) {
  test(`test-provider checkout decision ${action} settles once`, async ({ page, browser }, info) => {
    test.setTimeout(300_000);
    const lang = info.project.metadata.locale;
    const contexts = [];
    try {
      const actors = await foreignActors(browser, info.project, lang, contexts, 'support');
      const owner = await seededPlayer(page, lang);
      await page.goto(`/${lang}/shop/coins`);
      await submit(page, packageForm);
      const checkout = 'main form[action*="/shop/test-checkout/"]';
      const { url, fields } = await prepare(page, checkout, { action });
      const intent = new URL(url).pathname.split('/').at(-1);
      const snapshot = () => query(
        `SELECT i.state,
          (SELECT count(*)::int FROM payment_events WHERE intent_id=i.id) AS events,
          (SELECT count(*)::int FROM wallet_entries WHERE operation_id=i.id) AS credits,
          (SELECT coalesce(sum(available_delta), 0)::text FROM wallet_entries WHERE operation_id=i.id) AS coins
         FROM payment_intents i WHERE i.id=$1 AND i.account_id=$2 AND i.provider='test'`,
        [intent, owner.id],
      );
      const baseline = await snapshot();
      expect(baseline).toEqual([{ state: 'pending', events: 0, credits: 0, coins: '0' }]);
      await rejected({ page, url, fields, snapshot, baseline });
      for (const { screen, csrf } of actors) {
        expect((await sendForm(screen, url, { ...fields, _csrf: csrf })).status()).toBe(404);
        expect(await snapshot()).toEqual(baseline);
      }
      await invalid({
        page, url, fields, snapshot, baseline, message: storeError[lang],
        samples: [
          { name: 'empty decision', patch: { action: '' } },
          { name: 'overlong decision', patch: { action: 'x'.repeat(512) } },
          { name: 'numeric decision', patch: { action: '1' } },
          { name: 'extra field outside the decision contract', patch: { amount: '1' } },
        ],
      });
      await page.locator(checkout).locator(`button[value=${action}]`).click();
      await expect(page).toHaveURL(new RegExp(`/${lang}/shop/coins\\?checkout=${intent}$`));
      const paid = action === 'pay';
      const settled = [{ state: paid ? 'succeeded' : 'failed', events: 1, credits: paid ? 1 : 0, coins: paid ? '1234' : '0' }];
      await expect.poll(snapshot, { timeout: 30_000 }).toEqual(settled);
      await page.reload();
      const message = paid
        ? { pl: 'Płatność przyjęta, coiny zostały zaksięgowane.', en: 'Payment accepted; coins have been credited.' }
        : { pl: 'Płatność odrzucona lub anulowana. Nie przyznano coinów.', en: 'Payment rejected or cancelled. No coins were credited.' };
      await expect(page.locator('main')).toContainText(message[lang]);
      expect((await sendForm(page, url, fields)).status()).toBe(303);
      expect(await snapshot()).toEqual(settled);
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });
}

for (const statusMethod of ['POST', 'PUT']) {
  test(`ticket owner reply, close, and reopen enforce ownership and version (${statusMethod})`, async ({ page, browser }, info) => {
    test.setTimeout(300_000);
    const lang = info.project.metadata.locale;
    const contexts = [];
    try {
      const actors = await foreignActors(browser, info.project, lang, contexts, 'content_editor');
      const owner = await seededPlayer(page, lang);
      await page.goto(`/${lang}/support/tickets/create`);
      await submit(page, formSelector, { subject: 'Ticket mutation ownership', body: 'Original question' });
      await expect(page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
      const id = new URL(page.url()).pathname.split('/').at(-1);
      const snapshot = () => query(
        `SELECT t.status, t.lock_version, m.body FROM tickets t
         JOIN ticket_messages m ON m.ticket_id=t.id WHERE t.id=$1 AND t.account_id=$2
         ORDER BY m.id`,
        [id, owner.id],
      );
      const error = lang === 'pl'
        ? 'Nie udało się wykonać żądania. Odśwież zgłoszenie i spróbuj ponownie.'
        : 'The request could not be completed. Reload the ticket and try again.';
      const statusChange = (status, visible, lock_version) => ({
        action: 'status', fields: { status },
        samples: [
          { name: 'empty status', patch: { status: '' } },
          { name: 'overlong status', patch: { status: 'x'.repeat(512) } },
          { name: 'malformed lock version', patch: { lock_version: 'bad' } },
          { name: 'lock version below one', patch: { lock_version: '0' } },
        ],
        submit: async selector => {
          await page.locator(`${selector} select[name=status]`).selectOption(status);
          await submit(page, selector);
        },
        visible,
        effect: ['Original question', 'Owner follow-up'].map(body => ({
          status, lock_version, body,
        })),
      });
      const variants = [
        {
          action: 'reply', fields: { body: 'Owner follow-up' },
          samples: [
            { name: 'empty reply', patch: { body: '' } },
            { name: 'overlong reply', patch: { body: 'x'.repeat(8001) } },
            { name: 'malformed lock version', patch: { lock_version: 'bad' } },
            { name: 'lock version below one', patch: { lock_version: '0' } },
          ],
          submit: selector => submit(page, selector, { body: 'Owner follow-up' }),
          visible: 'Owner follow-up',
          effect: ['Original question', 'Owner follow-up'].map(body => ({
            status: 'waiting_staff', lock_version: '2', body,
          })),
        },
        statusChange('closed', lang === 'pl' ? 'Zamknięte' : 'Closed', '3'),
        statusChange('open', lang === 'pl' ? 'Otwarte' : 'Open', '4'),
      ];
      expect(await snapshot()).toEqual([{ status: 'open', lock_version: '1', body: 'Original question' }]);
      for (const variant of variants) {
        await test.step(variant.action, async () => {
          const baseline = await snapshot();
          const selector = `main form[action$="/${variant.action}"]`;
          const { url, fields } = await prepare(page, selector, variant.fields);
          const method = variant.action === 'status' ? statusMethod : 'POST';
          await rejected({ page, url, fields, snapshot, baseline, method });
          for (const { screen, csrf } of actors) {
            expect((await sendForm(screen, url, { ...fields, _csrf: csrf }, method)).status()).toBe(404);
            expect(await snapshot()).toEqual(baseline);
          }
          await invalid({ page, url, fields, samples: variant.samples, message: error, snapshot, baseline, method });
          await useFormMethod(page, url, method, () => variant.submit(selector));
          await expect(page.locator('main')).toContainText(variant.visible);
          expect(await snapshot()).toEqual(variant.effect);
          expect((await sendForm(page, url, fields, method)).status()).toBe(409);
          expect(await snapshot()).toEqual(variant.effect);
        });
      }
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });
}

test('character creation queues one operation and exposes the pending Game outcome', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const anonymous = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  const contexts = [anonymous];
  try {
    const { id } = await seededPlayer(page, lang);
    const surface = `/${lang}/account/characters/new`;
    expect((await page.goto(surface)).status()).toBe(200);
    const form = page.locator(formSelector);
    const world = form.locator('select[name=world] option[value]:not([value=""])').first();
    await form.locator('select[name=world]').selectOption(await world.getAttribute('value'));
    await form.locator('input[name=sex]').first().check();
    await form.locator('input[name=vocation]').first().check();
    const name = 'Matrix ' + randomUUID().replace(/[^a-f]/g, '').padEnd(10, 'a').slice(0, 10);
    const { url, fields } = await prepare(page, formSelector, { name });
    const snapshot = () => query(
      'SELECT id, state, receipt_json FROM character_commands WHERE account_id=$1 ORDER BY id',
      [id],
    );
    const baseline = await snapshot();
    expect(baseline).toEqual([]);
    await rejected({ page, url, fields, snapshot, baseline });
    await wrongActor({ page: await anonymous.newPage(), lang, url, fields, status: 403, snapshot, baseline });
    const error = lang === 'pl'
      ? 'Sprawdź reguły imion i wybierz prawidłowy świat, płeć oraz profesję przed ponownym wysłaniem.'
      : 'Check the name rules and choose a valid world, sex and vocation before submitting again.';
    const cases = [
      ['empty name', { name: '' }],
      ['overlong name', { name: 'A'.repeat(256) }],
      ['wrong world identifier type', { world: '1' }],
      ['submission identifier outside the v7 contract', { submission: randomUUID() }],
    ];
    for (const [label, patch] of cases) {
      await test.step(label, async () => {
        const response = await sendForm(page, url, { ...fields, ...patch });
        expect(response.status()).toBe(302);
        expect(response.headers().location).toBe(surface);
        await page.goto(surface);
        await expect(page.getByRole('status')).toHaveText(error);
        expect(await snapshot()).toEqual(baseline);
      });
    }
    // The invalid submissions return the values in a one-use flash. Start again
    // and take the fresh submission ID from the actual browser form.
    await page.goto(surface);
    await form.locator('select[name=world]').selectOption(fields.world);
    await form.locator(`input[name=sex][value="${fields.sex}"]`).check();
    await form.locator(`input[name=vocation][value="${fields.vocation}"]`).check();
    const fresh = await prepare(page, formSelector, { name });
    await submit(page, formSelector, { name });
    await expect(page).toHaveURL(/\/characters\/requests\/[a-f0-9-]+$/);
    const target = new URL(page.url()).pathname;
    await expect(page.getByRole('status')).toHaveText(lang === 'pl'
      ? 'Trwa przetwarzanie prośby. Bez wyniku z gry rezultat pozostaje nieznany. Odśwież stronę, aby sprawdzić ponownie.'
      : 'Your request is being processed. Without a result from Game, its outcome is still unknown. Refresh this page to check again.');
    const queued = [{ id: target.split('/').at(-1), state: 'PENDING', receipt_json: null }];
    expect(await snapshot()).toEqual(queued);
    for (const { screen, account } of await foreignActors(browser, info.project, lang, contexts, 'content_editor')) {
      expect((await sendForm(screen, fresh.url, fresh.fields)).status()).toBe(403);
      expect((await screen.goto(target)).status()).toBe(404);
      expect(await snapshot()).toEqual(queued);
      expect(await query('SELECT id FROM character_commands WHERE account_id=$1', [account.id])).toEqual([]);
    }
    const replay = await sendForm(page, fresh.url, fresh.fields);
    expect(replay.status()).toBe(302);
    expect(replay.headers().location).toBe(target);
    expect(await snapshot()).toEqual(queued);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('support ticket reply and resolution require the support permission', async ({ page, browser }, info) => {
  test.setTimeout(720_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const actor = async role => {
    const context = await browser.newContext({
      ...info.project.use, baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true,
    });
    contexts.push(context);
    const page = await context.newPage();
    await seededPlayer(page, lang, role);
    return page;
  };
  try {
    const owner = await seededPlayer(page, lang);
    await page.goto(`/${lang}/support/tickets/create`);
    await submit(page, formSelector, { subject: 'Staff matrix question', body: 'Player original' });
    await expect(page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
    const ownerPath = new URL(page.url()).pathname;
    const ticket = ownerPath.split('/').at(-1);
    const staffPath = `/${lang}/admin/support/tickets/${ticket}`;
    const support = await actor('support');
    const editor = await actor('content_editor');
    const other = await actor();
    const actorToken = async screen => {
      const response = await screen.goto(`/${lang}/support/tickets/create`);
      expect(response.status()).toBe(200);
      return screen.locator(`${formSelector} [name=_csrf]`).inputValue();
    };
    const unauthorized = [
      { page: editor, token: await actorToken(editor) },
      { page: other, token: await actorToken(other) },
    ];
    const snapshot = () => query(
      `SELECT t.status, t.lock_version, t.unread_reply, m.author_kind, m.body
       FROM tickets t JOIN ticket_messages m ON m.ticket_id=t.id
       WHERE t.id=$1 AND t.account_id=$2 ORDER BY m.id`,
      [ticket, owner.id],
    );
    const error = lang === 'pl'
      ? 'Nie udało się wykonać żądania. Odśwież zgłoszenie i spróbuj ponownie.'
      : 'The request could not be completed. Reload the ticket and try again.';
    const verifyRefusals = async (selector, values, samples, method = 'POST') => {
      const policy = selector.includes('/status') ? 'support-ticket-status' : 'support-ticket-reply';
      await expect.poll(async () => {
        const [row] = await query(
          `SELECT COALESCE(max(count),0)::int AS count FROM web_rate_limits
           WHERE limiter_name=$1 AND window_start=date_trunc('minute',clock_timestamp())`, [policy],
        );
        return row.count;
      }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(6);
      const baseline = await snapshot();
      const { url, fields } = await prepare(support, selector, values);
      await rejected({ page: support, url, fields, snapshot, baseline, method });
      for (const actor of unauthorized) {
        expect((await sendForm(actor.page, url, { ...fields, _csrf: actor.token }, method)).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      await invalid({ page: support, url, fields, samples, message: error, snapshot, baseline, method });
      return { url, fields };
    };

    expect((await support.goto(staffPath)).status()).toBe(200);
    const replySelector = 'main form[action$="/reply"]';
    const body = 'Visible support reply';
    const reply = await verifyRefusals(replySelector, { body }, [
      { name: 'empty reply', patch: { body: '' } },
      { name: 'overlong reply', patch: { body: 'x'.repeat(8001) } },
      { name: 'wrong internal flag type', patch: { internal: 'bad' } },
      { name: 'version below one', patch: { lock_version: '0' } },
    ]);
    await submit(support, replySelector, { body });
    await expect(support.locator('main')).toContainText(body);
    const replied = [
      { status: 'waiting_user', lock_version: '2', unread_reply: true, author_kind: 'user', body: 'Player original' },
      { status: 'waiting_user', lock_version: '2', unread_reply: true, author_kind: 'staff', body },
    ];
    expect(await snapshot()).toEqual(replied);
    expect((await sendForm(support, reply.url, reply.fields)).status()).toBe(409);
    expect(await snapshot()).toEqual(replied);

    const statusSelector = 'main form[action$="/status"]';
    const statusSamples = [
      { name: 'empty status', patch: { status: '' } },
      { name: 'overlong status', patch: { status: 'x'.repeat(512) } },
      { name: 'wrong version type', patch: { lock_version: 'x' } },
      { name: 'version below one', patch: { lock_version: '0' } },
    ];
    const statusUpdate = await verifyRefusals(statusSelector, { status: 'resolved' }, statusSamples);
    await support.locator(`${statusSelector} select[name=status]`).selectOption('resolved');
    await submit(support, statusSelector);
    await page.goto(ownerPath);
    await expect(page.locator('main')).toContainText(body);
    await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Rozwiązane' : 'Resolved');
    const resolved = replied.map(row => ({ ...row, status: 'resolved', lock_version: '3' }));
    expect(await snapshot()).toEqual(resolved);
    expect((await sendForm(support, statusUpdate.url, statusUpdate.fields)).status()).toBe(409);
    expect(await snapshot()).toEqual(resolved);

    const readSelector = 'main form[action$="/read"]';
    const acknowledged = resolved.map(row => ({ ...row, unread_reply: false }));
    const { url, fields } = await prepare(page, readSelector);
    await rejected({ page, url, fields, snapshot, baseline: resolved });
    for (const actor of unauthorized) {
      expect((await sendForm(actor.page, url, { ...fields, _csrf: actor.token })).status()).toBe(404);
      expect(await snapshot()).toEqual(resolved);
    }
    await invalid({ page, url, fields, snapshot, baseline: resolved, message: error, samples: [
      { name: 'empty reply ID', patch: { reply: '' } },
      { name: 'overlong reply ID', patch: { reply: '1'.repeat(128) } },
      { name: 'wrong pagination cursor type', patch: { after: 'bad' } },
    ] });
    const stale = await sendForm(page, url, { ...fields, reply: '00000000-0000-0000-0000-000000000000' });
    expect(stale.status()).toBe(302);
    expect(await snapshot()).toEqual(resolved);
    await submit(page, readSelector);
    await expect(page.locator(readSelector)).toHaveCount(0);
    await expect(page.locator('main')).toContainText(body);
    expect(await snapshot()).toEqual(acknowledged);
    expect((await sendForm(page, url, fields)).status()).toBe(302);
    expect(await snapshot()).toEqual(acknowledged);

    expect((await support.goto(staffPath)).status()).toBe(200);
    const reopened = await verifyRefusals(statusSelector, { status: 'open' }, statusSamples, 'PUT');
    await support.locator(`${statusSelector} select[name=status]`).selectOption('open');
    await useFormMethod(support, reopened.url, 'PUT', () => submit(support, statusSelector));
    await expect(support).toHaveURL(new RegExp(`/${lang}/admin/support/tickets/${ticket}$`));
    await page.goto(ownerPath);
    await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Otwarte' : 'Open');
    const open = acknowledged.map(row => ({ ...row, status: 'open', lock_version: '4' }));
    expect(await snapshot()).toEqual(open);
    expect((await sendForm(support, reopened.url, reopened.fields, 'PUT')).status()).toBe(409);
    expect(await snapshot()).toEqual(open);

    const internal = 'Staff-only follow-up for this ticket';
    const note = await verifyRefusals(replySelector, { body: internal, internal: '1' }, [
      { name: 'empty note', patch: { body: '' } },
      { name: 'overlong note', patch: { body: 'x'.repeat(8001) } },
      { name: 'invalid internal flag', patch: { internal: 'bad' } },
      { name: 'version below one', patch: { lock_version: '0' } },
    ]);
    await submit(support, replySelector, { body: internal, internal: true });
    await expect(support.getByText(internal, { exact: true })).toBeVisible();
    const noted = [
      ...open.map(row => ({ ...row, lock_version: '5' })),
      { ...open[0], author_kind: 'staff', body: internal, lock_version: '5' },
    ];
    expect(await snapshot()).toEqual(noted);
    expect(await query("SELECT internal FROM ticket_messages WHERE ticket_id=$1 AND body=$2", [ticket, internal]))
      .toEqual([{ internal: true }]);
    await page.goto(ownerPath);
    await expect(page.getByText(body, { exact: true })).toBeVisible();
    await expect(page.getByText(internal, { exact: true })).toHaveCount(0);
    expect((await sendForm(support, note.url, note.fields)).status()).toBe(409);
    expect(await snapshot()).toEqual(noted);

    let actual = noted;
    for (const [status, method, label] of [
      ['waiting_user', 'POST', lang === 'pl' ? 'Oczekuje na Ciebie' : 'Waiting for you'],
      ['waiting_staff', 'PUT', lang === 'pl' ? 'Oczekuje na pomoc' : 'Waiting for support'],
      ['closed', 'POST', lang === 'pl' ? 'Zamknięte' : 'Closed'],
    ]) {
      const update = await verifyRefusals(statusSelector, { status }, statusSamples, method);
      await support.locator(`${statusSelector} select[name=status]`).selectOption(status);
      await useFormMethod(support, update.url, method, () => submit(support, statusSelector));
      await page.goto(ownerPath);
      await expect(page.locator('main')).toContainText(label);
      actual = actual.map(row => ({ ...row, status, lock_version: String(Number(row.lock_version) + 1) }));
      expect(await snapshot()).toEqual(actual);
      expect((await sendForm(support, update.url, update.fields, method)).status()).toBe(409);
      expect(await snapshot()).toEqual(actual);
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('player report persists one private report for the submitted request key', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const reader = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  const contexts = [reader];
  try {
    const other = await reader.newPage();
    const owner = await seededPlayer(page, lang);
    const create = `/${lang}/support/reports/create`;
    expect((await page.goto(create)).status()).toBe(200);
    const values = {
      target_reference: `Local report target ${randomUUID()}`,
      evidence_summary: 'Synthetic evidence for the browser mutation matrix.',
    };
    const { url, fields } = await prepare(page, formSelector, values);
    const snapshot = () => query(
      'SELECT id, target_reference, evidence_summary, status FROM player_reports WHERE reporter_account_id=$1',
      [owner.id],
    );
    const baseline = await snapshot();
    expect(baseline).toEqual([]);
    await rejected({ page, url, fields, snapshot, baseline });
    await wrongActor({ page: other, lang, url, fields, status: 302, snapshot, baseline });
    const error = lang === 'pl'
      ? 'Sprawdź rodzaj zgłoszenia, pasującą kategorię, cel i długość opisu. Jeśli klucz żądania jest nieprawidłowy, odśwież formularz.'
      : 'Check the report type, matching category, target and evidence lengths. Reload the form if the request key is invalid.';
    const hasReportBudget = async () => {
      const [row] = await query(
        `SELECT attempts < 3 OR window_started_at <= now() - interval '1 minute' AS allowed
         FROM player_report_submission_limits WHERE account_id=$1`,
        [owner.id],
      );
      return row?.allowed ?? true;
    };
    for (const [name, patch] of [
      ['empty target', { target_reference: '' }],
      ['overlong evidence', { evidence_summary: 'a'.repeat(4001) }],
      ['malformed request key', { request_key: 'bad' }],
      ['target beyond 160 characters', { target_reference: 'a'.repeat(161) }],
    ]) {
      await test.step(name, async () => {
        await expect.poll(hasReportBudget, { timeout: 65_000 }).toBe(true);
        const response = await sendForm(page, url, { ...fields, ...patch });
        expect(response.status()).toBe(302);
        expect(response.headers().location).toBe(create);
        await page.goto(create);
        await expect(page.getByRole('alert')).toHaveText(error);
        expect(await snapshot()).toEqual(baseline);
      });
    }
    const fresh = await prepare(page, formSelector, values);
    await submit(page, formSelector, values);
    await expect(page).toHaveURL(/\/support\/reports\/[a-f0-9-]+$/);
    const target = new URL(page.url()).pathname;
    await expect(page.locator('main')).toContainText(values.target_reference);
    await expect(page.locator('main')).toContainText(values.evidence_summary);
    const submitted = [{ id: target.split('/').at(-1), ...values, status: 'submitted' }];
    expect(await snapshot()).toEqual(submitted);
    const replay = await sendForm(page, fresh.url, fresh.fields);
    expect(replay.status()).toBe(302);
    expect(replay.headers().location).toBe(target);
    expect(await snapshot()).toEqual(submitted);
    for (const { screen, account } of await foreignActors(browser, info.project, lang, contexts, 'content_editor')) {
      expect((await sendForm(screen, fresh.url, fresh.fields)).status()).toBe(403);
      expect((await screen.goto(target)).status()).toBe(404);
      expect(await query('SELECT id FROM player_reports WHERE reporter_account_id=$1', [account.id])).toEqual([]);
      expect(await snapshot()).toEqual(submitted);
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('wallet transfer binds one debit and credit to the sending actor', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const recipientContext = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  const contexts = [recipientContext];
  try {
    const sender = await seededPlayer(page, lang);
    const recipient = await seededPlayer(await recipientContext.newPage(), lang);
    const actors = await foreignActors(browser, info.project, lang, contexts, 'content_editor');

    await page.goto(`/${lang}/shop/coins`);
    await submit(page, packageForm);
    const checkout = new URL(page.url()).pathname;
    const intent = checkout.split('/').at(-1);
    await page.locator('main form button[value=pay]').click();
    const wallet = account => query(
      'SELECT coalesce(sum(available_delta), 0)::text AS available FROM wallet_entries WHERE account_id=$1',
      [account],
    );
    await expect.poll(() => wallet(sender.id), { timeout: 30_000 }).toEqual([{ available: '1234' }]);
    expect((await query('SELECT state, provider FROM payment_intents WHERE id=$1', [intent])))
      .toEqual([{ state: 'succeeded', provider: 'test' }]);

    const transfer = `/${lang}/shop/transfer`;
    const selector = 'main form[action$="/shop/transfer"]';
    expect((await page.goto(transfer)).status()).toBe(200);
    const values = { recipient: recipient.id, amount: '34' };
    const { url, fields } = await prepare(page, selector, values);
    const snapshot = async () => ({
      entries: await query(
        'SELECT account_id, available_delta FROM wallet_entries WHERE operation_id=$1 ORDER BY available_delta',
        [fields.operation],
      ),
      sender: await wallet(sender.id),
      recipient: await wallet(recipient.id),
    });
    const baseline = { entries: [], sender: [{ available: '1234' }], recipient: [{ available: '0' }] };
    expect(await snapshot()).toEqual(baseline);
    await rejected({ page, url, fields, snapshot, baseline });
    for (const { screen, csrf } of actors) {
      expect((await sendForm(screen, url, { ...fields, _csrf: csrf })).status()).toBe(403);
      expect(await snapshot()).toEqual(baseline);
    }
    await invalid({ page, url, fields, snapshot, baseline, message: storeError[lang], samples: [
      { name: 'empty amount', patch: { amount: '' } },
      { name: 'overlong amount', patch: { amount: '9'.repeat(128) } },
      { name: 'wrong recipient type', patch: { recipient: 'bad' } },
      { name: 'zero below the minimum transfer', patch: { amount: '0' } },
    ] });

    await submit(page, selector, values);
    await expect(page).toHaveURL(new RegExp(`/${lang}/shop/history$`));
    await expect(page.locator('main dl dd').first()).toHaveText('1200');
    const done = {
      entries: [
        { account_id: sender.id, available_delta: '-34' },
        { account_id: recipient.id, available_delta: '34' },
      ],
      sender: [{ available: '1200' }],
      recipient: [{ available: '34' }],
    };
    expect(await snapshot()).toEqual(done);
    expect((await sendForm(page, url, fields)).status()).toBe(303);
    expect(await snapshot()).toEqual(done);
    for (const { screen, csrf } of actors) {
      expect((await sendForm(screen, url, { ...fields, _csrf: csrf })).status()).toBe(409);
      expect(await snapshot()).toEqual(done);
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('unwired hunt sharing, shop purchasing and bearer-only watches expose their closed boundaries', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const player = await seededPlayer(page, lang);
  await page.goto(`/${lang}/account/privacy`);
  const { fields } = await prepare(page, formSelector);
  const base = new URL(page.url()).origin;
  const huntShare = `/${lang}/account/hunts/${uuid7()}/share`;
  const unavailableShare = await page.goto(huntShare);
  expect(unavailableShare.status()).toBe(503);
  expect(unavailableShare.headers()['cache-control']).toContain('no-store');
  await expect(page.locator(formSelector)).toHaveCount(0);
  const refusedShare = await sendForm(page, new URL(huntShare, base).href, fields);
  expect(refusedShare.status()).toBe(503);
  expect(refusedShare.headers()['cache-control']).toContain('no-store');

  const shop = `/${lang}/shop`;
  const shopPage = await page.goto(shop);
  expect(shopPage.status()).toBe(200);
  await expect(page.locator(formSelector)).toHaveCount(0);
  const operation = uuid7();
  const deniedPurchase = await sendForm(page, new URL(shop, base).href, {
    _csrf: fields._csrf, operation, character: uuid7(), world: uuid7(),
    offer: 'matrix', price: '1', quantity: '1', revision: 'matrix',
  });
  expect(deniedPurchase.status()).toBe(503);
  expect(await deniedPurchase.text()).toContain(lang === 'pl'
    ? 'Operacja jest niedostępna. Powodzenie nie zostało potwierdzone.'
    : 'This operation is unavailable. No success has been confirmed.');
  expect(await query('SELECT order_id FROM store_orders WHERE account_id=$1', [player.id])).toEqual([]);

  const watch = new URL('/api/v1/stats/watches', base);
  for (const method of ['POST', 'DELETE']) {
    const response = await page.request.fetch(watch.href, {
      method, headers: { origin: base }, data: { source: 'matrix' },
    });
    expect(response.status()).toBe(401);
    expect(response.headers()['cache-control']).toContain('no-store');
    expect(await query('SELECT account_id FROM stats_watches WHERE account_id=$1', [player.id])).toEqual([]);
  }
});

for (const prefix of ['locale', 'unprefixed']) {
  test(`session revocation confines every ${prefix} alias to the signed-in account`, async ({ page, browser }, info) => {
    test.setTimeout(600_000);
    const lang = info.project.metadata.locale;
    const localized = `/${lang}/account/sessions`;
    const base = prefix === 'locale' ? localized : '/account/sessions';
    const contexts = [];
    const actorPage = async () => {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      return context.newPage();
    };
    try {
      const owner = await seededPlayer(page, lang);
      const openOwnerSession = async () => {
        const extra = await actorPage();
        await login(extra, lang, owner);
        await expect(extra).toHaveURL(new RegExp(`/${lang}/$`));
      };
      const other = await actorPage();
      await seededPlayer(other, lang);
      const editor = await actorPage();
      await seededPlayer(editor, lang, 'content_editor');
      const tokens = [];
      const othersForm = 'main form[action$="/others"]';
      for (const outsider of [other, editor]) {
        expect((await outsider.goto(localized)).status()).toBe(200);
        const { fields } = await prepare(outsider, othersForm);
        tokens.push({ outsider, csrf: fields._csrf });
      }
      const snapshot = async () => ({
        sessions: (await query('SELECT session_id FROM web_sessions WHERE account_id=$1 ORDER BY session_id', [owner.id]))
          .map(row => row.session_id),
        events: await query(
          "SELECT kind, session_id FROM security_events WHERE account_id=$1 AND kind<>'login_succeeded' ORDER BY id",
          [owner.id],
        ),
      });
      const [{ session_id: current }] = await query('SELECT session_id FROM web_sessions WHERE account_id=$1', [owner.id]);
      const expectedEvents = [];
      for (const method of ['POST', 'DELETE']) {
        await openOwnerSession();
        const baseline = await snapshot();
        const target = baseline.sessions.find(id => id !== current);
        expect(baseline.sessions).toHaveLength(2);
        expect(Boolean(target)).toBe(true);
        expect((await page.goto(base)).status()).toBe(200);
        const selector = `main form[action$="/${target}"]`;
        const form = await prepare(page, selector);
        const url = new URL(`${base}/${target}`, form.url).href;
        await rejected({ page, url, fields: form.fields, snapshot, baseline, method });
        for (const { outsider, csrf } of tokens) {
          const denied = await sendForm(outsider, url, { _csrf: csrf }, method);
          expect(denied.status()).toBe(method === 'POST' ? 302 : 303);
          await outsider.goto(denied.headers().location);
          await expect(outsider.getByRole('status')).toHaveText(lang === 'pl'
            ? 'Wybrana sesja jest nieaktywna lub niedostępna.'
            : 'The selected session is inactive or unavailable.');
          expect(await snapshot()).toEqual(baseline);
        }
        const malformed = await sendForm(page, new URL(`${base}/invalid`, form.url).href, form.fields, method);
        expect(malformed.status()).toBe(400);
        expect(await snapshot()).toEqual(baseline);
        await useFormMethod(page, form.url, method, () => submit(page, selector), url);
        await expect(page.locator(selector)).toHaveCount(0);
        await expect(page.getByRole('status')).toHaveText(lang === 'pl' ? 'Sesje zostały wylogowane.' : 'Sessions signed out.');
        expectedEvents.push({ kind: 'session_revoked', session_id: target });
        expect(await snapshot()).toEqual({ sessions: [current], events: expectedEvents });
        expect((await sendForm(page, url, form.fields, method)).status()).toBe(method === 'POST' ? 302 : 303);
        expect(await snapshot()).toEqual({ sessions: [current], events: expectedEvents });
      }

      const anonymous = await actorPage();
      for (const method of ['POST', 'DELETE']) {
        await openOwnerSession();
        const baseline = await snapshot();
        await page.goto(base);
        const form = await prepare(page, othersForm);
        const url = new URL(method === 'POST' ? `${base}/others` : base, form.url).href;
        await rejected({ page, url, fields: form.fields, snapshot, baseline, method });
        await anonymous.goto(`/${lang}/login`);
        const token = await anonymous.locator('main form [name=_csrf]').inputValue();
        expect((await sendForm(anonymous, url, { _csrf: token }, method)).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
        for (const { outsider, csrf } of tokens) {
          const response = await sendForm(outsider, url, { _csrf: csrf, account_id: owner.id }, method);
          expect(response.status()).toBe(method === 'POST' ? 302 : 303);
          expect(await snapshot()).toEqual(baseline);
        }
        await useFormMethod(page, form.url, method, () => submit(page, othersForm), url);
        await expect(page.getByRole('status')).toHaveText(lang === 'pl' ? 'Sesje zostały wylogowane.' : 'Sessions signed out.');
        expectedEvents.push({ kind: 'other_sessions_revoked', session_id: null });
        expect(await snapshot()).toEqual({ sessions: [current], events: expectedEvents });
      }

      await openOwnerSession();
      const baseline = await snapshot();
      await page.goto(base);
      const all = 'main form[action$="/all"]';
      const form = await prepare(page, all);
      const url = new URL(`${base}/all`, form.url).href;
      await rejected({ page, url, fields: form.fields, snapshot, baseline });
      await wrongActor({ page: anonymous, lang, url, fields: form.fields, status: 403, snapshot, baseline });
      for (const { outsider, csrf } of tokens) {
        expect((await sendForm(outsider, url, { _csrf: csrf, account_id: owner.id })).status()).toBe(302);
        expect(await snapshot()).toEqual(baseline);
      }
      await useFormMethod(page, form.url, 'POST', () => submit(page, all), url);
      await expect(page).toHaveURL(new RegExp(`/${lang}/login$`));
      expectedEvents.push({ kind: 'all_sessions_revoked', session_id: null });
      expect(await snapshot()).toEqual({ sessions: [], events: expectedEvents });
      expect((await sendForm(page, url, form.fields)).status()).toBe(403);
      expect(await snapshot()).toEqual({ sessions: [], events: expectedEvents });
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });
}

test('journal write boundary refuses notes without source authority and isolates email preferences', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const anonymous = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  const contexts = [anonymous];
  try {
    const actor = await seededPlayer(page, lang);
    const outsiders = await foreignActors(browser, info.project, lang, contexts, 'content_editor');
    const report = `/${lang}/account/journal/reports`;
    const unavailable = await page.goto(report);
    expect(unavailable.status()).toBe(503);
    expect(unavailable.headers()['cache-control']).toContain('no-store');
    await expect(page.locator(formSelector)).toHaveCount(0);
    await page.goto(`/${lang}/account/privacy`);
    const ready = await prepare(page, formSelector);
    const fields = { _csrf: ready.fields._csrf };
    const note = new URL(`${report}/notes?kind=character&id=${uuid7()}`, ready.url).href;
    const notes = () => query('SELECT target_id FROM pd_report_notes WHERE account_id=$1', [actor.id]);
    const baseline = await notes();
    expect(baseline).toEqual([]);
    const payload = { ...fields, body: 'Matrix note on an unavailable journal source' };
    await rejected({ page, url: note, fields: payload, snapshot: notes, baseline });
    const denied = await sendForm(page, note, payload);
    expect(denied.status()).toBe(404);
    expect(denied.headers()['cache-control']).toContain('no-store');
    expect(await notes()).toEqual(baseline);
    for (const { screen, account, csrf } of outsiders) {
      expect((await sendForm(screen, note, { ...payload, _csrf: csrf })).status()).toBe(404);
      expect(await notes()).toEqual(baseline);
      expect(await query('SELECT target_id FROM pd_report_notes WHERE account_id=$1', [account.id])).toEqual([]);
    }

    const email = new URL(`${report}/email`, ready.url).href;
    const snapshot = () => query('SELECT email_enabled FROM pd_report_preferences WHERE account_id=$1', [actor.id]);
    const preferences = await snapshot();
    expect(preferences).toEqual([]);
    const optIn = { ...fields, enabled: 'true' };
    await rejected({ page, url: email, fields: optIn, snapshot, baseline: preferences });
    await wrongActor({
      page: await anonymous.newPage(), lang, url: email, fields: optIn, status: 403, snapshot, baseline: preferences,
    });
    for (const { screen, account, csrf } of outsiders) {
      expect((await sendForm(screen, email, { ...optIn, _csrf: csrf, account_id: actor.id })).status()).toBe(422);
      expect(await snapshot()).toEqual(preferences);
      expect(await query('SELECT email_enabled FROM pd_report_preferences WHERE account_id=$1', [account.id])).toEqual([]);
    }
    await invalid({
      page, url: email, fields: optIn, snapshot, baseline: preferences,
      message: lang === 'pl'
        ? 'Sprawdź ustawienie wiadomości e-mail i spróbuj ponownie.'
        : 'Check your email preference and try again.',
      samples: [
        { name: 'empty checkbox', patch: { enabled: '' } },
        { name: 'overlong checkbox', patch: { enabled: 'true'.repeat(512) } },
        { name: 'wrong checkbox type', patch: { enabled: 'abc' } },
        { name: 'numeric flag outside the checkbox contract', patch: { enabled: '2' } },
      ],
    });
    expect((await sendForm(page, email, optIn)).status()).toBe(204);
    expect(await snapshot()).toEqual([{ email_enabled: true }]);
    expect((await sendForm(page, email, optIn)).status()).toBe(204);
    expect(await snapshot()).toEqual([{ email_enabled: true }]);
    expect((await sendForm(page, email, fields)).status()).toBe(204);
    expect(await snapshot()).toEqual([{ email_enabled: false }]);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('localized and unprefixed badge forms persist only the owner selection', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const other = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const owner = await seededPlayer(page, lang);
    const anon = await other.newPage();
    const snapshot = () => query('SELECT badge_keys FROM account_badge_selections WHERE account_id=$1', [owner.id]);
    expect(await snapshot()).toEqual([]);
    const local = `/${lang}/account/badges`;
    for (const { path, selected } of [{ path: local, selected: true }, { path: '/account/badges', selected: false }]) {
      const baseline = await snapshot();
      expect((await page.goto(path)).status()).toBe(200);
      const badge = page.locator('[name=badge][value=account-age]');
      await expect(badge).toBeEnabled();
      await badge.setChecked(selected);
      const { url, fields } = await prepare(page, formSelector);
      await rejected({ page, url, fields, snapshot, baseline });
      await wrongActor({ page: anon, lang, url, fields, status: 303, snapshot, baseline });
      await invalid({
        page, url, fields, snapshot, baseline,
        message: lang === 'pl'
          ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
          : 'Some information could not be accepted. Check your input and try again.',
        samples: [
          { name: 'empty badge key', patch: { badge: '' } },
          { name: 'overlong badge key', patch: { badge: 'a'.repeat(512) } },
          { name: 'wrong character identifier type', patch: { profile: 'abc' } },
          { name: 'nil UUID at the identifier boundary', patch: { profile: '00000000-0000-0000-0000-000000000000' } },
        ],
      });
      await submit(page, formSelector);
      await expect(page).toHaveURL(new URL(local, url).href);
      await expect(badge).toBeChecked({ checked: selected });
      const saved = [{ badge_keys: selected ? ['account-age'] : [] }];
      expect(await snapshot()).toEqual(saved);
      expect((await sendForm(page, url, fields)).status()).toBe(303);
      expect(await snapshot()).toEqual(saved);
    }
  } finally {
    await other.close();
  }
});

test('notification preferences and signed browser unsubscribe keep one owner preference', async ({ page, browser }, info) => {
  test.setTimeout(360_000);
  const lang = info.project.metadata.locale;
  const anonymous = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const owner = await seededPlayer(page, lang);
    const reader = await anonymous.newPage();
    const preferences = `/${lang}/account/notifications`;
    const state = () => query(
      'SELECT category, email_enabled, portal_enabled FROM account_notification_prefs WHERE account_id=$1 ORDER BY category',
      [owner.id],
    );
    const baseline = await state();
    expect(baseline).toEqual([]);
    expect((await page.goto(preferences)).status()).toBe(200);
    await page.locator('[name=support_portal]').uncheck();
    const { url, fields } = await prepare(page, formSelector, {
      news_email: 'true', news_portal: 'true', support_email: 'true',
    });
    await rejected({ page, url, fields, snapshot: state, baseline });
    await wrongActor({ page: reader, lang, url, fields, status: 403, snapshot: state, baseline });
    const invalidMessage = lang === 'pl'
      ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    const samples = [
      { name: 'empty checkbox', patch: { news_email: '' } },
      { name: 'overlong checkbox', patch: { news_portal: 'true'.repeat(512) } },
      { name: 'wrong checkbox type', patch: { support_email: 'abc' } },
      { name: 'numeric flag outside the checkbox contract', patch: { support_portal: '2' } },
    ];
    await invalid({ page, url, fields, samples, message: invalidMessage, snapshot: state, baseline });
    await submit(page, formSelector, {
      news_email: true, news_portal: true, support_email: true, support_portal: false,
    });
    await expect(page.locator('[name=news_email]')).toBeChecked();
    await expect(page.locator('[name=support_portal]')).not.toBeChecked();
    const subscribed = [
      { category: 'news', email_enabled: true, portal_enabled: true },
      { category: 'support', email_enabled: true, portal_enabled: false },
    ];
    expect(await state()).toEqual(subscribed);
    expect((await sendForm(page, url, fields)).status()).toBe(303);
    expect(await state()).toEqual(subscribed);

    const raw = `unsubscribe:${owner.id}:news:${Math.floor(Date.now() / 1000) + 3600}`;
    const signature = createHmac('sha256', Buffer.from(process.env.OTERYN_NOTIFY_UNSUBSCRIBE_KEY))
      .update(raw).digest('hex');
    const target = new URL(`/${lang}/unsubscribe`, url);
    const form = { 'List-Unsubscribe': 'One-Click' };
    const attempts = [
      ['', 'missing signed capability'],
      ['x'.repeat(513), 'length above the token bound'],
      ['1', 'malformed capability'],
      [`${raw}:${'0'.repeat(64)}`, 'signature refused for this account'],
    ];
    await reader.goto(`/${lang}/login`);
    await expect.poll(async () => {
      const [row] = await query(
        "SELECT COALESCE(max(count),0)::int AS count FROM web_rate_limits WHERE limiter_name='identity-email-token' AND window_start=date_trunc('minute',clock_timestamp())"
      );
      return row.count;
    }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(4);
    for (const [token, name] of attempts) {
      await test.step(name, async () => {
        if (token) target.searchParams.set('token', token);
        else target.searchParams.delete('token');
        const response = await sendForm(reader, target.href, form);
        expect(response.status()).toBe(422);
        expect(await response.text()).toContain(invalidMessage);
        expect(await state()).toEqual(subscribed);
      });
    }
    target.searchParams.set('token', `${raw}:${signature}`);
    expect((await reader.goto(target.href)).status()).toBe(200);
    await reader.locator(`${formSelector} button`).click();
    await expect(reader.locator('main h1')).toHaveText(lang === 'pl' ? 'Opcjonalne wiadomości e-mail są wyłączone.' : 'Optional e-mail disabled.');
    const unsubscribed = [
      { ...subscribed[0], email_enabled: false },
      subscribed[1],
    ];
    expect(await state()).toEqual(unsubscribed);
    expect((await sendForm(reader, target.href, form)).status()).toBe(200);
    expect(await state()).toEqual(unsubscribed);
    await page.goto(preferences);
    await expect(page.locator('[name=news_email]')).not.toBeChecked();
    await expect(page.locator('[name=support_email]')).toBeChecked();
  } finally {
    await anonymous.close();
  }
});

test('unpublished Game content leaves legacy item trade, atlas and recruitment closed', async ({ page }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const owner = await seededPlayer(page, lang);
  await page.goto(`/${lang}/account/privacy`);
  const { url, fields } = await prepare(page, formSelector);
  const snapshot = async () => ({
    markers: await query('SELECT id FROM atlas_private_markers WHERE account_id=$1', [owner.id]),
    reports: await query('SELECT id FROM atlas_map_reports WHERE account_id=$1', [owner.id]),
    posts: await query('SELECT id FROM pd_party_posts WHERE account_id=$1', [owner.id]),
  });
  const baseline = { markers: [], reports: [], posts: [] };
  expect(await snapshot()).toEqual(baseline);
  const atlas = await page.goto(`/${lang}/atlas`);
  expect(atlas.status()).toBe(503);
  expect(atlas.headers()['cache-control']).toContain('no-store');
  await expect(page.locator('main form[method=post]')).toHaveCount(0);
  expect(await snapshot()).toEqual(baseline);
  const catalog = `/${lang}/bazaar/items`;
  const asset = uuid7();
  const itemActions = ['accept', 'reject', 'publish', 'renew', 'cancel', 'bid', 'offer', 'buy-now'];
  expect((await page.goto(`${catalog}/new`)).status()).toBe(200);
  await expect(page.locator('main form[action$="/account/bazaar/sell"]')).toHaveCount(1);
  const listing = await sendForm(page, new URL(catalog, url).href, { _csrf: fields._csrf });
  expect(listing.status()).toBe(405);
  for (const action of itemActions) {
    const path = `${catalog}/${asset}/${action}`;
    const response = await sendForm(page, new URL(path, url).href, { _csrf: fields._csrf });
    expect(response.status(), path).toBe(404);
  }
  expect(await query('SELECT id FROM bazaar_listings WHERE owner=$1', [owner.id])).toEqual([]);

  const board = `/${lang}/party`;
  expect((await page.goto(board)).status()).toBe(200);
  const partyForm = `main form[method=post][action="${board}"]`;
  const now = Math.floor(Date.now() / 1000);
  const values = {
    character: uuid7(), purpose: 'hunt', min_level: '1', max_level: '100',
    profession: 'any', starts_at: String(now + 60), expires_at: String(now + 3600), note: 'Local hunt',
  };
  const formation = await prepare(page, partyForm, values);
  await rejected({ page, ...formation, snapshot, baseline });
  expect((await sendForm(page, formation.url, formation.fields)).status()).toBe(403);
  expect(await snapshot()).toEqual(baseline);
  for (const action of ['remove', 'report']) {
    const response = await sendForm(page, new URL(`${board}/${asset}/${action}`, url).href, {
      _csrf: formation.fields._csrf, ...(action === 'report' ? { reason: 'Local recruitment report' } : {}),
    });
    expect(response.status()).toBe(404);
    expect(await snapshot()).toEqual(baseline);
  }
});


test('atlas participation API persists owner choice while the source-dependent form is unavailable', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const context = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const owner = await seededPlayer(page, lang);
    const anonymous = await context.newPage();
    const path = `/${lang}/atlas/places`;
    const unavailable = await page.goto(path);
    expect(unavailable.status()).toBe(503);
    expect(unavailable.headers()['cache-control']).toContain('no-store');
    await expect(page.locator(formSelector)).toHaveCount(0);
    await page.goto(`/${lang}/account/privacy`);
    const privacy = await prepare(page, formSelector);
    const url = new URL(`${path}/participation`, privacy.url).href;
    const fields = { _csrf: privacy.fields._csrf, participating: 'true' };
    const snapshot = () => query('SELECT participating FROM pd_places_preferences WHERE account_id=$1', [owner.id]);
    const baseline = await snapshot();
    expect(baseline).toEqual([]);
    expect(await (await page.request.get(url)).json()).toEqual({ participating: false });
    await rejected({ page, url, fields, snapshot, baseline });
    await wrongActor({ page: anonymous, lang, url, fields, status: 403, snapshot, baseline });
    for (const participating of ['', 'true'.repeat(512), '1', 'true ']) {
      expect((await sendForm(page, url, { ...fields, participating })).status()).toBe(400);
      expect(await snapshot()).toEqual(baseline);
    }
    expect((await sendForm(page, url, { ...fields, account_id: uuid7() })).status()).toBe(422);
    expect(await snapshot()).toEqual(baseline);
    for (const participating of [true, false]) {
      const changed = await sendForm(page, url, { ...fields, participating: String(participating) });
      expect(changed.status()).toBe(303);
      expect(changed.headers().location).toBe(path);
      expect(await (await page.request.get(url)).json()).toEqual({ participating });
      expect(await snapshot()).toEqual([{ participating }]);
    }
  } finally {
    await context.close();
  }
});

test('atlas annotation API writes owner data while the unavailable atlas hides its forms', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const owner = await seededPlayer(page, lang);
    const outsiders = await foreignActors(browser, info.project, lang, contexts, 'content_editor');
    const markerPath = '/atlas/private-markers';
    const reportPath = '/atlas/map-reports';
    const browserPath = `/${lang}/atlas`;
    expect((await page.goto(browserPath)).status()).toBe(503);
    await expect(page.locator('main form')).toHaveCount(0);
    const ownResponse = await page.request.get(markerPath);
    expect(ownResponse.status()).toBe(200);
    const ownMarkers = await ownResponse.json();
    expect(ownMarkers.markers).toEqual([]);
    for (const { screen } of outsiders) {
      const response = await screen.request.get(markerPath);
      expect(response.status()).toBe(200);
      expect((await response.json()).markers).toEqual([]);
    }
    const origin = new URL(page.url()).origin;
    const request = (actor, url, method, data, token) => actor.request.fetch(url, {
      method, data,
      headers: { origin, ...(token === undefined ? {} : { 'x-csrf-token': token }) },
    });
    const snapshot = async () => ({
      markers: await query('SELECT id,x,y,floor,label FROM atlas_private_markers WHERE account_id=$1', [owner.id]),
      reports: await query(
        'SELECT x,y,floor,description,status FROM atlas_map_reports WHERE account_id=$1',
        [owner.id],
      ),
    });
    const baseline = { markers: [], reports: [] };
    expect(await snapshot()).toEqual(baseline);
    const text = 'Local atlas annotation';
    const values = { x: 42, y: 86, floor: 7, text };
    const sampleIds = outsiders.map(({ account }) => account.id);
    const outsiderState = () => query(
      `SELECT account_id FROM atlas_private_markers WHERE account_id=ANY($1::uuid[])
       UNION ALL SELECT account_id FROM atlas_map_reports WHERE account_id=ANY($1::uuid[])`, [sampleIds],
    );
    for (const [path, max] of [[markerPath, 80], [reportPath, 2000]]) {
      for (const token of [undefined, 'invalid']) {
        expect((await request(page, path, 'POST', values, token)).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      for (const invalid of [
        { text: '' }, { text: 'x'.repeat(max + 1) }, { x: '42' }, { floor: 256 },
      ]) {
        expect((await request(page, path, 'POST', { ...values, ...invalid }, ownMarkers.csrf_token)).status()).toBe(422);
        expect(await snapshot()).toEqual(baseline);
      }
      for (const { screen, csrf } of outsiders) {
        expect((await request(screen, path, 'POST', { ...values, account_id: owner.id }, csrf)).status()).toBe(422);
        expect(await snapshot()).toEqual(baseline);
        expect(await outsiderState()).toEqual([]);
      }
    }
    const created = await request(page, markerPath, 'POST', values, ownMarkers.csrf_token);
    expect(created.status()).toBe(201);
    const marker = { id: (await created.json()).id, x: 42, y: 86, floor: 7, label: text };
    expect(await snapshot()).toEqual({ ...baseline, markers: [marker] });
    expect((await (await page.request.get(markerPath)).json()).markers).toEqual([marker]);
    const target = `${markerPath}/${marker.id}`;
    for (const token of [undefined, 'invalid']) {
      expect((await request(page, target, 'DELETE', undefined, token)).status()).toBe(403);
      expect(await snapshot()).toEqual({ ...baseline, markers: [marker] });
    }
    for (const { screen, csrf } of outsiders) {
      expect((await (await screen.request.get(markerPath)).json()).markers).toEqual([]);
      expect((await request(screen, target, 'DELETE', undefined, csrf)).status()).toBe(404);
      expect(await snapshot()).toEqual({ ...baseline, markers: [marker] });
    }
    expect((await request(page, target, 'DELETE', undefined, ownMarkers.csrf_token)).status()).toBe(204);
    expect((await (await page.request.get(markerPath)).json()).markers).toEqual([]);
    expect((await request(page, target, 'DELETE', undefined, ownMarkers.csrf_token)).status()).toBe(404);
    expect(await snapshot()).toEqual(baseline);

    expect((await request(page, reportPath, 'POST', { ...values, text: 'x'.repeat(9) }, ownMarkers.csrf_token)).status()).toBe(422);
    expect(await snapshot()).toEqual(baseline);
    expect((await request(page, reportPath, 'POST', values, ownMarkers.csrf_token)).status()).toBe(201);
    expect(await snapshot()).toEqual({
      ...baseline, reports: [{ x: 42, y: 86, floor: 7, description: text, status: 'pending' }],
    });
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('console exposes unavailable Game agents and creates no read requests', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const viewer = await seededPlayer(page, lang, 'game_viewer');
    const outsiders = await foreignActors(browser, info.project, lang, contexts, 'content_editor');
    await page.goto(`/${lang}/account/privacy`);
    const { url, fields } = await prepare(page, formSelector);
    const actorIDs = [viewer.id, ...outsiders.map(({ account }) => account.id)];
    const snapshot = () => query('SELECT request_id FROM admin_control_requests WHERE actor=ANY($1::uuid[])', [actorIDs]);
    const baseline = await snapshot();
    expect(baseline).toEqual([]);
    const agent = 'matrix-no-agent';
    const unavailable = lang === 'pl' ? 'Niedostępne' : 'Unavailable';
    const config = [
      ['dashboard', 'ops.overview.get', ''],
      ['worlds', 'world.list', ''],
      ['worlds', 'world.get', 'matrix-world'],
      ['nodes', 'node.list', ''],
      ['nodes', 'node.get', 'matrix-node'],
      ['database', 'db.status', ''],
      ['database', 'db.migrations.list', ''],
    ];
    for (const [section, operation, target] of config) {
      const path = `/console/${section}`;
      const params = new URLSearchParams({ lang, agent, ...(target ? { target } : {}) });
      expect((await page.goto(`${path}?${params}`)).status()).toBe(200);
      await expect(page.locator('main')).toContainText(`R0 · ${unavailable}`);
      await expect(page.locator('main form[method=post]')).toHaveCount(0);
      const options = {
        page, url: new URL(path, url).href, fields: { _csrf: fields._csrf, lang, agent, operation, target },
        snapshot, baseline,
      };
      await rejected(options);
      for (const { screen, csrf } of outsiders) {
        expect((await sendForm(screen, options.url, { ...options.fields, _csrf: csrf })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      expect((await sendForm(page, options.url, options.fields)).status()).toBe(503);
      expect(await snapshot()).toEqual(baseline);
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('consent forms record independent scoped grants and revocations', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const context = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const owner = await seededPlayer(page, lang);
    const anonymous = await context.newPage();
    const target = `/${lang}/account/consents`;
    const kinds = ['personalised_ads', 'play_session'];
    const state = () => query(
      "SELECT document_kind,document_version,granted,actor,source FROM account_consents WHERE account_id=$1 AND document_kind IN ('personalised_ads','play_session') ORDER BY accepted_at,id",
      [owner.id],
    );
    let baseline = [];
    expect(await state()).toEqual(baseline);
    expect((await page.goto(target)).status()).toBe(200);
    const key = createHash('sha256').update(owner.id).digest();
    for (const kind of kinds) {
      await expect.poll(async () => {
        const [row] = await query(
          "SELECT COALESCE(max(count),0)::int AS count FROM web_rate_limits WHERE limiter_name='identity-consent-mutation' AND window_start=date_trunc('minute',clock_timestamp()) AND key_hash=$1",
          [key],
        );
        return row.count;
      }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(3);
      const selector = `main form:has([name=kind][value="${kind}"])`;
      const consent = page.locator(`${selector} [name=granted]`);
      await expect(consent).not.toBeChecked();
      const { url, fields } = await prepare(page, selector, { granted: 'true' });
      await rejected({ page, url, fields, snapshot: state, baseline });
      await wrongActor({ page: anonymous, lang, url, fields, status: 303, snapshot: state, baseline });
      for (const patch of [
        { version: '' }, { version: 'a'.repeat(129) }, { granted: 'abc' }, { kind: 'play_session ' },
      ]) {
        const refused = await sendForm(page, url, { ...fields, ...patch });
        expect(refused.status()).toBe(400);
        expect(await refused.text()).toContain(lang === 'pl'
          ? 'Sprawdź żądanie i spróbuj ponownie.' : 'Check the request and try again.');
        expect(await state()).toEqual(baseline);
      }
      const outdated = await sendForm(page, url, { ...fields, version: '2000-01-01.1' });
      expect(outdated.status()).toBe(409);
      expect(await outdated.text()).toContain(lang === 'pl'
        ? 'Treść zgody uległa zmianie.'
        : 'The consent text has changed.');
      expect(await state()).toEqual(baseline);
      await submit(page, selector, { granted: true });
      await expect(consent).toBeChecked();
      const decision = {
        document_kind: kind, document_version: fields.version, granted: true, actor: owner.id, source: 'web',
      };
      baseline = [...baseline, decision];
      expect(await state()).toEqual(baseline);
      await submit(page, selector, { granted: false });
      await expect(consent).not.toBeChecked();
      baseline = [...baseline, { ...decision, granted: false }];
      expect(await state()).toEqual(baseline);
    }
  } finally {
    await context.close();
  }
});

test('OAuth consent form approves or denies the checked client and binds its code to the acting account', async ({ page, browser }, info) => {
  test.setTimeout(360_000);
  const lang = info.project.metadata.locale;
  const context = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  const contexts = [context];
  try {
    const owner = await seededPlayer(page, lang);
    const anonymous = await context.newPage();
    const callback = new URL('/__oauth/callback', page.url()).href;
    const oauth = new URL('/oauth/authorize', page.url()).href;
    const readCodes = () => query(
      'SELECT account_id,client_id,used_at IS NOT NULL AS used FROM oauth_authorization_codes WHERE account_id=$1 ORDER BY created_at',
      [owner.id],
    );
    const client = 'e2e-local';
    const state = randomUUID();
    const verifier = randomBytes(32).toString('base64url');
    const request = {
      response_type: 'code', client_id: client, redirect_uri: callback,
      scope: 'game:ticket', state,
      code_challenge: createHash('sha256').update(verifier).digest('base64url'),
      code_challenge_method: 'S256',
    };
    const consent = `${oauth}?${new URLSearchParams(request)}`;
    expect((await page.goto(consent)).status()).toBe(200);
    const choice = 'main form:has(button[name=decision])';
    const { url, fields } = await prepare(page, choice, { decision: 'approve' });
    expect(await readCodes()).toEqual([]);
    await rejected({ page, url, fields, snapshot: readCodes, baseline: [] });
    await wrongActor({ page: anonymous, lang, url, fields, status: 400, snapshot: readCodes, baseline: [] });
    for (const { screen, account, csrf } of await foreignActors(browser, info.project, lang, contexts, 'content_editor')) {
      for (const decision of ['approve', 'deny']) {
        const attempt = await sendForm(screen, url, { ...fields, _csrf: csrf, decision, account_id: owner.id });
        expect(attempt.status()).toBe(400);
        expect(await attempt.json()).toEqual({
          error: 'invalid_request',
          error_description: 'The request is missing a required parameter or is malformed.',
        });
        expect(await readCodes()).toEqual([]);
        expect(await query(
          'SELECT account_id FROM oauth_authorization_codes WHERE account_id=$1', [account.id],
        )).toEqual([]);
        expect(await query(
          "SELECT event FROM oauth_security_events WHERE account_id=$1 AND event='consent_denied'",
          [account.id],
        )).toEqual([]);
      }
    }

    for (const patch of [
      { decision: '' }, { decision: 'a'.repeat(513) },
      { response_type: '1' }, { code_challenge: 'a'.repeat(42) },
    ]) {
      const denied = await sendForm(page, url, { ...fields, ...patch });
      expect(denied.status()).toBe(400);
      expect(await denied.json()).toEqual({
        error: 'invalid_request',
        error_description: 'The request is missing a required parameter or is malformed.',
      });
      expect(await readCodes()).toEqual([]);
    }

    await page.locator(`${choice} button[value=deny]`).click();
    await expect(page).toHaveURL(/\/__oauth\/callback\?/);
    const refusal = new URL(page.url());
    expect(refusal.searchParams.get('error')).toBe('access_denied');
    expect(refusal.searchParams.get('state')).toBe(state);
    expect(await readCodes()).toEqual([]);
    expect(await query(
      "SELECT event FROM oauth_security_events WHERE account_id=$1 AND event='consent_denied'",
      [owner.id],
    )).toEqual([{ event: 'consent_denied' }]);

    expect((await page.goto(consent)).status()).toBe(200);
    await page.locator(`${choice} button[value=approve]`).click();
    await expect(page).toHaveURL(/\/__oauth\/callback\?/);
    const returned = new URL(page.url());
    const code = returned.searchParams.get('code');
    expect(Boolean(code)).toBe(true);
    expect(returned.searchParams.get('state')).toBe(state);
    expect(await readCodes()).toEqual([{ account_id: owner.id, client_id: client, used: false }]);
    const exchange = {
      grant_type: 'authorization_code', client_id: client, redirect_uri: callback,
      code, code_verifier: verifier,
    };
    expect((await page.request.post('/oauth/token', { form: exchange })).status()).toBe(200);
    expect((await page.request.post('/oauth/token', { form: exchange })).status()).toBe(400);
    expect(await readCodes()).toEqual([{ account_id: owner.id, client_id: client, used: true }]);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('localized and unprefixed login, MFA login and logout aliases rotate only the acting session', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const acting = await seededPlayer(page, lang);
    const existing = [];
    for (const role of [undefined, 'content_editor']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const other = await context.newPage();
      const account = await seededPlayer(other, lang, role);
      await other.goto(`/${lang}/account/privacy`);
      existing.push({ other, id: account.id, token: (await prepare(other, formSelector)).fields._csrf });
    }
    const state = async () => ({
      sessions: (await query('SELECT session_id FROM web_sessions WHERE account_id=$1 ORDER BY session_id', [acting.id]))
        .map(row => row.session_id),
      logins: (await query("SELECT count(*)::int AS count FROM security_events WHERE account_id=$1 AND kind='login_succeeded'", [acting.id]))[0].count,
      otherSessions: await query(
        'SELECT account_id,session_id FROM web_sessions WHERE account_id=ANY($1::uuid[]) ORDER BY account_id,session_id',
        [existing.map(other => other.id)],
      ),
    });
    await logout(page, lang);
    const loginPage = `/${lang}/login`;
    const variants = [
      [`/${lang}/login`, `/${lang}/logout`], ['/login', '/logout'],
      [`/${lang}/mfa/login`, `/${lang}/logout`], ['/mfa/login', '/logout'],
    ];
    for (const [entry, exit] of variants) {
      await test.step(entry, async () => {
        const baseline = await state();
        expect(baseline.sessions).toEqual([]);
        expect(baseline.otherSessions).toHaveLength(2);
        expect((await page.goto(loginPage)).status()).toBe(200);
        const values = { email: acting.email, password: acting.password };
        const { url, fields } = await prepare(page, formSelector, values);
        const alias = new URL(entry, url).href;
        await rejected({ page, url: alias, fields, snapshot: state, baseline });
        await waitForLoginBudget(7);
        for (const { other, token } of existing) {
          const response = await sendForm(other, alias, { ...fields, _csrf: token });
          expect(response.status()).toBe(302);
          expect(response.headers().location).toBe(`/${lang}/`);
          expect(await state()).toEqual(baseline);
        }
        for (const patch of [
          { email: '' }, { email: 'a'.repeat(255) + '@example.invalid' },
          { email: '1' }, { password: 'a'.repeat(1025) },
        ]) {
          const response = await sendForm(page, alias, { ...fields, ...patch });
          expect(response.status()).toBe(302);
          expect(response.headers().location).toBe(loginPage);
          await page.goto(loginPage);
          await expect(page.getByRole('alert')).toHaveText(lang === 'pl'
            ? 'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.'
            : 'The request could not be verified. Check your details and try again.');
          expect(await state()).toEqual(baseline);
        }
        await useFormMethod(page, url, 'POST', () => submit(page, formSelector, values), alias);
        await expect(page).toHaveURL(new URL(`/${lang}/`, url).href);
        const signedIn = await state();
        expect(signedIn).toEqual({ ...baseline, sessions: [expect.any(String)], logins: baseline.logins + 1 });
        expect((await sendForm(page, alias, fields)).status()).toBe(403);
        expect(await state()).toEqual(signedIn);

        await page.goto(`/${lang}/account/privacy`);
        const logoutFields = { _csrf: (await prepare(page, formSelector)).fields._csrf };
        const real = new URL(`/${lang}/logout`, url).href;
        const alternate = new URL(exit, url).href;
        await rejected({ page, url: alternate, fields: logoutFields, snapshot: state, baseline: signedIn });
        await useFormMethod(page, real, 'POST', () => logout(page, lang), alternate);
        await expect(page).toHaveURL(new URL(`/${lang}/`, url).href);
        const signedOut = { ...signedIn, sessions: [] };
        expect(await state()).toEqual(signedOut);
        expect((await sendForm(page, alternate, logoutFields)).status()).toBe(403);
        expect(await state()).toEqual(signedOut);
        await page.goto(`/${lang}/account/sessions`);
        await expect(page).toHaveURL(new URL(loginPage, url).href);
      });
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('native account checkout applies the locked test-provider decision only once', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const owner = await seededPlayer(page, lang, 'content_editor');
    const foreign = [];
    for (const role of [undefined, 'support']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const other = await context.newPage();
      await seededPlayer(other, lang, role);
      await other.goto(`/${lang}/account/privacy`);
      foreign.push({ other, token: (await prepare(other, formSelector)).fields._csrf });
    }
    const client = 'e2e-local';
    const verifier = randomBytes(32).toString('base64url');
    const callback = new URL('/__oauth/callback', page.url()).href;
    const authorization = new URLSearchParams({
      response_type: 'code', client_id: client, redirect_uri: callback,
      scope: 'game:account-services', state: randomUUID(),
      code_challenge: createHash('sha256').update(verifier).digest('base64url'),
      code_challenge_method: 'S256',
    });
    expect((await page.goto(`/oauth/authorize?${authorization}`)).status()).toBe(200);
    await page.locator('main button[name=decision][value=approve]').click();
    await expect(page).toHaveURL(/\/__oauth\/callback\?/);
    const code = new URL(page.url()).searchParams.get('code');
    expect(Boolean(code)).toBe(true);
    const token = await page.request.post('/oauth/token', { form: {
      grant_type: 'authorization_code', client_id: client, redirect_uri: callback,
      code, code_verifier: verifier,
    } });
    expect(token.status()).toBe(200);
    const headers = { authorization: `Bearer ${(await token.json()).access_token}` };

    for (const decision of ['pay', 'fail']) {
      const api = '/api/v1/native-account/coin-purchases';
      const catalogue = await page.request.get(`${api}/catalogue`, { headers });
      expect(catalogue.status()).toBe(200);
      const { packages, sandbox, version } = await catalogue.json();
      expect(sandbox).toBe(true);
      expect(packages).toHaveLength(1);
      const pkg = packages[0];
      const created = await page.request.post(`${api}/orders`, {
        headers, data: {
          version, request_id: uuid7(), package_id: pkg.id, quote_id: pkg.quote_id, character_id: null,
        },
      });
      expect(created.status()).toBe(200);
      const order = await created.json();
      expect(order.sandbox).toBe(true);
      expect(order.state).toBe('awaiting_payment');
      expect((await page.goto(order.checkout_url)).status()).toBe(200);
      await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Zakup monet w trybie testowym' : 'Sandbox coin checkout');
      const form = `main form:has([name=action][value=${decision}])`;
      const { url, fields } = await prepare(page, form);
      const snapshot = () => query(
        `SELECT o.state,o.receipt_id IS NOT NULL AS receipt,
            (SELECT count(*)::int FROM payment_events WHERE intent_id=o.payment_intent_id) AS events,
            (SELECT count(*)::int FROM wallet_entries WHERE operation_id=o.payment_intent_id) AS credits,
            (SELECT coalesce(sum(available_delta),0)::text FROM wallet_entries WHERE operation_id=o.payment_intent_id) AS coins
         FROM coin_purchase_orders o WHERE o.id=$1 AND o.payer_account_id=$2`,
        [order.order_id, owner.id],
      );
      const baseline = [{ state: 'awaiting_payment', receipt: false, events: 0, credits: 0, coins: '0' }];
      expect(await snapshot()).toEqual(baseline);
      await rejected({ page, url, fields, snapshot, baseline });
      for (const { other, token } of foreign) {
        expect((await sendForm(other, url, { ...fields, _csrf: token })).status()).toBe(404);
        expect(await snapshot()).toEqual(baseline);
      }
      for (const action of ['', 'x'.repeat(513), '1', 'pay ']) {
        const response = await page.request.post(url, {
          form: { ...fields, action },
          headers: { origin: new URL(url).origin, 'accept-language': lang },
          maxRedirects: 0,
        });
        expect(response.status()).toBe(400);
        expect(response.headers()['content-language']).toBe(lang);
        expect(await response.text()).toContain(lang === 'pl'
          ? 'Sprawdź żądanie i spróbuj ponownie.'
          : 'Check the request and try again.');
        expect(await snapshot()).toEqual(baseline);
      }
      const response = page.waitForResponse(response => response.url() === url && response.request().method() === 'POST');
      await submit(page, form);
      const submitted = await response;
      const requestOrigin = (await submitted.request().allHeaders()).origin;
      expect(submitted.status()).toBe(303);
      expect(requestOrigin).toBe(new URL(url).origin);
      const paid = decision === 'pay';
      const state = paid ? 'paid' : 'failed';
      const stateLabel = lang === 'pl'
        ? (paid ? 'Opłacono' : 'Niepowodzenie')
        : (paid ? 'Paid' : 'Failed');
      await expect(page.locator('main dl dd').last()).toHaveText(stateLabel);
      await expect(page.locator(formSelector)).toHaveCount(0);
      const closed = [{ state, receipt: paid, events: 1, credits: paid ? 1 : 0, coins: paid ? String(pkg.coins) : '0' }];
      expect(await snapshot()).toEqual(closed);
      for (const action of [decision, paid ? 'fail' : 'pay']) {
        expect((await sendForm(page, url, { ...fields, action })).status()).toBe(303);
        expect(await snapshot()).toEqual(closed);
      }
      if (paid) {
        const context = await browser.newContext({
          baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
          viewport: info.project.use.viewport,
        });
        contexts.push(context);
        const admin = await context.newPage();
        await seededPlayer(admin, lang, 'platform_admin');
        expect((await admin.goto(`/${lang}/account/privacy`)).status()).toBe(200);
        const { fields } = await prepare(admin, formSelector);
        const review = await admin.goto(`/admin/players/${owner.id}`);
        expect(review.status()).toBe(200);
        await expect(admin.locator('main')).toContainText(lang === 'pl'
          ? 'Brak blokady zakupu monet.' : 'No coin purchase hold.');
        await expect(admin.locator('button[value="release-coin-hold"]')).toHaveCount(0);
        const hold = async () => ({
          order: await snapshot(),
          active: await query('SELECT hold_id FROM coin_purchase_holds WHERE account_id=$1', [owner.id]),
          released: await query('SELECT hold_id FROM coin_purchase_hold_releases WHERE account_id=$1', [owner.id]),
        });
        const before = { order: closed, active: [], released: [] };
        const proposed = {
          _csrf: fields._csrf, action: 'release-coin-hold', hold: uuid7(),
          reason: 'Verify absence of a reversal hold on a local sandbox payment.',
        };
        expect(await hold()).toEqual(before);
        await rejected({ page: admin, url: review.url(), fields: proposed, snapshot: hold, baseline: before });
        for (const { other, token } of foreign) {
          expect((await sendForm(other, review.url(), { ...proposed, _csrf: token })).status()).toBe(403);
          expect(await hold()).toEqual(before);
        }
        expect((await sendForm(admin, review.url(), proposed)).status()).toBe(409);
        expect(await hold()).toEqual(before);
      }
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('news aliases preserve bilingual publication, revision conflicts and scoped deletion', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const auth = [];
  try {
    const editor = await seededPlayer(page, lang, 'content_editor');
    for (const role of [undefined, 'support']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, locale: lang, ignoreHTTPSErrors: true,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const reader = await context.newPage();
      await seededPlayer(reader, lang, role);
      await reader.goto(`/${lang}/account/privacy`);
      const { fields } = await prepare(reader, formSelector);
      auth.push({ reader, csrf: fields._csrf });
    }
    const selector = 'main form:has(input[name=slug])';
    const publicTitle = lang === 'pl' ? 'Aktualizacja portalu' : 'Portal update';
    const snapshot = slug => () => query(
      "SELECT id,title,body,title_pl,body_pl,updated_at::text AS revision FROM news WHERE slug=$1",
      [slug],
    );
    const verify = async ({ form, target, method = 'POST', values, state, samples }) => {
      const prepared = await prepare(page, form, values);
      const url = target || prepared.url;
      const { fields } = prepared;
      const baseline = await state();
      await rejected({ page, url, fields, snapshot: state, baseline, method });
      for (const { reader, csrf } of auth) {
        expect((await sendForm(reader, url, { ...fields, _csrf: csrf }, method)).status()).toBe(403);
        expect(await state()).toEqual(baseline);
      }
      for (const [patch, status] of samples) {
        const refused = await sendForm(page, url, { ...fields, ...patch }, method);
        expect(refused.status()).toBe(status);
        const message = status === 409
          ? (lang === 'pl' ? 'Wpis zmienił się od czasu otwarcia formularza. Odśwież stronę i spróbuj ponownie.'
            : 'The news entry changed after you opened the form. Reload and try again.')
          : (lang === 'pl' ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
            : 'Some information could not be accepted. Check your input and try again.');
        expect(await refused.text()).toContain(message);
        expect(await state()).toEqual(baseline);
      }
      return { ...prepared, target: url };
    };
    const invalidNews = [
      [{ title: '' }, 422], [{ title: 'a'.repeat(201) }, 422],
      [{ published_at: 'not-an-instant' }, 422], [{ slug: 'a'.repeat(161) }, 422],
    ];
    const published = new Date(Date.now() - 60_000).toISOString();
    for (const deleteMethod of ['POST', 'DELETE']) {
      const slug = `matrix-news-${randomUUID()}`;
      const state = snapshot(slug);
      const initial = {
        slug, title: 'Portal update', title_pl: 'Aktualizacja portalu', body: 'Initial version',
        body_pl: 'Pierwsza wersja', published_at: published, translation_published_at: published,
      };
      const creation = `/admin/news/create?lang=${lang}`;
      expect((await page.goto(creation)).status()).toBe(200);
      expect(await state()).toEqual([]);
      await verify({ form: selector, values: initial, state, samples: invalidNews });
      await submit(page, selector, initial);
      await expect(page).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit$/);
      const [record] = await state();
      expect(record).toEqual({
        id: expect.any(String), title: initial.title, body: initial.body, title_pl: initial.title_pl,
        body_pl: initial.body_pl, revision: expect.any(String),
      });
      const edit = `/admin/news/${record.id}/edit?lang=${lang}`;
      const publicNews = `/${lang}/news/${slug}`;
      expect((await page.goto(publicNews)).status()).toBe(200);
      await expect(page.locator('main h1')).toHaveText(publicTitle);
      expect((await page.goto(edit)).status()).toBe(200);

      const updateMethod = deleteMethod === 'POST' ? 'POST' : 'PUT';
      const content = { body: `Revision accepted by ${updateMethod}`, body_pl: `Zapis przez ${updateMethod}` };
      const revision = await verify({
        form: selector, method: updateMethod, values: content, state, samples: invalidNews,
      });
      await useFormMethod(page, revision.url, updateMethod, () => submit(page, selector, content));
      await expect(page.locator('[name=body]')).toHaveValue(content.body);
      const [revised] = await state();
      expect(revised).toEqual({ ...record, ...content, revision: expect.any(String) });
      expect(revised.revision).not.toBe(record.revision);
      expect((await sendForm(page, revision.url, revision.fields, updateMethod)).status()).toBe(409);
      expect(await state()).toEqual([revised]);
      expect((await page.goto(publicNews)).status()).toBe(200);
      await expect(page.locator('main')).toContainText(lang === 'pl' ? content.body_pl : content.body);
      await page.goto(edit);

      const remove = 'main form[action$="/delete"]';
      const destination = deleteMethod === 'DELETE' ? new URL(`/admin/news/${record.id}`, page.url()).href : undefined;
      const deletion = await verify({ form: remove, target: destination, method: deleteMethod, state, samples: [
        [{ expected_revision: '' }, 409], [{ expected_revision: 'a'.repeat(257) }, 409],
        [{ expected_revision: '1' }, 409], [{ expected_revision: record.revision }, 409],
      ] });
      await useFormMethod(page, deletion.url, deleteMethod, () => submit(page, remove), deletion.target);
      await expect(page).toHaveURL(/\/admin\/news$/);
      await expect(page.locator(`main a[href*="${record.id}"]`)).toHaveCount(0);
      expect(await state()).toEqual([]);
      expect((await sendForm(page, deletion.target, deletion.fields, deleteMethod)).status()).toBe(404);
      expect(await state()).toEqual([]);
      expect((await page.goto(publicNews)).status()).toBe(404);
      expect(await query(
        "SELECT action FROM audit_log WHERE target=$1 AND actor=$2 ORDER BY created_at,id",
        [record.id, editor.id],
      )).toEqual([
        { action: 'cms.news_created' }, { action: 'cms.news_updated' }, { action: 'cms.news_deleted' },
      ]);
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('editorial event forms enforce publishing authority through creation, edits and status changes', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const editor = await seededPlayer(page, lang, 'content_editor');
    const unauthorized = [];
    for (const role of [undefined, 'support']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const outsider = await context.newPage();
      await seededPlayer(outsider, lang, role);
      await outsider.goto(`/${lang}/account/privacy`);
      const { fields } = await prepare(outsider, formSelector);
      unauthorized.push({ outsider, token: fields._csrf });
    }

    const slug = `matrix-event-${randomUUID()}`;
    const eventSnapshot = () => query(
      `SELECT e.id,e.status,e.lock_version,e.created_by,e.updated_by,e.published_by,t.title
       FROM events e JOIN event_translations t ON t.event_id=e.id WHERE t.slug=$1 AND t.locale='en'`,
      [slug],
    );
    const inputError = lang === 'pl'
      ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    const editorHeaders = { 'accept-language': lang };
    await page.setExtraHTTPHeaders(editorHeaders);
    const reject = async (selector, fields, samples, method = 'POST') => {
      const baseline = await eventSnapshot();
      const form = await prepare(page, selector, fields);
      await rejected({ page, ...form, method, snapshot: eventSnapshot, baseline });
      for (const { outsider, token } of unauthorized) {
        expect((await sendForm(outsider, form.url, { ...form.fields, _csrf: token }, method)).status()).toBe(403);
        expect(await eventSnapshot()).toEqual(baseline);
      }
      await invalid({ page, ...form, method, samples, message: inputError, snapshot: eventSnapshot, baseline });
      return form;
    };
    const title = 'Matrix event version one';
    const dateInput = hours => new Date(Date.now() + hours * 3600000).toISOString().slice(0, 16);
    const reason = 'Verify the planned schedule and the published event details';
    const draft = {
      reason,
      starts_at: dateInput(1), ends_at: dateInput(24),
      en_title: title, en_slug: slug, en_summary: 'Local event', en_body: 'English event description',
      pl_title: 'Lokalne wydarzenie', pl_slug: slug, pl_summary: 'Wydarzenie testowe', pl_body: 'Opis wydarzenia',
    };
    const detailsSelector = 'main form:has([name=en_title])';
    const invalidDraft = [
      { name: 'empty title', patch: { en_title: '' } },
      { name: 'overlong title', patch: { en_title: 'a'.repeat(201) } },
      { name: 'invalid time representation', patch: { starts_at: 'not-a-datetime' } },
      { name: 'invalid revision representation', patch: { lock_version: '1.5' } },
      { name: 'end time at the start boundary', patch: { ends_at: draft.starts_at } },
    ];
    expect((await page.goto('/admin/events/create')).status()).toBe(200);
    await expect(page).toHaveURL(/\/admin\/events\/create$/);
    expect(await eventSnapshot()).toEqual([]);
    await reject(detailsSelector, draft, invalidDraft);
    await submit(page, detailsSelector, draft);
    await expect(page).toHaveURL(/\/admin\/events\/[0-9a-f-]+\/edit$/);
    await expect(page.locator('[name=en_title]')).toHaveValue(title);
    const edit = new URL(page.url()).pathname;
    const id = edit.split('/').at(-2);
    let stored = {
      id, status: 'draft', lock_version: '1', created_by: editor.id,
      updated_by: editor.id, published_by: null, title,
    };
    expect(await eventSnapshot()).toEqual([stored]);

    for (const method of ['POST', 'PUT']) {
      const nextTitle = `Matrix title edited through ${method}`;
      const data = { en_title: nextTitle, reason };
      const form = await reject(detailsSelector, data, invalidDraft, method);
      await useFormMethod(page, form.url, method, () => submit(page, detailsSelector, data));
      await expect(page.locator('[name=en_title]')).toHaveValue(nextTitle);
      stored = { ...stored, title: nextTitle, lock_version: String(Number(stored.lock_version) + 1) };
      expect(await eventSnapshot()).toEqual([stored]);
      expect((await sendForm(page, form.url, form.fields, method)).status()).toBe(409);
      expect(await eventSnapshot()).toEqual([stored]);
    }

    for (const [status, method] of [
      ['scheduled', 'POST'], ['published', 'PUT'], ['archived', 'POST'], ['draft', 'PUT'],
    ]) {
      const selector = 'main form[action$="/status"]';
      const form = await reject(selector, { status, reason }, [
        { name: 'empty target status', patch: { status: '' } },
        { name: 'overlong target status', patch: { status: 'x'.repeat(512) } },
        { name: 'numeric target status', patch: { status: '1' } },
        { name: 'invalid revision representation', patch: { lock_version: '1.5' } },
        { name: 'unknown status beyond the accepted values', patch: { status: 'archived ' } },
      ], method);
      await page.locator(`${selector} select[name=status]`).selectOption(status);
      await useFormMethod(page, form.url, method, () => submit(page, selector, { reason }));
      await expect(page).toHaveURL(new URL(edit, form.url).href);
      stored = {
        ...stored, status, published_by: status === 'draft' ? null : editor.id,
        lock_version: String(Number(stored.lock_version) + 1),
      };
      expect(await eventSnapshot()).toEqual([stored]);
      expect((await sendForm(page, form.url, form.fields, method)).status()).toBe(409);
      expect(await eventSnapshot()).toEqual([stored]);
      const visible = await page.goto(`/${lang}/events/${slug}`);
      expect(visible.status()).toBe(status === 'published' ? 200 : 404);
      if (status === 'published') {
        await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? draft.pl_title : stored.title);
      }
      await page.goto(edit);
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('Game player search refuses unqualified actors and exposes the unavailable agent state', async ({ page, browser }, info) => {
  test.setTimeout(420_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const agent = await seededPlayer(page, lang, 'game_support');
    const outsiders = [];
    for (const role of [undefined, 'content_editor']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const outsider = await context.newPage();
      await seededPlayer(outsider, lang, role);
      await outsider.goto(`/${lang}/account/privacy`);
      outsiders.push({ outsider, token: (await prepare(outsider, formSelector)).fields._csrf });
    }
    await page.goto(`/${lang}/account/privacy`);
    const { url, fields } = await prepare(page, formSelector);
    const path = '/admin/console/players';
    const target = new URL(path, url).href;
    const input = { _csrf: fields._csrf, q: 'Matrix Player', lang };
    const state = () => query('SELECT request_id FROM admin_control_requests WHERE actor=$1', [agent.id]);
    const baseline = await state();
    expect(baseline).toEqual([]);
    const unavailable = lang === 'pl' ? 'Niedostępne' : 'Unavailable';
    expect((await page.goto(`${path}?lang=${lang}`)).status()).toBe(200);
    await expect(page.locator('main')).toContainText(unavailable);
    await expect(page.locator(formSelector)).toHaveCount(0);
    await rejected({ page, url: target, fields: input, snapshot: state, baseline });
    for (const { outsider, token } of outsiders) {
      expect((await sendForm(outsider, target, { ...input, _csrf: token })).status()).toBe(403);
      expect(await state()).toEqual(baseline);
    }
    for (const q of ['', 'a'.repeat(65), '&&', 'a']) {
      expect((await sendForm(page, target, { ...input, q })).status()).toBe(400);
      expect(await state()).toEqual(baseline);
    }
    const qualified = await sendForm(page, target, input);
    expect(qualified.status()).toBe(303);
    const result = new URL(qualified.headers().location, target);
    expect(result.pathname).toBe(path);
    expect(result.searchParams.get('q')).toBe(input.q);
    // The secure GET requires same-origin navigation. Set the header on the
    // actual navigation that follows the POST's server-generated destination.
    expect((await page.goto(result.href, { referer: target })).status()).toBe(200);
    await expect(page.locator('main')).toContainText(unavailable);
    await expect(page.locator(formSelector)).toHaveCount(0);
    expect(await state()).toEqual(baseline);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('staff roles create, edit, grant and revoke only under admin.roles.manage', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const open = async role => {
    const context = await browser.newContext({
      baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
      viewport: info.project.use.viewport,
    });
    contexts.push(context);
    const screen = await context.newPage();
    const player = await seededPlayer(screen, lang, role);
    await screen.goto(`/${lang}/account/privacy`);
    const { fields } = await prepare(screen, formSelector);
    return { screen, player, token: fields._csrf };
  };
  try {
    const manager = await seededPlayer(page, lang, 'security_admin');
    await confirmStaffPassword(page, lang, manager.password);
    const target = await open();
    const editor = await open('content_editor');
    const player = await open();
    const unauthorized = [player, editor];
    const key = `matrix_${randomUUID().replaceAll('-', '')}`;
    const url = new URL('/admin/roles', page.url()).href;
    const state = async () => {
      const [record] = await query('SELECT id FROM roles WHERE key=$1', [key]);
      if (!record) return null;
      return {
        id: record.id,
        permissions: (await query('SELECT permission FROM role_permissions WHERE role_id=$1 ORDER BY permission', [record.id])).map(r => r.permission),
        members: (await query('SELECT account_id FROM account_roles WHERE role_id=$1 ORDER BY account_id', [record.id])).map(r => r.account_id),
      };
    };
    const verify = async (selector, values, samples) => {
      const baseline = await state();
      const { fields } = await prepare(page, selector, values);
      await rejected({ page, url, fields, snapshot: state, baseline });
      for (const { screen, token } of unauthorized) {
        expect((await sendForm(screen, url, { ...fields, _csrf: token })).status()).toBe(403);
        expect(await state()).toEqual(baseline);
      }
      for (const [patch, status] of samples) {
        const refused = await sendForm(page, url, { ...fields, ...patch });
        expect(refused.status()).toBe(status);
        const message = status === 400
          ? (lang === 'pl' ? 'Sprawdź żądanie i spróbuj ponownie.' : 'Check the request and try again.')
          : (lang === 'pl' ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
            : 'Some information could not be accepted. Check your input and try again.');
        expect(await refused.text()).toContain(message);
        expect(await state()).toEqual(baseline);
      }
    };

    expect((await page.goto(url)).status()).toBe(200);
    expect(await state()).toBeNull();
    const create = 'main form:has(button[value=create])';
    const first = 'admin.access';
    const invalidPermissions = [
      [{ permissions: 'x'.repeat(500) }, 400],
      [{ permissions: '42' }, 400],
    ];
    await verify(create, { action: 'create', key, permissions: first }, [
      [{ key: '' }, 400], [{ key: 'x'.repeat(129) }, 400], ...invalidPermissions,
    ]);
    await submit(page, create, { key, permissions: first });
    await expect(page).toHaveURL(url);
    const created = await state();
    expect(created).toEqual({ id: expect.any(String), permissions: [first], members: [] });
    await rolePage(page, created.id);
    await expect(page.locator('main')).toContainText(key);

    const edit = 'main form:has(select[name=role]):has([name=permissions])';
    const next = 'admin.access audit.view';
    await verify(edit, { action: 'set-permissions', role: created.id, permissions: next }, [
      ...invalidPermissions, [{ role: 'x' }, 422],
      [{ role: '00000000-0000-0000-0000-000000000000' }, 400],
    ]);
    await action(page, edit, { role: created.id, permissions: next }, 'set-permissions');
    await rolePage(page, created.id);
    const amended = { ...created, permissions: [first, 'audit.view'] };
    expect(await state()).toEqual(amended);
    await expect(page.locator('main')).toContainText(key);

    const membership = 'main form:has([name=account])';
    for (const operation of ['grant', 'revoke']) {
      const reason = operation === 'grant' ? { reason: 'role_assignment' } : {};
      await verify(membership, { action: operation, account: target.player.id, role: created.id, ...reason }, [
        [{ action: '' }, 400], [{ action: 'x'.repeat(129) }, 400],
        [{ role: 'x' }, 422], [{ account: '00000000-0000-0000-0000-000000000000' }, 400],
      ]);
      await action(page, membership, { account: target.player.id, role: created.id, ...reason }, operation);
      await rolePage(page, created.id);
      expect(await state()).toEqual({ ...amended, members: operation === 'grant' ? [target.player.id] : [] });
      await expect(page.locator('main')).toContainText(key);
    }
    expect(await query(
      "SELECT action FROM audit_log WHERE actor=$1 AND target IN ($2,$3) AND action LIKE 'rbac.role.%' ORDER BY created_at,id",
      [manager.id, created.id, target.player.id],
    )).toEqual([
      { action: 'rbac.role.created' }, { action: 'rbac.role.permissions_changed' },
      { action: 'rbac.role.granted' }, { action: 'rbac.role.revoked' },
    ]);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

for (const [prefix, changeMethod] of [
  ['locale', 'POST'], ['locale', 'PUT'], ['unprefixed', 'POST'], ['unprefixed', 'PUT'],
]) {
  test(`sudo and password change bind ${prefix} ${changeMethod} to the active account`, async ({ page, browser }, info) => {
    test.setTimeout(600_000);
    const lang = info.project.metadata.locale;
    const context = await browser.newContext({
      baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
      viewport: info.project.use.viewport,
    });
    try {
      const owner = await seededPlayer(page, lang);
      const outsider = await context.newPage();
      await seededPlayer(outsider, lang, 'content_editor');
      await outsider.goto(`/${lang}/account/privacy`);
      const foreignToken = (await prepare(outsider, formSelector)).fields._csrf;
      const snapshot = async () => ({
        events: (await query('SELECT kind FROM credential_security_events WHERE account_id=$1 ORDER BY id', [owner.id]))
          .map(row => row.kind),
        sessions: await query(
          'SELECT session_id, sudo_until IS NOT NULL AS sudo FROM web_sessions WHERE account_id=$1 ORDER BY session_id',
          [owner.id],
        ),
        generation: (await query('SELECT native_security_generation FROM accounts WHERE id=$1', [owner.id]))[0].native_security_generation,
      });
      const first = await snapshot();
      expect(first.events).toEqual([]);
      expect(first.sessions).toHaveLength(1);
      const error = lang === 'pl' ? 'Sprawdź obecne hasło oraz pola nowego hasła.'
        : 'Check your current password and the new password fields.';
      const prefixPath = prefix === 'locale' ? `/${lang}` : '';
      const sudo = `/${lang}/account/sudo`;
      const change = `/${lang}/password/change`;

      const verifyRefusals = async ({ path, alias, method, values, baseline, samples }) => {
        expect((await page.goto(path)).status()).toBe(200);
        const form = await prepare(page, formSelector, values);
        const url = new URL(alias, form.url).href;
        const expectedRedirect = method === 'PUT' ? 303 : 302;
        await rejected({ page, url, fields: form.fields, method, snapshot, baseline });
        expect((await sendForm(outsider, url, { ...form.fields, _csrf: foreignToken }, method)).status())
          .toBe(expectedRedirect);
        expect(await snapshot()).toEqual(baseline);
        for (const { name, patch } of samples) {
          await test.step(name, async () => {
            const denied = await sendForm(page, url, { ...form.fields, ...patch }, method);
            expect(denied.status()).toBe(expectedRedirect);
            expect(denied.headers().location).toBe(path);
            await page.goto(path);
            await expect(page.getByRole('alert')).toHaveText(error);
            expect(await snapshot()).toEqual(baseline);
          });
        }
        return { form, url };
      };
      const sudoFields = { current_password: owner.password };
      const sudoInput = await verifyRefusals({
        path: sudo, alias: `${prefixPath}/account/sudo`, method: 'POST', values: sudoFields, baseline: first,
        samples: [
          { name: 'empty current password', patch: { current_password: '' } },
          { name: 'overlong current password', patch: { current_password: 'x'.repeat(1025) } },
          { name: 'numeric input does not verify', patch: { current_password: '1' } },
          { name: 'current password with one extra character', patch: { current_password: owner.password + 'x' } },
        ],
      });
      await useFormMethod(page, sudoInput.form.url, 'POST',
        () => submit(page, formSelector, sudoFields), sudoInput.url);
      await expect(page).toHaveURL(new URL(change, sudoInput.url).href);
      const elevated = await snapshot();
      expect(elevated.events).toEqual(['sudo_confirmed']);
      expect(elevated.sessions).toHaveLength(1);
      expect(elevated.sessions[0].sudo).toBe(true);
      expect((await sendForm(page, sudoInput.url, sudoInput.form.fields)).status()).toBe(403);
      expect(await snapshot()).toEqual(elevated);

      const next = password();
      await seedPasswordRange(next);
      const passwordFields = { current_password: owner.password, password: next, password_confirmation: next };
      const changeInput = await verifyRefusals({
        path: change, alias: `${prefixPath}/password/change`, method: changeMethod,
        values: passwordFields, baseline: elevated,
        samples: [
          { name: 'empty new password', patch: { password: '' } },
          { name: 'overlong new password', patch: { password: 'x'.repeat(1025) } },
          { name: 'confirmation does not bind the new password', patch: { password_confirmation: '1234' } },
          { name: 'nine characters below the minimum', patch: { password: 'a'.repeat(9), password_confirmation: 'a'.repeat(9) } },
        ],
      });
      await useFormMethod(page, changeInput.form.url, changeMethod,
        () => submit(page, formSelector, passwordFields), changeInput.url);
      await expect(page).toHaveURL(/\/login$/);
      const changed = await snapshot();
      expect(changed.events).toEqual(['sudo_confirmed', 'password_changed']);
      expect(changed.sessions).toEqual([]);
      expect(Number(changed.generation)).toBe(Number(elevated.generation) + 1);
      expect((await sendForm(page, changeInput.url, changeInput.form.fields, changeMethod)).status()).toBe(403);
      expect(await snapshot()).toEqual(changed);
      await login(page, lang, { ...owner, password: next });
      await expect(page).toHaveURL(new URL(`/${lang}/`, changeInput.url).href);
    } finally {
      await context.close();
    }
  });
}

test('WebAuthn browser ceremonies save, authenticate and remove only the owner credential', async ({ page, browser, context }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const origin = new URL(info.project.use.baseURL).origin;
  const path = `/${lang}/account/passkeys`;
  const cdp = await context.newCDPSession(page);
  const others = [];
  await cdp.send('WebAuthn.enable');
  const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', { options: {
    protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true,
    automaticPresenceSimulation: true, isUserVerified: true,
  } });
  try {
    const owner = await seededPlayer(page, lang);
    const foreign = [];
    for (const role of [undefined, 'content_editor']) {
      const context = await browser.newContext({
        baseURL: origin, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      others.push(context);
      const other = await context.newPage();
      const actor = await seededPlayer(other, lang, role);
      expect((await other.goto(`/${lang}/account/privacy`)).status()).toBe(200);
      const { fields } = await prepare(other, formSelector);
      foreign.push({ other, token: fields._csrf, actor });

    }
    const json = (screen, url, data, token) => screen.request.post(url, {
      data, headers: { origin, ...(token === undefined ? {} : { 'x-csrf-token': token }) },
    });
    const keys = () => query('SELECT id FROM account_passkeys WHERE account_id=$1', [owner.id]);
    const sessions = () => query('SELECT session_id FROM web_sessions WHERE account_id=$1', [owner.id]);
    const keyPage = page.locator('#passkeys');
    const passwordField = page.locator('#passkey-password');
    const register = page.locator('[data-passkey=register]');
    const authenticate = page.locator('[data-passkey=authenticate]');
    const localStatus = page.locator('#passkey-status');
    const reserve = async (needed = 1) => {
      await waitForLoginBudget(needed);
      await expect.poll(async () => {
        const [row] = await query(
          "SELECT COALESCE(max(count),0)::int AS count FROM web_rate_limits WHERE limiter_name='identity-security-mutation' AND window_start=date_trunc('minute',clock_timestamp())",
        );
        return row.count;
      }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(10 - needed);
    };
    const noCsrf = async (url, data, state, baseline) => {
      for (const token of [undefined, 'invalid']) {
        expect((await json(page, url, data, token)).status()).toBe(403);
        expect(await state()).toEqual(baseline);
      }
    };
    const browserFinish = async (phase, click) => {
      const pattern = new URL(`${path}/${phase}/finish`, origin).href;
      let proof;
      const check = async route => {
        try {
          proof = route.request().postDataJSON();
          const hash = createHash('sha256').update(proof.token).digest();
          const ceremony = () => query('SELECT account_id,purpose FROM passkey_ceremonies WHERE token_hash=$1', [hash]);
          const baseline = [{ account_id: phase === 'register' ? owner.id : null, purpose: phase }];
          expect(await ceremony()).toEqual(baseline);
          const token = await keyPage.getAttribute('data-csrf');
          await noCsrf(pattern, proof, ceremony, baseline);
          await reserve(5);
          for (const { other, token } of foreign) {
            expect((await json(other, pattern, proof, token)).status()).toBe(403);
            expect(await ceremony()).toEqual(baseline);
          }
          for (const patch of [
            { token: '' }, { token: '1'.repeat(129) },
            { token: 9 }, { token: '0'.repeat(63) },
          ]) {
            expect((await json(page, pattern, { ...proof, ...patch }, token)).status()).toBe(403);
            expect(await ceremony()).toEqual(baseline);
          }
        } finally {
          await route.continue();
        }
      };
      await page.route(pattern, check);
      try {
        const completed = page.waitForResponse(response => response.url() === pattern && response.request().method() === 'POST', { timeout: 120_000 });
        await click();
        expect((await completed).status()).toBe(200);
      } finally {
        await page.unroute(pattern, check);
      }
      return proof;
    };
    expect((await page.goto(path)).status()).toBe(200);
    const initial = await sessions();
    expect(initial).toHaveLength(1);
    expect(await keys()).toEqual([]);
    const start = `${path}/register/start`;
    const state = () => query("SELECT purpose FROM passkey_ceremonies WHERE account_id=$1", [owner.id]);
    await noCsrf(start, { password: owner.password }, state, []);
    await reserve(6);
    for (const { other, token } of foreign) {
      expect((await json(other, start, { password: owner.password }, token)).status()).toBe(403);
      expect(await state()).toEqual([]);
    }
    const ownToken = await keyPage.getAttribute('data-csrf');
    for (const invalid of ['', 'x'.repeat(1025), 1, `${owner.password}x`]) {
      expect((await json(page, start, { password: invalid }, ownToken)).status()).toBe(403);
      expect(await state()).toEqual([]);
    }
    await passwordField.fill('Wrong local password!');
    await register.click();
    await expect(localStatus).toHaveText(await localStatus.getAttribute('data-error'));
    await reserve(2);
    await passwordField.fill(owner.password);
    const registration = await browserFinish('register', () => register.click());
    await expect(authenticate).toBeVisible();
    expect(await state()).toEqual([]);
    expect(await sessions()).toEqual([]);
    const credential = await keys();
    expect(credential).toEqual([{ id: expect.any(String) }]);
    await reserve(1);
    expect((await json(page, `${path}/register/finish`, registration, await keyPage.getAttribute('data-csrf'))).status()).toBe(403);
    expect(await keys()).toEqual(credential);

    const authStart = `${path}/authenticate/start`;
    await noCsrf(authStart, {}, sessions, []);
    await reserve(7);
    for (const { other, token } of foreign) {
      expect((await json(other, authStart, {}, token)).status()).toBe(403);
      expect(await sessions()).toEqual([]);
    }
    for (const invalid of [null, '', 1, { account: owner.id }]) {
      expect((await json(page, authStart, invalid, await keyPage.getAttribute('data-csrf'))).status()).toBe(403);
      expect(await sessions()).toEqual([]);
    }
    const assertion = await browserFinish('authenticate', () => authenticate.click());
    await expect(register).toBeVisible();
    const signedIn = await sessions();
    expect(signedIn).toHaveLength(1);
    expect(signedIn).not.toEqual(initial);
    await reserve(1);
    expect((await json(page, `${path}/authenticate/finish`, assertion, await keyPage.getAttribute('data-csrf'))).status()).toBe(403);
    expect(await sessions()).toEqual(signedIn);

    const id = credential[0].id;
    const remove = `${path}/${id}/remove`;
    await noCsrf(remove, { password: owner.password }, keys, credential);
    await reserve(7);
    for (const { other, token, actor } of foreign) {
      expect((await json(other, remove, { password: actor.password }, token)).status()).toBe(403);
      expect(await keys()).toEqual(credential);
    }
    for (const invalid of [null, '', 1, 'x'.repeat(1025)]) {
      expect((await json(page, remove, { password: invalid }, await keyPage.getAttribute('data-csrf'))).status()).toBe(403);
      expect(await keys()).toEqual(credential);
    }
    await passwordField.fill(owner.password);
    await page.locator(`[data-remove="${id}"]`).click();
    await expect(authenticate).toBeVisible();
    expect(await keys()).toEqual([]);
    expect(await sessions()).toEqual([]);
  } finally {
    await cdp.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId });
    await cdp.detach();
    await Promise.all(others.map(context => context.close()));
  }
});

test('account termination queues once and a fresh owner session cancels the pending request', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const path = `/${lang}/account/privacy/delete`;
  const contexts = [];
  try {
    const owner = await seededPlayer(page, lang);
    const outsiders = [];
    for (const role of [undefined, 'content_editor']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const other = await context.newPage();
      await seededPlayer(other, lang, role);
      await other.goto(`/${lang}/account/privacy`);
      outsiders.push({ other, token: (await prepare(other, formSelector)).fields._csrf });
    }
    const sudo = async (screen, account) => {
      expect((await screen.goto(`/${lang}/account/sudo`)).status()).toBe(200);
      await submit(screen, formSelector, { current_password: account.password });
      await expect(screen).toHaveURL(new RegExp(`/${lang}/password/change$`));
      expect((await screen.goto(path)).status()).toBe(200);
    };
    const state = (account = owner) => query(
      `SELECT id,cancelled_at IS NOT NULL AS cancelled,finalized_at IS NOT NULL AS finalized,
              scheduled_for>clock_timestamp() AND cancelled_at IS NULL AS grace
       FROM account_terminations WHERE account_id=$1 ORDER BY id`,
      [account.id],
    );
    const crossAccount = async (url, fields, expected, baseline) => {
      for (const { other, token } of outsiders) {
        expect((await sendForm(other, url, { ...fields, _csrf: token })).status()).toBe(expected);
        expect(await state()).toEqual(baseline);
      }
    };
    const validation = lang === 'pl'
      ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    await sudo(page, owner);
    const create = 'main form:has([name=confirmation])';
    const values = { current_password: owner.password, confirmation: 'TERMINATE' };
    const request = await prepare(page, create, values);
    expect(await state()).toEqual([]);
    await rejected({ page, ...request, snapshot: state, baseline: [] });
    await crossAccount(request.url, request.fields, 403, []);
    await invalid({ page, ...request, snapshot: state, baseline: [], message: validation, samples: [
      { name: 'empty confirmation', patch: { confirmation: '' } },
      { name: 'confirmation beyond the exact keyword boundary', patch: { confirmation: 'TERMINATE ' } },
    ] });
    const sampleContext = await browser.newContext({
      baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
      viewport: info.project.use.viewport,
    });
    contexts.push(sampleContext);
    const sample = await sampleContext.newPage();
    const sampleOwner = await seededPlayer(sample, lang);
    await sudo(sample, sampleOwner);
    const negative = await prepare(sample, create, { current_password: sampleOwner.password });
    await invalid({ page: sample, ...negative, snapshot: () => state(sampleOwner), baseline: [], message: validation, samples: [
      { name: 'overlong confirmation', patch: { confirmation: 'x'.repeat(513) } },
      { name: 'numeric confirmation', patch: { confirmation: '1' } },
    ] });
    await submit(page, create, values);
    await expect(page).toHaveURL(new RegExp(`/${lang}/login$`));
    const pending = [{ id: expect.any(String), cancelled: false, finalized: false, grace: true }];
    expect(await state()).toEqual(pending);
    expect(await query('SELECT session_id FROM web_sessions WHERE account_id=$1', [owner.id])).toEqual([]);
    expect((await sendForm(page, request.url, request.fields)).status()).toBe(403);
    expect(await state()).toEqual(pending);

    await login(page, lang, owner);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    await sudo(page, owner);
    const cancel = 'main form[action$="/cancel"]';
    const cancellation = await prepare(page, cancel, { current_password: owner.password });
    await rejected({ page, ...cancellation, snapshot: state, baseline: pending });
    await crossAccount(cancellation.url, cancellation.fields, 404, pending);
    for (const [current_password, status] of [['', 422], ['x'.repeat(1025), 422], ['1', 403], [`${owner.password}x`, 403]]) {
      const refused = await sendForm(page, cancellation.url, { ...cancellation.fields, current_password });
      expect(refused.status()).toBe(status);
      const denied = lang === 'pl' ? 'Nie masz uprawnień do wyświetlenia tej strony.'
        : 'You do not have permission to view this page.';
      expect(await refused.text()).toContain(status === 422 ? validation : denied);
      expect(await state()).toEqual(pending);
    }
    await submit(page, cancel, { current_password: owner.password });
    await expect(page).toHaveURL(new URL(path, cancellation.url).href);
    await expect(page.locator(cancel)).toHaveCount(0);
    const cancelled = [{ ...pending[0], cancelled: true, grace: false }];
    expect(await state()).toEqual(cancelled);
    expect((await sendForm(page, cancellation.url, cancellation.fields)).status()).toBe(404);
    expect(await state()).toEqual(cancelled);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('other players and unrelated staff cannot name an owner in account settings', async ({ page, browser }, info) => {
  test.setTimeout(420_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const owner = await seededPlayer(page, lang);
    const outsiders = [];
    for (const role of [undefined, 'content_editor']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const actor = await context.newPage();
      const account = await seededPlayer(actor, lang, role);
      await actor.goto(`/${lang}/account/privacy`);
      outsiders.push({ actor, account, token: (await prepare(actor, formSelector)).fields._csrf });
    }
    const ids = [owner.id, ...outsiders.map(actor => actor.account.id)];
    const state = async () => ({
      privacy: await query(
        'SELECT id,public_account_association,public_status_visible,locale FROM accounts WHERE id=ANY($1::uuid[]) ORDER BY id',
        [ids],
      ),
      badges: await query('SELECT account_id,badge_keys FROM account_badge_selections WHERE account_id=ANY($1::uuid[]) ORDER BY account_id', [ids]),
      consents: await query(
        "SELECT account_id,document_kind,granted FROM account_consents WHERE account_id=ANY($1::uuid[]) AND document_kind IN ('play_session','personalised_ads') ORDER BY account_id,document_kind",
        [ids],
      ),
      notify: await query(
        'SELECT account_id,category,email_enabled,portal_enabled FROM account_notification_prefs WHERE account_id=ANY($1::uuid[]) ORDER BY account_id,category',
        [ids],
      ),
      places: await query('SELECT account_id,participating FROM pd_places_preferences WHERE account_id=ANY($1::uuid[]) ORDER BY account_id', [ids]),
    });
    const baseline = await state();
    const check = async (url, fields, methods = ['POST']) => {
      for (const method of methods) {
        for (const { actor, token } of outsiders) {
          const response = await sendForm(actor, url, { ...fields, _csrf: token, account_id: owner.id }, method);
          expect(response.status(), `${method} ${new URL(url).pathname}`).toBe(422);
          expect(await state()).toEqual(baseline);
        }
      }
    };

    await page.goto(`/${lang}/account/privacy`);
    const privacy = await prepare(page, formSelector, { public_account_association: '1', public_status_visible: '1' });
    await check(privacy.url, privacy.fields, ['POST', 'PUT']);

    for (const path of [`/${lang}/account/badges`, '/account/badges']) {
      expect((await page.goto(path)).status()).toBe(200);
      const { url, fields } = await prepare(page, formSelector, { badge: 'account-age' });
      await check(url, fields);
      await expect(page.locator('main fieldset [role=status]')).toHaveText(lang === 'pl' ? 'Niedostępne' : 'Unavailable');
      await expect(page.locator('select[name=profile] option')).toHaveCount(1);
      await expect(page.locator('select[name=ranking] option')).toHaveCount(1);
    }

    expect((await page.goto(`/${lang}/account/notifications`)).status()).toBe(200);
    const prefs = await prepare(page, formSelector, { news_email: 'true' });
    await check(prefs.url, prefs.fields);

    expect((await page.goto(`/${lang}/account/consents`)).status()).toBe(200);
    for (const kind of ['play_session', 'personalised_ads']) {
      const consent = await prepare(page, `main form:has([name=kind][value=${kind}])`, { granted: 'true' });
      await check(consent.url, consent.fields);
    }
    await check(new URL(`/${lang}/atlas/places/participation`, privacy.url).href, { participating: 'true' });
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

for (const kind of ['coins', 'premium_days']) {
  test(`gift issuance and redemption of ${kind} persist one audited batch and grant`, async ({ page, browser }, info) => {
    test.setTimeout(600_000);
    const lang = info.project.metadata.locale;
    const contexts = [];
    try {
      const playerAccount = await seededPlayer(page, lang);
      const actorPages = [];
      for (const role of ['platform_admin', undefined, 'content_editor']) {
        const context = await browser.newContext({
          baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
          viewport: info.project.use.viewport,
        });
        contexts.push(context);
        const screen = await context.newPage();
        const account = await seededPlayer(screen, lang, role);
        await screen.goto(`/${lang}/account/privacy`);
        actorPages.push({ screen, account, csrf: (await prepare(screen, formSelector)).fields._csrf });
      }
      const [admin, ...wrong] = actorPages;
      const path = `/${lang}/admin/store/codes`;
      expect((await admin.screen.goto(path)).status()).toBe(200);
      const now = Math.floor(Date.now() / 1000);
      const coins = kind === 'coins';
      const amount = coins ? 37 : 5;
      const values = {
        amount: String(amount), count: '1', valid_from: String(now - 60), valid_until: String(now + 3600),
        note: `Disposable mutation matrix issuance ${randomUUID()}`,
      };
      await admin.screen.locator('main select[name=kind]').selectOption(kind);
      const { url, fields } = await prepare(admin.screen, formSelector, values);
      const batchState = () => query(
        `SELECT b.kind,b.amount::text,b.max_per_code,b.max_per_account,b.created_by,
           (SELECT count(*)::int FROM gift_codes WHERE batch_id=b.id) AS codes,
           (SELECT count(*)::int FROM audit_log WHERE request_id=b.id AND action='store.codes.generated') AS audit
         FROM code_batches b WHERE b.id=$1`,
        [fields.request_id],
      );
      expect(await batchState()).toEqual([]);
      await rejected({ page: admin.screen, url, fields, snapshot: batchState, baseline: [] });
      for (const other of wrong) {
        expect((await sendForm(other.screen, url, { ...fields, _csrf: other.csrf })).status()).toBe(403);
        expect(await batchState()).toEqual([]);
      }
      const validation = lang === 'pl' ? 'Sprawdź żądanie i spróbuj ponownie.' : 'Check the request and try again.';
      await invalid({
        page: admin.screen, url, fields, message: validation, snapshot: batchState, baseline: [],
        samples: [
          { name: 'empty note', patch: { note: '' } },
          { name: 'overlong note', patch: { note: 'x'.repeat(501) } },
          { name: 'wrong amount type', patch: { amount: 'not-an-integer' } },
          { name: 'too many codes', patch: { count: '10001' } },
        ],
      });

      const issued = admin.screen.waitForResponse(response => response.url() === url && response.request().method() === 'POST'
        && response.status() === 200);
      await submit(admin.screen, formSelector, values);
      const issuance = await (await issued).json();
      expect(issuance.batch_id).toBe(fields.request_id);
      expect(issuance.codes.length).toBe(1);
      const state = [{
        kind, amount: String(amount), max_per_code: 1, max_per_account: 1,
        created_by: admin.account.id, codes: 1, audit: 1,
      }];
      expect(await batchState()).toEqual(state);
      const repeated = await sendForm(admin.screen, url, fields);
      expect(repeated.status()).toBe(200);
      const repeat = await repeated.json();
      expect(repeat.batch_id).toBe(fields.request_id);
      expect(repeat.codes.length).toBe(0);
      expect(await batchState()).toEqual(state);

      const redemptionPath = `/${lang}/shop/codes/redeem`;
      const profile = `/${lang}/shop/coins`;
      expect((await page.goto(profile)).status()).toBe(200);
      const firstPaymentForm = 'main form[action$="/shop/coins"]';
      const csrf = (await prepare(page, firstPaymentForm)).fields._csrf;
      const redemptionUrl = new URL(redemptionPath, page.url()).href;
      const operation = uuid7();
      const redeem = (screen, csrf, data) => screen.request.post(redemptionUrl, {
        data, headers: {
          origin: new URL(redemptionUrl).origin, 'accept-language': lang,
          ...(csrf === undefined ? {} : { 'x-csrf-token': csrf }),
        },
        maxRedirects: 0,
      });
      const data = { code: issuance.codes[0], operation };
      const redemptions = () => query(
        `SELECT r.id,r.account_id,r.idempotency_key,w.available_delta::text AS credit,
               p.duration_days AS premium_days,p.effective_until
         FROM code_redemptions r JOIN gift_codes c ON c.id=r.code_id
         LEFT JOIN wallet_entries w ON w.operation_id=r.id
         LEFT JOIN premium_ledger p ON p.request_id=r.idempotency_key AND p.account_id=r.account_id
         WHERE c.batch_id=$1 ORDER BY r.id`,
        [fields.request_id],
      );
      for (const token of [undefined, 'invalid']) {
        expect((await redeem(page, token, data)).status()).toBe(403);
        expect(await redemptions()).toEqual([]);
      }
      const badInputs = [
        { code: '' }, { code: 'x'.repeat(65) }, { code: 30 },
        { operation: '00000000-0000-0000-0000-000000000000' },
      ];
      for (const [index, sample] of badInputs.entries()) {
        const { screen, csrf: validationToken } = wrong[index % wrong.length];
        const response = await redeem(screen, validationToken, { ...data, ...sample });
        expect(response.status()).toBe(422);
        expect(await response.text()).toContain(validation);
        expect(await redemptions()).toEqual([]);
      }

      const accepted = await redeem(page, csrf, data);
      expect(accepted.status()).toBe(200);
      const receipt = await accepted.json();
      expect(receipt.amount).toBe(amount);
      const effects = await redemptions();
      expect(effects).toEqual([{
        id: receipt.redemption_id, account_id: playerAccount.id, idempotency_key: operation,
        credit: coins ? String(amount) : null, premium_days: coins ? null : amount,
        effective_until: coins ? null : expect.any(Date),
      }]);
      if (coins) {
        expect((await page.goto(`/${lang}/shop/history`)).status()).toBe(200);
        await expect(page.locator('main dl dd').first()).toHaveText(String(amount));
      } else {
        expect((await page.goto(`/${lang}/panel/premium`)).status()).toBe(200);
        await expect(page.locator('main')).toContainText(effects[0].effective_until.toISOString().slice(0, 10));
      }
      for (const other of wrong) {
        expect((await redeem(other.screen, other.csrf, { ...data, operation: uuid7() })).status()).toBe(403);
        expect(await redemptions()).toEqual(effects);
      }
      const replay = await redeem(page, csrf, data);
      expect(replay.status()).toBe(200);
      expect((await replay.json()).redemption_id).toBe(receipt.redemption_id);
      expect(await redemptions()).toEqual(effects);
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });
}

for (const prefix of ['locale', 'unprefixed']) {
  test(`password reset ${prefix} aliases deliver one capability and consume it once`, async ({ page, browser }, info) => {
    test.setTimeout(600_000);
    const lang = info.project.metadata.locale;
    const routes = prefix === 'locale' ? `/${lang}` : '';
    const contexts = [];
    const newActor = async role => {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      return { screen, account: await seededPlayer(screen, lang, role) };
    };
    try {
      const { screen: priorSession, account: owner } = await newActor();
      const outsiders = [];
      for (const role of [undefined, 'content_editor']) {
        const { screen } = await newActor(role);
        await screen.goto(`/${lang}/account/privacy`);
        outsiders.push({ screen, token: (await prepare(screen, formSelector)).fields._csrf });
      }
      const snapshot = async () => ({
        reset: await query(
          'SELECT used_at IS NOT NULL AS used,last_reset_at IS NOT NULL AS completed FROM account_password_resets WHERE account_id=$1',
          [owner.id],
        ),
        sessions: await query('SELECT session_id FROM web_sessions WHERE account_id=$1', [owner.id]),
        generation: (await query('SELECT native_security_generation FROM accounts WHERE id=$1', [owner.id]))[0].native_security_generation,
      });
      const budget = async (policy, remaining) => {
        await expect.poll(async () => {
          const [row] = await query(
            "SELECT COALESCE(max(count),0)::int AS count FROM web_rate_limits WHERE limiter_name=$1 AND window_start=date_trunc('minute',clock_timestamp())",
            [policy],
          );
          return row.count;
        }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(remaining);
      };
      const signedRefused = async (url, fields, baseline) => {
        for (const { screen, token } of outsiders) {
          const denied = await sendForm(screen, url, { ...fields, _csrf: token });
          expect(denied.status()).toBe(302);
          expect(denied.headers().location).toBe(`/${lang}/`);
          expect(await snapshot()).toEqual(baseline);
        }
      };
      const localizedNotice = lang === 'pl'
        ? 'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.'
        : 'If an account exists for that email, a password reset link has been sent.';
      const localizedRefusal = lang === 'pl'
        ? 'Link jest nieprawidłowy lub wygasł albo hasła nie są identyczne. W razie potrzeby poproś o nowy link.'
        : 'This password reset link is invalid or expired, or the passwords do not match. Request a new link if needed.';
      const forgot = `/${lang}/forgot-password`;
      expect((await page.goto(forgot)).status()).toBe(200);
      const requestValues = { email: owner.email };
      const request = await prepare(page, formSelector, requestValues);
      const first = new URL(`${routes}/forgot-password`, request.url).href;
      const baseline = await snapshot();
      expect(baseline.reset).toEqual([]);
      expect(baseline.sessions).toHaveLength(1);
      await rejected({ page, url: first, fields: request.fields, snapshot, baseline });
      await budget('identity-password-recovery-source', 3);
      await signedRefused(first, request.fields, baseline);
      for (const email of ['', 'a'.repeat(256) + '@local.test', '123', 'a@']) {
        const denied = await sendForm(page, first, { ...request.fields, email });
        expect(denied.status()).toBe(302);
        expect(denied.headers().location).toBe(forgot);
        expect((await page.goto(forgot)).status()).toBe(200);
        await expect(page.getByRole('status')).toHaveText(localizedNotice);
        expect(await snapshot()).toEqual(baseline);
      }
      await useFormMethod(page, request.url, 'POST', () => submit(page, formSelector, requestValues), first);
      await expect(page.getByRole('status')).toHaveText(localizedNotice);
      const pending = { ...baseline, reset: [{ used: false, completed: false }] };
      expect(await snapshot()).toEqual(pending);
      const mail = await mailLink(page, owner.email, /https:\/\/localhost:\d+\/(?:pl|en)\/reset-password\/[a-f0-9]{64}/, 120_000);
      expect((await page.goto(mail)).status()).toBe(200);
      const newPassword = password();
      await seedPasswordRange(newPassword);
      const values = { email: owner.email, password: newPassword, password_confirmation: newPassword };
      const change = await prepare(page, formSelector, values);
      const reset = new URL(`${routes}/reset-password`, change.url).href;
      await rejected({ page, url: reset, fields: change.fields, snapshot, baseline: pending });
      await budget('identity-password-recovery-source', 3);
      await budget('identity-password-reset', 0);
      await signedRefused(reset, change.fields, pending);
      for (const patch of [
        { email: '' }, { token: 'x'.repeat(65) }, { token: 'type' },
        { password: 'a'.repeat(9), password_confirmation: 'a'.repeat(9) },
      ]) {
        const denied = await sendForm(page, reset, { ...change.fields, ...patch });
        expect(denied.status()).toBe(302);
        const next = new URL(denied.headers().location, reset);
        expect(next.pathname.startsWith(`/${lang}/reset-password`)).toBe(true);
        expect((await page.goto(next.href)).status()).toBe(200);
        await expect(page.getByRole('alert')).toHaveText(localizedRefusal);
        expect(await snapshot()).toEqual(pending);
      }
      await budget('identity-password-reset', 3);
      expect((await page.goto(mail)).status()).toBe(200);
      await useFormMethod(page, change.url, 'POST', () => submit(page, formSelector, values), reset);
      await expect(page).toHaveURL(new RegExp(`/${lang}/login$`));
      const updated = await snapshot();
      expect(updated.reset).toEqual([{ used: true, completed: true }]);
      expect(updated.sessions).toEqual([]);
      expect(Number(updated.generation)).toBe(Number(pending.generation) + 1);
      await priorSession.goto(`/${lang}/account/sessions`);
      await expect(priorSession).toHaveURL(new RegExp(`/${lang}/login$`));
      expect((await page.goto(mail)).status()).toBe(200);
      const replay = await prepare(page, formSelector, values);
      const refused = await sendForm(page, reset, replay.fields);
      expect(refused.status()).toBe(302);
      expect((await page.goto(refused.headers().location)).status()).toBe(200);
      await expect(page.getByRole('alert')).toHaveText(localizedRefusal);
      expect(await snapshot()).toEqual(updated);
      await login(page, lang, { ...owner, password: newPassword });
      await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });
}

test('wiki editorial forms persist bilingual revisions and each review action under the correct authority', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const editor = await seededPlayer(page, lang, 'content_editor');
    const wrong = [];
    for (const role of [undefined, 'support']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      await seededPlayer(screen, lang, role);
      await screen.goto(`/${lang}/account/privacy`);
      wrong.push({ screen, csrf: (await prepare(screen, formSelector)).fields._csrf });
    }
    const slug = `matrix-wiki-${randomUUID()}`;
    const content = {
      en_title: 'Matrix editorial guide', en_slug: slug, en_summary: 'Bilingual editorial guide.',
      en_markdown: '## Guide details\n\nDrafted through the staff form.',
      pl_title: 'Poradnik redakcyjny', pl_slug: slug, pl_summary: 'Poradnik w dwóch językach.',
      pl_markdown: '## Szczegóły poradnika\n\nTreść z formularza zespołu.',
      note: 'Local mutation matrix editorial qualification.',
    };
    const snapshot = () => query(
      `SELECT g.id,g.status,g.version::int,g.revision_version::int,g.entity_key,
         (SELECT jsonb_agg(jsonb_build_object('version',v.version,'locale',v.locale,'title',v.title,
            'editor',v.editor,'source',v.source_version) ORDER BY v.version,v.locale)
          FROM wiki_revisions v WHERE v.guide_id=g.id) AS revisions,
         ARRAY(SELECT action FROM wiki_reviews WHERE guide_id=g.id ORDER BY id) AS reviews
       FROM wiki_guides g WHERE EXISTS(SELECT 1 FROM wiki_revisions r WHERE r.guide_id=g.id AND r.slug=$1)`,
      [slug],
    );
    const validation = lang === 'pl' ? 'Sprawdź dane.' : 'Check the input.';
    const check = async (selector, values, samples) => {
      const prepared = await prepare(page, selector, values);
      const baseline = await snapshot();
      await rejected({ page, ...prepared, snapshot, baseline });
      for (const { screen, csrf } of wrong) {
        expect((await sendForm(screen, prepared.url, { ...prepared.fields, _csrf: csrf })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      await invalid({ page, ...prepared, samples, message: validation, snapshot, baseline });
      return prepared;
    };
    const draftSamples = [
      { name: 'empty English title', patch: { en_title: '' } },
      { name: 'overlong Polish title', patch: { pl_title: 'x'.repeat(201) } },
      { name: 'wrong version type', patch: { lock_version: 'text' } },
      { name: 'invalid slug boundary', patch: { en_slug: '-invalid' } },
    ];
    const versionSamples = [
      { name: 'empty version', patch: { lock_version: '' } },
      { name: 'overlong version', patch: { lock_version: '1'.repeat(80) } },
      { name: 'wrong version type', patch: { lock_version: 'text' } },
      { name: 'first version above signed 64-bit limit', patch: { lock_version: '9223372036854775808' } },
    ];
    const phase = {
      draft: lang === 'pl' ? 'Szkic' : 'Draft',
      in_review: lang === 'pl' ? 'W przeglądzie' : 'In review',
      published: lang === 'pl' ? 'Opublikowany' : 'Published',
      archived: lang === 'pl' ? 'Zarchiwizowany' : 'Archived',
    };
    const article = `/${lang}/wiki/articles/${slug}`;
    const create = 'main form[action="/admin/wiki/articles"]';
    expect((await page.goto('/admin/wiki/articles/create')).status()).toBe(200);
    await check(create, content, draftSamples);
    expect(await snapshot()).toEqual([]);
    await submit(page, create, content);
    await expect(page).toHaveURL(/\/admin\/wiki\/articles\/[a-f0-9-]+\/edit$/);
    await expect(page.locator('main h1 + p')).toContainText(phase.draft);
    const edit = new URL(page.url()).pathname;
    const id = edit.split('/').at(-2);
    const revisions = (version, en, source = null) => ['en', 'pl'].map(locale => ({
      version, locale, title: locale === 'en' ? en : content.pl_title, editor: editor.id, source,
    }));
    let row = { id, status: 'draft', version: 1, revision_version: 1, entity_key: null,
      revisions: revisions(1, content.en_title), reviews: [] };
    expect(await snapshot()).toEqual([row]);
    expect((await page.request.get(article)).status()).toBe(404);

    const update = `main form[action="/admin/wiki/articles/${id}"]`;
    const title = 'Revised editorial guide';
    const changed = await check(update, { en_title: title }, draftSamples);
    await submit(page, update, { en_title: title });
    await expect(page.locator(`${update} [name=en_title]`)).toHaveValue(title);
    row = { ...row, version: 2, revision_version: 2,
      revisions: [...row.revisions, ...revisions(2, title)] };
    expect(await snapshot()).toEqual([row]);
    expect((await sendForm(page, changed.url, changed.fields)).status()).toBe(409);
    expect(await snapshot()).toEqual([row]);

    expect((await page.goto(edit.replace(/\/edit$/, '/revisions'))).status()).toBe(200);
    const restore = 'main form[action$="/revisions/1/restore"]';
    const first = `main article:has(h2:text-is("${content.en_title} (en) · 1")) ${restore.replace('main ', '')}`;
    const restoring = await check(first, {}, versionSamples);
    await submit(page, first);
    await expect(page.locator(`${update} [name=en_title]`)).toHaveValue(content.en_title);
    row = { ...row, version: 3, revision_version: 3,
      revisions: [...row.revisions, ...revisions(3, content.en_title, 1)] };
    expect(await snapshot()).toEqual([row]);
    expect((await sendForm(page, restoring.url, restoring.fields)).status()).toBe(409);
    expect(await snapshot()).toEqual([row]);

    for (const [action, status] of [
      ['submit-review', 'in_review'], ['return-draft', 'draft'], ['submit-review', 'in_review'],
      ['publish', 'published'], ['unpublish', 'draft'], ['archive', 'archived'],
    ]) {
      const selector = `main form[action$="/${action}"]`;
      const transition = await check(selector, {}, versionSamples);
      await submit(page, selector);
      await expect(page.locator('main h1 + p')).toContainText(phase[status]);
      row = { ...row, status, version: row.version + 1, reviews: [...row.reviews, action] };
      expect(await snapshot()).toEqual([row]);
      const visibility = await page.request.get(article);
      expect(visibility.status()).toBe(status === 'published' ? 200 : 404);
      if (status === 'published') {
        expect((await page.goto(article)).status()).toBe(200);
        await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? content.pl_title : content.en_title);
        await page.goto(edit);
      }
      expect((await sendForm(page, transition.url, transition.fields)).status()).toBe(409);
      expect(await snapshot()).toEqual([row]);
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('editorial media forms persist verified bytes and remove one asset through each alias', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const editor = await seededPlayer(page, lang, 'content_editor');
    const outsiders = [];
    for (const role of [undefined, 'support']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const other = await context.newPage();
      await seededPlayer(other, lang, role);
      await other.goto(`/${lang}/account/privacy`);
      outsiders.push({ other, token: (await prepare(other, formSelector)).fields._csrf });
    }
    const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAZAAAAAUCAYAAACj6Wu2AAAAaUlEQVR4nO3VMQ0AIQDAQMxh5LXhEzR8F0Jyw+3dOua3NgD8NW4HAPAmAwEgMRAAEgMBIDEQABIDASAxEAASAwEgMRAAEgMBIDEQABIDASAxEAASAwEgMRAAEgMBIDEQABIDASAxEACSAymFjKpAKJtZAAAAAElFTkSuQmCC', 'base64');
    const image = { name: 'local.png', mimeType: 'image/png', buffer: png };
    const path = `/admin/media?lang=${lang}`;
    const upload = 'main form[action="/admin/media"]';
    const origin = new URL(info.project.use.baseURL).origin;
    const target = new URL('/admin/media', origin).href;
    const invalidImage = lang === 'pl'
      ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    const invalidID = lang === 'pl'
      ? 'Sprawdź żądanie i spróbuj ponownie.'
      : 'Check the request and try again.';
    for (const method of ['POST', 'DELETE']) {
      expect((await page.goto(path)).status()).toBe(200);
      const alt = `Matrix verified image ${randomUUID()}`;
      const snapshot = () => query(
        'SELECT id,alt_text,mime_type,width,height,octet_length(content)>0 AS stored,thumbnail IS NOT NULL AS thumb FROM media WHERE alt_text=$1',
        [alt],
      );
      const token = await page.locator(`${upload} [name=_csrf]`).inputValue();
      const send = (screen, csrf, multipart) => screen.request.post(target, {
        multipart, headers: {
          origin, 'accept-language': lang, ...(csrf === undefined ? {} : { 'x-csrf-token': csrf }),
        }, maxRedirects: 0,
      });
      const uploadAudit = () => query(
        "SELECT target FROM audit_log WHERE actor=$1 AND action='editorial_media.uploaded' ORDER BY created_at,id",
        [editor.id],
      );
      const before = await uploadAudit();
      expect(await snapshot()).toEqual([]);
      for (const badToken of [undefined, 'invalid']) {
        expect((await send(page, badToken, { image, alt_text: alt })).status()).toBe(403);
        expect(await snapshot()).toEqual([]);
        expect(await uploadAudit()).toEqual(before);
      }
      for (const { other, token } of outsiders) {
        expect((await send(other, token, { image, alt_text: alt })).status()).toBe(403);
        expect(await snapshot()).toEqual([]);
        expect(await uploadAudit()).toEqual(before);
      }
      for (const input of [
        { image, alt_text: '' }, { image, alt_text: 'a'.repeat(501) },
        { image: { ...image, name: 'local.jpeg', mimeType: 'image/jpeg' }, alt_text: alt },
        { image: { ...image, buffer: Buffer.alloc(0) }, alt_text: alt },
      ]) {
        const refusal = await send(page, token, input);
        expect(refusal.status()).toBe(422);
        expect(await refusal.text()).toBe(invalidImage);
        expect(await snapshot()).toEqual([]);
        expect(await uploadAudit()).toEqual(before);
      }

      await page.locator(`${upload} [name=image]`).setInputFiles(image);
      await page.locator(`${upload} [name=alt_text]`).fill(alt);
      await page.locator(`${upload} button`).click();
      const visible = page.getByRole('img', { name: alt, exact: true });
      await expect(visible).toBeVisible();
      const [stored] = await snapshot();
      expect(stored).toEqual({
        id: expect.any(String), alt_text: alt, mime_type: 'image/png',
        width: 400, height: 20, stored: true, thumb: true,
      });
      expect(await uploadAudit()).toEqual([...before, { target: stored.id }]);
      const thumb = await visible.getAttribute('src');
      const content = thumb.replace(/\/thumbnail$/, '/content');
      const fetched = await page.request.get(content);
      expect(fetched.status()).toBe(200);
      expect(fetched.headers()['content-type']).toBe('image/png');
      expect((await fetched.body()).length).toBeGreaterThan(0);

      const selector = `main form[action="/admin/media/${stored.id}/delete"]`;
      const deletion = await prepare(page, selector);
      const revoke = method === 'DELETE' ? new URL(`/admin/media/${stored.id}`, origin).href : deletion.url;
      await rejected({ page, url: revoke, fields: deletion.fields, snapshot, baseline: [stored], method });
      for (const { other, token } of outsiders) {
        expect((await sendForm(other, revoke, { ...deletion.fields, _csrf: token }, method)).status()).toBe(403);
        expect(await snapshot()).toEqual([stored]);
      }
      for (const invalid of ['%20', 'a'.repeat(37), 'not-an-id', '00000000-0000-0000-0000-00000000000']) {
        const malformed = new URL(`/admin/media/${invalid}${method === 'POST' ? '/delete' : ''}`, origin);
        const refusal = await page.request.fetch(malformed.href, {
          method, form: deletion.fields, headers: { origin, 'accept-language': lang }, maxRedirects: 0,
        });
        expect(refusal.status()).toBe(400);
        expect(await refusal.text()).toBe(invalidID);
        expect(await snapshot()).toEqual([stored]);
      }
      await useFormMethod(page, deletion.url, method, () => submit(page, selector), revoke);
      await expect(page).toHaveURL(new URL('/admin/media', origin).href);
      await expect(visible).toHaveCount(0);
      expect(await snapshot()).toEqual([]);
      expect((await sendForm(page, revoke, deletion.fields, method)).status()).toBe(404);
      expect((await page.request.get(content)).status()).toBe(404);
      expect(await snapshot()).toEqual([]);
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

for (const prefix of ['locale', 'unprefixed']) {
  test(`email change and delivered capabilities use the ${prefix} aliases exactly once`, async ({ page, browser }, info) => {
    test.setTimeout(600_000);
    const lang = info.project.metadata.locale;
    const contexts = [];
    const route = path => prefix === 'locale' ? `/${lang}${path}` : path;
    const open = async role => {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, locale: lang, ignoreHTTPSErrors: true,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      return { screen, account: await seededPlayer(screen, lang, role) };
    };
    const activateRecovery = async (screen, owner) => {
      expect((await screen.goto(`/${lang}/account/recovery-key`)).status()).toBe(200);
      await submit(screen, formSelector, { current_password: owner.password });
      const key = (await screen.locator('main .op-code').innerText()).trim();
      expect(key.length).toBeGreaterThan(0);
      await submit(screen, formSelector, { current_password: owner.password, recovery_key: key });
      return key;
    };
    const status = lang === 'pl' ? 'Odmowa lub nieprawidłowe dane.' : 'Request refused or invalid input.';
    const pendingStatus = lang === 'pl'
      ? 'Sprawdź oba adresy e-mail, aby potwierdzić lub anulować zmianę.'
      : 'Check both email addresses for confirmation and cancellation links.';
    const requestPath = '/account/security/email';
    try {
      const owner = await seededPlayer(page, lang);
      const wrong = [await open(), await open('content_editor')];
      const sample = await open();
      const key = await activateRecovery(page, owner);
      const sampleKey = await activateRecovery(sample.screen, sample.account);
      const state = (account = owner) => query(
        `SELECT a.email,a.native_security_generation::int AS generation,
           (SELECT jsonb_agg(jsonb_build_object('id',e.id,'email',e.new_email,'kind',e.kind,
               'immediate',e.immediate,'confirmed',e.confirmed_at IS NOT NULL,
               'applied',e.applied_at IS NOT NULL,'reverted',e.reverted_at IS NOT NULL,
               'cancelled',e.cancelled_at IS NOT NULL)
               ORDER BY e.id) FROM account_email_changes e WHERE e.account_id=a.id) AS changes
         FROM accounts a WHERE a.id=$1`,
        [account.id],
      );
      const email = `matrix-changed-${randomUUID()}@local.test`;
      const values = { email, email_confirmation: email, current_password: owner.password, proof: key, recovery: true };
      const opening = async (screen, fields) => {
        expect((await screen.goto(route(requestPath))).status()).toBe(200);
        return prepare(screen, formSelector, fields);
      };
      const before = await state();
      expect(before).toEqual([{ email: owner.email, generation: expect.any(Number), changes: null }]);
      const request = await opening(page, values);
      await rejected({ page, ...request, snapshot: state, baseline: before });
      for (const actor of wrong) {
        const theirBefore = await state(actor.account);
        const theirForm = await opening(actor.screen, {});
        const refused = await sendForm(actor.screen, request.url, { ...request.fields, _csrf: theirForm.fields._csrf });
        expect(refused.status()).toBe(302);
        await actor.screen.goto(new URL(refused.headers().location, request.url).href);
        await expect(actor.screen.getByRole('status')).toHaveText(status);
        expect(await state()).toEqual(before);
        expect(await state(actor.account)).toEqual(theirBefore);
      }
      const samples = [
        { fields: { email: '', email_confirmation: '' }, main: true },
        { fields: { email: `${'a'.repeat(255)}@local.test` } },
        { fields: { recovery: 'broken' } },
        { fields: { email_confirmation: `mismatch-${email}` }, main: true },
      ];
      for (const { fields, main } of samples) {
        const screen = main ? page : sample.screen;
        const actor = main ? owner : sample.account;
        const actorBefore = await state(actor);
        const offer = await opening(screen, {
          ...values, current_password: actor.password, proof: main ? key : sampleKey, ...fields,
        });
        const refused = await sendForm(screen, offer.url, offer.fields);
        expect(refused.status()).toBe(302);
        await screen.goto(new URL(refused.headers().location, offer.url).href);
        await expect(screen.getByRole('status')).toHaveText(status);
        expect(await state(actor)).toEqual(actorBefore);
      }
      // The owner has three allowed submissions in this hour, including the two validation cases above.
      await opening(page, values);
      await submit(page, formSelector, values);
      await expect(page.getByRole('status')).toHaveText(pendingStatus);
      const pending = { id: expect.any(String), email, kind: 'primary', immediate: true,
        confirmed: false, applied: false, reverted: false, cancelled: false };
      expect(await state()).toEqual([{ ...before[0], changes: [pending] }]);
      const delivered = {};
      for (const [operation, mailbox] of [['confirm', email], ['revert', owner.email]]) {
        const pattern = new RegExp(`https://localhost:\\d+/email-change/${operation}/[a-f0-9]{64}`);
        const mailed = new URL(await mailLink(page, mailbox, pattern, 120_000));
        delivered[operation] = route(mailed.pathname);
      }
      for (const [operation, changes, nextEmail] of [
        ['confirm', { confirmed: true, applied: true }, email],
        ['revert', { confirmed: true, applied: true, reverted: true }, owner.email],
      ]) {
        await expect.poll(async () => {
          const [row] = await query(
            `SELECT COALESCE(max(count),0)::int AS count FROM web_rate_limits
             WHERE limiter_name IN ('identity-email-token','identity-anonymous-source')
             AND window_start=date_trunc('minute',clock_timestamp())`,
          );
          return row.count;
        }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(2);
        const current = await state();
        const target = delivered[operation];
        expect((await page.goto(target)).status()).toBe(200);
        const form = await prepare(page, formSelector);
        await rejected({ page, ...form, snapshot: state, baseline: current });
        for (const actor of wrong) {
          expect((await actor.screen.goto(`/${lang}/account/privacy`)).status()).toBe(200);
          const { fields } = await prepare(actor.screen, formSelector);
          const forged = randomBytes(32).toString('hex');
          const url = new URL(route(`/email-change/${operation}/${forged}`), form.url).href;
          const refused = await sendForm(actor.screen, url, fields);
          expect(refused.status()).toBe(302);
          expect((await actor.screen.goto(refused.headers().location)).status()).toBe(200);
          await expect(actor.screen.getByRole('status')).toHaveText(status);
          expect(await state()).toEqual(current);
        }
        const malformed = ['x', 'a'.repeat(65), 'z'.repeat(64), 'a'.repeat(63)];
        for (const capability of malformed) {
          const url = new URL(route(`/email-change/${operation}/${capability}`), form.url).href;
          const refused = await sendForm(page, url, form.fields);
          expect(refused.status()).toBe(302);
          await page.goto(encodeURI(refused.headers().location));
          await expect(page.getByRole('status')).toHaveText(status);
          expect(await state()).toEqual(current);
        }
        expect((await page.goto(target)).status()).toBe(200);
        const sent = page.waitForResponse(response => response.url() === form.url && response.request().method() === 'POST');
        await submit(page, formSelector);
        const response = await sent;
        expect(response.status()).toBe(302);
        expect(response.headers().location).toBe(`/${lang}/login`);
        await expect(page).toHaveURL(new RegExp(`/${lang}/login$`));
        const applied = { email: nextEmail, generation: current[0].generation + 1,
          changes: [{ ...pending, ...changes }] };
        expect(await state()).toEqual([applied]);
        expect((await page.goto(target)).status()).toBe(200);
        await submit(page, formSelector);
        await expect(page.getByRole('status')).toHaveText(status);
        expect(await state()).toEqual([applied]);
      }
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });
}

for (const prefix of ['locale', 'unprefixed']) {
  test(`registration and verification ${prefix} aliases create one verified account through the real work challenge`, async ({ page, browser }, info) => {
    test.setTimeout(600_000);
    const lang = info.project.metadata.locale;
    const alias = prefix === 'locale' ? `/${lang}` : '';
    const origin = new URL(info.project.use.baseURL).origin;
    const contexts = [];
    try {
      const wrong = [];
      for (const role of [undefined, 'content_editor']) {
        const context = await browser.newContext({
          baseURL: origin, ignoreHTTPSErrors: true, locale: lang,
          viewport: info.project.use.viewport,
        });
        contexts.push(context);
        const actor = await context.newPage();
        await seededPlayer(actor, lang, role);
        await actor.goto(`/${lang}/account/privacy`);
        wrong.push({ actor, token: (await prepare(actor, formSelector)).fields._csrf });
      }
      const budget = async (name, remaining) => {
        await expect.poll(async () => {
          const [row] = await query(
            "SELECT COALESCE(max(count),0)::int AS count FROM web_rate_limits WHERE limiter_name=$1 AND window_start=date_trunc('minute',clock_timestamp())",
            [name],
          );
          return row.count;
        }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(remaining);
      };
      const email = `matrix-registration-${randomUUID()}@local.test`;
      const secret = password();
      await seedPasswordRange(secret);
      const state = () => query(
        'SELECT id,status,email_verified_at IS NOT NULL AS verified FROM accounts WHERE lower(email)=$1',
        [email],
      );
      expect((await page.goto(`/${lang}/register`)).status()).toBe(200);
      const rendered = page.locator(formSelector);
      const terms = await rendered.locator('[name=terms]').getAttribute('value');
      const privacy = await rendered.locator('[name=privacy]').getAttribute('value');
      const values = { email, password: secret, password_confirmation: secret };
      const form = await prepare(page, formSelector, { ...values, terms, privacy });
      const register = new URL(`${alias}/register`, form.url).href;
      const challenge = new URL(`${alias}/register/abuse/challenge`, form.url).href;
      expect(await state()).toEqual([]);
      await rejected({ page, url: register, fields: form.fields, snapshot: state, baseline: [] });
      await budget('identity-registration-attempt', 2);
      await budget('identity-registration', 1);
      for (const { actor, token } of wrong) {
        const response = await sendForm(actor, register, { ...form.fields, _csrf: token, email: '' });
        expect(response.status()).toBe(303);
        expect(response.headers().location).toBe('/account');
        expect(await state()).toEqual([]);
      }
      await budget('identity-registration-attempt', 0);
      await budget('identity-registration', 0);
      const invalid = lang === 'pl' ? 'Podaj poprawny adres e-mail.' : 'Enter a valid e-mail address.';
      for (const malformed of ['', 'x'.repeat(255) + '@local.test', '123', 'a@']) {
        const response = await sendForm(page, register, { ...form.fields, email: malformed });
        expect(response.status()).toBe(422);
        expect(await response.text()).toContain(invalid);
        expect(await state()).toEqual([]);
      }

      await budget('identity-registration-challenge', 4);
      const binding = createHash('sha256').update(form.fields._csrf).digest();
      const works = () => query('SELECT count(*)::int AS pending FROM registration_work WHERE form_hash=$1', [binding]);
      const baseline = await works();
      for (const csrf of [undefined, 'invalid']) {
        const response = await page.request.post(challenge, {
          headers: { origin, ...(csrf === undefined ? {} : { 'x-csrf-token': csrf }) },
        });
        expect(response.status()).toBe(403);
        expect(await works()).toEqual(baseline);
        expect(await state()).toEqual([]);
      }
      const renderedChallenge = new URL('/register/abuse/challenge', form.url).href;
      const rewriteChallenge = route => route.continue({ url: challenge });
      const registration = async route => {
        if (route.request().method() === 'POST') {
          try {
            expect(await works()).toEqual([{ pending: baseline[0].pending + 1 }]);
          } finally {
            await route.continue({ url: register });
          }
        } else {
          await route.continue();
        }
      };
      if (renderedChallenge !== challenge) await page.route(renderedChallenge, rewriteChallenge);
      await page.route(form.url, registration);
      try {
        const issuance = page.waitForResponse(response =>
          new URL(response.url()).pathname.endsWith('/register/abuse/challenge') && response.request().method() === 'POST'
        );
        await submit(page, formSelector, { ...values, terms: true, privacy: true });
        expect((await issuance).status()).toBe(200);
        await expect(page).toHaveURL(new RegExp(`/${lang}/register/check-inbox$`), { timeout: 270_000 });
      } finally {
        await page.unroute(form.url, registration);
        await page.unroute(renderedChallenge, rewriteChallenge);
      }
      const [pending] = await state();
      expect(pending).toEqual({ id: expect.any(String), status: 'pending_verification', verified: false });
      expect(await works()).toEqual(baseline);

      const inbox = `/${lang}/register/check-inbox`;
      const resendPage = `/${lang}/verify-email/resend`;
      expect((await page.goto(resendPage)).status()).toBe(200);
      const resend = await prepare(page, formSelector, { email });
      const targetResend = new URL(`${alias}/verify-email/resend`, resend.url).href;
      const delivery = async () => ({
        account: await state(),
        jobs: (await query("SELECT count(*)::int AS count FROM outbox WHERE kind='email.verify' AND payload->>'account_id'=$1", [pending.id]))[0].count,
        links: (await query("SELECT count(*)::int AS count FROM account_email_tokens WHERE account_id=$1 AND used_at IS NULL", [pending.id]))[0].count,
      });
      const starting = { account: [pending], jobs: 1, links: 1 };
      expect(await delivery()).toEqual(starting);
      await rejected({ page, url: targetResend, fields: resend.fields, snapshot: delivery, baseline: starting });
      await budget('identity-registration', 3);
      for (const { actor, token } of wrong) {
        const ignored = await sendForm(actor, targetResend, { ...resend.fields, _csrf: token });
        expect(ignored.status()).toBe(303);
        expect(ignored.headers().location).toBe(inbox);
        expect(await delivery()).toEqual(starting);
      }
      await budget('identity-registration', 0);
      for (const email of ['', 'x'.repeat(255) + '@local.test', '123', 'a@']) {
        const response = await sendForm(page, targetResend, { ...resend.fields, email });
        expect(response.status()).toBe(303);
        expect(response.headers().location).toBe(inbox);
        expect(await delivery()).toEqual(starting);
      }
      await expect.poll(async () => {
        const [row] = await query(
          "SELECT updated_at<=clock_timestamp()-interval '60 seconds' AS available FROM account_email_tokens WHERE account_id=$1",
          [pending.id],
        );
        return row?.available;
      }, { timeout: 65_000, intervals: [1000] }).toBe(true);
      await budget('identity-registration', 3);
      expect((await page.goto(resendPage)).status()).toBe(200);
      const repeat = await prepare(page, formSelector, { email });
      await useFormMethod(page, repeat.url, 'POST', () => submit(page, formSelector, { email }), targetResend);
      await expect(page).toHaveURL(new URL(inbox, resend.url).href);
      const resent = { ...starting, jobs: 2 };
      expect(await delivery()).toEqual(resent);
      expect((await sendForm(page, targetResend, repeat.fields)).status()).toBe(303);
      expect(await delivery()).toEqual(resent);

      const link = await mailLink(page, email, /https:\/\/localhost:\d+\/(?:pl|en)\/verify-email\?token=(?:3D)?[a-f0-9]{64}/, 120_000);
      await budget('identity-verification-source', 3);
      expect((await page.goto(link)).status()).toBe(200);
      const verification = await prepare(page, formSelector);
      const target = new URL(`${alias}/verify-email`, verification.url);
      target.search = new URL(verification.url).search;
      const pendingState = [pending];
      await rejected({ page, url: target.href, fields: verification.fields, snapshot: state, baseline: pendingState });
      for (const attempt of ['', 'x'.repeat(513), 'not-a-token', '0'.repeat(63)]) {
        const rejectedToken = new URL(target);
        rejectedToken.searchParams.set('token', attempt);
        const refused = await sendForm(page, rejectedToken.href, verification.fields);
        expect(refused.status()).toBe(422);
        expect(await refused.text()).toContain(lang === 'pl'
          ? 'Ten link jest nieprawidłowy lub wygasł.'
          : 'This link is invalid or has expired.');
        expect(await state()).toEqual(pendingState);
      }
      await useFormMethod(page, verification.url, 'POST', () => submit(page, formSelector), target.href);
      await expect(page.locator('main')).toContainText(lang === 'pl'
        ? 'Twój adres e-mail został zweryfikowany.'
        : 'Your e-mail has been verified.');
      const verified = [{ ...pending, verified: true, status: 'active' }];
      expect(await state()).toEqual(verified);
      const used = () => query(
        'SELECT count(*)::int AS count FROM account_email_tokens WHERE account_id=$1 AND used_at IS NOT NULL',
        [pending.id],
      );
      expect(await used()).toEqual([{ count: 1 }]);
      expect((await sendForm(page, target.href, verification.fields)).status()).toBe(403);
      expect(await state()).toEqual(verified);
      expect(await used()).toEqual([{ count: 1 }]);
      await login(page, lang, values);
      await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });
}

test('world catalogue actions require a supported imported version and catalogue permissions', async ({ page, browser }, info) => {
  test.setTimeout(420_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    await seededPlayer(page, lang, 'platform_admin');
    const outsiders = [];
    for (const role of [undefined, 'content_editor']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      await seededPlayer(screen, lang, role);
      await screen.goto(`/${lang}/account/privacy`);
      outsiders.push({ screen, token: (await prepare(screen, formSelector)).fields._csrf });
    }
    await page.goto(`/${lang}/account/privacy`);
    const csrf = (await prepare(page, formSelector)).fields._csrf;
    const path = '/admin/world';
    expect((await page.goto(path)).status()).toBe(200);
    await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Świat gry' : 'Game world');
    await expect(page.locator('main p code').first()).toBeEmpty();
    await expect(page.locator('main form[method=post]')).toHaveCount(0);
    const snapshot = async () => ({
      active: await query('SELECT version_id,previous_version_id FROM world_active_version WHERE singleton'),
      catalog: await query('SELECT snapshot_id FROM world_catalog_publication WHERE singleton'),
      ready: await query("SELECT id FROM world_versions WHERE status='ready' ORDER BY id"),
      writes: await query("SELECT id FROM audit_log WHERE action LIKE 'world.%' ORDER BY id"),
    });
    const baseline = await snapshot();
    expect(baseline).toEqual({
      active: [{ version_id: null, previous_version_id: null }], catalog: [{ snapshot_id: null }],
      ready: [], writes: [],
    });
    const version = uuid7();
    const url = new URL(path, page.url()).href;
    for (const [action, allowed] of [
      ['publish', 'true'], ['visibility', 'true'], ['visibility', 'false'], ['names', 'true'],
    ]) {
      const fields = { _csrf: csrf, action, version, kind: 'items', key: 'fixture',
        allowed, pl: 'Przedmiot testowy', en: 'Test item' };
      await rejected({ page, url, fields, snapshot, baseline });
      for (const { screen, token } of outsiders) {
        expect((await sendForm(screen, url, { ...fields, _csrf: token })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      const refused = await sendForm(page, url, fields);
      expect(refused.status()).toBe(404);
      expect(await snapshot()).toEqual(baseline);
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('admin control step-up consumes one TOTP and approval requires an accepted dry run', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const staff = await seededPlayer(page, lang);
    expect(await query(
      "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='content_editor' RETURNING role_id",
      [staff.id],
    )).toHaveLength(1);
    expect((await page.goto(`/${lang}/mfa`)).status()).toBe(200);
    await submit(page, 'main form[action$="/mfa/enroll"]', { password: staff.password });
    const totp = new TOTP({ secret: await page.locator('main .op-prose').innerText(), algorithm: 'SHA1', digits: 6, period: 30 });
    await submit(page, 'main form[action$="/mfa/confirm"]', { password: staff.password, code: totp.generate() });
    const recovery = await page.locator('textarea[readonly]').first().inputValue();
    await signInWithRecoveryCode(page, lang, staff, recovery);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));

    const wrong = [];
    for (const role of [undefined, 'game_viewer']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      const actor = await seededPlayer(screen, lang, role);
      await screen.goto(`/${lang}/account/privacy`);
      wrong.push({ screen, actor, csrf: (await prepare(screen, formSelector)).fields._csrf });
    }
    const state = () => query(
      `SELECT account_id,coalesce(data ? 'admin_control_step_up',false) AS proof,
              data#>>'{admin_control_request_step_up,request_id}' AS bound
       FROM web_sessions WHERE account_id=ANY($1) ORDER BY account_id`,
      [[staff.id, ...wrong.map(other => other.actor.id)]],
    );
    const baseline = await state();
    expect(baseline).toHaveLength(3);
    expect(baseline.every(row => row.proof === false && row.bound === null)).toBe(true);
    const requestId = uuid7();
    const stepUp = `/admin/control/step-up?request_id=${requestId}`;
    expect((await page.goto(stepUp)).status()).toBe(200);
    const ready = await prepare(page, formSelector, { code: '000000' });
    await rejected({ page, ...ready, snapshot: state, baseline });
    for (const { screen, csrf } of wrong) {
      expect((await sendForm(screen, ready.url, { ...ready.fields, _csrf: csrf })).status()).toBe(403);
      expect(await state()).toEqual(baseline);
    }
    const message = lang === 'pl' ? 'Nieprawidłowe dane' : 'Invalid input';
    await invalid({ page, ...ready, message, snapshot: state, baseline, samples: [
      { name: 'empty TOTP', patch: { code: '' } },
      { name: 'overlong TOTP', patch: { code: '0'.repeat(65) } },
      { name: 'wrong type', patch: { code: 'abcdef' } },
      { name: 'one digit below the boundary', patch: { code: '00000' } },
    ] });
    const available = async () => query(
      "SELECT last_step < floor(extract(epoch FROM clock_timestamp())/30)::bigint AS ready FROM account_mfa_totp WHERE account_id=$1",
      [staff.id],
    );
    await expect.poll(available, { timeout: 35_000 }).toEqual([{ ready: true }]);
    const code = totp.generate();
    const result = page.waitForResponse(response => response.url() === ready.url && response.request().method() === 'POST');
    await submit(page, formSelector, { code });
    expect((await result).status()).toBe(200);
    const completed = lang === 'pl'
      ? 'Potwierdzenie zapisane. Wróć do pierwotnej operacji i wyślij ją ponownie.'
      : 'Confirmation recorded. Return to the original operation and submit it again.';
    await expect(page.locator('main h1')).toHaveText(completed);
    const proved = await state();
    expect(proved).toEqual(baseline.map(row => row.account_id === staff.id ? { ...row, proof: true, bound: requestId } : row));
    expect((await sendForm(page, ready.url, { ...ready.fields, code })).status()).toBe(403);
    expect(await state()).toEqual(proved);

    const missing = uuid7();
    const approval = new URL(`/admin/control/approvals/${missing}`, ready.url).href;
    const durable = () => query('SELECT id FROM admin_control_approvals WHERE preview_id=$1', [missing]);
    expect((await page.goto(stepUp)).status()).toBe(200);
    const csrf = (await prepare(page, formSelector)).fields._csrf;
    const unavailable = await page.goto(approval);
    expect(unavailable.status()).toBe(403);
    await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Odmowa wykonania operacji' : 'This action is refused');
    await expect(page.locator(formSelector)).toHaveCount(0);
    await rejected({ page, url: approval, fields: { _csrf: csrf }, snapshot: durable, baseline: [] });
    for (const { screen, csrf } of wrong) {
      expect((await sendForm(screen, approval, { _csrf: csrf })).status()).toBe(403);
      expect(await durable()).toEqual([]);
    }
    for (const id of ['-', 'x'.repeat(64), '1', missing.slice(0, -1)]) {
      const response = await sendForm(page, approval.replace(missing, id), { _csrf: csrf });
      expect(response.status()).toBe(422);
      expect(await response.text()).toContain(message);
      expect(await durable()).toEqual([]);
    }
    expect((await sendForm(page, approval, { _csrf: csrf })).status()).toBe(403);
    expect(await durable()).toEqual([]);
    expect(await state()).toEqual(proved);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('staff refunds a real uncredited test payment through one independent approval', { tag: '@uncredited-payment' }, async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const origin = new URL(info.project.use.baseURL).origin;
  try {
    const owner = await seededPlayer(page, lang);
    const staff = await foreignActors(browser, info.project, lang, contexts, 'content_editor');
    const reviewers = [];
    for (let index = 0; index < 2; index += 1) {
      const context = await browser.newContext({
        baseURL: origin, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      reviewers.push({ screen, account: await seededPlayer(screen, lang, 'platform_admin') });
    }
    const [proposer, reviewer] = reviewers;
    await page.goto(`/${lang}/shop/coins`);
    const packageForm = 'main form[action$="/shop/coins"]';
    const order = await prepare(page, packageForm);
    await submit(page, packageForm);
    await expect(page).toHaveURL(/\/shop\/test-checkout\/[a-f0-9-]+$/);
    const intent = new URL(page.url()).pathname.split('/').at(-1);
    expect(await query('SELECT payment_intent_id FROM store_web_topups WHERE operation_id=$1 AND account_id=$2',
      [order.fields.operation, owner.id])).toEqual([{ payment_intent_id: intent }]);
    await submit(page, 'main form[action*="/shop/test-checkout/"]');
    await expect(page.locator('main')).toContainText(lang === 'pl'
      ? 'Płatność przyjęta, środki zostaną zaksięgowane po przetworzeniu.'
      : 'Payment accepted; funds will be credited after processing.');
    const payment = () => query(
      `SELECT state,provider,amount::int,currency,
              (SELECT count(*)::int FROM wallet_entries WHERE operation_id=$1) AS credits,
              (SELECT count(*)::int FROM payment_events WHERE intent_id=$1) AS events
       FROM payment_intents WHERE id=$1 AND account_id=$2`, [intent, owner.id],
    );
    const paid = [{ state: 'succeeded', provider: 'test', amount: 250, currency: 'PLN', credits: 0, events: 1 }];
    expect(await payment()).toEqual(paid);

    const panel = '/admin/commerce';
    expect((await proposer.screen.goto(panel)).status()).toBe(200);
    const form = 'main form[action="/admin/commerce/refund"]';
    const data = { target: intent, reason: 'refund_requested' };
    const offer = await prepare(proposer.screen, form, data);
    const operation = offer.fields.request_id;
    const snapshot = async () => ({
      payment: await payment(),
      review: await query(
        "SELECT actor_id,approved_by,target_id,command->>'kind' AS kind,result FROM commerce_operations WHERE id=$1", [operation],
      ),
      dispatch: await query(
        `SELECT state,provider,provider_reference IS NOT NULL AS submitted,
           (SELECT count(*)::int FROM audit_log WHERE request_id=$1 AND action='commerce.refund.dispatched') AS confirmed
         FROM commerce_refunds WHERE operation_id=$1 AND intent_id=$2`, [operation, intent],
      ),
    });
    const validation = lang === 'pl' ? 'Nieprawidłowe dane. Sprawdź identyfikatory, kwotę i powód.'
      : 'Invalid input. Check the identifiers, amount and reason.';
    const reasonValidation = lang === 'pl' ? 'Wybierz zatwierdzony powód.' : 'Choose an approved reason.';
    const initial = { payment: paid, review: [], dispatch: [] };
    const refusal = async (screen, action, baseline) => {
      await rejected({ page: screen, ...action, snapshot, baseline });
      for (const { screen: outsider, csrf } of staff) {
        expect((await sendForm(outsider, action.url, { ...action.fields, _csrf: csrf })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
    };
    await refusal(proposer.screen, offer, initial);
    await invalid({ page: proposer.screen, ...offer, snapshot, baseline: initial, message: validation, samples: [
      { name: 'wrong target identifier format', patch: { target: 'text' } },
    ] });
    await invalid({ page: proposer.screen, ...offer, snapshot, baseline: initial, message: reasonValidation, samples: [
      { name: 'empty reason', patch: { reason: '' } },
      { name: 'overlong reason code', patch: { reason: 'x'.repeat(501) } },
      { name: 'unrecognized short reason code', patch: { reason: 'x'.repeat(9) } },
      { name: 'approval reason in a proposal', patch: { reason: 'refund_verified' } },
    ] });
    await proposer.screen.locator(form).locator('select[name=reason]').selectOption(data.reason);
    await submit(proposer.screen, form, { target: data.target });
    await expect(proposer.screen).toHaveURL(new URL(panel, origin).href);
    const approval = `main form[action="${panel}/approvals/${operation}"]`;
    await expect(proposer.screen.locator(approval)).toBeVisible();
    await expect(proposer.screen.locator(`${approval} button`)).toHaveCount(0);
    const proposal = { actor_id: proposer.account.id, approved_by: null, target_id: intent, kind: 'refund', result: null };
    const pending = { ...initial, review: [proposal] };
    expect(await snapshot()).toEqual(pending);
    expect((await sendForm(proposer.screen, offer.url, offer.fields)).status()).toBe(303);
    expect(await snapshot()).toEqual(pending);

    expect((await reviewer.screen.goto(panel)).status()).toBe(200);
    const approvalReason = 'refund_verified';
    const decision = await prepare(reviewer.screen, approval, { reason: approvalReason });
    await refusal(reviewer.screen, decision, pending);
    const own = await prepare(proposer.screen, approval, { reason: approvalReason });
    expect((await sendForm(proposer.screen, decision.url, own.fields)).status()).toBe(403);
    expect(await snapshot()).toEqual(pending);
    for (const value of ['_', 'x'.repeat(128), '123', operation.slice(0, -1)]) {
      const denied = await sendForm(reviewer.screen, decision.url.replace(operation, value), decision.fields);
      expect(denied.status()).toBe(422);
      expect(await denied.text()).toContain(validation);
      expect(await snapshot()).toEqual(pending);
    }
    await invalid({ page: reviewer.screen, ...decision, snapshot, baseline: pending, message: reasonValidation, samples: [
      { name: 'withdrawal reason in an approval', patch: { reason: 'refund_withdrawn' } },
    ] });
    await reviewer.screen.locator(approval).locator('select[name=reason]').selectOption(approvalReason);
    await submit(reviewer.screen, approval);
    await expect(reviewer.screen).toHaveURL(new URL(panel, origin).href);
    await expect(reviewer.screen.locator(approval)).toHaveCount(0);
    const refunded = {
      ...pending,
      review: [{ ...proposal, approved_by: reviewer.account.id,
        result: { intent, queued: true, amount: 250, currency: 'PLN' },
      }],
      dispatch: [{ state: 'done', provider: 'test', submitted: true, confirmed: 1 }],
    };
    await expect.poll(snapshot, { timeout: 60_000, intervals: [1000] }).toEqual(refunded);
    expect((await sendForm(reviewer.screen, decision.url, decision.fields)).status()).toBe(303);
    expect(await snapshot()).toEqual(refunded);

    const [record] = await query('SELECT provider_reference FROM payment_intents WHERE id=$1', [intent]);
    const event = JSON.stringify({
      event_id: `matrix-refund:${intent}`, intent_id: intent, provider_reference: record.provider_reference,
      amount: { minor_units: 250, currency: 'PLN' }, state: 'refunded',
    });
    const providerKey = readFileSync(process.env.PORTAL_E2E_REFUND_KEY_FILE);
    const signature = createHmac('sha256', providerKey).update(event).digest('base64');
    const settle = () => page.request.post('/__test/payments/webhook', {
      data: event, headers: { 'content-type': 'application/json', 'x-test-payment-signature': signature },
    });
    expect((await settle()).status()).toBe(204);
    await page.reload();
    await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Płatność zwrócona.' : 'Payment refunded.');
    const verified = { ...refunded, payment: [{ ...paid[0], state: 'refunded', events: 2 }] };
    expect(await snapshot()).toEqual(verified);
    expect((await settle()).status()).toBe(204);
    expect(await snapshot()).toEqual(verified);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('editorial page forms save, refresh and delete a bilingual publication with immutable revisions', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const editor = await seededPlayer(page, lang, 'content_editor');
    const wrong = [];
    for (const role of [undefined, 'support']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      await seededPlayer(screen, lang, role);
      await screen.goto(`/${lang}/account/privacy`);
      wrong.push({ screen, csrf: (await prepare(screen, formSelector)).fields._csrf });
    }
    const slug = `e2e-page-${randomUUID()}`;
    const snapshot = () => query(
      `SELECT e.locale,e.title,e.body,e.revision::int,e.source_revision::int,e.published_at IS NOT NULL AS published
       FROM editorial_pages e JOIN pages p ON p.id=e.page_id WHERE p.slug=$1 ORDER BY e.locale`,
      [slug],
    );
    const change = async (locale, operation, values) => {
      const path = `/admin/pages/${slug}/${locale}`;
      expect((await page.goto(path)).status()).toBe(200);
      const baseline = await snapshot();
      const form = await prepare(page, formSelector, { ...values, action: operation });
      await rejected({ page, ...form, snapshot, baseline });
      for (const { screen, csrf } of wrong) {
        expect((await sendForm(screen, form.url, { ...form.fields, _csrf: csrf })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      const error = locale === 'pl'
        ? 'Sprawdź żądanie i spróbuj ponownie.' : 'Check the request and try again.';
      for (const patch of [{ title: '' }, { title: 'x'.repeat(201) }, { published_at: 'not-a-date' }, { legal_version: 'q'.repeat(81) }]) {
        const denied = await sendForm(page, form.url, { ...form.fields, ...patch });
        expect(denied.status()).toBe(400);
        expect(await denied.text()).toContain(error);
        expect(await snapshot()).toEqual(baseline);
      }
      if (operation === 'save') await submit(page, formSelector, values);
      else await page.locator(`${formSelector} button[value=delete]`).click();
      await expect(page).toHaveURL(new URL('/admin/pages', form.url).href);
      const after = await snapshot();
      expect((await sendForm(page, form.url, form.fields)).status()).toBe(409);
      expect(await snapshot()).toEqual(after);
      return after;
    };
    const published_at = new Date(Date.now() - 60_000).toISOString();
    const visible = async (locale, expected) => {
      const response = await page.goto(`/${locale}/pages/${slug}`);
      expect(response.status()).toBe(expected ? 200 : 404);
      if (expected) await expect(page.locator('main h1')).toHaveText(expected);
    };
    const en = { title: 'Matrix editorial page', body: 'English source editorial text.', published_at };
    const pl = { title: 'Strona redakcyjna matrix', body: 'Polskie tłumaczenie treści.', published_at };
    expect(await snapshot()).toEqual([]);
    const english = { locale: 'en', title: en.title, body: en.body, revision: expect.any(Number), source_revision: 1, published: true };
    const first = await change('en', 'save', en);
    expect(first).toEqual([english]);
    expect(first[0].revision).toBeGreaterThan(1);
    const [source] = first;
    const polish = { locale: 'pl', title: pl.title, body: pl.body, revision: expect.any(Number), source_revision: source.revision, published: true };
    await visible('en', en.title);
    const translated = await change('pl', 'save', pl);
    expect(translated).toEqual([source, polish]);
    expect(translated[1].revision).toBeGreaterThan(source.revision);
    await visible('pl', pl.title);
    const changed = { ...en, title: 'Matrix revised editorial page' };
    const refreshed = await change('en', 'save', changed);
    const [newSource] = refreshed;
    expect(refreshed).toEqual([{
      ...source, title: changed.title, revision: expect.any(Number), source_revision: source.revision,
    }, translated[1]]);
    expect(newSource.revision).toBeGreaterThan(source.revision);
    await visible('en', changed.title);
    await visible('pl');
    expect(await change('pl', 'save', pl)).toEqual([newSource, {
      ...translated[1], source_revision: newSource.revision,
    }]);
    await visible('pl', pl.title);
    expect(await change('pl', 'delete', {})).toEqual([newSource]);
    await visible('pl');
    await visible('en', changed.title);
    expect(await change('en', 'delete', {})).toEqual([]);
    await visible('en');
    expect(await query(
      "SELECT action,after->>'locale' AS locale FROM audit_log WHERE actor=$1 AND target=$2 ORDER BY created_at,id",
      [editor.id, slug],
    )).toEqual([
      { action: 'cms.page.saved', locale: 'en' }, { action: 'cms.page.saved', locale: 'pl' },
      { action: 'cms.page.saved', locale: 'en' }, { action: 'cms.page.saved', locale: 'pl' },
      { action: 'cms.page.deleted', locale: 'pl' }, { action: 'cms.page.deleted', locale: 'en' },
    ]);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});


test('staff announcement forms keep drafts private and publish the reviewed bilingual version', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const editor = await seededPlayer(page, lang, 'content_editor');
    const wrong = [];
    for (const role of [undefined, 'support']) {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      await seededPlayer(screen, lang, role);
      await screen.goto(`/${lang}/account/privacy`);
      wrong.push({ screen, token: (await prepare(screen, formSelector)).fields._csrf });
    }
    await page.setExtraHTTPHeaders({ 'accept-language': lang });
    const title = `Local announcement ${randomUUID()}`;
    const title_pl = `Lokalne ogłoszenie ${randomUUID()}`;
    const values = {
      title, title_pl, body: 'Current English maintenance detail.',
      body_pl: 'Aktualna informacja o konserwacji.',
      starts_at: new Date(Date.now() - 90_000).toISOString(),
      ends_at: new Date(Date.now() + 600_000).toISOString(),
    };
    const snapshot = () => query(
      `SELECT id,title,title_pl,publication_state,severity,lock_version::int AS version,
              created_by,updated_by,translation_published_at IS NOT NULL AS approved
       FROM announcements WHERE title=$1`, [title],
    );
    const validation = lang === 'pl' ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    const validate = async (patch, invalidFields) => {
      const offer = await prepare(page, formSelector, patch);
      const baseline = await snapshot();
      await rejected({ page, ...offer, snapshot, baseline });
      for (const { screen, token } of wrong) {
        expect((await sendForm(screen, offer.url, { ...offer.fields, _csrf: token })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      for (const invalid of invalidFields) {
        const response = await page.request.post(offer.url, {
          form: { ...offer.fields, ...invalid },
          headers: { origin: new URL(offer.url).origin, 'accept-language': lang },
          maxRedirects: 0,
        });
        expect(response.status()).toBe(422);
        expect(await response.text()).toContain(validation);
        expect(await snapshot()).toEqual(baseline);
      }
      return offer;
    };
    const invalidFields = [{ title: '' }, { body: 'x'.repeat(10001) }, { starts_at: 'text' },
      { ends_at: values.starts_at }];
    expect((await page.goto('/admin/announcements/create')).status()).toBe(200);
    expect(await snapshot()).toEqual([]);
    await validate(values, invalidFields);
    await submit(page, formSelector, values);
    await expect(page).toHaveURL(/\/admin\/announcements\/[a-f0-9-]+\/edit$/);
    await expect(page.locator('[name=title]')).toHaveValue(title);
    const editorPath = new URL(page.url()).pathname;
    const id = editorPath.split('/').at(-2);
    const draft = { id, title, title_pl, publication_state: 'draft', severity: 'info', version: 1,
      created_by: editor.id, updated_by: editor.id, approved: false };
    expect(await snapshot()).toEqual([draft]);
    const ticker = `/${lang}/announcements/ticker`;
    {
      const before = await page.request.get(ticker);
      expect(before.status()).toBe(200);
      expect(await before.text()).not.toContain(lang === 'pl' ? title_pl : title);
    }

    const approval = { translation_published_at: new Date(Date.now() - 60_000).toISOString() };
    const publication = await validate({ ...approval, publication_state: 'published', severity: 'warning' }, invalidFields);
    await page.locator('[name=publication_state]').selectOption('published');
    await page.locator('[name=severity]').selectOption('warning');
    await submit(page, formSelector, approval);
    await expect(page.locator('[name=publication_state]')).toHaveValue('published');
    const published = { ...draft, publication_state: 'published', severity: 'warning', version: 2, approved: true };
    expect(await snapshot()).toEqual([published]);
    for (const [locale, expected] of [['en', title], ['pl', title_pl]]) {
      const response = await page.request.get(`/${locale}/announcements/ticker`);
      expect(response.status()).toBe(200);
      expect(await response.text()).toContain(expected);
    }
    expect((await sendForm(page, publication.url, publication.fields)).status()).toBe(409);
    expect(await snapshot()).toEqual([published]);
    const maintenance = await validate({ severity: 'maintenance' }, invalidFields);
    await page.locator('[name=severity]').selectOption('maintenance');
    await submit(page, formSelector);
    await expect(page.locator('[name=severity]')).toHaveValue('maintenance');
    const maintenanceNotice = { ...published, severity: 'maintenance', version: 3 };
    expect(await snapshot()).toEqual([maintenanceNotice]);
    expect(await (await page.request.get(ticker)).text()).toContain(lang === 'pl' ? title_pl : title);
    expect((await sendForm(page, maintenance.url, maintenance.fields)).status()).toBe(409);
    expect(await snapshot()).toEqual([maintenanceNotice]);
    await page.locator('[name=publication_state]').selectOption('draft');
    await submit(page, formSelector);
    await expect(page.locator('[name=publication_state]')).toHaveValue('draft');
    expect(await snapshot()).toEqual([{ ...maintenanceNotice, publication_state: 'draft', version: 4 }]);
    const retired = await page.request.get(ticker);
    expect(retired.status()).toBe(200);
    expect(await retired.text()).not.toContain(lang === 'pl' ? title_pl : title);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

for (const prefix of ['locale', 'unprefixed']) {
  test(`MFA ${prefix} aliases bind enrollment, emailed proof and remembered-device removal to the owner`, async ({ page, browser }, info) => {
    test.setTimeout(600_000);
    const lang = info.project.metadata.locale;
    const base = prefix === 'locale' ? `/${lang}/mfa` : '/mfa';
    const settings = `/${lang}/mfa`;
    const challengePath = `${settings}/challenge`;
    const contexts = [];
    try {
      const owner = await seededPlayer(page, lang);
      const outsiders = [];
      for (const role of [undefined, 'content_editor']) {
        const context = await browser.newContext({
          baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
          viewport: info.project.use.viewport,
        });
        contexts.push(context);
        const other = await context.newPage();
        await seededPlayer(other, lang, role);
        await other.goto(`/${lang}/account/privacy`);
        outsiders.push({ other, role, token: (await prepare(other, formSelector)).fields._csrf });
      }
      const state = async () => ({
        factor: await query('SELECT confirmed,generation FROM account_mfa_totp WHERE account_id=$1', [owner.id]),
        codes: (await query('SELECT count(*)::int AS n FROM account_recovery_codes WHERE account_id=$1', [owner.id]))[0].n,
        tokens: await query("SELECT purpose,count(*)::int AS count FROM account_mfa_tokens WHERE account_id=$1 GROUP BY purpose ORDER BY purpose", [owner.id]),
        sessions: (await query('SELECT count(*)::int AS n FROM web_sessions WHERE account_id=$1', [owner.id]))[0].n,
      });
      const reserve = async (policy, remaining) => {
        await expect.poll(async () => {
          const [row] = await query(
            "SELECT COALESCE(max(count),0)::int AS count FROM web_rate_limits WHERE limiter_name=$1 AND window_start=date_trunc('minute',clock_timestamp())",
            [policy],
          );
          return row.count;
        }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(remaining);
      };
      const error = lang === 'pl'
        ? 'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.'
        : 'The request could not be verified. Check your details and try again.';
      const refuse = async (form, name, values) => {
        const data = await prepare(page, form, values);
        const url = new URL(`${base}/${name}`, data.url).href;
        const baseline = await state();
        await rejected({ page, url, fields: data.fields, snapshot: state, baseline });
        for (const { other, role, token } of outsiders) {
          const response = await sendForm(other, url, { ...data.fields, _csrf: token });
          const status = name === 'devices' ? (role ? 200 : 403) : 302;
          expect(response.status()).toBe(status);
          if (name === 'devices' && role) expect(await response.text()).toContain(error);
          expect(await state()).toEqual(baseline);
        }
        return { ...data, target: url, baseline };
      };
      const enroll = 'main form[action$="/mfa/enroll"]';
      expect((await page.goto(settings)).status()).toBe(200);
      const enrollment = await refuse(enroll, 'enroll', { password: owner.password });
      expect(enrollment.baseline).toEqual({ factor: [], codes: 0, tokens: [], sessions: 1 });
      await reserve('identity-mfa-enrollment', 0);
      for (const password of ['', 'x'.repeat(1025), '1', `${owner.password}x`]) {
        const response = await sendForm(page, enrollment.target, { ...enrollment.fields, password });
        expect(response.status()).toBe(302);
        expect(response.headers().location).toBe(settings);
        await page.goto(settings);
        await expect(page.getByRole('alert')).toHaveText(error);
        expect(await state()).toEqual(enrollment.baseline);
      }
      await useFormMethod(page, enrollment.url, 'POST', () => submit(page, enroll, { password: owner.password }), enrollment.target);
      const confirm = 'main form[action$="/mfa/confirm"]';
      await expect(page.locator(confirm)).toBeVisible();
      const opened = await state();
      expect(opened).toEqual({ ...enrollment.baseline, factor: [{ confirmed: false, generation: expect.any(String) }] });
      const totp = new TOTP({ secret: await page.locator('main .op-prose').innerText(), algorithm: 'SHA1', digits: 6, period: 30 });
      const verification = await refuse(confirm, 'confirm', { password: owner.password, code: '000000' });
      await reserve('identity-mfa-enrollment', 0);
      for (const code of ['', '1'.repeat(65), 'x', '99999']) {
        const response = await sendForm(page, verification.target, { ...verification.fields, code });
        expect(response.status()).toBe(302);
        expect(response.headers().location).toBe(settings);
        expect(await state()).toEqual(opened);
      }
      const values = { password: owner.password, code: totp.generate() };
      await useFormMethod(page, verification.url, 'POST', () => submit(page, confirm, values), verification.target);
      const recovery = page.locator('textarea[readonly]');
      await expect(recovery).toHaveCount(8);
      const confirmed = await state();
      expect(confirmed).toEqual({
        factor: [{ ...opened.factor[0], confirmed: true }], codes: 8, tokens: [], sessions: 0,
      });
      expect((await sendForm(page, verification.target, verification.fields)).status()).toBe(403);
      expect(await state()).toEqual(confirmed);

      await login(page, lang, owner);
      await expect(page).toHaveURL(new URL(challengePath, page.url()).href);
      const email = 'main form[action$="/mfa/email"]';
      const dispatch = await refuse(email, 'email');
      await useFormMethod(page, dispatch.url, 'POST', () => submit(page, email), dispatch.target);
      await expect(page).toHaveURL(new URL(challengePath, page.url()).href);
      const code = (await mailLink(page, owner.email, /(?:^|\r?\n)[a-f0-9]{64}(?:\r?\n|$)/)).trim();
      expect((await state()).tokens).toEqual([{ purpose: 'challenge', count: 1 }, { purpose: 'email', count: 1 }]);
      const challenge = 'main form[action$="/mfa/challenge"]';
      const proof = await refuse(challenge, 'challenge', { code, remember: 'yes' });
      await reserve('identity-mfa-challenge', 0);
      for (const invalid of ['', 'x'.repeat(65), 'type', '12345']) {
        const response = await sendForm(page, proof.target, { ...proof.fields, code: invalid });
        expect(response.status()).toBe(302);
        expect(response.headers().location).toBe(challengePath);
        await page.goto(challengePath);
        await expect(page.getByRole('alert')).toHaveText(error);
        expect(await state()).toEqual(proof.baseline);
      }
      await useFormMethod(page, proof.url, 'POST', () => submit(page, challenge, { code, remember: true }), proof.target);
      await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
      const signedIn = await state();
      expect(signedIn.sessions).toBe(1);
      expect(signedIn.tokens).toEqual([{ purpose: 'grant', count: 1 }, { purpose: 'trusted', count: 1 }]);
      expect((await sendForm(page, proof.target, proof.fields)).status()).toBe(403);
      expect(await state()).toEqual(signedIn);

      expect((await page.goto(settings)).status()).toBe(200);
      const device = 'main form[action$="/mfa/devices"]';
      const remove = await refuse(device, 'devices');
      for (const value of ['', '0'.repeat(65), '1', '0'.repeat(63)]) {
        const response = await sendForm(page, remove.target, { ...remove.fields, device: value });
        expect(response.status()).toBe(200);
        expect(await response.text()).toContain(error);
        expect(await state()).toEqual(signedIn);
      }
      await useFormMethod(page, remove.url, 'POST', () => submit(page, device), remove.target);
      await expect(page.locator(device)).toHaveCount(0);
      const cleared = { ...signedIn, tokens: [{ purpose: 'grant', count: 1 }] };
      expect(await state()).toEqual(cleared);
      expect((await sendForm(page, remove.target, remove.fields)).status()).toBe(200);
      expect(await state()).toEqual(cleared);
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });
}
for (const [prefix, revokeMethod] of [
  ['locale', 'POST'], ['locale', 'DELETE'], ['unprefixed', 'POST'], ['unprefixed', 'DELETE'],
]) {
  test(`recovery key ${prefix} ${revokeMethod} lifecycle binds activation, recovery and revocation to the owner`, async ({ page, browser }, info) => {
    test.setTimeout(720_000);
    const lang = info.project.metadata.locale;
    const start = `/${lang}/account/recovery-key`;
    const path = prefix === 'locale' ? start : '/account/recovery-key';
    const recoverPath = `/${lang}/recovery-key`;
    const recoveryAlias = prefix === 'locale' ? recoverPath : '/recovery-key';
    const contexts = [];
    const another = async role => {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      return { screen, user: await seededPlayer(screen, lang, role) };
    };
    try {
      const owner = await seededPlayer(page, lang);
      const outsiders = [];
      for (const role of [undefined, 'content_editor']) {
        const { screen } = await another(role);
        await screen.goto(`/${lang}/account/privacy`);
        outsiders.push({ screen, token: (await prepare(screen, formSelector)).fields._csrf });
      }
      const stats = account => async () => ({
        key: await query(
          'SELECT confirmed_at IS NOT NULL AS active,used_at IS NOT NULL AS used,revoked_at IS NOT NULL AS revoked FROM account_recovery_keys WHERE account_id=$1',
          [account.id],
        ),
        events: (await query('SELECT kind FROM account_recovery_key_events WHERE account_id=$1 ORDER BY id', [account.id])).map(row => row.kind),
        generation: (await query('SELECT native_security_generation FROM accounts WHERE id=$1', [account.id]))[0].native_security_generation,
      });
      const checkOwnership = async (url, fields, state, baseline, method = 'POST') => {
        for (const { screen, token } of outsiders) {
          const response = await sendForm(screen, url, { ...fields, _csrf: token }, method);
          expect(response.status()).toBe(method === 'DELETE' ? 303 : 302);
          expect((await screen.goto(response.headers().location)).status()).toBe(200);
          await expect(screen.getByRole('status')).toHaveText(localizedError);
          expect(await state()).toEqual(baseline);
        }
      };
      const localizedError = lang === 'pl'
        ? /Sprawdź|nieprawidłowe|niedostępna/
        : /Check your email|credentials are invalid/;
      const bad = async (screen, url, fields, patches, snapshot, method = 'POST') => {
        const baseline = await snapshot();
        for (const patch of patches) {
          const denied = await sendForm(screen, url, { ...fields, ...patch }, method);
          expect(denied.status()).toBe(method === 'DELETE' ? 303 : 302);
          expect((await screen.goto(denied.headers().location)).status()).toBe(200);
          await expect(screen.getByRole('status')).toHaveText(localizedError);
          expect(await snapshot()).toEqual(baseline);
        }
      };
      const begin = async (screen, account, verify) => {
        expect((await screen.goto(start)).status()).toBe(200);
        const values = { current_password: account.password };
        const contact = await prepare(screen, formSelector, values);
        const target = new URL(path, contact.url).href;
        if (verify) await verify(screen, target, contact.fields);
        await useFormMethod(screen, contact.url, 'POST', () => submit(screen, formSelector, values), target);
        await expect(screen.locator('.op-code')).toBeVisible();
        return (await screen.locator('.op-code').innerText()).trim();
      };

      const generator = await another();
      const sampleState = stats(generator.user);
      await begin(generator.screen, generator.user, async (screen, url, fields) => {
        const baseline = await sampleState();
        await rejected({ page: screen, url, fields, snapshot: sampleState, baseline });
        await checkOwnership(url, fields, sampleState, baseline);
        await bad(screen, url, fields, [
          { current_password: '' }, { current_password: 'x'.repeat(1025) },
          { current_password: '1' }, { current_password: `${generator.user.password}x` },
        ], sampleState);
      });
      expect(await sampleState()).toEqual({ key: [{ active: false, used: false, revoked: false }], events: ['generated'], generation: expect.any(String) });

      const validation = await another();
      const sampleKey = await begin(validation.screen, validation.user);
      expect(sampleKey.length).toBe(51);
      const sampleConfirmation = await prepare(validation.screen, formSelector, {
        current_password: validation.user.password, recovery_key: '0'.repeat(50),
      });
      await bad(validation.screen, new URL(`${path}/confirm`, sampleConfirmation.url).href,
        sampleConfirmation.fields, [{}], stats(validation.user));

      const key = await begin(page, owner);
      const state = stats(owner);
      const pending = await state();
      expect(pending.key).toEqual([{ active: false, used: false, revoked: false }]);
      const confirm = await prepare(page, formSelector, { current_password: owner.password, recovery_key: key });
      const confirmation = new URL(`${path}/confirm`, confirm.url).href;
      await rejected({ page, url: confirmation, fields: confirm.fields, snapshot: state, baseline: pending });
      await checkOwnership(confirmation, confirm.fields, state, pending);
      await bad(page, confirmation, confirm.fields, [
        { recovery_key: '' }, { recovery_key: 'x'.repeat(161) }, { recovery_key: '1' },
      ], state);
      await useFormMethod(page, confirm.url, 'POST', () => submit(page, formSelector, {
        current_password: owner.password, recovery_key: key,
      }), confirmation);
      await expect(page.getByRole('status')).toHaveText(lang === 'pl' ? 'Klucz odzyskiwania jest aktywny.' : 'Your recovery key is active.');
      const active = await state();
      expect(active.key).toEqual([{ active: true, used: false, revoked: false }]);
      expect(active.events).toEqual(['generated', 'confirmed']);

      await logout(page, lang);
      await page.goto(recoverPath);
      const next = password();
      await seedPasswordRange(next);
      const values = { email: owner.email, recovery_key: key, password: next, password_confirmation: next };
      const recovery = await prepare(page, formSelector, values);
      const target = new URL(recoveryAlias, recovery.url).href;
      await rejected({ page, url: target, fields: recovery.fields, snapshot: state, baseline: active });
      // The local browser contexts share one hourly source bucket. Reset the
      // disposable fixture between independent authorization and field cases.
      const freshRecoveryWindow = () => query(
        "DELETE FROM web_rate_limits WHERE limiter_name='identity-recovery-key-use'",
      );
      await freshRecoveryWindow();
      for (const { screen, token } of outsiders) {
        const denied = await sendForm(screen, target, { ...recovery.fields, _csrf: token, recovery_key: randomUUID() });
        expect(denied.status()).toBe(302);
        expect(denied.headers().location).toBe(recoverPath);
        expect(await state()).toEqual(active);
      }
      await freshRecoveryWindow();
      await expect.poll(async () => {
        const [limit] = await query(
          `SELECT COALESCE(max(count),0)::int AS count FROM web_rate_limits
           WHERE limiter_name='identity-anonymous-source' AND window_start=date_trunc('minute',clock_timestamp())`,
        );
        return limit.count;
      }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(5);
      await bad(page, target, recovery.fields, [
        { email: '' }, { email: 'x'.repeat(254) + '@local.test' }, { email: '1' },
        { email: `other-${randomUUID()}@local.test`, password: 'x'.repeat(9), password_confirmation: 'x'.repeat(9) },
      ], state);
      await useFormMethod(page, recovery.url, 'POST', () => submit(page, formSelector, values), target);
      await expect(page).toHaveURL(new RegExp(`/${lang}/login$`));
      const used = await state();
      expect(used.key).toEqual([{ active: true, used: true, revoked: false }]);
      expect(used.events).toEqual(['generated', 'confirmed', 'used']);
      expect(Number(used.generation)).toBe(Number(active.generation) + 1);
      expect((await sendForm(page, target, recovery.fields)).status()).toBe(403);
      expect(await state()).toEqual(used);
      await login(page, lang, { ...owner, password: next });
      await expect(page).toHaveURL(new RegExp(`/${lang}/$`));

      await validation.screen.goto(start);
      const badRevoke = await prepare(validation.screen, formSelector, { current_password: '1' });
      const badRevokeUrl = new URL(revokeMethod === 'POST' ? `${path}/revoke` : path, badRevoke.url).href;
      await bad(validation.screen, badRevokeUrl, badRevoke.fields, [
        {}, { current_password: `${validation.user.password}x` },
      ], stats(validation.user), revokeMethod);

      const revoker = await another();
      await begin(revoker.screen, revoker.user);
      await revoker.screen.goto(start);
      const revocationState = stats(revoker.user);
      const before = await revocationState();
      const revoke = await prepare(revoker.screen, formSelector, { current_password: revoker.user.password });
      const revokeUrl = new URL(revokeMethod === 'POST' ? `${path}/revoke` : path, revoke.url).href;
      await rejected({ page: revoker.screen, url: revokeUrl, fields: revoke.fields, snapshot: revocationState, baseline: before, method: revokeMethod });
      await checkOwnership(revokeUrl, revoke.fields, revocationState, before, revokeMethod);
      await bad(revoker.screen, revokeUrl, revoke.fields, [
        { current_password: '' }, { current_password: 'x'.repeat(1025) },
      ], revocationState, revokeMethod);
      await revoker.screen.locator('[name=current_password]').fill(revoker.user.password);
      const original = new URL(`${start}/revoke`, revoke.url).href;
      await useFormMethod(
        revoker.screen, original, revokeMethod,
        () => revoker.screen.locator('button[formaction$="/revoke"]').click(),
        revokeUrl,
      );
      await expect(revoker.screen).toHaveURL(new URL(start, revoke.url).href);
      const revoked = await revocationState();
      expect(revoked.key).toEqual([{ active: false, used: false, revoked: true }]);
      expect(revoked.events).toEqual(['generated', 'revoked']);
      expect((await sendForm(revoker.screen, revokeUrl, revoke.fields, revokeMethod)).status()).toBe(revokeMethod === 'DELETE' ? 303 : 302);
      expect(await revocationState()).toEqual(revoked);
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });
}

test('legacy support queues preserve actor permissions, private notes and report resolution', async ({ page, browser }, info) => {
  test.setTimeout(900_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const open = async role => {
    const context = await browser.newContext({
      baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
      viewport: info.project.use.viewport,
    });
    contexts.push(context);
    const screen = await context.newPage();
    const account = await seededPlayer(screen, lang, role);
    await screen.goto(`/${lang}/account/privacy`);
    return { screen, account, csrf: (await prepare(screen, formSelector)).fields._csrf };
  };
  try {
    const owner = await seededPlayer(page, lang);
    const staff = await open('support');
    const wrong = [await open(), await open('content_editor')];
    const invalidMessage = lang === 'pl'
      ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    const prepareChecks = async (selector, values, snapshot, samples) => {
      const proposal = await prepare(staff.screen, selector, values);
      const baseline = await snapshot();
      await rejected({ page: staff.screen, ...proposal, snapshot, baseline });
      for (const { screen, csrf } of wrong) {
        expect((await sendForm(screen, proposal.url, { ...proposal.fields, _csrf: csrf })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      await invalid({ page: staff.screen, ...proposal, samples, message: invalidMessage, snapshot, baseline });
      return proposal;
    };
    const createTicket = `/${lang}/support/tickets/create`;
    const subject = `Legacy support matrix ${randomUUID()}`;
    expect((await page.goto(createTicket)).status()).toBe(200);
    await submit(page, formSelector, { subject, body: 'Original matrix support request.' });
    await expect(page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
    const ownerTicket = new URL(page.url()).pathname;
    const ticketId = ownerTicket.split('/').at(-1);
    const staffTicket = `/admin/support/tickets/${ticketId}`;
    expect((await staff.screen.goto(staffTicket)).status()).toBe(200);
    const ticket = () => query(
      `SELECT status,lock_version::int AS version,
         (SELECT jsonb_agg(jsonb_build_object('body',body,'internal',internal,'author',author_kind) ORDER BY id)
          FROM ticket_messages WHERE ticket_id=$1) AS messages
       FROM tickets WHERE id=$1 AND account_id=$2`, [ticketId, owner.id],
    );
    const reply = 'main form:has([name=action][value=reply])';
    let expected = { status: 'open', version: 1, messages: [
      { body: 'Original matrix support request.', internal: false, author: 'user' },
    ] };
    expect(await ticket()).toEqual([expected]);
    for (const internal of [false, true]) {
      const body = internal ? 'Private support matrix comment.' : 'Public support matrix reply.';
      const proposal = await prepareChecks(reply, { body, ...(internal ? { internal: 'true' } : {}) }, ticket, [
        { name: 'empty reply', patch: { body: '' } },
        { name: 'overlong reply', patch: { body: 'x'.repeat(8001) } },
        { name: 'wrong lock version type', patch: { lock_version: 'text' } },
        { name: 'reply below the minimum non-whitespace length', patch: { body: ' ' } },
      ]);
      await submit(staff.screen, reply, { body, internal });
      await expect(staff.screen.getByText(body, { exact: true })).toBeVisible();
      expected = { status: 'waiting_user', version: expected.version + 1,
        messages: [...expected.messages, { body, internal, author: 'staff' }] };
      expect(await ticket()).toEqual([expected]);
      expect((await sendForm(staff.screen, proposal.url, proposal.fields)).status()).toBe(409);
      expect(await ticket()).toEqual([expected]);
      await page.goto(ownerTicket);
      await expect(page.getByText(body, { exact: true })).toHaveCount(internal ? 0 : 1);
    }
    const statusForm = 'main form:has([name=action][value=ticket_status])';
    for (const status of ['waiting_staff', 'waiting_user', 'resolved', 'closed', 'open']) {
      const proposal = await prepareChecks(statusForm, { status }, ticket, [
        { name: 'empty status', patch: { status: '' } },
        { name: 'overlong status', patch: { status: 'x'.repeat(128) } },
        { name: 'wrong version type', patch: { lock_version: 'v1' } },
        { name: 'version below one', patch: { lock_version: '0' } },
      ]);
      await staff.screen.locator(`${statusForm} [name=status]`).selectOption(status);
      await submit(staff.screen, statusForm);
      await expect(staff.screen).toHaveURL(new URL(staffTicket, proposal.url).href);
      expected = { ...expected, status, version: expected.version + 1 };
      expect(await ticket()).toEqual([expected]);
      expect((await sendForm(staff.screen, proposal.url, proposal.fields)).status()).toBe(409);
      expect(await ticket()).toEqual([expected]);
    }

    const makeReport = async () => {
      const target = `Matrix report ${randomUUID()}`;
      expect((await page.goto(`/${lang}/support/reports/create`)).status()).toBe(200);
      await submit(page, formSelector, { target_reference: target, evidence_summary: 'Disposable test evidence.' });
      await expect(page).toHaveURL(/\/support\/reports\/[a-f0-9-]+$/);
      const ownerUrl = new URL(page.url()).pathname;
      return { id: ownerUrl.split('/').at(-1), ownerUrl };
    };
    for (const outcome of ['actioned', 'rejected']) {
      const { id, ownerUrl } = await makeReport();
      const report = () => query(
        'SELECT status,lock_version::int AS version,public_outcome,moderator_notes,assigned_to FROM player_reports WHERE id=$1 AND reporter_account_id=$2',
        [id, owner.id],
      );
      const staffReport = `/admin/moderation/reports/${id}`;
      expect((await staff.screen.goto(staffReport)).status()).toBe(200);
      const edit = 'main form:has([name=action][value=moderate])';
      let baseline = { status: 'submitted', version: 1, public_outcome: null, moderator_notes: null, assigned_to: null };
      for (const status of ['submitted', 'reviewing', outcome, 'closed']) {
        expect(await report()).toEqual([baseline]);
        const values = { public_outcome: `Staff outcome ${status} ${id}`, moderator_notes: 'Private case review.' };
        const proposal = await prepareChecks(edit, { ...values, status }, report, [
          { name: 'empty status', patch: { status: '' } },
          { name: 'overlong public outcome', patch: { public_outcome: 'x'.repeat(4001) } },
          { name: 'wrong version type', patch: { lock_version: 'v1' } },
          { name: 'version below one', patch: { lock_version: '0' } },
        ]);
        await staff.screen.locator(`${edit} [name=status]`).selectOption(status);
        await submit(staff.screen, edit, values);
        await expect(staff.screen).toHaveURL(new URL(staffReport, proposal.url).href);
        baseline = { ...values, status, version: baseline.version + 1, assigned_to: staff.account.id };
        expect(await report()).toEqual([baseline]);
        expect((await sendForm(staff.screen, proposal.url, proposal.fields)).status()).toBe(409);
        expect(await report()).toEqual([baseline]);
        expect((await page.goto(ownerUrl)).status()).toBe(200);
        await expect(page.getByText(values.public_outcome, { exact: true })).toBeVisible();
        await expect(page.getByText(values.moderator_notes, { exact: true })).toHaveCount(0);
      }
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('staff commerce separates submission, approval, cancellation and wallet effects', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const db = pool();
  try {
    const owner = await seededPlayer(page, lang);
    const staffPage = async role => {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      const account = await seededPlayer(screen, lang, role);
      await screen.goto(`/${lang}/account/privacy`);
      return { screen, account, token: (await prepare(screen, formSelector)).fields._csrf };
    };
    const proposer = (await staffPage('platform_admin')).screen;
    const reviewer = await staffPage('platform_admin');
    await page.goto(`/${lang}/account/privacy`);
    const wrongActors = [
      { screen: page, token: (await prepare(page, formSelector)).fields._csrf },
      await staffPage('content_editor'),
    ];
    const topup = await paidTopup(page, lang, db);
    const digest = (id, target) => query(
      `SELECT o.target_id,o.command,o.requires_approval,o.approved_by,o.result,
         (SELECT count(*)::int FROM wallet_entries WHERE operation_id=o.id) AS ledger_adjustments,
         (SELECT count(*)::int FROM premium_ledger WHERE request_id=o.id) AS premium_changes,
         (SELECT count(*)::int FROM commerce_refunds WHERE operation_id=o.id) AS refunds
       FROM commerce_operations o WHERE o.id=$1 AND o.target_id=$2`,
      [id, target],
    );
    const error = lang === 'pl' ? 'Nieprawidłowe dane. Sprawdź identyfikatory, kwotę i powód.'
      : 'Invalid input. Check the identifiers, amount and reason.';
    const reasonError = lang === 'pl' ? 'Wybierz zatwierdzony powód.' : 'Choose an approved reason.';
    const inputError = lang === 'pl'
      ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    const noMutation = async (screen, offer, snapshot, baseline) => {
      await rejected({ page: screen, ...offer, snapshot, baseline });
      for (const actor of wrongActors) {
        expect((await sendForm(actor.screen, offer.url, { ...offer.fields, _csrf: actor.token })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
    };
    const panel = '/admin/commerce';
    const submitOrder = async (kind, target, extra, success = true) => {
      expect((await proposer.goto(panel)).status()).toBe(200);
      const selector = `main form[action="${panel}/${kind}"]`;
      const refund = kind === 'refund';
      const reason = refund ? 'refund_requested' : 'Authorized local browser mutation matrix qualification.';
      const values = { target, reason, ...extra };
      const offer = await prepare(proposer, selector, values);
      const id = offer.fields.request_id;
      const snapshot = () => digest(id, target);
      await noMutation(proposer, offer, snapshot, []);
      const boundary = kind === 'grant' ? { days: '367' } : kind === 'adjust' ? { amount: '0' } : { reason: 'x'.repeat(9) };
      await invalid({
        page: proposer, ...offer, message: refund ? reasonError : error, snapshot, baseline: [],
        samples: [
          { name: 'empty reason', patch: { reason: '' } },
          { name: 'overlong reason', patch: { reason: 'x'.repeat(501) } },
          { name: 'outside the allowed range', patch: boundary },
        ],
      });
      await invalid({
        page: proposer, ...offer, message: error, snapshot, baseline: [],
        samples: [{ name: 'wrong target identifier type', patch: { target: 'not-a-uuid' } }],
      });
      if (refund) {
        await proposer.locator(selector).locator('select[name=reason]').selectOption(reason);
        await submit(proposer, selector, { target, ...extra });
      } else {
        await submit(proposer, selector, values);
      }
      if (!success) {
        await expect(proposer.locator('main h1')).toHaveText(lang === 'pl' ? 'Konflikt zmian' : 'Update conflict');
        await expect(proposer.locator('main')).toContainText(lang === 'pl'
          ? 'Ta akcja jest sprzeczna z bieżącym stanem.' : 'This action conflicts with the current state.');
        expect(await snapshot()).toEqual([]);
        return;
      }
      await expect(proposer).toHaveURL(/\/admin\/commerce$/);
      const submitted = await snapshot();
      expect(submitted).toEqual([{
        target_id: target, command: expect.objectContaining({ kind }),
        requires_approval: ['grant', 'adjust', 'refund'].includes(kind), approved_by: null,
        result: ['grant', 'adjust', 'refund'].includes(kind) ? null : expect.any(Object),
        ledger_adjustments: 0, premium_changes: kind === 'revoke' ? 1 : 0, refunds: 0,
      }]);
      expect((await sendForm(proposer, offer.url, offer.fields)).status()).toBe(303);
      expect(await snapshot()).toEqual(submitted);
      return { id, snapshot, submitted };
    };
    const review = async ({ id, snapshot, submitted }, cancel = false) => {
      const selector = `main form[action="${panel}/approvals/${id}"]`;
      await expect(proposer.locator(`${selector} button`)).toHaveCount(0);
      expect((await reviewer.screen.goto(panel)).status()).toBe(200);
      const reason = cancel ? 'value_withdrawn' : 'value_verified';
      const prepared = await prepare(reviewer.screen, selector, { reason });
      const url = cancel ? new URL(`${panel}/cancellations/${id}`, prepared.url).href : prepared.url;
      await noMutation(reviewer.screen, { ...prepared, url }, snapshot, submitted);
      const self = (await prepare(proposer, selector)).fields._csrf;
      expect((await sendForm(proposer, url, { ...prepared.fields, _csrf: self })).status()).toBe(403);
      expect(await snapshot()).toEqual(submitted);
      for (const faulty of ['bad', 'x'.repeat(64), '1', id.slice(0, -1)]) {
        const malformed = url.replace(id, faulty);
        const response = await sendForm(reviewer.screen, malformed, prepared.fields);
        expect(response.status()).toBe(422);
        expect(await response.text()).toContain(error);
        expect(await snapshot()).toEqual(submitted);
      }
      await invalid({
        page: reviewer.screen, ...prepared, url, snapshot, baseline: submitted, message: reasonError,
        samples: [{ name: 'missing approval or cancellation reason', patch: { reason: '' } }],
      });
      await reviewer.screen.locator(selector).locator('select[name=reason]').selectOption(reason);
      if (cancel) await reviewer.screen.locator(`${selector} button[formaction$="/${id}"]`).click();
      else await submit(reviewer.screen, selector);
      await expect(reviewer.screen).toHaveURL(/\/admin\/commerce$/);
      await expect(reviewer.screen.locator(selector)).toHaveCount(0);
      const after = await snapshot();
      expect(after[0].approved_by).toBe(reviewer.account.id);
      expect(await query('SELECT staff_reason FROM audit_log WHERE request_id=$1 AND action=$2',
        [id, cancel ? 'commerce.cancelled' : 'commerce.approved'])).toEqual([{ staff_reason: reason }]);
      expect((await sendForm(reviewer.screen, url, prepared.fields)).status()).toBe(303);
      expect(await snapshot()).toEqual(after);
      return after[0];
    };

    const grant = await submitOrder('grant', owner.id, { days: '7' });
    expect((await review(grant)).premium_changes).toBe(1);
    const premium = await query(
      'SELECT event_type,duration_days,effective_until FROM premium_ledger WHERE request_id=$1 AND account_id=$2',
      [grant.id, owner.id],
    );
    expect(premium).toEqual([{ event_type: 'GRANT', duration_days: 7, effective_until: expect.any(Date) }]);
    expect((await page.goto(`/${lang}/panel/premium`)).status()).toBe(200);
    await expect(page.locator('main')).toContainText(premium[0].effective_until.toISOString().slice(0, 10));
    const revoke = await submitOrder('revoke', owner.id, {});
    expect(await query('SELECT event_type FROM premium_ledger WHERE request_id=$1', [revoke.id])).toEqual([{ event_type: 'REVOKE' }]);

    const correction = await submitOrder('adjust', owner.id, { amount: '13' });
    expect((await review(correction)).ledger_adjustments).toBe(1);
    expect(await query('SELECT available_delta::text FROM wallet_entries WHERE operation_id=$1 AND account_id=$2',
      [correction.id, owner.id])).toEqual([{ available_delta: '13' }]);
    await page.goto(`/${lang}/shop/history`);
    await expect(page.locator('main dl dd').first()).toHaveText(String(Number(topup.coins) + 13));
    const pendingCancellation = await submitOrder('adjust', owner.id, { amount: '17' });
    const cancelled = await review(pendingCancellation, true);
    expect(cancelled.result).toEqual({ status: 'cancelled' });
    expect(cancelled.ledger_adjustments).toBe(0);

    const payments = `${panel}/reconcile`;
    const paymentId = topup.payment_intent_id;
    expect((await proposer.goto(`${payments}?lang=${lang}&id=${paymentId}`)).status()).toBe(200);
    const paymentForm = `main form[action^="${payments}?"]`;
    const offer = await prepare(proposer, paymentForm, { reason: 'evidence_reviewed' });
    const snapshot = () => digest(offer.fields.request_id, paymentId);
    await noMutation(proposer, offer, snapshot, []);
    await invalid({
      page: proposer, ...offer, message: inputError, snapshot, baseline: [],
      samples: [
        { name: 'wrong payment identifier type', patch: { id: 'not-a-uuid' } },
        { name: 'invalid review version', patch: { version: 'invalid' } },
      ],
    });
    await invalid({
      page: proposer, ...offer, message: reasonError, snapshot, baseline: [],
      samples: [
        { name: 'empty reason', patch: { reason: '' } },
        { name: 'overlong reason', patch: { reason: 'x'.repeat(501) } },
        { name: 'unknown short reason', patch: { reason: 'x'.repeat(9) } },
      ],
    });
    await proposer.locator(paymentForm).locator('select[name=reason]').selectOption(offer.fields.reason);
    await submit(proposer, paymentForm);
    const reviewUrl = new URL(`${payments}?lang=${lang}&id=${paymentId}`, info.project.use.baseURL);
    await expect(proposer).toHaveURL(reviewUrl.href);
    const reviewed = [{
      target_id: paymentId, command: { kind: 'reconcile' }, requires_approval: false, approved_by: null,
      result: {
        resolution_code: 'reviewed_no_payment_state_change', state: 'succeeded',
        intent: paymentId, review_version: offer.fields.version,
      },
      ledger_adjustments: 0, premium_changes: 0, refunds: 0,
    }];
    expect(await snapshot()).toEqual(reviewed);
    expect((await sendForm(proposer, offer.url, offer.fields)).status()).toBe(303);
    expect(await snapshot()).toEqual(reviewed);
    expect(await query("SELECT staff_reason FROM audit_log WHERE request_id=$1 AND action='commerce.payment.reviewed'",
      [offer.fields.request_id])).toEqual([{ staff_reason: offer.fields.reason }]);
    await submitOrder('refund', topup.payment_intent_id, {}, false);
    expect(await query('SELECT state FROM payment_intents WHERE id=$1', [topup.payment_intent_id]))
      .toEqual([{ state: 'succeeded' }]);
  } finally {
    await db.end();
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('enforcement forms bind sanctions and appeal decisions to staff authority and the viewed revision', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const open = async role => {
    const context = await browser.newContext({
      baseURL: info.project.use.baseURL, locale: lang, ignoreHTTPSErrors: true,
      viewport: info.project.use.viewport,
    });
    contexts.push(context);
    const screen = await context.newPage();
    const account = await seededPlayer(screen, lang, role);
    await screen.goto(`/${lang}/account/privacy`);
    return { screen, account, token: (await prepare(screen, formSelector)).fields._csrf };
  };
  try {
    const staff = await seededPlayer(page, lang, 'moderator');
    const target = await open();
    const wrong = [target, await open('content_editor')];
    const base = '/admin/moderation/enforcement';
    const state = id => () => query(
      `SELECT s.account_id,s.status,s.category,s.public_reason,s.moderator_notes,s.created_by,
         s.lock_version::int AS version,a.status AS appeal,a.version::int AS appeal_version,a.outcome
       FROM sanctions s JOIN support_appeal_workflows a ON a.sanction_id=s.id WHERE s.id=$1`, [id],
    );
    const verification = lang === 'pl'
      ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    const check = async (input, snapshot, baseline, samples) => {
      await rejected({ page, ...input, snapshot, baseline });
      for (const { screen, token } of wrong) {
        expect((await sendForm(screen, input.url, { ...input.fields, _csrf: token })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      await invalid({ page, ...input, message: verification, snapshot, baseline, samples });
    };
    const form = 'main form:has([name=action][value=sanction])';
    const now = Math.floor(Date.now() / 1000);
    const values = {
      target: target.account.id, effective_at: String(now - 10), expires_at: String(now + 3600),
      public_reason: 'Synthetic enforcement case for a disposable player.',
      moderator_notes: 'Internal review for the local mutation matrix.',
    };
    const sanctionInvalid = effectiveAt => [
      { name: 'empty public reason', patch: { public_reason: '' } },
      { name: 'overlong public reason', patch: { public_reason: 'x'.repeat(4001) } },
      { name: 'wrong category type', patch: { category: '1' } },
      { name: 'ends at the first boundary', patch: { expires_at: effectiveAt } },
    ];
    const issue = async (category = 'warning', bounds = {}) => {
      const fields = { ...values, ...bounds };
      expect((await page.goto(`${base}/create`)).status()).toBe(200);
      const proposal = await prepare(page, form, { ...fields, category });
      const id = new URL(proposal.url).pathname.split('/').at(-1);
      const snapshot = state(id);
      await check(proposal, snapshot, [], sanctionInvalid(fields.effective_at));
      await page.locator(`${form} [name=category]`).selectOption(category);
      await submit(page, form, fields);
      await expect(page).toHaveURL(proposal.url);
      await expect(page.locator('main h2')).toHaveText(values.public_reason);
      const record = {
        account_id: target.account.id, status: 'active', category, public_reason: values.public_reason,
        moderator_notes: values.moderator_notes, created_by: staff.id, version: 1, appeal: 'none',
        appeal_version: 1, outcome: null,
      };
      expect(await snapshot()).toEqual([record]);
      expect((await sendForm(page, proposal.url, proposal.fields)).status()).toBe(409);
      expect(await snapshot()).toEqual([record]);
      return { id, snapshot, record, fields };
    };
    const end = async (entry, status) => {
      const { snapshot, record, fields } = entry;
      const update = await prepare(page, form, { ...fields, category: record.category, status });
      await check(update, snapshot, [record], sanctionInvalid(fields.effective_at));
      await page.locator(`${form} [name=category]`).selectOption(record.category);
      await page.locator(`${form} [name=status]`).selectOption(status);
      await submit(page, form, fields);
      await expect(page.locator('main h2')).toHaveText(fields.public_reason);
      const ended = [{ ...record, status, version: 2 }];
      expect(await snapshot()).toEqual(ended);
      expect((await sendForm(page, update.url, update.fields)).status()).toBe(409);
      expect(await snapshot()).toEqual(ended);
    };
    await end(await issue('restriction', {
      effective_at: String(now - 120), expires_at: String(now - 60),
    }), 'expired');
    await end(await issue('suspension'), 'revoked');

    const appealForm = 'main form:has([name=action][value=appeal])';
    for (const decision of ['accepted', 'rejected']) {
      const created = await issue();
      const before = await prepare(page, appealForm, { appeal_status: 'reviewing' });
      expect((await sendForm(page, before.url, before.fields)).status()).toBe(409);
      expect(await created.snapshot()).toEqual([created.record]);
      // The Portal has no browser route that submits an owner's appeal.
      // Seed that prerequisite in the disposable database after issuing the real sanction.
      expect(await query(
        `UPDATE support_appeal_workflows SET status='requested',version=version+1,message=$2
         WHERE sanction_id=$1 AND status='none' RETURNING status`,
        [created.id, 'Local owner statement for staff review.'],
      )).toEqual([{ status: 'requested' }]);
      await page.reload();
      let row = { ...created.record, appeal: 'requested', appeal_version: 2 };
      for (const appeal_status of ['reviewing', decision]) {
        const appeal_outcome = `Local appeal decision ${appeal_status}.`;
        const offer = await prepare(page, appealForm, { appeal_status, appeal_outcome });
        await check(offer, created.snapshot, [row], [
          { name: 'empty appealed state', patch: { appeal_status: '' } },
          { name: 'overlong appeal outcome', patch: { appeal_outcome: 'x'.repeat(4001) } },
          { name: 'wrong appealed state type', patch: { appeal_status: '1' } },
          { name: 'state outside the supported values', patch: { appeal_status: 'accepted ' } },
        ]);
        await page.locator(`${appealForm} [name=appeal_status]`).selectOption(appeal_status);
        await submit(page, appealForm, { appeal_outcome });
        await expect(page.getByText(appeal_outcome, { exact: false })).toBeVisible();
        row = { ...row, appeal: appeal_status, outcome: appeal_outcome, version: row.version + 1,
          appeal_version: row.appeal_version + 1, status: appeal_status === 'accepted' ? 'revoked' : 'active' };
        expect(await created.snapshot()).toEqual([row]);
        expect((await sendForm(page, offer.url, offer.fields)).status()).toBe(409);
        expect(await created.snapshot()).toEqual([row]);
      }
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

for (const kind of ['character', 'item']) {
  test(`fake Game confirms one Bazaar ${kind} listing and rejects duplicate custody`, async ({ page, browser }, info) => {
    test.setTimeout(600_000);
    const lang = info.project.metadata.locale;
    const contexts = [];
    const open = async role => {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      return { screen, account: await seededPlayer(screen, lang, role) };
    };
    const elevate = async (screen, account, hasMfa) => {
      if (!hasMfa) {
        await enrollMfa(screen, lang, account);
        const code = await screen.locator('textarea[readonly]').first().inputValue();
        await signInWithRecoveryCode(screen, lang, account, code);
      }
      expect((await screen.goto(`/${lang}/account/sudo`)).status()).toBe(200);
      await submit(screen, formSelector, { current_password: account.password });
      await expect(screen).toHaveURL(new RegExp(`/${lang}/password/change$`));
    };
    try {
      const seller = await seededPlayer(page, lang);
      await elevate(page, seller, false);
      const buyer = await open();
      await elevate(buyer.screen, buyer.account, false);
      const editor = await open('content_editor');
      await elevate(editor.screen, editor.account, true);
      const outsiders = [];
      for (const { screen } of [buyer, editor]) {
        await screen.goto(`/${lang}/account/privacy`);
        outsiders.push({ screen, token: (await prepare(screen, formSelector)).fields._csrf });
      }
      await elevate(page, seller, true);
      const character = uuid7();
      const asset = kind === 'character' ? character : uuid7();
      const destination = uuid7();
      const world = '0192a000-0000-7000-8000-000000000012';
      const created = await page.request.post('/__bazaar_game/seed', { data: {
        seller: seller.id, buyer: buyer.account.id, kind, asset_id: asset,
        seller_character_id: character, buyer_character_id: destination,
      } });
      expect(created.status()).toBe(201);
      const start = `/${lang}/account/bazaar/sell`;
      expect((await page.goto(start)).status()).toBe(200);
      const listing = 'main form[action$="/sell"]';
      await page.locator(`${listing} select[name=kind]`).selectOption(kind);
      const values = {
        world_id: world, seller_character_id: character, asset_id: asset,
        expected_revision: '7', quantity: '1', definition_key: 'fixture-sword',
        definition_revision: 'v1', starting_bid: '100', duration_days: '1',
      };
      const { url, fields } = await prepare(page, listing, { ...values, publish_snapshot: 'true' });
      const state = () => query(
        `SELECT l.id,l.owner,l.kind,l.state,l.price,a.asset_id AS asset,
            ARRAY(SELECT o.command FROM bazaar_authority_receipts r JOIN bazaar_game_outbox o USING(operation_id) WHERE o.listing_id=l.id ORDER BY o.command) AS receipts
         FROM bazaar_listings l JOIN bazaar_authority_assets a ON a.listing_id=l.id
         WHERE a.asset_id=$1`, [asset],
      );
      expect(await state()).toEqual([]);
      await rejected({ page, url, fields, snapshot: state, baseline: [] });
      const reserveListing = async () => {
        await expect.poll(async () => {
          const [row] = await query(
            "SELECT COALESCE(max(count),0)::int AS count FROM web_rate_limits WHERE limiter_name='marketplace-listing' AND window_start=date_trunc('minute',clock_timestamp())"
          );
          return row.count;
        }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(2);
      };
      const samples = [
        { name: 'empty listing kind', patch: { kind: '' } },
        { name: 'overlong listing kind', patch: { kind: 'x'.repeat(512) } },
        { name: 'wrong revision representation', patch: { expected_revision: 'abc' } },
        { name: 'duration beyond the auction boundary', patch: { duration_days: '15' } },
      ];
      for (const sample of samples) {
        await reserveListing();
        await invalid({
          page, url, fields, snapshot: state, baseline: [],
          message: lang === 'pl'
            ? 'Prośba została odrzucona. Sprawdź konto i dane obiektu.'
            : 'This request was refused. Check your account and the asset details.',
          samples: [sample],
        });
      }
      await reserveListing();
      await submit(page, listing, { ...values, publish_snapshot: true });
      await expect(page).toHaveURL(/\/bazaar\/[a-f0-9-]+$/);
      const id = new URL(page.url()).pathname.split('/').at(-1);
      await expect.poll(async () => {
        await page.reload();
        return page.locator(`[data-listing="${id}"]`).getAttribute('data-state');
      }, { timeout: 60_000, intervals: [1000] }).toBe('active');
      const receipts = kind === 'character'
        ? ['BindBazaarCharacterTransfer', 'LockBazaarCharacter']
        : ['EscrowBazaarItem'];
      const locked = [{ id, owner: seller.id, kind, state: 'active', price: '100', asset, receipts }];
      expect(await state()).toEqual(locked);
      const fixture = await page.request.get(`/__bazaar_game/state?asset=${asset}`);
      expect(fixture.status()).toBe(200);
      const game = await fixture.json();
      const operations = await query('SELECT operation_id,command FROM bazaar_game_outbox WHERE listing_id=$1 ORDER BY command', [id]);
      expect(operations.map(operation => operation.command)).toEqual(receipts);
      for (const operation of operations) {
        expect(game.decisions.filter(decision => decision.operation === operation.operation_id
          && decision.command === operation.command && decision.outcome === 'COMMITTED')).toHaveLength(1);
      }
      for (const { screen, token } of outsiders) {
        expect((await sendForm(screen, url, { ...fields, _csrf: token })).status()).toBe(409);
        expect(await state()).toEqual(locked);
      }
      await reserveListing();
      expect((await sendForm(page, url, fields)).status()).toBe(303);
      expect(await state()).toEqual(locked);
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });
}

test('staff player forms record a note, revoke sessions and require independent MFA reset approval', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const open = async role => {
    const context = await browser.newContext({
      baseURL: info.project.use.baseURL, locale: lang, ignoreHTTPSErrors: true,
      viewport: info.project.use.viewport,
    });
    contexts.push(context);
    const screen = await context.newPage();
    const account = await seededPlayer(screen, lang, role);
    await screen.goto(`/${lang}/account/privacy`);
    return { screen, account, token: (await prepare(screen, formSelector)).fields._csrf };
  };
  try {
    const player = await seededPlayer(page, lang);
    await enrollMfa(page, lang, player);
    const codes = page.locator('textarea[readonly]');
    await expect(codes).toHaveCount(8);
    const backups = await codes.allTextContents();
    await signInWithRecoveryCode(page, lang, player, backups[0]);
    const support = await open('support');
    const admin = await open('platform_admin');
    const wrong = [await open(), await open('content_editor')];
    const path = `/admin/players/${player.id}`;
    const snapshot = async () => ({
      notes: await query('SELECT author,body FROM player_notes WHERE account_id=$1 ORDER BY id', [player.id]),
      sessions: await query('SELECT count(*)::int AS count FROM web_sessions WHERE account_id=$1', [player.id]),
      identity: await query(
        'SELECT login_mfa_required,native_security_generation::int AS generation FROM accounts WHERE id=$1', [player.id],
      ),
      evidence: await query(
        'SELECT id,ticket_id,verified_by FROM support_mfa_reset_evidence WHERE account_id=$1 ORDER BY id', [player.id],
      ),
      resets: await query(
        'SELECT id,requested_by,approved_by,evidence,reason FROM player_mfa_reset_requests WHERE account_id=$1 ORDER BY id', [player.id],
      ),
    });
    const button = operation => `main form:has(button[name=action][value=${operation}])`;
    const reason = 'Verified report from the disposable player tools fixture.';
    const wrongText = field => [
      [{ [field]: '' }, 400], [{ [field]: 'x'.repeat(field === 'body' ? 4001 : 501) }, 400],
      [{ action: 'bad-type' }, 400], [{ [field]: ' ' }, 400],
    ];
    const wrongId = field => [
      [{ [field]: '' }, 422], [{ [field]: 'x'.repeat(65) }, 422], [{ [field]: '1' }, 422],
      [{ [field]: '00000000-0000-0000-0000-000000000000' }, 400],
    ];
    const check = async (actor, operation, values, samples) => {
      const selector = button(operation);
      const proposal = await prepare(actor.screen, selector, { action: operation, ...values });
      const baseline = await snapshot();
      await rejected({ page: actor.screen, ...proposal, snapshot, baseline });
      for (const { screen, token } of wrong) {
        expect((await sendForm(screen, proposal.url, { ...proposal.fields, _csrf: token })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      const messages = {
        400: lang === 'pl' ? 'Sprawdź żądanie i spróbuj ponownie.' : 'Check the request and try again.',
        422: lang === 'pl' ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
          : 'Some information could not be accepted. Check your input and try again.',
      };
      for (const [patch, status] of samples) {
        const denied = await sendForm(actor.screen, proposal.url, { ...proposal.fields, ...patch });
        expect(denied.status()).toBe(status);
        expect(await denied.text()).toContain(messages[status]);
        expect(await snapshot()).toEqual(baseline);
      }
      return { selector, ...proposal, baseline };
    };
    expect((await support.screen.goto(path)).status()).toBe(200);
    const note = await check(support, 'note', { body: reason }, wrongText('body'));
    await action(support.screen, note.selector, { body: reason }, 'note');
    await expect(support.screen.getByText(reason, { exact: true })).toBeVisible();
    expect(await snapshot()).toEqual({ ...note.baseline, notes: [{ author: support.account.id, body: reason }] });

    const logout = await check(support, 'logout', { reason }, wrongText('reason'));
    await action(support.screen, logout.selector, { reason }, 'logout');
    await expect(support.screen).toHaveURL(new URL(path, logout.url).href);
    await page.goto(`/${lang}/account/sessions`);
    await expect(page).toHaveURL(/\/login$/);
    expect((await snapshot()).sessions).toEqual([{ count: 0 }]);
    await signInWithRecoveryCode(page, lang, player, backups[1]);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));

    expect((await page.goto(`/${lang}/support/tickets/create`)).status()).toBe(200);
    await page.locator('main select[name=category]').selectOption('account');
    await submit(page, formSelector, { subject: 'Disposable account ownership ticket', body: reason });
    await expect(page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
    const ticket = new URL(page.url()).pathname.split('/').at(-1);
    const proof = await check(support, 'verify-identity', { ticket }, wrongId('ticket'));
    await action(support.screen, proof.selector, { ticket }, 'verify-identity');
    await expect(support.screen).toHaveURL(/\?evidence=[a-f0-9-]+$/);
    const evidence = new URL(support.screen.url()).searchParams.get('evidence');
    expect((await snapshot()).evidence).toEqual([{ id: evidence, ticket_id: ticket, verified_by: support.account.id }]);
    const request = await check(support, 'request-reset', { evidence, reason }, wrongId('evidence'));
    await action(support.screen, request.selector, { evidence, reason }, 'request-reset');
    await expect(support.screen).toHaveURL(/\?request=[a-f0-9-]+$/);
    const requestId = new URL(support.screen.url()).searchParams.get('request');
    const queued = [{ id: requestId, requested_by: support.account.id, approved_by: null, evidence, reason }];
    expect((await snapshot()).resets).toEqual(queued);

    expect((await admin.screen.goto(path)).status()).toBe(200);
    const approval = await check(admin, 'approve-reset', { request: requestId }, wrongId('request'));
    expect((await sendForm(support.screen, approval.url, { ...approval.fields, _csrf: support.token })).status()).toBe(403);
    expect(await snapshot()).toEqual(approval.baseline);
    await action(admin.screen, approval.selector, { request: requestId }, 'approve-reset');
    await expect(admin.screen).toHaveURL(new URL(path, approval.url).href);
    const cleared = await snapshot();
    expect(cleared.resets).toEqual([{ ...queued[0], approved_by: admin.account.id }]);
    expect(cleared.identity[0].login_mfa_required).toBe(false);
    expect(cleared.sessions).toEqual([{ count: 0 }]);
    expect((await sendForm(admin.screen, approval.url, approval.fields)).status()).toBe(403);
    expect(await snapshot()).toEqual(cleared);
    await login(page, lang, player);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));

    const kicked = await check(admin, 'kick', { reason }, wrongText('reason'));
    await action(admin.screen, kicked.selector, { reason }, 'kick');
    await expect(admin.screen).toHaveURL(new URL(path, kicked.url).href);
    expect((await snapshot()).identity).toEqual([{
      ...kicked.baseline.identity[0], generation: kicked.baseline.identity[0].generation + 1,
    }]);
    expect(await query(
      "SELECT action FROM audit_log WHERE target=$1 AND action IN ('players.note.added','players.logout_all','players.kick','players.mfa_reset.identity_verified','players.mfa_reset.requested','players.mfa_reset.completed') ORDER BY created_at,id", [player.id],
    )).toEqual(['players.note.added', 'players.logout_all', 'players.mfa_reset.identity_verified',
      'players.mfa_reset.requested', 'players.mfa_reset.completed', 'players.kick'].map(action => ({ action })));
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('referral link and invitation forms bind eligible players and preserve one invitation on replay', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const path = `/${lang}/account/referrals`;
  const open = async role => {
    const context = await browser.newContext({
      baseURL: info.project.use.baseURL, locale: lang, ignoreHTTPSErrors: true,
      viewport: info.project.use.viewport,
    });
    contexts.push(context);
    const screen = await context.newPage();
    const account = await seededPlayer(screen, lang, role);
    await screen.goto(`/${lang}/account/privacy`);
    return { screen, account, token: (await prepare(screen, formSelector)).fields._csrf };
  };
  const [coverage] = await query('SELECT covered_since::text FROM portal_referral_coverage WHERE id');
  try {
    const inviter = await seededPlayer(page, lang);
    const existing = [await open(), await open('content_editor')];
    const state = async () => ({
      links: await query('SELECT code FROM referral_links WHERE account_id=$1', [inviter.id]),
      sent: await query(
        `SELECT i.recipient,i.kind,count(a.id)::int AS deliveries
         FROM referral_email_invitations i LEFT JOIN referral_email_attempts a ON a.invitation_id=i.id
         WHERE i.inviter_id=$1 GROUP BY i.id ORDER BY i.id`, [inviter.id],
      ),
      bound: await query('SELECT invitee_id,kind FROM referral_campaign_invitations WHERE inviter_id=$1 ORDER BY id', [inviter.id]),
    });
    const empty = { links: [], sent: [], bound: [] };
    expect(await state()).toEqual(empty);
    const validation = lang === 'pl' ? 'Sprawdź adres e-mail i spróbuj ponownie.'
      : 'Check the e-mail address and try again.';
    const rejectedInput = async (offer, invalid, baseline) => {
      for (const patch of invalid) {
        const response = await sendForm(page, offer.url, { ...offer.fields, ...patch });
        expect(response.status()).toBe(400);
        expect(await response.text()).toContain(validation);
        expect(await state()).toEqual(baseline);
      }
    };
    const checkOverride = async (offer, baseline) => {
      for (const { screen, token, account } of existing) {
        const submitted = await sendForm(screen, offer.url, {
          ...offer.fields, _csrf: token, inviter_id: inviter.id,
        });
        expect(submitted.status()).toBe(422);
        expect(await state()).toEqual(baseline);
        expect(await query('SELECT code FROM referral_links WHERE account_id=$1', [account.id])).toEqual([]);
      }
    };
    expect((await page.goto(path)).status()).toBe(200);
    const button = 'main button[name=kind][value=link]';
    const linkForm = 'main form:has(button[name=kind][value=link])';
    const create = await prepare(page, linkForm, { kind: 'link' });
    await rejected({ page, ...create, snapshot: state, baseline: empty });
    await checkOverride(create, empty);
    await rejectedInput(create, [{ kind: '' }, { kind: 'x'.repeat(128) }, { kind: '1' }, { kind: 'link ' }], empty);
    await page.locator(button).click();
    const shown = page.locator('#invite-link');
    await expect(shown).toBeVisible();
    const link = new URL(await shown.inputValue());
    expect(link.pathname).toMatch(new RegExp(`/${lang}/referrals/[a-f0-9-]+$`));
    const linked = { ...empty, links: [{ code: link.pathname.split('/').at(-1) }] };
    expect(await state()).toEqual(linked);
    expect((await sendForm(page, create.url, create.fields)).status()).toBe(303);
    expect(await state()).toEqual(linked);

    const newSelector = 'main form:has(input[name=kind][value=new])';
    const recipient = `matrix-recipient-${randomUUID()}@local.test`;
    const invitation = await prepare(page, newSelector, { email: recipient });
    await rejected({ page, ...invitation, snapshot: state, baseline: linked });
    await checkOverride(invitation, linked);
    const badAddress = [{ email: '' }, { email: `${'a'.repeat(255)}@local.test` }, { email: 'type' }, { email: '.first@local.test' }];
    await rejectedInput(invitation, badAddress, linked);
    await submit(page, newSelector, { email: recipient });
    const notice = lang === 'pl'
      ? 'Jeśli adres spełnia warunki, zaproszenie trafi do kolejki. Dane kont nie są ujawniane.'
      : 'If the address is eligible, the invitation is queued. No account details are disclosed.';
    await expect(page.getByRole('status')).toHaveText(notice);
    const mailed = { ...linked, sent: [{ recipient, kind: 'new', deliveries: 1 }] };
    expect(await state()).toEqual(mailed);
    expect((await sendForm(page, invitation.url, invitation.fields)).status()).toBe(303);
    expect(await state()).toEqual(mailed);

    const returning = 'main form:has(input[name=kind][value=returning])';
    const comeback = await prepare(page, returning, { email: existing[0].account.email });
    await rejected({ page, ...comeback, snapshot: state, baseline: mailed });
    await checkOverride(comeback, mailed);
    await rejectedInput(comeback, badAddress, mailed);
    await submit(page, returning, { email: existing[0].account.email });
    await expect(page.getByRole('status')).toHaveText(notice);
    expect(await state()).toEqual(mailed);

    // Use the same historical coverage prerequisites as the referral service tests.
    const buddy = existing[0];
    await query("UPDATE portal_referral_coverage SET covered_since=clock_timestamp()-interval '13 months' WHERE id");
    await query("UPDATE accounts SET created_at=clock_timestamp()-interval '14 months' WHERE id=$1", [buddy.account.id]);
    const aged = await query(
      `UPDATE referral_activity SET last_activity=clock_timestamp()-interval '13 months',
         created_at=clock_timestamp()-interval '13 months' WHERE account_id=$1 RETURNING account_id`,
      [buddy.account.id],
    );
    expect(aged).toEqual([{ account_id: buddy.account.id }]);
    const qualified = await prepare(page, returning, { email: buddy.account.email });
    await submit(page, returning, { email: buddy.account.email });
    await expect(page.getByRole('status')).toHaveText(notice);
    const bothMailed = {
      ...mailed,
      sent: [...mailed.sent, { recipient: buddy.account.email, kind: 'returning', deliveries: 1 }],
      bound: [{ invitee_id: buddy.account.id, kind: 'returning' }],
    };
    expect(await state()).toEqual(bothMailed);
    expect((await sendForm(page, qualified.url, qualified.fields)).status()).toBe(303);
    expect(await state()).toEqual(bothMailed);
    const returningLink = await mailLink(buddy.screen, buddy.account.email,
      /https:\/\/localhost:\d+\/(?:pl|en)\/referrals\/[a-f0-9-]{36}/, 120_000);
    expect((await buddy.screen.goto(returningLink)).status()).toBe(200);
    const returningForm = await prepare(buddy.screen, formSelector);
    await rejected({ page: buddy.screen, ...returningForm, snapshot: state, baseline: bothMailed });
    const unrelated = existing[1];
    expect((await sendForm(unrelated.screen, returningForm.url, {
      ...returningForm.fields, _csrf: unrelated.token,
    })).status()).toBe(403);
    expect(await state()).toEqual(bothMailed);
    await submit(buddy.screen, formSelector);
    await expect(buddy.screen).toHaveURL(new URL(path, returningLink).href);
    const accepted = bothMailed;
    expect(await state()).toEqual(accepted);
    expect((await sendForm(buddy.screen, returningForm.url, returningForm.fields)).status()).toBe(303);
    expect(await state()).toEqual(accepted);

    const invitee = await open();
    expect((await invitee.screen.goto(link.href)).status()).toBe(200);
    const accept = await prepare(invitee.screen, formSelector);
    await rejected({ page: invitee.screen, ...accept, snapshot: state, baseline: accepted });
    await page.goto(link.href);
    const ownerToken = (await prepare(page, formSelector)).fields._csrf;
    for (const { screen, token } of [{ screen: page, token: ownerToken }, ...existing]) {
      expect((await sendForm(screen, accept.url, { ...accept.fields, _csrf: token })).status()).toBe(403);
      expect(await state()).toEqual(accepted);
    }
    const nilLink = new URL(`/${lang}/referrals/00000000-0000-0000-0000-000000000000`, link);
    const refusal = await sendForm(invitee.screen, nilLink.href, accept.fields);
    expect(refusal.status()).toBe(400);
    expect(await refusal.text()).toContain(validation);
    expect(await state()).toEqual(accepted);
    await submit(invitee.screen, formSelector);
    await expect(invitee.screen).toHaveURL(new URL(path, accept.url).href);
    const bound = { ...accepted, bound: [...accepted.bound, { invitee_id: invitee.account.id, kind: 'new' }] };
    expect(await state()).toEqual(bound);
    expect((await sendForm(invitee.screen, accept.url, accept.fields)).status()).toBe(303);
    expect(await state()).toEqual(bound);
    await page.goto(path);
    await expect(page.locator('section[aria-labelledby=invitations] li')).toHaveText([lang === 'pl'
      ? 'Nowy gracz — Oczekiwanie na płatny zakup' : 'New player — Waiting for a paid purchase']);
  } finally {
    await query('UPDATE portal_referral_coverage SET covered_since=$1::timestamptz WHERE id', [coverage.covered_since]);
    await Promise.all(contexts.map(context => context.close()));
  }
});

async function createBazaarListing(seller, buyer, lang, kind = 'item', currency = 'COINS') {
  const character = uuid7();
  const asset = kind === 'character' ? character : uuid7();
  const destination = uuid7();
  const seeded = await seller.page.request.post('/__bazaar_game/seed', { data: {
    kind, asset_id: asset, seller: seller.id, buyer: buyer.id,
    seller_character_id: character, buyer_character_id: destination,
  } });
  expect(seeded.status()).toBe(201);
  expect((await seller.page.goto(`/${lang}/account/bazaar/sell`)).status()).toBe(200);
  const form = 'main form[action$="/sell"]';
  await seller.page.locator(`${form} [name=kind]`).selectOption(kind);
  await seller.page.locator(`${form} [name=currency]`).selectOption(currency);
  await submit(seller.page, form, {
    world_id: '0192a000-0000-7000-8000-000000000012',
    seller_character_id: character, asset_id: asset, expected_revision: '7', quantity: '1',
    definition_key: 'fixture-sword', definition_revision: 'v1', starting_bid: '100',
    duration_days: '1', publish_snapshot: true,
  });
  await expect(seller.page).toHaveURL(/\/bazaar\/[a-f0-9-]+$/);
  const id = new URL(seller.page.url()).pathname.split('/').at(-1);
  await expect.poll(async () => {
    await seller.page.reload();
    return seller.page.locator(`[data-listing="${id}"]`).getAttribute('data-state');
  }, { timeout: 60_000, intervals: [1000] }).toBe('active');
  return { id, asset, destination, url: `/${lang}/bazaar/${kind === 'item' ? 'items/' : ''}${id}` };
}

test('Bazaar coin bid and seller controls apply one hold, one resume and one Game return', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const open = async role => {
    const context = await browser.newContext({
      baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
      viewport: info.project.use.viewport,
    });
    contexts.push(context);
    const screen = await context.newPage();
    return { screen, actor: await seededPlayer(screen, lang, role) };
  };
  try {
    const seller = await seededPlayer(page, lang);
    await enrollMfa(page, lang, seller);
    const codes = page.locator('textarea[readonly]');
    await expect(codes).toHaveCount(8);
    const backups = await codes.allTextContents();
    await signInWithRecoveryCode(page, lang, seller, backups[0]);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    const buyer = await open();
    const editor = await open('content_editor');
    const db = pool();
    try {
      await paidTopup(buyer.screen, lang, db);
    } finally {
      await db.end();
    }
    const actors = [page, editor.screen, buyer.screen];
    const tokens = new Map();
    for (const actor of actors) {
      await actor.goto(`/${lang}/account/privacy`);
      tokens.set(actor, (await prepare(actor, formSelector)).fields._csrf);
    }
    const offer = async () => {
      expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
      await submit(page, formSelector, { current_password: seller.password });
      await expect(page).toHaveURL(new RegExp(`/${lang}/password/change$`));
      return createBazaarListing(
        { page, id: seller.id }, { page: buyer.screen, id: buyer.actor.id }, lang, 'item',
      );
    };
    const error = lang === 'pl'
      ? 'Prośba została odrzucona. Sprawdź konto i dane obiektu.'
      : 'This request was refused. Check your account and the asset details.';
    const actorRefusals = async (persons, url, fields, snapshot, baseline) => {
      for (const actor of persons) {
        expect((await sendForm(actor, url, { ...fields, _csrf: tokens.get(actor) })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
    };
    const lot = await offer();
    const bidState = async () => ({
      holds: await query('SELECT account_id,amount FROM bazaar_coin_holds WHERE id=$1 ORDER BY account_id', [lot.id]),
      bids: await query('SELECT buyer,character_id,maximum FROM bazaar_authority_bid_requests WHERE listing_id=$1 ORDER BY request_id', [lot.id]),
    });
    const baseline = { holds: [], bids: [] };
    expect(await bidState()).toEqual(baseline);
    expect((await buyer.screen.goto(lot.url)).status()).toBe(200);
    const bid = 'main form[action$="/bids"]';
    const amount = { buyer_character_id: lot.destination, maximum: '200' };
    const request = await prepare(buyer.screen, bid, amount);
    await rejected({ page: buyer.screen, ...request, snapshot: bidState, baseline });
    await actorRefusals([page, editor.screen], request.url, request.fields, bidState, baseline);
    await invalid({
      page: buyer.screen, ...request, snapshot: bidState, baseline, message: error,
      samples: [
        { name: 'empty bid', patch: { maximum: '' } },
        { name: 'overlong bid', patch: { maximum: '1'.repeat(128) } },
        { name: 'non-numeric bid', patch: { maximum: 'x' } },
        { name: 'zero bid', patch: { maximum: '0' } },
      ],
    });
    await submit(buyer.screen, bid, amount);
    await expect(buyer.screen).toHaveURL(new URL(`/${lang}/bazaar/${lot.id}`, request.url).href);
    const held = {
      holds: [{ account_id: buyer.actor.id, amount: '200' }],
      bids: [{ buyer: buyer.actor.id, character_id: lot.destination, maximum: '200' }],
    };
    expect(await bidState()).toEqual(held);
    await buyer.screen.goto(`/${lang}/shop/history`);
    await expect(buyer.screen.locator('main dl dd').first()).toHaveText('1034');
    expect((await sendForm(buyer.screen, request.url, request.fields)).status()).toBe(303);
    expect(await bidState()).toEqual(held);

    const second = await offer();
    const [sellerBinding] = await query(
      'SELECT session_hash FROM bazaar_authority_assets WHERE listing_id=$1', [second.id],
    );
    await signInWithRecoveryCode(page, lang, seller, backups[1]);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
    await submit(page, formSelector, { current_password: seller.password });
    expect((await page.goto(second.url)).status()).toBe(200);
    const control = 'main form[action$="/control"]';
    const controlState = async () => ({
      listing: await query(
        `SELECT l.state,a.cancel_requested,a.session_hash=$2 AS original_session
         FROM bazaar_listings l JOIN bazaar_authority_assets a ON a.listing_id=l.id WHERE l.id=$1`,
        [second.id, sellerBinding.session_hash],
      ),
      operations: await query(
        "SELECT b.action,b.actor FROM bazaar_authority_browser_requests b WHERE b.listing_id=$1 ORDER BY b.created_at,b.request_id",
        [second.id],
      ),
      returns: await query(
        "SELECT o.operation_id,r.outcome FROM bazaar_game_outbox o JOIN bazaar_authority_receipts r USING(operation_id) WHERE o.listing_id=$1 AND o.command='ReturnBazaarItem'",
        [second.id],
      ),
    });
    let before = { listing: [{ state: 'active', cancel_requested: false, original_session: true }], operations: [], returns: [] };
    expect(await controlState()).toEqual(before);
    for (const decision of ['resume', 'cancel']) {
      const form = await prepare(page, control, { action: decision });
      await rejected({ page, ...form, snapshot: controlState, baseline: before });
      await actorRefusals([buyer.screen, editor.screen], form.url, form.fields, controlState, before);
      await invalid({
        page, ...form, snapshot: controlState, baseline: before, message: error,
        samples: [
          { name: 'empty control action', patch: { action: '' } },
          { name: 'overlong control action', patch: { action: 'x'.repeat(512) } },
          { name: 'wrong control request type', patch: { request_id: 'invalid' } },
          { name: 'unrecognized action beyond the accepted values', patch: { action: 'resume ' } },
        ],
      });
      await page.locator(`${control} button[value=${decision}]`).click();
      await expect(page).toHaveURL(new URL(`/${lang}/bazaar/${second.id}`, form.url).href);
      const expected = [...before.operations, { action: decision, actor: seller.id }];
      if (decision === 'resume') {
        await expect(page.locator(`[data-listing="${second.id}"]`)).toHaveAttribute('data-state', 'active');
        before = { listing: [{ state: 'active', cancel_requested: false, original_session: false }], operations: expected, returns: [] };
      } else {
        await expect.poll(async () => {
          await page.reload();
          return page.locator(`[data-listing="${second.id}"]`).getAttribute('data-state');
        }, { timeout: 60_000, intervals: [1000] }).toBe('cancelled');
        before = {
          listing: [{ state: 'cancelled', cancel_requested: true, original_session: false }], operations: expected,
          returns: [{ operation_id: expect.any(String), outcome: 'COMMITTED' }],
        };
      }
      expect(await controlState()).toEqual(before);
      expect((await sendForm(page, form.url, form.fields)).status()).toBe(303);
      expect(await controlState()).toEqual(before);
      expect(await bidState()).toEqual(held);
    }
    const evidence = await (await page.request.get(`/__bazaar_game/state?asset=${second.asset}`)).json();
    expect(evidence.asset.owner).toBe(seller.id);
    expect(evidence.asset.consumed).toBe(true);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('Bazaar report and promotion refuse unavailable authority even for a live Game listing', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const open = async role => {
    const context = await browser.newContext({
      baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
      viewport: info.project.use.viewport,
    });
    contexts.push(context);
    const screen = await context.newPage();
    const actor = await seededPlayer(screen, lang, role);
    await screen.goto(`/${lang}/account/privacy`);
    return { screen, actor, csrf: (await prepare(screen, formSelector)).fields._csrf };
  };
  try {
    const seller = await seededPlayer(page, lang);
    await enrollMfa(page, lang, seller);
    const code = await page.locator('textarea[readonly]').first().inputValue();
    await signInWithRecoveryCode(page, lang, seller, code);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    const buyer = await open();
    const unrelated = await open('content_editor');
    const admin = await open('platform_admin');
    await page.goto(`/${lang}/account/sudo`);
    await submit(page, formSelector, { current_password: seller.password });
    const lot = await createBazaarListing({ page, id: seller.id }, { page: buyer.screen, id: buyer.actor.id }, lang);
    await page.goto(`/${lang}/account/privacy`);
    const sellerToken = (await prepare(page, formSelector)).fields._csrf;
    const effects = async () => ({
      listing: await query('SELECT state FROM bazaar_listings WHERE id=$1', [lot.id]),
      reports: await query('SELECT id FROM bazaar_reports WHERE listing_id=$1', [lot.id]),
      promotions: await query('SELECT id FROM bazaar_promotions WHERE listing_id=$1', [lot.id]),
      ledger: await query('SELECT operation_id FROM wallet_entries WHERE account_id=$1', [seller.id]),
    });
    const baseline = { listing: [{ state: 'active' }], reports: [], promotions: [], ledger: [] };
    expect(await effects()).toEqual(baseline);
    const panel = `/admin/${lang}/bazaar`;
    expect((await admin.screen.goto(panel)).status()).toBe(200);
    const selector = 'main form:has([name=action][value=fees])';
    const proposed = { promotion_price: '19', reason: 'Local disposable promotion quote qualification.' };
    const fee = await prepare(admin.screen, selector, proposed);
    const fees = () => query(
      'SELECT coin_bps,gold_bps,listing_fee::text,promotion_price::text,actor,reason FROM bazaar_fee_revisions WHERE id=$1',
      [fee.fields.id],
    );
    await rejected({ page: admin.screen, ...fee, snapshot: fees, baseline: [] });
    for (const { screen, csrf } of [buyer, unrelated]) {
      expect((await sendForm(screen, fee.url, { ...fee.fields, _csrf: csrf })).status()).toBe(403);
      expect(await fees()).toEqual([]);
    }
    const bad = lang === 'pl'
      ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    await invalid({ page: admin.screen, ...fee, message: bad, snapshot: fees, baseline: [], samples: [
      { name: 'empty reason', patch: { reason: '' } },
      { name: 'overlong reason', patch: { reason: 'x'.repeat(501) } },
      { name: 'wrong numeric type', patch: { promotion_price: 'nineteen' } },
      { name: 'zero promotion price', patch: { promotion_price: '0' } },
    ] });
    await submit(admin.screen, selector, proposed);
    await expect(admin.screen).toHaveURL(new URL(panel, fee.url).href);
    const revised = [{
      coin_bps: 1000, gold_bps: 250, listing_fee: '0', promotion_price: '19',
      actor: admin.actor.id, reason: proposed.reason,
    }];
    expect(await fees()).toEqual(revised);
    await expect(admin.screen.locator('main')).toContainText(fee.fields.id);
    expect((await sendForm(admin.screen, fee.url, fee.fields)).status()).toBe(503);
    expect(await fees()).toEqual(revised);

    const report = `/${lang}/account/bazaar/${lot.id}/report`;
    expect((await buyer.screen.goto(report)).status()).toBe(200);
    await expect(buyer.screen.locator('main h2')).toHaveText(lang === 'pl' ? 'Zgłoś ofertę' : 'Report listing');
    const detail = 'Synthetic evidence about the Game-backed listing.';
    const complaint = await prepare(buyer.screen, formSelector, { detail });
    await rejected({ page: buyer.screen, ...complaint, snapshot: effects, baseline });
    for (const { screen, csrf } of [{ screen: page, csrf: sellerToken }, unrelated]) {
      expect((await sendForm(screen, complaint.url, { ...complaint.fields, _csrf: csrf })).status()).toBe(403);
      expect(await effects()).toEqual(baseline);
    }
    await invalid({ page: buyer.screen, ...complaint, message: bad, snapshot: effects, baseline, samples: [
      { name: 'empty detail', patch: { detail: '' } },
      { name: 'overlong detail', patch: { detail: 'x'.repeat(1001) } },
      { name: 'wrong category', patch: { category: '1' } },
      { name: 'detail one character below the minimum', patch: { detail: '123456789' } },
    ] });
    await submit(buyer.screen, formSelector, { detail });
    await expect(buyer.screen.locator('main')).toContainText(lang === 'pl'
      ? 'Nie masz uprawnień do wyświetlenia tej strony.' : 'You do not have permission to view this page.');
    expect(await effects()).toEqual(baseline);
    expect((await sendForm(buyer.screen, complaint.url, complaint.fields)).status()).toBe(403);
    expect(await effects()).toEqual(baseline);

    expect((await page.goto(`/${lang}/account/bazaar/${lot.id}/promote`)).status()).toBe(200);
    await expect(page.locator('main h2')).toHaveText(lang === 'pl' ? 'Wyróżnij ofertę' : 'Feature listing');
    const promotion = await prepare(page, formSelector);
    expect(promotion.fields.fee_revision).toBe(fee.fields.id);
    expect(promotion.fields.amount).toBe('19');
    await rejected({ page, ...promotion, snapshot: effects, baseline });
    for (const { screen, csrf } of [buyer, unrelated]) {
      expect((await sendForm(screen, promotion.url, { ...promotion.fields, _csrf: csrf })).status()).toBe(403);
      expect(await effects()).toEqual(baseline);
    }
    await invalid({ page, ...promotion, message: bad, snapshot: effects, baseline, samples: [
      { name: 'empty promotion amount', patch: { amount: '' } },
      { name: 'overlong promotion amount', patch: { amount: '9'.repeat(80) } },
      { name: 'wrong fee identifier type', patch: { fee_revision: 'text' } },
      { name: 'promotion identifier below the v7 boundary', patch: { id: '00000000-0000-0000-0000-000000000000' } },
    ] });
    await submit(page, formSelector);
    await expect(page.locator('main')).toContainText(lang === 'pl'
      ? 'Nie masz uprawnień do wyświetlenia tej strony.' : 'You do not have permission to view this page.');
    expect(await effects()).toEqual(baseline);
    expect((await sendForm(page, promotion.url, promotion.fields)).status()).toBe(403);
    expect(await effects()).toEqual(baseline);
    expect(await fees()).toEqual(revised);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('a Gold bid reserves exactly one amount in the fake Game bank', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const seller = await seededPlayer(page, lang);
    await enrollMfa(page, lang, seller);
    const recovery = await page.locator('textarea[readonly]').first().inputValue();
    await signInWithRecoveryCode(page, lang, seller, recovery);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    const [buyer, editor] = await foreignActors(browser, info.project, lang, contexts, 'content_editor');
    await page.goto(`/${lang}/account/sudo`);
    await submit(page, formSelector, { current_password: seller.password });
    const lot = await createBazaarListing({ page, id: seller.id }, { page: buyer.screen, id: buyer.account.id }, lang, 'item', 'GOLD');
    await page.goto(`/${lang}/account/privacy`);
    const { fields } = await prepare(page, formSelector);
    const gameBanks = async () => {
      const response = await page.request.get(`/__bazaar_game/state?asset=${lot.asset}`);
      expect(response.status()).toBe(200);
      const { banks } = await response.json();
      return banks;
    };
    const amounts = bank => bank ? { balance: bank.balance, held: bank.held } : null;
    const source = async () => amounts((await gameBanks())[buyer.account.id]);
    const unrelatedBanks = async () => {
      const banks = await gameBanks();
      return [amounts(banks[seller.id]), amounts(banks[editor.account.id])];
    };
    const bank = await source();
    const otherFunds = await unrelatedBanks();
    const snapshot = async () => ({
      bank: await source(),
      bids: await query(
        "SELECT buyer,maximum,phase FROM bazaar_authority_gold_bids WHERE listing_id=$1 AND phase<>'refused' ORDER BY request_id",
        [lot.id],
      ),
      coins: await query('SELECT account_id FROM bazaar_coin_holds WHERE id=$1', [lot.id]),
      listing: await query('SELECT leader,price,state FROM bazaar_listings WHERE id=$1', [lot.id]),
    });
    const baseline = { bank, bids: [], coins: [], listing: [{ leader: null, price: '100', state: 'active' }] };
    expect(bank.held).toBe(0);
    expect(await snapshot()).toEqual(baseline);
    expect((await buyer.screen.goto(lot.url)).status()).toBe(200);
    const bid = 'main form[action$="/bids"]';
    const form = { buyer_character_id: lot.destination, maximum: '200' };
    const placeBid = async screen => {
      const controls = screen.locator(bid);
      await controls.locator('[name=buyer_character_id]').fill(form.buyer_character_id);
      await controls.locator('[name=maximum]').fill(form.maximum);
      await controls.locator('button[type=submit]').click();
    };
    const order = await prepare(buyer.screen, bid, form);
    await rejected({ page: buyer.screen, ...order, snapshot, baseline });
    const selfBid = await sendForm(page, order.url, { ...order.fields, _csrf: fields._csrf });
    expect(selfBid.status()).toBe(403);
    expect(await snapshot()).toEqual(baseline);

    // A Gold request reaches the Game bank before a trusted receipt can
    // determine whether the destination character belongs to this bidder.
    expect((await editor.screen.goto(lot.url)).status()).toBe(200);
    const foreign = await prepare(editor.screen, bid, form);
    expect(foreign.fields.request_id).not.toBe(order.fields.request_id);
    await placeBid(editor.screen);
    await expect(editor.screen).toHaveURL(new URL(`/${lang}/bazaar/${lot.id}`, foreign.url).href);
    const refusal = () => query(
      `SELECT b.buyer,b.phase,b.release_operation_id,r.outcome,r.receipt::jsonb->>'result_code' AS reason
       FROM bazaar_authority_gold_bids b JOIN bazaar_authority_receipts r ON r.operation_id=b.hold_operation_id
       WHERE b.request_id=$1 AND b.listing_id=$2`, [foreign.fields.request_id, lot.id],
    );
    const refused = [{
      buyer: editor.account.id, phase: 'refused', release_operation_id: null,
      outcome: 'REJECTED', reason: 'CHAR_CMD_OWNER_MISMATCH',
    }];
    await expect.poll(refusal, { timeout: 90_000, intervals: [1000] }).toEqual(refused);
    expect(await snapshot()).toEqual(baseline);
    expect(await unrelatedBanks()).toEqual(otherFunds);
    expect((await sendForm(editor.screen, foreign.url, foreign.fields)).status()).toBe(303);
    expect(await refusal()).toEqual(refused);
    expect(await snapshot()).toEqual(baseline);
    expect(await unrelatedBanks()).toEqual(otherFunds);
    await invalid({ page: buyer.screen, ...order, snapshot, baseline, message: lang === 'pl'
      ? 'Prośba została odrzucona. Sprawdź konto i dane obiektu.'
      : 'This request was refused. Check your account and the asset details.', samples: [
      { name: 'empty maximum', patch: { maximum: '' } },
      { name: 'overlong maximum', patch: { maximum: '1'.repeat(80) } },
      { name: 'wrong number format', patch: { maximum: 'text' } },
      { name: 'zero maximum', patch: { maximum: '0' } },
    ] });
    await placeBid(buyer.screen);
    await expect(buyer.screen).toHaveURL(new URL(`/${lang}/bazaar/${lot.id}`, order.url).href);
    const accepted = {
      ...baseline, bank: { ...bank, held: 200 },
      bids: [{ buyer: buyer.account.id, maximum: '200', phase: 'accepted' }],
      listing: [{ leader: buyer.account.id, price: '100', state: 'active' }],
    };
    await expect.poll(snapshot, { timeout: 60_000, intervals: [1000] }).toEqual(accepted);
    await buyer.screen.reload();
    await expect(buyer.screen.locator(`[data-listing="${lot.id}"]`)).toHaveAttribute('data-state', 'active');
    expect((await sendForm(buyer.screen, order.url, order.fields)).status()).toBe(303);
    expect(await snapshot()).toEqual(accepted);
    // Advance only this fixture auction's real deadline to observe delivery.
    await query("UPDATE bazaar_listings SET ends_at=clock_timestamp()-interval '1 second' WHERE id=$1", [lot.id]);
    await expect.poll(() => query('SELECT state FROM bazaar_listings WHERE id=$1', [lot.id]),
      { timeout: 90_000, intervals: [1000] }).toEqual([{ state: 'completed' }]);
    await buyer.screen.reload();
    await expect(buyer.screen.locator('main')).toContainText(lang === 'pl' ? 'Sprzedaż zakończona' : 'Sale completed');
    expect(await source()).toEqual({ balance: bank.balance - 100, held: 0 });
    expect(await unrelatedBanks()).toEqual([
      { ...otherFunds[0], balance: otherFunds[0].balance + 97 }, otherFunds[1],
    ]);
    expect(await query(
      'SELECT gross_amount,fee_amount,seller_amount FROM bazaar_authority_gold_sales WHERE listing_id=$1',
      [lot.id],
    )).toEqual([{ gross_amount: '100', fee_amount: '3', seller_amount: '97' }]);
    expect((await snapshot()).coins).toEqual([]);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('staff Bazaar forms commit review, restriction and compensated invalidation under independent authority', async ({ page, browser }, info) => {
  test.setTimeout(720_000);
  const lang = info.project.metadata.locale;
  const contexts = [];
  try {
    const seller = await seededPlayer(page, lang);
    await enrollMfa(page, lang, seller);
    const recovery = await page.locator('textarea[readonly]').first().inputValue();
    await signInWithRecoveryCode(page, lang, seller, recovery);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    const [player, editor] = await foreignActors(browser, info.project, lang, contexts, 'content_editor');
    const authorize = async () => {
      const context = await browser.newContext({
        baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
        viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      return { screen, account: await seededPlayer(screen, lang, 'platform_admin') };
    };
    const requester = await authorize();
    const reviewer = await authorize();
    await page.goto(`/${lang}/account/sudo`);
    await submit(page, formSelector, { current_password: seller.password });
    const lot = await createBazaarListing({ page, id: seller.id }, { page: player.screen, id: player.account.id }, lang);
    const panel = `/admin/${lang}/bazaar`;
    const form = action => `main form:has([name=action][value=${action}])`;
    const reason = 'Verify staff marketplace custody in the disposable fixture.';
    const message = lang === 'pl'
      ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    const validate = async (screen, selector, values, snapshot, patches) => {
      expect((await screen.goto(panel)).status()).toBe(200);
      const params = await prepare(screen, selector, values);
      const baseline = await snapshot();
      await rejected({ page: screen, ...params, snapshot, baseline });
      for (const { screen: outsider, csrf } of [player, editor]) {
        expect((await sendForm(outsider, params.url, { ...params.fields, _csrf: csrf })).status()).toBe(403);
        expect(await snapshot()).toEqual(baseline);
      }
      await invalid({ page: screen, ...params, message, snapshot, baseline,
        samples: patches.map(([name, patch]) => ({ name, patch })) });
      return params;
    };
    const submitStaffMutation = async (screen, selector, values, target) => {
      const form = screen.locator(selector);
      for (const [name, value] of Object.entries(values)) {
        await form.locator(`[name="${name}"]`).fill(value);
      }
      const refresh = screen.waitForResponse(response => response.url() === target && response.request().method() === 'GET',
        { timeout: 120_000 }).then(response => response.status(), () => null);
      const [posted] = await Promise.all([
        screen.waitForResponse(response => response.url() === target && response.request().method() === 'POST',
          { timeout: 120_000 }),
        form.locator('button[type=submit], button:not([type])').click(),
      ]);
      expect(posted.status()).toBe(303);
      expect(posted.headers().location).toBe(panel);
      expect(await refresh).toBe(200);
    };
    const nil = '00000000-0000-0000-0000-000000000000';
    const identifiers = name => [
      ['empty identifier', { [name]: '' }], ['overlong identifier', { [name]: '1'.repeat(65) }],
      ['wrong identifier type', { [name]: 'text' }], ['nil identifier outside the v7 boundary', { [name]: nil }],
    ];
    const recover = () => query(
      `SELECT l.state, h.revision::int AS hold_revision, i.id, i.state AS approval, i.approved_by, i.requested_by,
         (SELECT count(*)::int FROM bazaar_moderation_outbox WHERE listing_id=$1) AS reconciliations,
         (SELECT count(*)::int FROM bazaar_game_outbox o JOIN bazaar_authority_receipts r USING(operation_id)
          WHERE o.listing_id=$1 AND o.command='ReturnBazaarItem' AND r.outcome='COMMITTED') AS returns
       FROM bazaar_listings l LEFT JOIN bazaar_moderation_holds h ON h.listing_id=l.id
       LEFT JOIN bazaar_invalidation_requests i ON i.listing_id=l.id WHERE l.id=$1`, [lot.id],
    );
    const baseline = { state: 'active', hold_revision: null, id: null, approval: null, approved_by: null,
      requested_by: null, reconciliations: 0, returns: 0 };
    expect(await recover()).toEqual([baseline]);
    const suspend = form('suspend');
    const detail = { listing: lot.id, reason };
    const suspension = await validate(requester.screen, suspend, detail, recover, [
      ['empty reason', { reason: '' }], ['overlong reason', { reason: 'x'.repeat(501) }],
      ['wrong listing type', { listing: 'text' }], ['reason below ten characters', { reason: '123456789' }],
    ]);
    await submitStaffMutation(requester.screen, suspend, detail, suspension.url);
    await expect(requester.screen.locator('main')).toContainText(lot.id);
    const suspended = { ...baseline, hold_revision: 1 };
    expect(await recover()).toEqual([suspended]);

    const restriction = form('restrict');
    const restricted = {
      world: '0192a000-0000-7000-8000-000000000012', family: 'item', production_key: `matrix-restricted-${randomUUID()}`,
      revision_ref: 'v1', minimum_value: '4', reason,
    };
    const rules = () => query(
      'SELECT production_key,minimum_value::int AS minimum,denied,actor FROM bazaar_item_restrictions WHERE production_key=$1',
      [restricted.production_key],
    );
    const policy = await validate(requester.screen, restriction, restricted, rules, [
      ['empty family', { family: '' }], ['overlong reason', { reason: 'x'.repeat(501) }],
      ['wrong minimum type', { minimum_value: 'text' }], ['zero minimum', { minimum_value: '0' }],
    ]);
    await submitStaffMutation(requester.screen, restriction, restricted, policy.url);
    await expect(requester.screen.locator('main')).toContainText(restricted.production_key);
    expect(await rules()).toEqual([{
      production_key: restricted.production_key, minimum: 4, denied: true, actor: requester.account.id,
    }]);

    // The live player's report endpoint requires an integration outside the
    // fixture. Seed the two historical reports that staff are reviewing.
    for (const [reporter, accepted] of [[player.account, true], [editor.account, false]]) {
      const id = uuid7();
      await query('INSERT INTO bazaar_reports(id,listing_id,reporter,category,detail) VALUES($1,$2,$3,$4,$5)',
        [id, lot.id, reporter.id, 'fraud', reason]);
      const report = () => query('SELECT state,reviewed_by FROM bazaar_reports WHERE id=$1', [id]);
      const selector = `main form:has([name=action][value=review]):has([name=id][value="${id}"])`;
      const review = await validate(requester.screen, selector, { accepted: String(accepted) }, report, identifiers('id'));
      await requester.screen.locator(`${selector} select[name=accepted]`).selectOption(String(accepted));
      await submitStaffMutation(requester.screen, selector, {}, review.url);
      await expect(requester.screen.locator(selector)).toHaveCount(0);
      const saved = [{ state: accepted ? 'accepted' : 'dismissed', reviewed_by: requester.account.id }];
      expect(await report()).toEqual(saved);
      expect((await sendForm(requester.screen, review.url, review.fields)).status()).toBe(409);
      expect(await report()).toEqual(saved);
    }

    const request = form('request');
    const proposed = await validate(requester.screen, request, detail, recover, identifiers('id'));
    await submitStaffMutation(requester.screen, request, detail, proposed.url);
    await expect(requester.screen.locator('main')).toContainText(proposed.fields.id);
    const pending = { ...suspended, id: proposed.fields.id, approval: 'pending', requested_by: requester.account.id };
    expect(await recover()).toEqual([pending]);
    expect((await sendForm(requester.screen, proposed.url, proposed.fields)).status()).toBe(503);
    expect(await recover()).toEqual([pending]);
    const approve = form('approve');
    const verdict = await validate(reviewer.screen, approve, { id: pending.id }, recover, identifiers('id'));
    const self = await prepare(requester.screen, approve, { id: pending.id });
    expect((await sendForm(requester.screen, self.url, self.fields)).status()).toBe(403);
    expect(await recover()).toEqual([pending]);
    await submitStaffMutation(reviewer.screen, approve, { id: pending.id }, verdict.url);
    await expect(reviewer.screen).toHaveURL(new URL(panel, verdict.url).href);
    await expect(reviewer.screen.locator('main')).not.toContainText(pending.id);
    const expected = { ...pending, state: 'cancelled', hold_revision: null, approval: 'approved',
      approved_by: reviewer.account.id, reconciliations: 1, returns: 1 };
    await expect.poll(recover, { timeout: 90_000, intervals: [1000] }).toEqual([expected]);
    expect((await sendForm(reviewer.screen, verdict.url, verdict.fields)).status()).toBe(409);
    expect(await recover()).toEqual([expected]);
    const game = await page.request.get(`/__bazaar_game/state?asset=${lot.asset}`);
    expect(game.status()).toBe(200);
    expect((await game.json()).asset.owner).toBe(seller.id);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});
