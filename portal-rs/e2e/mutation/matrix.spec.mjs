import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, password, pool, submit } from '../helpers.mjs';
import { enrollMfa, loginWithRecoveryCode, player, query } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';

async function prepare(page, selector, values) {
  const form = page.locator(selector);
  await expect(form).toHaveCount(1);
  const { url, fields } = await form.evaluate(element => ({
    url: new URL(element.getAttribute('action') || element.ownerDocument.URL, element.baseURI).href,
    fields: Object.fromEntries(new FormData(element)),
  }));
  expect(Boolean(fields._csrf), 'the rendered form supplies a CSRF token').toBe(true);
  return { url, fields: { ...fields, ...values } };
}

async function post(page, url, form) {
  return page.request.post(url, {
    form, headers: { origin: new URL(url).origin }, maxRedirects: 0,
  });
}

async function rejected({ page, url, fields, snapshot, baseline }) {
  await csrfRejected(page, url, fields);
  expect(await snapshot()).toEqual(baseline);
  expect((await post(page, url, { ...fields, _csrf: 'invalid' })).status()).toBe(403);
  expect(await snapshot()).toEqual(baseline);
}

async function wrongActor({ page, lang, url, fields, status, snapshot, baseline }) {
  await page.goto(`/${lang}/login`);
  const csrf = await page.locator('main form [name=_csrf]').inputValue();
  expect(Boolean(csrf), 'anonymous actor has its own CSRF token').toBe(true);
  const response = await post(page, url, { ...fields, _csrf: csrf });
  expect(response.status()).toBe(status);
  expect(await snapshot()).toEqual(baseline);
}

async function invalid({ page, url, fields, samples, message, snapshot, baseline }) {
  for (const { name, patch } of samples) {
    await test.step(name, async () => {
      const response = await post(page, url, { ...fields, ...patch });
      expect(response.status()).toBe(422);
      expect(await response.text()).toContain(message);
      expect(await snapshot()).toEqual(baseline);
    });
  }
}

const portal = fileURLToPath(new URL('../../', import.meta.url));

async function seededAccount(lang) {
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
  return { ...account, id };
}

async function seededPlayer(page, lang) {
  const account = await seededAccount(lang);
  await login(page, lang, account);
  return account;
}

async function seededStaff(page, lang, role, permission, allowed) {
  const account = await seededAccount(lang);
  expect(await query(
    `INSERT INTO account_roles(account_id,role_id)
     SELECT $1,id FROM roles WHERE key=$2 AND built_in RETURNING account_id`,
    [account.id, role],
  )).toEqual([{ account_id: account.id }]);
  expect(await query(
    `SELECT EXISTS(SELECT 1 FROM account_roles ar JOIN role_permissions rp ON rp.role_id=ar.role_id
                   WHERE ar.account_id=$1 AND rp.permission=$2) AS allowed`,
    [account.id, permission],
  )).toEqual([{ allowed }]);
  await login(page, lang, account);
  await enrollMfa(page, lang, account);
  const recovery = await page.locator('textarea[readonly]').first().inputValue();
  await loginWithRecoveryCode(page, lang, account, recovery);
  return account;
}

async function foreignPost(page, lang, url, fields, status, snapshot, baseline) {
  expect((await page.goto(`/${lang}/account/privacy`)).status()).toBe(200);
  const { fields: { _csrf } } = await prepare(page, formSelector, {});
  expect((await post(page, url, { ...fields, _csrf })).status()).toBe(status);
  expect(await snapshot()).toEqual(baseline);
}

const formSelector = 'main form[method=post]';

