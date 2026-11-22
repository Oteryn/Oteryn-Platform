import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash, createHmac, randomBytes, randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { login, logout, password, pool, seedPasswordRange, submit, waitForLoginBudget } from '../helpers.mjs';
import { action, enrollMfa, loginWithRecoveryCode, player, query, rolePage } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';

import { prepare, sendForm, useFormMethod, rejected, wrongActor, invalid } from './matrix-helpers.mjs';

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
    await loginWithRecoveryCode(page, lang, account, code);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
  }
  return { ...account, id };
}

const formSelector = 'main form[method=post]';

for (const method of ['POST', 'PUT']) {
  test(`account privacy preferences (${method})`, async ({ page, browser }, info) => {
    const lang = info.project.metadata.locale;
    const anonymous = await browser.newContext({
      baseURL: info.project.use.baseURL,
      ignoreHTTPSErrors: true,
      locale: lang,
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
  });
  try {
    const owner = await player(page, lang);
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
    const replay = await sendForm(page, url, fields);
    expect(replay.status()).toBe(302);
    expect(replay.headers().location).toBe(target);
    expect(await snapshot()).toEqual(created);
  } finally {
    await anonymous.close();
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
      pl: 'Przepustka sezonowa jest obecnie niedostępna. Nie pobrano coinów ani nie wydano nagród.',
      en: 'The season pass is currently unavailable. No coins were charged and no rewards were issued.',
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
  await seededPlayer(page, lang);
  for (const surface of unavailable) {
    await test.step(`${surface.family} (${surface.routes.join(', ')})`, async () => {
      const response = await page.goto(surface.target(lang));
      expect(response.status()).toBe(surface.status);
      expect(response.headers()['cache-control']).toContain('no-store');
      await expect(page.locator('main')).toContainText(surface.message[lang]);
      await expect(page.locator(formSelector)).toHaveCount(0);
    });
  }
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
  });
  try {
    const owner = await player(page, lang);
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
    const replay = await sendForm(page, url, fields);
    expect(replay.status()).toBe(303);
    expect(replay.headers().location).toBe(`/${lang}/shop/test-checkout/${intent}`);
    expect(await snapshot()).toEqual(created);
  } finally {
    await anonymous.close();
  }
});

for (const action of ['pay', 'reject', 'cancel']) {
  test(`test-provider checkout decision ${action} settles once`, async ({ page, browser }, info) => {
    test.setTimeout(180_000);
    const lang = info.project.metadata.locale;
    const foreign = await browser.newContext({
      baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    });
    try {
      const other = await foreign.newPage();
      await seededPlayer(other, lang);
      await other.goto(`/${lang}/shop/coins`);
      const foreignToken = await other.locator(`${packageForm} [name=_csrf]`).inputValue();

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
      expect((await sendForm(other, url, { ...fields, _csrf: foreignToken })).status()).toBe(404);
      expect(await snapshot()).toEqual(baseline);
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
      await foreign.close();
    }
  });
}

for (const statusMethod of ['POST', 'PUT']) {
  test(`ticket owner reply, close, and reopen enforce ownership and version (${statusMethod})`, async ({ page, browser }, info) => {
    const lang = info.project.metadata.locale;
    const foreign = await browser.newContext({
      baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    });
    try {
      const other = await foreign.newPage();
      await seededPlayer(other, lang);
      await other.goto(`/${lang}/support/tickets/create`);
      const foreignToken = await other.locator(`${formSelector} [name=_csrf]`).inputValue();

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
          expect((await sendForm(other, url, { ...fields, _csrf: foreignToken }, method)).status()).toBe(404);
          expect(await snapshot()).toEqual(baseline);
          await invalid({ page, url, fields, samples: variant.samples, message: error, snapshot, baseline, method });
          await useFormMethod(page, url, method, () => variant.submit(selector));
          await expect(page.locator('main')).toContainText(variant.visible);
          expect(await snapshot()).toEqual(variant.effect);
          expect((await sendForm(page, url, fields, method)).status()).toBe(409);
          expect(await snapshot()).toEqual(variant.effect);
        });
      }
    } finally {
      await foreign.close();
    }
  });
}

