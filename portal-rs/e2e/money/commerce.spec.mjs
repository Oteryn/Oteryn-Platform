import { expect, test } from '../fixtures.mjs';
import { csrfRejected, logout, pool } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';
import {
  balance, burst, creditEvidence, expectStatuses, formAt, loginSeeded, parallel,
  paymentEvent, postForm, seedAccount, topUp, webhook,
} from './fixtures.mjs';

test('parallel coin purchase replays share one pending intent and one provider credit', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = seedAccount(lang);
  await loginSeeded(page, lang, account);
  const url = `/${lang}/shop/coins`;
  const form = await formAt(page, url);
  await csrfRejected(page, url, form);
  const replies = await burst(() => postForm(page, url, form));
  expectStatuses(replies, { 303: 10, 429: parallel - 10 });
  const intents = await query(
    'SELECT id, provider_reference, state FROM payment_intents WHERE account_id=$1', [account.id],
  );
  expect(intents).toHaveLength(1);
  const [{ id, provider_reference, state }] = intents;
  expect(state).toBe('pending');
  expect(replies.filter(reply => reply.status === 303).map(reply => reply.location))
    .toEqual(Array(10).fill(`/${lang}/shop/test-checkout/${id}`));
  expect(await query(
    'SELECT operation_id, payment_intent_id FROM store_web_topups WHERE account_id=$1', [account.id],
  )).toEqual([{ operation_id: form.operation, payment_intent_id: id }]);
  await balance(page, lang, account.id, 0);

  const event = paymentEvent({ id, provider_reference });
  expectStatuses(await burst(() => webhook(page, event)), { 204: parallel });
  await creditEvidence(id, account.id);
  expect(await query('SELECT provider_event_id, state FROM payment_events WHERE intent_id=$1', [id]))
    .toEqual([{ provider_event_id: event.event_id, state: 'succeeded' }]);
  const [{ attempts }] = await query(
    "SELECT attempts FROM outbox WHERE kind='wallet.top_up' AND idempotency_key=$1", [id],
  );
  await query(
    `UPDATE outbox SET state='pending', run_after=now() WHERE kind='wallet.top_up' AND idempotency_key=$1`, [id],
  );
  await creditEvidence(id, account.id);
  expect(await query(
    "SELECT attempts FROM outbox WHERE kind='wallet.top_up' AND idempotency_key=$1", [id],
  )).toEqual([{ attempts: attempts + 1 }]);
  await balance(page, lang, account.id, 1234);
});

test('parallel coin purchases cannot bind one operation key to two buyers', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const first = seedAccount(lang);
  const second = seedAccount(lang);
  const context = await browser.newContext({
    baseURL: info.project.use.baseURL,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const otherPage = await context.newPage();
    await loginSeeded(page, lang, first);
    await loginSeeded(otherPage, lang, second);
    const url = `/${lang}/shop/coins`;
    const firstForm = await formAt(page, url);
    const secondForm = await formAt(otherPage, url);
    const replies = await burst(async index => {
      const primary = index % 2 === 0;
      return {
        account: primary ? first.id : second.id,
        ...await postForm(primary ? page : otherPage, url, {
          ...(primary ? firstForm : secondForm), operation: firstForm.operation,
        }),
      };
    });
    const intents = await query(
      'SELECT id, account_id, state FROM payment_intents WHERE account_id=ANY($1::uuid[])', [[first.id, second.id]],
    );
    expect(intents).toHaveLength(1);
    const [intent] = intents;
    expect([first.id, second.id]).toContain(intent.account_id);
    expect(intent.state).toBe('pending');
    expect(await query(
      'SELECT account_id, payment_intent_id FROM store_web_topups WHERE operation_id=$1', [firstForm.operation],
    )).toEqual([{ account_id: intent.account_id, payment_intent_id: intent.id }]);
    expectStatuses(replies.filter(reply => reply.account === intent.account_id), { 303: 10, 429: 2 });
    expectStatuses(replies.filter(reply => reply.account !== intent.account_id), { 409: 10, 429: 2 });
    await balance(page, lang, first.id, 0);
    await balance(otherPage, lang, second.id, 0);
  } finally {
    await context.close();
  }
});

test('parallel test checkout pay creates one decision and honours coin purchase conflicts', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = seedAccount(lang);
  await loginSeeded(page, lang, account);
  const intent = await topUp(page, lang);
  const url = `/${lang}/shop/coins`;
  const [{ operation_id }] = await query('SELECT operation_id FROM store_web_topups WHERE payment_intent_id=$1', [intent.id]);
  const form = await formAt(page, url);
  expect((await postForm(page, url, { ...form, operation: operation_id, package: 'changed-package' })).status).toBe(409);

  const checkoutURL = `/${lang}/shop/test-checkout/${intent.id}`;
  const checkout = await formAt(page, checkoutURL);
  const replies = await burst(() => postForm(page, checkoutURL, { ...checkout, action: 'pay' }));
  expectStatuses(replies, { 303: 8, 429: parallel - 8 });
  expect(replies.filter(reply => reply.status === 303).map(reply => reply.location))
    .toEqual(Array(8).fill(`/${lang}/shop/coins?checkout=${intent.id}`));
  await creditEvidence(intent.id, account.id);
  expect(await query('SELECT provider_event_id, state FROM payment_events WHERE intent_id=$1', [intent.id]))
    .toEqual([{ provider_event_id: `checkout:${intent.id}:paid`, state: 'succeeded' }]);
  await balance(page, lang, account.id, 1234);
});