test('account privacy preferences', async ({ page, browser }, info) => {
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
      `SELECT a.public_account_association, a.public_status_visible, a.locale,
              a.privacy_revision,
              ARRAY(SELECT e.revision::text FROM account_privacy_events e
                    WHERE e.account_id=a.id ORDER BY e.revision) AS events
       FROM accounts a WHERE a.id=$1`,
      [account.id],
    );
    const baseline = await snapshot();
    expect(baseline).toHaveLength(1);
    const { url, fields } = await prepare(page, formSelector, {
      public_account_association: '1', public_status_visible: '1',
    });
    await rejected({ page, url, fields, snapshot, baseline });
    await wrongActor({ page: await anonymous.newPage(), lang, url, fields, status: 403, snapshot, baseline });
    await invalid({
      page, url, fields, snapshot, baseline,
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
    const events = [...baseline[0].events];
    for (const increment of [1n, 2n]) {
      await page.locator('[name=public_account_association]').check();
      await page.locator('[name=public_status_visible]').check();
      const [saved] = await Promise.all([
        page.waitForResponse(response => response.url() === url && response.request().method() === 'POST'),
        page.locator(formSelector).getByRole('button', {
          name: lang === 'pl' ? 'Zapisz ustawienia' : 'Save preferences', exact: true,
        }).click(),
      ]);
      expect(saved.status()).toBe(303);
      expect(saved.headers().location).toBe(`/${lang}/account/privacy`);
      await expect(page.locator('[name=public_account_association]')).toBeChecked();
      await expect(page.locator('[name=public_status_visible]')).toBeChecked();
      const revision = String(BigInt(baseline[0].privacy_revision) + increment);
      events.push(revision);
      expect(await snapshot()).toEqual([{
        public_account_association: true, public_status_visible: true, locale: lang,
        privacy_revision: revision, events,
      }]);
    }
  } finally {
    await anonymous.close();
  }
});

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
      `SELECT t.id, t.account_id, t.request_key, t.subject, m.body FROM tickets t
       LEFT JOIN ticket_messages m ON m.ticket_id=t.id WHERE t.account_id=$1
       ORDER BY t.id, m.id`,
      [owner.id],
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
    const created = [{
      id: target.split('/').at(-1), account_id: owner.id, request_key: fields.request_key, subject, body,
    }];
    expect(await snapshot()).toEqual(created);
    const replay = await post(page, url, fields);
    expect(replay.status()).toBe(302);
    expect(replay.headers().location).toBe(target);
    expect(await snapshot()).toEqual(created);
  } finally {
    await anonymous.close();
  }
});

