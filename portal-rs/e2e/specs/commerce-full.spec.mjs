import { escapeRegex } from '../regex.mjs';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, pool, submit } from '../helpers.mjs';
import { paidTopup, uuid7 } from '../staff-commerce-helpers.mjs';
import { postAs, pressCheckout, readForm, seedPlayer, signInPlayer, withUnpublishedDownloads } from '../commerce-helpers.mjs';

const text = {
  en: {
    shop: 'Shop', coins: 'Top up coins', history: 'Balance and history', transfer: 'Transfer coins',
    skip: 'Skip to content', available: 'Available coins', reserved: 'Reserved coins',
    buy: 'Buy', package: 'Package', count: '1234 coins', price: 'PLN 2.50', payment: 'Amount', coinLabel: 'Coins',
    bank: 'Test bank', pay: 'Pay (test)', reject: 'Reject payment', cancel: 'Cancel',
    banner: 'TEST MODE – no real money is charged', pending: 'Choose a test payment result.',
    failed: 'Payment rejected or cancelled. No coins were credited.',
    credited: 'Payment accepted; coins have been credited.',
    unavailable: 'This operation is unavailable. No success has been confirmed.',
    invalid: 'Check the submitted values.',
    denied: 'This operation is not permitted or the balance is insufficient.',
    conflict: 'The quote changed or the operation key was already used. Reload before trying again.',
    ownId: 'Your transfer ID', recipient: 'Recipient account ID', amount: 'Amount of coins',
    submit: 'Confirm', empty: 'No entries.', date: 'Date (UTC)',
    delta: 'Available / reserved change', operation: 'Operation reference',
    transferHint: 'Ask the recipient to give you their transfer ID.',
    topupHint: 'You will be redirected to payment. Resubmitting this form will not result in a duplicate payment.',
    confirm: 'Check the recipient, quantity and price before confirming. Retrying this form uses the same operation key.',
  },
  pl: {
    shop: 'Sklep', coins: 'Doładuj coiny', history: 'Saldo i historia', transfer: 'Przekaż coiny',
    skip: 'Przejdź do treści', available: 'Dostępne coiny', reserved: 'Zarezerwowane coiny',
    buy: 'Kup', package: 'Pakiet', count: '1234 coiny', price: '2,50 zł', payment: 'Kwota', coinLabel: 'Coiny',
    bank: 'Bank testowy', pay: 'Zapłać (test)', reject: 'Odrzuć płatność', cancel: 'Anuluj',
    banner: 'TRYB TESTOWY – żadne prawdziwe pieniądze nie są pobierane',
    pending: 'Wybierz wynik płatności testowej.',
    failed: 'Płatność odrzucona lub anulowana. Nie przyznano coinów.',
    credited: 'Płatność przyjęta, coiny zostały zaksięgowane.',
    unavailable: 'Operacja jest niedostępna. Powodzenie nie zostało potwierdzone.',
    invalid: 'Sprawdź przesłane wartości.',
    denied: 'Operacja jest niedozwolona lub saldo jest niewystarczające.',
    conflict: 'Oferta się zmieniła lub klucz operacji został już użyty. Odśwież stronę przed kolejną próbą.',
    ownId: 'Twój identyfikator do przelewów', recipient: 'Identyfikator konta odbiorcy',
    amount: 'Liczba coinów', submit: 'Potwierdź', empty: 'Brak wpisów.', date: 'Data (UTC)',
    delta: 'Zmiana dostępnych / zarezerwowanych', operation: 'Identyfikator operacji',
    transferHint: 'Poproś odbiorcę o jego identyfikator do przelewów.',
    topupHint: 'Zostaniesz przekierowany do płatności. Ponowne wysłanie tego formularza nie spowoduje podwójnej płatności.',
    confirm: 'Sprawdź odbiorcę, liczbę i cenę przed potwierdzeniem. Ponowienie tego formularza używa tego samego klucza operacji.',
  },
};

const coinsForm = 'main form[action$="/shop/coins"]';
const transferForm = 'main form[action$="/shop/transfer"]';

