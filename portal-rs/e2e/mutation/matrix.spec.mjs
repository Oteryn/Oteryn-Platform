import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash, createHmac, randomBytes, randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { login, logout, password, pool, submit, waitForLoginBudget } from '../helpers.mjs';
import { enrollMfa, loginWithRecoveryCode, player, query } from '../staff-rbac-helpers.mjs';
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