test('unavailable Game-backed browser actions refuse writes without durable effects', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const options = {
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  };
  const anonymous = await browser.newContext(options);
  const foreign = await browser.newContext(options);
  try {
    const owner = await seededPlayer(page, lang);
    const other = await foreign.newPage();
    const visitor = await anonymous.newPage();
    const stranger = await seededPlayer(other, lang);
    const accounts = [owner.id, stranger.id];
    const csrf = async actor => {
      expect((await actor.goto(`/${lang}/account/privacy`)).status()).toBe(200);
      return (await prepare(actor, formSelector, {})).fields._csrf;
    };
    const ownerCsrf = await csrf(page);
    const foreignCsrf = await csrf(other);
    expect((await visitor.goto(`/${lang}/login`)).status()).toBe(200);
    const anonymousCsrf = (await prepare(visitor, formSelector, {})).fields._csrf;

    const character = uuid7();
    const deck = uuid7();
    const battle = uuid7();
    const card = uuid7();
    const season = uuid7();
    const buy = uuid7();
    const claim = uuid7();
    const puzzle = uuid7();
    const missing = uuid7();

    // Portal-owned records provide real foreign IDs. Game ownership and progress
    // remain unavailable, so the fixtures cannot authorize starting or playing.
    await query(
      `INSERT INTO collection_decks (id,account_id,character_id,name,cards)
       VALUES ($1,$2,$3,'Ownership fixture',ARRAY[$4::uuid])`,
      [deck, owner.id, character, card],
    );
    const fighter = { card, health: 30 };
    await query(
      `INSERT INTO collection_battles (id,account_id,character_id,deck_id,state)
       VALUES ($1,$2,$3,$4,$5)`,
      [battle, owner.id, character, deck, {
        ruleset: 1, turn: 0, player: [fighter], enemy: [fighter],
        outcome: 'active', seed: 1, last_action: null,
      }],
    );

    const collectionState = () => query(
      `SELECT (SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM collection_decks d
                WHERE account_id=ANY($1::uuid[]) OR id=$2) AS decks,
              (SELECT jsonb_agg(to_jsonb(b) ORDER BY id) FROM collection_battles b
                WHERE account_id=ANY($1::uuid[]) OR id=$3) AS battles`,
      [accounts, deck, battle],
    );
    const passState = () => query(
      `SELECT (SELECT jsonb_agg(to_jsonb(p) ORDER BY account_id,season_id) FROM pass_progress p
                WHERE account_id=ANY($1::uuid[]) OR season_id=$2) AS progress,
              (SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM pass_claims c
                WHERE account_id=ANY($1::uuid[]) OR season_id=$2) AS claims,
              (SELECT jsonb_agg(to_jsonb(w) ORDER BY id) FROM wallet_operations w
                WHERE account_id=ANY($1::uuid[]) OR id=ANY($3::uuid[])) AS operations,
              (SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM wallet_entries e
                WHERE account_id=ANY($1::uuid[]) OR operation_id=ANY($3::uuid[])) AS entries,
              (SELECT jsonb_agg(to_jsonb(u) ORDER BY entitlement_id) FROM store_fulfilment_units u
                WHERE account_id=ANY($1::uuid[])) AS entitlements,
              (SELECT jsonb_agg(to_jsonb(o) ORDER BY operation_id) FROM store_operations o
                JOIN store_fulfilment_units u USING (entitlement_id)
                WHERE u.account_id=ANY($1::uuid[]) OR o.operation_id=ANY($3::uuid[])) AS delivery`,
      [accounts, season, [buy, claim]],
    );
    const puzzleState = () => query(
      `SELECT (SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM puzzles_answers a
                WHERE account_id=ANY($1::uuid[]) OR puzzle_id=$2) AS answers,
              (SELECT jsonb_agg(to_jsonb(s) ORDER BY account_id) FROM puzzles_streaks s
                WHERE account_id=ANY($1::uuid[])) AS streaks`,
      [accounts, puzzle],
    );
    const surfaces = [
      {
        target: `collection/${character}`, status: 503, anonymousStatus: 303,
        message: lang === 'pl'
          ? 'Kolekcja jest niedostępna. Wymagana jest weryfikacja aktualnego prywatnego postępu i właściciela postaci.'
          : 'Collection is unavailable. Current private progress and ownership must be verified.',
        snapshot: collectionState,
        writes: [
          { route: `collection/${character}/decks`, form: { name: 'Rejected deck', first: card }, owner: 503, other: 503 },
          { route: `collection/decks/${deck}/battle`, owner: 503, other: 403 },
          { route: `collection/decks/${deck}/delete`, ownerRoute: `collection/decks/${missing}/delete`, owner: 403, other: 403 },
          { route: `collection/battles/${battle}`, form: { expected: '0', action: 'strike' }, owner: 503, other: 403 },
          { route: `collection/battles/${battle}/forfeit`, ownerRoute: `collection/battles/${missing}/forfeit`, owner: 403, other: 403 },
        ],
      },
      {
        target: 'pass', status: 503, anonymousStatus: 401,
        message: lang === 'pl'
          ? 'Przepustka sezonowa jest obecnie niedostępna. Nie pobrano coinów ani nie wydano nagród.'
          : 'The season pass is currently unavailable. No coins were charged and no rewards were issued.',
        snapshot: passState,
        writes: [
          { route: `pass/${season}/buy`, form: { operation: buy }, owner: 404, other: 404 },
          { route: `pass/${season}/claim`, form: { operation: claim, reward: uuid7() }, owner: 404, other: 404 },
        ],
      },
      {
        target: 'puzzles', status: 200, anonymousStatus: 303,
        message: lang === 'pl' ? 'Dzisiejsze zagadki są niedostępne.' : "Today's puzzles are unavailable.",
        snapshot: puzzleState,
        writes: [{ route: `puzzles/${puzzle}/answer`, form: { choice: '0' }, owner: 404, other: 404 }],
      },
    ];
    for (const surface of surfaces) {
      const view = await page.goto(`/${lang}/${surface.target}`);
      expect(view.status()).toBe(surface.status);
      expect(view.headers()['cache-control']).toContain('no-store');
      await expect(page.locator('main')).toContainText(surface.message);
      await expect(page.locator(formSelector)).toHaveCount(0);
      const baseline = await surface.snapshot();
      for (const { route, ownerRoute = route, form = {}, owner: ownerStatus, other: otherStatus } of surface.writes) {
        await test.step(`POST /${lang}/${route}`, async () => {
          const url = new URL(`/${lang}/${route}`, page.url()).href;
          await rejected({ page, url, fields: form, snapshot: surface.snapshot, baseline });
          const attempts = [
            [visitor, route, anonymousCsrf, surface.anonymousStatus],
            [other, route, foreignCsrf, otherStatus],
            [page, ownerRoute, ownerCsrf, ownerStatus],
          ];
          for (const [actor, target, token, status] of attempts) {
            const response = await post(actor, new URL(`/${lang}/${target}`, url).href, { ...form, _csrf: token });
            expect(response.status()).toBe(status);
            if (status === 303) expect(response.headers().location).toBe(`/${lang}/login`);
            expect(await surface.snapshot()).toEqual(baseline);
          }
        });
      }
    }
  } finally {
    await Promise.all([anonymous.close(), foreign.close()]);
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
    const replay = await post(page, url, fields);
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
      expect((await post(other, url, { ...fields, _csrf: foreignToken })).status()).toBe(404);
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
      expect((await post(page, url, fields)).status()).toBe(303);
      expect(await snapshot()).toEqual(settled);
    } finally {
      await foreign.close();
    }
  });
}