async function refusal(response, status, message) {
  expect(response.status()).toBe(status);
  expect((await response.text()).includes(message), `localized response contains ${message}`).toBe(true);
}

test('shop links, unavailable catalogue, and empty or malformed wallet history', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const l = text[lang];
  await signInPlayer(page, lang);
  await page.goto(`/${lang}/shop`);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(l.shop);
  await expect(page.getByRole('status')).toHaveText(l.unavailable);
  await expect(page.getByText(l.empty, { exact: true })).toBeVisible();
  for (const selector of ['a.skip[href="#main"]', 'header.op-card a[href="#main"]']) {
    const skip = page.locator(selector);
    await expect(skip).toHaveText(l.skip);
    await skip.focus();
    await page.keyboard.press('Enter');
    await expect(page.locator('main')).toBeFocused();
  }
  const nav = page.getByRole('navigation', { name: l.shop });
  for (const [name, suffix] of [[l.coins, '/coins'], [l.transfer, '/transfer'], [l.history, '/history'], [l.shop, '']]) {
    await nav.getByRole('link', { name, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/${escapeRegex(lang)}/shop${escapeRegex(suffix)}$`));
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(name);
    await expect(page.getByText(l.available, { exact: true })).toBeVisible();
    await expect(page.getByText(l.reserved, { exact: true })).toBeVisible();
  }
  await nav.getByRole('link', { name: l.history, exact: true }).click();
  await expect(page.locator('main dl dd')).toHaveText(['0', '0']);
  await expect(page.getByText(l.empty, { exact: true })).toBeVisible();
  await nav.getByRole('link', { name: l.coins, exact: true }).click();
  const topup = await readForm(page.locator(coinsForm));
  const shop = new URL(`/${lang}/shop`, page.url()).href;
  await csrfRejected(page, shop);
  await refusal(await postAs(page, shop, { _csrf: topup.fields._csrf }), 503, l.unavailable);
  for (const cursor of ['invalid', '550e8400-e29b-41d4-a716-446655440000']) {
    const malformed = await page.goto(`/${lang}/shop/history?before=${cursor}`);
    expect(malformed.status()).toBe(400);
    await expect(page.getByRole('status')).toHaveText(l.invalid);
  }
});

for (const action of ['pay', 'reject', 'cancel']) {
  test(`test-bank ${action} settles only once and refuses invalid or foreign checkout`, async ({ page, browser }, info) => {
    test.setTimeout(180_000);
    const lang = info.project.metadata.locale;
    const l = text[lang];
    await signInPlayer(page, lang);
    const stranger = await browser.newContext({ ...info.project.use });
    const db = pool();
    try {
      await page.goto(`/${lang}/shop/coins`);
      const form = page.locator(coinsForm);
      await expect.poll(async () => (await form.getByRole('heading').innerText())
        .replace(/[\u2068\u2069]/g, '')).toBe(l.count);
      await expect(form.getByRole('button')).toHaveText(l.buy);
      await expect(form.getByText(l.price, { exact: true })).toBeVisible();
      await expect(form.getByText(l.topupHint)).toBeVisible();
      const topup = await readForm(form);
      await refusal(await postAs(page, topup.url, { ...topup.fields, operation: 'invalid' }), 422, l.invalid);
      await submit(page, coinsForm);
      await expect(page.getByRole('heading', { level: 1 })).toHaveText(l.bank);
      await expect(page.getByText(l.banner)).toBeVisible();
      await expect(page.getByRole('status')).toHaveText(l.pending);
      await expect(page.getByText(l.package, { exact: true })).toBeVisible();
      await expect(page.getByText(l.coinLabel, { exact: true })).toBeVisible();
      await expect(page.getByText(l.payment, { exact: true })).toBeVisible();
      await expect(page.locator('main').getByText(l.price, { exact: true })).toBeVisible();

      const checkout = await readForm(page.locator('main form'));
      const intent = new URL(checkout.url).pathname.split('/').at(-1);
      const topupReplay = await postAs(page, topup.url, topup.fields);
      expect(topupReplay.status()).toBe(303);
      expect(topupReplay.headers().location).toBe(new URL(checkout.url).pathname);
      const other = await stranger.newPage();
      const guest = await other.request.get(checkout.url, { maxRedirects: 0 });
      expect(guest.status()).toBe(303);
      expect(guest.headers().location).toBe(`/${lang}/login`);
      await signInPlayer(other, lang);
      expect((await other.goto(checkout.url)).status()).toBe(404);
      await other.goto(topup.url);
      const otherForm = await readForm(other.locator(coinsForm));
      expect((await postAs(other, checkout.url, { _csrf: otherForm.fields._csrf, action })).status()).toBe(404);

      await refusal(await postAs(page, checkout.url, { ...checkout.fields, action: 'invalid' }), 422, l.invalid);
      await pressCheckout(page, action, l[action]);
      const resultURL = `/${lang}/shop/coins?checkout=${intent}`;
      await expect(page).toHaveURL(new RegExp(`/shop/coins\\?checkout=${escapeRegex(intent)}$`));
      const replay = await postAs(page, checkout.url, { ...checkout.fields, action });
      expect(replay.status()).toBe(303);
      expect(replay.headers().location).toBe(resultURL);
      await refusal(await postAs(page, checkout.url, {
        ...checkout.fields, action: action === 'pay' ? 'reject' : 'pay',
      }), 409, l.conflict);

      const credits = async () => (await db.query(
        'SELECT available_delta FROM wallet_entries WHERE operation_id=$1', [intent],
      )).rows;
      if (action === 'pay') {
        await expect.poll(credits, { timeout: 30_000 }).toEqual([{ available_delta: '1234' }]);
      } else {
        await expect(page.getByRole('status')).toHaveText(l.failed);
        expect(await credits()).toEqual([]);
      }
      await page.goto(checkout.url);
      await expect(page.getByRole('status')).toHaveText(action === 'pay' ? l.credited : l.failed);
      await expect(page.locator('main form')).toHaveCount(0);
      await page.locator('main').getByRole('link', { name: l.coins }).click();
      await expect(page.getByRole('heading', { level: 1 })).toHaveText(l.coins);
    } finally {
      await stranger.close();
      await db.end();
    }
  });
}

test('transfer validates inputs, refuses insufficient funds and restricts replay to the same amount', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const l = text[lang];
  const sender = await signInPlayer(page, lang);
  const recipient = await seedPlayer(lang);
  const db = pool();
  try {
    await page.goto(`/${lang}/shop/transfer`);
    const form = page.locator(transferForm);
    await expect(page.getByLabel(l.ownId)).toHaveValue(sender.id);
    await expect(page.getByText(l.transferHint)).toBeVisible();
    await expect(form.getByLabel(l.recipient)).toBeVisible();
    await expect(form.getByLabel(l.amount)).toBeVisible();
    await expect(form.getByText(l.confirm)).toBeVisible();
    await expect(form.getByRole('button')).toHaveText(l.submit);
    const { url, fields } = await readForm(form);
    const values = { ...fields, recipient: recipient.id, amount: '5' };
    await csrfRejected(page, url, values);
    await refusal(await postAs(page, url, { ...values, amount: '0' }), 422, l.invalid);
    await refusal(await postAs(page, url, { ...values, recipient: sender.id }), 422, l.invalid);
    await refusal(await postAs(page, url, values), 403, l.denied);

    await paidTopup(page, lang, db);
    await page.goto(url);
    const funded = await readForm(form);
    const completed = { ...funded.fields, recipient: recipient.id, amount: '5' };
    await submit(page, transferForm, { recipient: recipient.id, amount: '5' });
    await expect(page).toHaveURL(new RegExp(`/${escapeRegex(lang)}/shop/history$`));
    await expect(page.locator('main dl dd')).toHaveText(['1229', '0']);
    const row = page.locator('main article').filter({ hasText: completed.operation });
    await expect(row.getByRole('heading')).toHaveText(l.operation);
    await expect(row.getByText(`${l.delta}: -5 / 0`)).toBeVisible();
    await expect(row.getByText(`${l.date}:`, { exact: false })).toBeVisible();
    expect((await postAs(page, url, completed)).status()).toBe(303);
    await refusal(await postAs(page, url, { ...completed, amount: '6' }), 409, l.conflict);
    expect((await db.query(
      'SELECT available_delta FROM wallet_entries WHERE operation_id=$1 ORDER BY available_delta',
      [completed.operation],
    )).rows).toEqual([{ available_delta: '-5' }, { available_delta: '5' }]);

    await db.query("UPDATE accounts SET status='suspended' WHERE id=$1", [recipient.id]);
    const denied = await postAs(page, url, { ...completed, operation: uuid7() });
    await refusal(denied, 403, l.denied);
  } finally {
    await db.end();
  }
});

test('payment history links to an account-owned printable receipt and the premium summary', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const l = text[lang];
  const labels = lang === 'pl' ? {
    payments: 'Płatności', empty: 'Brak płatności.', paid: 'Opłacona', receipt: 'Rachunek',
    end: 'Koniec premium (UTC)', noEnd: 'Brak daty końcowej', noPremium: 'Brak premium',
    print: 'Użyj polecenia Drukuj lub Zapisz jako HTML w przeglądarce, aby zachować rachunek.',
    amount: '250 PLN (najmniejsze jednostki)', paging: 'Strony historii płatności', previous: 'Poprzednia strona', next: 'Następna strona',
    states: ['Utworzona', 'Oczekująca', 'Nieudana', 'Zwrócona', 'Obciążenie zwrotne'],
  } : {
    payments: 'Payments', empty: 'No payments yet.', paid: 'Paid', receipt: 'Receipt',
    end: 'Premium ends (UTC)', noEnd: 'No end date', noPremium: 'No premium',
    print: 'Use your browser’s Print or Save as HTML command to keep this receipt.',
    amount: '250 PLN (minor units)', paging: 'Payment history pages', previous: 'Previous page', next: 'Next page',
    states: ['Created', 'Pending', 'Failed', 'Refunded', 'Chargeback'],
  };
  for (const section of ['payments', 'premium', 'payments/receipts/malformed']) {
    const guest = await page.request.get(`/${lang}/panel/${section}`, { maxRedirects: 0 });
    expect(guest.status()).toBe(303);
    expect(guest.headers().location).toBe(`/${lang}/login`);
  }
  const player = await signInPlayer(page, lang);
  const db = pool();
  const otherContext = await browser.newContext({ ...info.project.use });
  try {
    const history = `/${lang}/panel/payments`;
    await page.goto(history);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(labels.payments);
    await expect(page.getByText(labels.empty)).toBeVisible();
    const payment = await paidTopup(page, lang, db);
    await page.goto(history);
    const item = page.locator('main article').filter({ hasText: payment.payment_intent_id });
    await expect(item.getByText(labels.paid, { exact: true })).toBeVisible();
    await expect(item.getByText('Status', { exact: true })).toBeVisible();
    await expect(item.getByText(l.date, { exact: true })).toBeVisible();
    await expect(item.getByText(l.payment, { exact: true })).toBeVisible();
    await expect(item.getByText(labels.amount, { exact: true })).toBeVisible();
    await item.getByRole('link', { name: labels.receipt, exact: true }).click();
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(labels.receipt);
    await expect(page.locator('main article').getByRole('heading', { level: 2 })).toHaveText(`${labels.receipt} 1`);
    await expect(page.getByText(labels.print)).toBeVisible();
    await expect(page.getByText(labels.amount)).toBeVisible();

    const receipt = page.url();
    const other = await otherContext.newPage();
    await signInPlayer(other, lang);
    expect((await other.goto(receipt)).status()).toBe(404);
    expect((await other.goto(`/${lang}/panel/payments/receipts/malformed`)).status()).toBe(404);

    await page.locator('main').getByRole('link', { name: 'Premium', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1 })).toHaveText('Premium');
    for (const value of ['Status', labels.end, labels.noEnd, labels.noPremium]) {
      await expect(page.getByText(value, { exact: true })).toBeVisible();
    }
    await page.locator('main').getByRole('link', { name: labels.payments, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`${escapeRegex(history)}$`));
    expect((await page.goto(`${history}?page=0`)).status()).toBe(400);
    // Seed historical statuses below the real checkout to reach the 20-row page boundary.
    await db.query(
      `WITH history AS (
         SELECT uuidv7() AS id, states[1 + (i % cardinality(states))] AS state
         FROM generate_series(1,20) AS i
         CROSS JOIN (SELECT ARRAY['created','pending','failed','refunded','chargeback'] AS states) AS status
       )
       INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state,created_at)
       SELECT id,$1,'test','history:' || id,250,'PLN',state,now()-interval '1 day' FROM history`,
      [player.id],
    );
    await page.goto(history);
    await expect(page.locator('main article')).toHaveCount(20);
    for (const state of labels.states) {
      await expect(page.getByText(state, { exact: true }).first()).toBeVisible();
    }
    const paging = page.getByRole('navigation', { name: labels.paging });
    await paging.getByRole('link', { name: labels.next }).click();
    await expect(page).toHaveURL(new RegExp(`${escapeRegex(history)}\\?page=2$`));
    await expect(page.locator('main article')).toHaveCount(1);
    await paging.getByRole('link', { name: labels.previous }).click();
    await expect(page).toHaveURL(new RegExp(`${escapeRegex(history)}\\?page=1$`));
    await expect(item).toBeVisible();
  } finally {
    await otherContext.close();
    await db.end();
  }
});

test('wallet history older-entries link uses its last entry as a cursor', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const player = await signInPlayer(page, lang);
  const db = pool();
  try {
    // Seed prior ledger activity so the browser can exercise the 25-entry boundary.
    await db.query(
      "INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING", [player.id],
    );
    const { rows } = await db.query(
      `WITH operations AS (
         INSERT INTO wallet_operations(id,kind,account_id,amount,reason)
         SELECT uuidv7(), 'adjustment', $1, 1, 'Disposable wallet history pagination'
         FROM generate_series(1,26)
         RETURNING id
       )
       INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta)
       SELECT uuidv7(),id,$1,1,0 FROM operations
       RETURNING id,operation_id`, [player.id],
    );
    rows.sort((a, b) => a.id.localeCompare(b.id));
    const path = `/${lang}/shop/history`;
    await page.goto(path);
    await expect(page.locator('main article')).toHaveCount(25);
    await expect(page.locator('main dl dd')).toHaveText(['26', '0']);
    const older = page.getByRole('link', { name: lang === 'pl' ? 'Starsze wpisy' : 'Older entries' });
    await expect(older).toHaveAttribute('href', `${path}?before=${rows[1].id}`);
    await older.click();
    await expect(page.locator('main article')).toHaveCount(1);
    await expect(page.locator('main article')).toContainText(rows[0].operation_id);
    await expect(older).toHaveCount(0);
  } finally {
    await db.end();
  }
});

test('unpublished download channel refuses metadata, policy, and target requests', async ({ browser }, info) => {
  const lang = info.project.metadata.locale;
  test.setTimeout(240_000);
  await withUnpublishedDownloads(browser, lang, async page => {
    const playerView = await page.goto(`/${lang}/download`);
    expect(playerView.status()).toBe(404);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(lang === 'pl' ? 'Nie znaleziono strony' : 'Page not found');
    for (const channel of ['stable', 'beta']) {
      for (const suffix of ['metadata/timestamp.json', 'policy-v1.json', 'targets/client.zip']) {
        const path = `/updates/${channel}/${suffix}`;
        const missing = await page.request.get(path);
        expect(missing.status(), path).toBe(404);
        expect(missing.headers()['cache-control']).toBe('no-store');
        expect((await page.request.post(path, { data: 'browser publication attempt' })).status()).toBe(405);
      }
    }
    expect((await page.request.get('/updates/invalid/policy-v1.json')).status()).toBe(400);
  });
});