test('character creation queues one operation and exposes the pending Game outcome', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const anonymous = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
  });
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
    const replay = await sendForm(page, fresh.url, fresh.fields);
    expect(replay.status()).toBe(302);
    expect(replay.headers().location).toBe(target);
    expect(await snapshot()).toEqual(queued);
  } finally {
    await anonymous.close();
  }
});

test('support ticket reply and resolution require the support permission', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
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
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});

test('player report persists one private report for the submitted request key', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const reader = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
  });
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
    await seededPlayer(other, lang);
    expect((await other.goto(target)).status()).toBe(404);
    expect(await snapshot()).toEqual(submitted);
  } finally {
    await reader.close();
  }
});

test('wallet transfer binds one debit and credit to the sending actor', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const recipientContext = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
  });
  const otherContext = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
  });
  try {
    const sender = await seededPlayer(page, lang);
    const recipient = await seededPlayer(await recipientContext.newPage(), lang);
    const otherPage = await otherContext.newPage();
    await seededPlayer(otherPage, lang, 'content_editor');

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
    expect((await otherPage.goto(transfer)).status()).toBe(200);
    const otherToken = await otherPage.locator(`${selector} [name=_csrf]`).inputValue();
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
    expect((await sendForm(otherPage, url, { ...fields, _csrf: otherToken })).status()).toBe(403);
    expect(await snapshot()).toEqual(baseline);
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
    expect((await sendForm(otherPage, url, { ...fields, _csrf: otherToken })).status()).toBe(409);
    expect(await snapshot()).toEqual(done);
  } finally {
    await Promise.all([otherContext.close(), recipientContext.close()]);
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
  });
  try {
    const actor = await seededPlayer(page, lang);
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

    const email = new URL(`${report}/email`, ready.url).href;
    const snapshot = () => query('SELECT email_enabled FROM pd_report_preferences WHERE account_id=$1', [actor.id]);
    const preferences = await snapshot();
    expect(preferences).toEqual([]);
    const optIn = { ...fields, enabled: 'true' };
    await rejected({ page, url: email, fields: optIn, snapshot, baseline: preferences });
    await wrongActor({
      page: await anonymous.newPage(), lang, url: email, fields: optIn, status: 403, snapshot, baseline: preferences,
    });
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
    await anonymous.close();
  }
});

test('localized and unprefixed badge forms persist only the owner selection', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const other = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
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
  const context = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const owner = await seededPlayer(page, lang);
    const other = await context.newPage();
    await seededPlayer(other, lang);
    const markerPath = '/atlas/private-markers';
    const reportPath = '/atlas/map-reports';
    const browserPath = `/${lang}/atlas`;
    expect((await page.goto(browserPath)).status()).toBe(503);
    await expect(page.locator('main form')).toHaveCount(0);
    const ownResponse = await page.request.get(markerPath);
    expect(ownResponse.status()).toBe(200);
    const ownMarkers = await ownResponse.json();
    expect(ownMarkers.markers).toEqual([]);
    const otherMarkers = await (await other.request.get(markerPath)).json();
    expect(otherMarkers.markers).toEqual([]);
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
    }
    const created = await request(page, markerPath, 'POST', values, ownMarkers.csrf_token);
    expect(created.status()).toBe(201);
    const marker = { id: (await created.json()).id, x: 42, y: 86, floor: 7, label: text };
    expect(await snapshot()).toEqual({ ...baseline, markers: [marker] });
    expect((await (await page.request.get(markerPath)).json()).markers).toEqual([marker]);
    expect((await (await other.request.get(markerPath)).json()).markers).toEqual([]);
    const target = `${markerPath}/${marker.id}`;
    for (const token of [undefined, 'invalid']) {
      expect((await request(page, target, 'DELETE', undefined, token)).status()).toBe(403);
      expect(await snapshot()).toEqual({ ...baseline, markers: [marker] });
    }
    expect((await request(other, target, 'DELETE', undefined, otherMarkers.csrf_token)).status()).toBe(404);
    expect(await snapshot()).toEqual({ ...baseline, markers: [marker] });
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
    await context.close();
  }
});