test('parallel transfers cannot overdraw and changed replay conflicts', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const sender = seedAccount(lang);
  const recipient = seedAccount(lang);
  await loginSeeded(page, lang, sender);
  const intent = await topUp(page, lang);
  expect((await webhook(page, paymentEvent(intent))).status).toBe(204);
  await creditEvidence(intent.id, sender.id);

  const url = `/${lang}/shop/transfer`;
  const form = await formAt(page, url);
  const transfer = { ...form, recipient: recipient.id, amount: '34' };
  expect((await postForm(page, url, transfer)).status).toBe(303);
  expect((await postForm(page, url, { ...transfer, amount: '35' })).status).toBe(409);
  const replies = await burst(() => postForm(page, url,
    { ...transfer, operation: uuid7(), amount: '1200' }));
  expectStatuses(replies, { 303: 1, 403: 6, 429: parallel - 7 });
  const rows = await query('SELECT id, amount FROM wallet_transfers WHERE sender_id=$1', [sender.id]);
  expect(rows).toContainEqual({ id: form.operation, amount: '34' });
  const competing = rows.filter(row => row.id !== form.operation);
  expect(competing).toHaveLength(1);
  expect(competing[0].amount).toBe('1200');
  await balance(page, lang, sender.id, 0);
  await logout(page, lang);
  await loginSeeded(page, lang, recipient);
  await balance(page, lang, recipient.id, 1234);
});

test('identical parallel transfers produce one debit and one credit', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const sender = seedAccount(lang);
  const recipient = seedAccount(lang);
  await loginSeeded(page, lang, sender);
  const intent = await topUp(page, lang);
  expect((await webhook(page, paymentEvent(intent))).status).toBe(204);
  await creditEvidence(intent.id, sender.id);

  const url = `/${lang}/shop/transfer`;
  const form = await formAt(page, url);
  const replies = await burst(() => postForm(page, url, { ...form, recipient: recipient.id, amount: '234' }));
  expectStatuses(replies, { 303: 9, 429: parallel - 9 });
  expect(await query(
    'SELECT id, sender_id, recipient_id, amount FROM wallet_transfers WHERE sender_id=$1', [sender.id],
  )).toEqual([{ id: form.operation, sender_id: sender.id, recipient_id: recipient.id, amount: '234' }]);
  expect(await query(
    'SELECT account_id, available_delta, reserved_delta FROM wallet_entries WHERE operation_id=$1 ORDER BY available_delta',
    [form.operation],
  )).toEqual([
    { account_id: sender.id, available_delta: '-234', reserved_delta: '0' },
    { account_id: recipient.id, available_delta: '234', reserved_delta: '0' },
  ]);
  await balance(page, lang, sender.id, 1000);
  await logout(page, lang);
  await loginSeeded(page, lang, recipient);
  await balance(page, lang, recipient.id, 234);
});

test('competing checkout decisions honour the persisted winner', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = seedAccount(lang);
  await loginSeeded(page, lang, account);
  const intent = await topUp(page, lang);
  const url = `/${lang}/shop/test-checkout/${intent.id}`;
  const form = await formAt(page, url);
  // Creating the intent spends one of the account's ten mutation requests.
  const actions = ['pay', 'reject', 'pay', 'reject', 'pay', 'reject', 'pay', 'reject', 'pay'];
  const replies = await Promise.all(actions.map(action => postForm(page, url, { ...form, action })));
  const [{ state }] = await query('SELECT state FROM payment_intents WHERE id=$1', [intent.id]);
  expect(['succeeded', 'failed']).toContain(state);
  const winningAction = state === 'succeeded' ? 'pay' : 'reject';
  expectStatuses(replies, state === 'succeeded' ? { 303: 5, 409: 4 } : { 303: 4, 409: 5 });
  for (const [index, { status }] of replies.entries()) {
    expect(status).toBe(actions[index] === winningAction ? 303 : 409);
  }
  expect(await query('SELECT state FROM payment_events WHERE intent_id=$1', [intent.id])).toEqual([{ state }]);
  if (state === 'succeeded') {
    await creditEvidence(intent.id, account.id);
  } else {
    expect(await query('SELECT id FROM outbox WHERE idempotency_key=$1', [intent.id])).toEqual([]);
    expect(await query('SELECT id FROM invoices WHERE intent_id=$1', [intent.id])).toEqual([]);
  }
  await balance(page, lang, account.id, state === 'succeeded' ? 1234 : 0);
});