test('ticket owner reply and close enforce ownership and version', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  test.setTimeout(180_000);
  const options = {
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  };
  const foreign = await browser.newContext(options);
  const staff = await browser.newContext(options);
  const anonymous = await browser.newContext(options);
  try {
    const other = await foreign.newPage();
    await seededPlayer(other, lang);
    const unpermitted = await staff.newPage();
    await seededStaff(unpermitted, lang, 'content_editor', 'support.tickets.manage', false);
    const signedOut = await anonymous.newPage();

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
      {
        action: 'status', fields: { status: 'closed' },
        samples: [
          { name: 'empty status', patch: { status: '' } },
          { name: 'overlong status', patch: { status: 'x'.repeat(512) } },
          { name: 'malformed lock version', patch: { lock_version: 'bad' } },
          { name: 'lock version below one', patch: { lock_version: '0' } },
        ],
        submit: async selector => {
          await page.locator(`${selector} select[name=status]`).selectOption('closed');
          await submit(page, selector);
        },
        visible: lang === 'pl' ? 'Zamknięte' : 'Closed',
        effect: ['Original question', 'Owner follow-up'].map(body => ({
          status: 'closed', lock_version: '3', body,
        })),
      },
    ];
    expect(await snapshot()).toEqual([{ status: 'open', lock_version: '1', body: 'Original question' }]);
    for (const variant of variants) {
      await test.step(variant.action, async () => {
        const baseline = await snapshot();
        const selector = `main form[action$="/${variant.action}"]`;
        const { url, fields } = await prepare(page, selector, variant.fields);
        await rejected({ page, url, fields, snapshot, baseline });
        await foreignPost(other, lang, url, fields, 404, snapshot, baseline);
        await foreignPost(unpermitted, lang, url, fields, 404, snapshot, baseline);
        await wrongActor({ page: signedOut, lang, url, fields, status: 403, snapshot, baseline });
        await invalid({ page, url, fields, samples: variant.samples, message: error, snapshot, baseline });
        await variant.submit(selector);
        await expect(page.locator('main')).toContainText(variant.visible);
        expect(await snapshot()).toEqual(variant.effect);
        expect((await post(page, url, fields)).status()).toBe(409);
        expect(await snapshot()).toEqual(variant.effect);
      });
    }
  } finally {
    await Promise.all([foreign.close(), staff.close(), anonymous.close()]);
  }
});

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
        const response = await post(page, url, { ...fields, ...patch });
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
    const replay = await post(page, fresh.url, fresh.fields);
    expect(replay.status()).toBe(302);
    expect(replay.headers().location).toBe(target);
    expect(await snapshot()).toEqual(queued);
  } finally {
    await anonymous.close();
  }
});