test('console exposes unavailable Game agents and creates no read requests', async ({ page }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const viewer = await seededPlayer(page, lang, 'game_viewer');
  await page.goto(`/${lang}/account/privacy`);
  const { url, fields } = await prepare(page, formSelector);
  const snapshot = () => query('SELECT request_id FROM admin_control_requests WHERE actor=$1', [viewer.id]);
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
    expect((await sendForm(page, options.url, options.fields)).status()).toBe(503);
    expect(await snapshot()).toEqual(baseline);
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
        expect((await sendForm(page, url, { ...fields, ...patch })).status()).toBe(400);
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
  });
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
    await context.close();
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
      await expect(page.locator('main h1')).toHaveText('Sandbox coin checkout');
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
      await expect(page.locator('main dl dd').last()).toHaveText(state);
      await expect(page.locator(formSelector)).toHaveCount(0);
      const closed = [{ state, receipt: paid, events: 1, credits: paid ? 1 : 0, coins: paid ? String(pkg.coins) : '0' }];
      expect(await snapshot()).toEqual(closed);
      for (const action of [decision, paid ? 'fail' : 'pay']) {
        expect((await sendForm(page, url, { ...fields, action })).status()).toBe(303);
        expect(await snapshot()).toEqual(closed);
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
        expect((await sendForm(page, url, { ...fields, ...patch }, method)).status()).toBe(status);
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
    const draft = {
      starts_at: dateInput(1), ends_at: dateInput(24),
      en_title: title, en_slug: slug, en_summary: 'Local event', en_body: 'English event description',
      pl_title: 'Lokalne wydarzenie', pl_slug: slug, pl_summary: 'Wydarzenie testowe', pl_body: 'Opis wydarzenia',
    };
    const detailsSelector = 'main form:has([name=en_title])';
    const invalidDraft = [
      { name: 'empty title', patch: { en_title: '' } },
      { name: 'overlong title', patch: { en_title: 'a'.repeat(201) } },
      { name: 'invalid time representation', patch: { starts_at: 'not-a-datetime' } },
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
      const data = { en_title: nextTitle };
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
      const form = await reject(selector, { status }, [
        { name: 'empty target status', patch: { status: '' } },
        { name: 'overlong target status', patch: { status: 'x'.repeat(512) } },
        { name: 'numeric target status', patch: { status: '1' } },
        { name: 'unknown status beyond the accepted values', patch: { status: 'archived ' } },
      ], method);
      await page.locator(`${selector} select[name=status]`).selectOption(status);
      await useFormMethod(page, form.url, method, () => submit(page, selector));
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
        expect((await sendForm(page, url, { ...fields, ...patch })).status()).toBe(status);
        expect(await state()).toEqual(baseline);
      }
    };

    expect((await page.goto(url)).status()).toBe(200);
    expect(await state()).toBeNull();
    const create = 'main form:has([name=key])';
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
      await verify(membership, { action: operation, account: target.player.id, role: created.id }, [
        [{ action: '' }, 400], [{ action: 'x'.repeat(129) }, 400],
        [{ role: 'x' }, 422], [{ account: '00000000-0000-0000-0000-000000000000' }, 400],
      ]);
      await action(page, membership, { account: target.player.id, role: created.id }, operation);
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
      const context = await browser.newContext({ baseURL: origin, ignoreHTTPSErrors: true, locale: lang });
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