test('timed-out settlement stays pending until the signed event is redelivered', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = seedAccount(lang);
  await loginSeeded(page, lang, account);
  const intent = await topUp(page, lang);
  const event = paymentEvent(intent);
  const db = pool();
  const lock = await db.connect();
  try {
    await lock.query('BEGIN');
    await lock.query('SELECT id FROM payment_intents WHERE id=$1 FOR UPDATE', [intent.id]);
    expect((await webhook(page, event)).status).toBe(503);
    expect(await query('SELECT state FROM payment_intents WHERE id=$1', [intent.id]))
      .toEqual([{ state: 'pending' }]);
    expect(await query('SELECT id FROM payment_events WHERE intent_id=$1', [intent.id])).toEqual([]);
    expect(await query('SELECT id FROM outbox WHERE idempotency_key=$1', [intent.id])).toEqual([]);
    await formAt(page, `/${lang}/shop/test-checkout/${intent.id}`);
    await expect(page.locator('main [role=status]')).toHaveText(
      lang === 'pl' ? 'Wybierz wynik płatności testowej.' : 'Choose a test payment result.',
    );
    await balance(page, lang, account.id, 0);
  } finally {
    await lock.query('ROLLBACK');
    lock.release();
    await db.end();
  }
  expectStatuses(await burst(() => webhook(page, event)), { 204: parallel });
  await creditEvidence(intent.id, account.id);
  await balance(page, lang, account.id, 1234);
});

test('parallel refund and chargeback notices cannot reverse a delivered browser top-up', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = seedAccount(lang);
  await loginSeeded(page, lang, account);
  const intent = await topUp(page, lang);
  const paid = paymentEvent(intent);
  expect((await webhook(page, paid)).status).toBe(204);
  await creditEvidence(intent.id, account.id);

  const refunded = paymentEvent(intent, 'refunded');
  const chargeback = paymentEvent(intent, 'chargeback');
  expectStatuses(await burst(index => webhook(page, index % 2 === 0 ? refunded : chargeback)), { 400: parallel });
  expect(await query('SELECT state FROM payment_intents WHERE id=$1', [intent.id]))
    .toEqual([{ state: 'succeeded' }]);
  expect(await query('SELECT provider_event_id FROM payment_events WHERE intent_id=$1', [intent.id]))
    .toEqual([{ provider_event_id: paid.event_id }]);
  expect(await query('SELECT kind, state FROM outbox WHERE idempotency_key=$1', [intent.id]))
    .toEqual([{ kind: 'wallet.top_up', state: 'done' }]);
  await balance(page, lang, account.id, 1234);
});


for (const terminal of ['refunded', 'chargeback']) {
  test(`parallel ${terminal} notices defeat a queued credit and apply once`, async ({ page }, info) => {
    const lang = info.project.metadata.locale;
    const account = seedAccount(lang);
    await loginSeeded(page, lang, account);
    const intent = await topUp(page, lang);
    const paid = paymentEvent(intent);
    const reversal = paymentEvent(intent, terminal);

    // Defer only this fixture's new outbox row so the real webhook can record
    // provider reversal before the real worker is eligible to claim the credit.
    expect(intent.id).toMatch(/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/);
    const schedule = `money_credit_${intent.id.replaceAll('-', '')}`;
    try {
      await query(`
        CREATE FUNCTION ${schedule}() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
          NEW.run_after := 'infinity'::timestamptz;
          RETURN NEW;
        END;
        $$;
        CREATE TRIGGER ${schedule} BEFORE INSERT ON outbox FOR EACH ROW
        WHEN (NEW.kind='wallet.top_up' AND NEW.idempotency_key='${intent.id}')
        EXECUTE FUNCTION ${schedule}();
      `);
      expect((await webhook(page, paid)).status).toBe(204);
      expect(await query(
        "SELECT state, attempts FROM outbox WHERE kind='wallet.top_up' AND idempotency_key=$1", [intent.id],
      )).toEqual([{ state: 'pending', attempts: 0 }]);
      expectStatuses(await burst(() => webhook(page, reversal)), { 204: parallel });
    } finally {
      await query(`DROP TRIGGER IF EXISTS ${schedule} ON outbox; DROP FUNCTION IF EXISTS ${schedule}();`);
      await query(
        "UPDATE outbox SET run_after=now() WHERE kind='wallet.top_up' AND idempotency_key=$1", [intent.id],
      );
    }
    expect(await query('SELECT state FROM payment_intents WHERE id=$1', [intent.id])).toEqual([{ state: terminal }]);
    expect(await query(
      'SELECT provider_event_id, state FROM payment_events WHERE intent_id=$1 ORDER BY created_at', [intent.id],
    )).toEqual([
      { provider_event_id: paid.event_id, state: 'succeeded' },
      { provider_event_id: reversal.event_id, state: terminal },
    ]);
    await expect.poll(() => query(
      "SELECT last_error FROM outbox WHERE kind='wallet.top_up' AND idempotency_key=$1 AND attempts>=1 AND last_error IS NOT NULL",
      [intent.id],
    ), { timeout: 30_000 }).toEqual([{ last_error: 'top-up checkout/intent/job mismatch' }]);
    await balance(page, lang, account.id, 0);
  });
}