test('support ticket staff reply and status enforce RBAC and stale writes', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const options = {
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang,
    viewport: info.project.use.viewport,
  };
  const support = await browser.newContext(options);
  const editor = await browser.newContext(options);
  const foreign = await browser.newContext(options);
  const anonymous = await browser.newContext(options);
  try {
    const owner = await seededPlayer(page, lang);
    const operator = await support.newPage();
    const deniedStaff = await editor.newPage();
    const otherPlayer = await foreign.newPage();
    const signedOut = await anonymous.newPage();
    await seededStaff(operator, lang, 'support', 'support.tickets.manage', true);
    await seededStaff(deniedStaff, lang, 'content_editor', 'support.tickets.manage', false);
    await seededPlayer(otherPlayer, lang);
    await page.goto(`/${lang}/support/tickets/create`);
    await submit(page, formSelector, { subject: 'Staff coverage ticket', body: 'Player question' });
    await expect(page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
    const playerPath = new URL(page.url()).pathname;
    const id = playerPath.split('/').at(-1);
    const staffPath = `/admin/support/tickets/${id}`;
    const snapshot = () => query(
      `SELECT t.status, t.lock_version,
        ARRAY(SELECT m.body FROM ticket_messages m WHERE m.ticket_id=t.id ORDER BY m.id) AS messages
       FROM tickets t WHERE t.id=$1 AND t.account_id=$2`, [id, owner.id],
    );
    const cases = [
      { action: 'reply', fields: { body: 'Support answer' }, invalidFields: [{ body: '' }, { body: 'x'.repeat(8001) }],
        result: { status: 'waiting_user', lock_version: '2', messages: ['Player question', 'Support answer'] } },
      { action: 'ticket_status', fields: { status: 'resolved' }, invalidFields: [{ status: '' }, { status: 'not-a-status' }],
        result: { status: 'resolved', lock_version: '3', messages: ['Player question', 'Support answer'] } },
    ];
    for (const scenario of cases) {
      await test.step(scenario.action, async () => {
        expect((await operator.goto(staffPath)).status()).toBe(200);
        const { url, fields } = await prepare(operator, `main form:has(input[value="${scenario.action}"])`, scenario.fields);
        const baseline = await snapshot();
        await rejected({ page: operator, url, fields, snapshot, baseline });
        await wrongActor({ page: signedOut, lang, url, fields, status: 303, snapshot, baseline });
        await foreignPost(otherPlayer, lang, url, fields, 403, snapshot, baseline);
        await foreignPost(deniedStaff, lang, url, fields, 403, snapshot, baseline);
        for (const patch of [...scenario.invalidFields, { lock_version: 'invalid' }, { action: 'invalid' }]) {
          expect((await post(operator, url, { ...fields, ...patch })).status()).toBe(422);
          expect(await snapshot()).toEqual(baseline);
        }
        const success = await post(operator, url, fields);
        expect(success.status()).toBe(303);
        expect(success.headers().location).toBe(staffPath);
        expect(await snapshot()).toEqual([scenario.result]);
        expect((await post(operator, url, fields)).status()).toBe(409);
        expect(await snapshot()).toEqual([scenario.result]);
      });
    }
    await page.goto(playerPath);
    await expect(page.locator('main')).toContainText('Support answer');
    await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Rozwiązane' : 'Resolved');
  } finally {
    await Promise.all([support.close(), editor.close(), foreign.close(), anonymous.close()]);
  }
});
