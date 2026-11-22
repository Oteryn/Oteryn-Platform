import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, pool, submit } from '../helpers.mjs';
import { loginWithRecoveryCode } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../staff-commerce-helpers.mjs';
import { postAs, readForm, seedMfa, seedPlayer, signInPlayer } from '../commerce-helpers.mjs';

const commerce = '/admin/commerce';
const formFor = action => `form[action="${commerce}/${action}"]`;

async function staffActor(context, lang, db, permissions) {
  const account = await seedPlayer(lang);
  // Use the bootstrap pattern in admin.spec.mjs with a role scoped to this test.
  const role = (await db.query('INSERT INTO roles(key) VALUES($1) RETURNING id', [
    `commerce_${randomUUID().replaceAll('-', '')}`,
  ])).rows[0].id;
  await db.query('INSERT INTO role_permissions(role_id,permission) SELECT $1,unnest($2::text[])', [
    role, ['admin.access', ...permissions],
  ]);
  await db.query('INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)', [account.id, role]);
  const staff = await context.newPage();
  await loginWithRecoveryCode(staff, lang, account, await seedMfa(account));
  return staff;
}

async function settledPayment(db, account) {
  const id = uuid7();
  await db.query(
    "INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($1,$2,'test',$3,250,'PLN','succeeded')",
    [id, account, `test:${id}`],
  );
  return id;
}

async function openCommerce(actor, lang, allowed) {
  expect((await actor.goto(commerce)).status()).toBe(200);
  await expect(actor.locator('html')).toHaveAttribute('lang', lang);
  await expect(actor.getByRole('heading', { level: 1 })).toHaveText(lang === 'pl' ? 'Handel' : 'Commerce');
  for (const action of ['grant', 'revoke', 'adjust', 'reconcile', 'refund']) {
    await expect(actor.locator(formFor(action))).toHaveCount(Number(allowed.includes(action)));
  }
}

test('commerce permissions refuse a player and staff with access only', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const db = pool();
  const context = await browser.newContext({ ...info.project.use });
  try {
    const guest = await page.request.get(commerce, { maxRedirects: 0 });
    expect(guest.status()).toBe(303);
    const player = await signInPlayer(page, lang);
    expect((await page.goto(commerce)).status()).toBe(403);
    const staff = await staffActor(context, lang, db, []);
    expect((await staff.goto(commerce)).status()).toBe(403);
    const payment = await settledPayment(db, player.id);
    const inputs = [
      ['grant', { days: '7' }],
      ['revoke', {}],
      ['adjust', { amount: '15' }],
      ['reconcile', { target: payment }],
      ['refund', { target: payment }],
    ];
    for (const actor of [page, staff]) {
      await actor.goto(`/${lang}/shop/coins`);
      const token = await actor.locator('main input[name="_csrf"]').inputValue();
      for (const [action, args] of inputs) {
        const fields = {
          _csrf: token, request_id: uuid7(), target: player.id,
          reason: 'Verify commerce permission isolation', ...args,
        };
        const route = new URL(`${commerce}/${action}`, actor.url()).href;
        expect((await postAs(actor, route, fields)).status()).toBe(403);
      }
    }
  } finally {
    await context.close();
    await db.end();
  }
});

test('wallet correction requires a second staff member and can be approved or cancelled', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const player = await signInPlayer(page, lang);
  const requesterContext = await browser.newContext({ ...info.project.use });
  const reviewerContext = await browser.newContext({ ...info.project.use });
  const db = pool();
  try {
    const requester = await staffActor(requesterContext, lang, db, ['marketplace.manage']);
    const reviewer = await staffActor(reviewerContext, lang, db, ['marketplace.manage']);
    await openCommerce(requester, lang, ['adjust']);
    const adjustment = await readForm(requester.locator(formFor('adjust')));
    expect((await postAs(requester, adjustment.url, {
      ...adjustment.fields, target: player.id, amount: '0', reason: 'Invalid zero amount correction',
    })).status()).toBe(422);
    for (const decision of ['cancel', 'approve']) {
      await requester.goto(commerce);
      const { fields } = await readForm(requester.locator(formFor('adjust')));
      const id = fields.request_id;
      await submit(requester, formFor('adjust'), {
        target: player.id, amount: '15', reason: 'Propose disposable wallet correction',
      });
      await expect(requester).toHaveURL(new RegExp(`${commerce}$`));
      const proposal = requester.locator(formFor(`approvals/${id}`));
      await expect(proposal.getByRole('button')).toHaveCount(0);
      const self = await readForm(proposal);
      expect((await postAs(requester, self.url, self.fields)).status()).toBe(403);
      expect((await db.query('SELECT available_delta FROM wallet_entries WHERE operation_id=$1', [id])).rows).toEqual([]);

      await reviewer.goto(commerce);
      const review = reviewer.locator(formFor(`approvals/${id}`));
      const approve = lang === 'pl' ? 'Zatwierdź jako druga osoba z zespołu' : 'Approve as a second staff member';
      const cancel = lang === 'pl' ? 'Anuluj wniosek' : 'Cancel request';
      await expect(review.getByRole('button', { name: approve })).toBeVisible();
      await expect(review.getByRole('button', { name: cancel })).toBeVisible();
      const button = review.getByRole('button', { name: decision === 'cancel' ? cancel : approve });
      const details = await readForm(review);
      const endpoint = new URL(await button.getAttribute('formaction') ?? details.url, reviewer.url()).href;
      await csrfRejected(reviewer, endpoint);
      await button.click();
      await expect(reviewer).toHaveURL(new RegExp(`${commerce}$`));
      await expect(review).toHaveCount(0);
      expect((await postAs(reviewer, endpoint, details.fields)).status()).toBe(303);
      expect((await db.query('SELECT available_delta FROM wallet_entries WHERE operation_id=$1', [id])).rows)
        .toEqual(decision === 'approve' ? [{ available_delta: '15' }] : []);
    }
    await page.goto(`/${lang}/shop/history`);
    await expect(page.locator('main dl dd')).toHaveText(['15', '0']);
  } finally {
    await requesterContext.close();
    await reviewerContext.close();
    await db.end();
  }
});

test('Bazaar staff saves fees and item restrictions and refuses invalid or unauthorized changes', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const path = `/admin/${lang}/bazaar`;
  const db = pool();
  const context = await browser.newContext({ ...info.project.use });
  try {
    const guest = await page.request.get(path, { maxRedirects: 0 });
    expect(guest.status()).toBe(303);
    await signInPlayer(page, lang);
    expect((await page.goto(path)).status()).toBe(403);
    const staff = await staffActor(context, lang, db, ['marketplace.manage']);
    expect((await staff.goto(path)).status()).toBe(200);
    const headings = lang === 'pl'
      ? ['Moderacja bazaru', 'Wstrzymaj ofertę', 'Wnioskuj o unieważnienie', 'Zatwierdź unieważnienie', 'Wersja opłat', 'Ogranicz przedmiot']
      : ['Bazaar moderation', 'Suspend listing', 'Request invalidation', 'Approve invalidation', 'Fee revision', 'Restrict item'];
    for (const [i, name] of headings.entries()) {
      const matches = staff.getByRole('heading', { name, exact: true });
      await expect(matches).toHaveCount(i < 4 ? 1 : 2);
      await expect(matches.first()).toBeVisible();
    }
    const selector = 'form:has(input[name="action"][value="fees"])';
    const form = staff.locator(selector);
    const { fields, url } = await readForm(form);
    await expect(form.getByRole('button')).toHaveText(lang === 'pl' ? 'Zapisz' : 'Save');
    const values = { promotion_price: '25', reason: 'Disposable staff Bazaar fee revision' };
    expect((await postAs(staff, url, { ...fields, ...values, promotion_price: 'invalid' })).status()).toBe(422);
    await submit(staff, selector, values);
    await expect(staff).toHaveURL(new RegExp(`${path}$`));
    await expect(staff.getByText(fields.id, { exact: false })).toBeVisible();
    expect((await db.query(
      'SELECT coin_bps,gold_bps,listing_fee,promotion_price FROM bazaar_fee_revisions WHERE id=$1', [fields.id],
    )).rows).toEqual([{ coin_bps: 1000, gold_bps: 250, listing_fee: '0', promotion_price: '25' }]);

    const restrict = 'form:has(input[name="action"][value="restrict"])';
    const request = await readForm(staff.locator(restrict));
    const restriction = {
      world: '0192a000-0000-7000-8000-000000000012',
      family: 'e2e', production_key: `test-${uuid7()}`, revision_ref: 'v1',
      minimum_value: '100', reason: 'Disposable policy blocks a synthetic item',
    };
    const badRestriction = await postAs(staff, request.url, {
      ...request.fields, ...restriction, minimum_value: '0',
    });
    expect(badRestriction.status()).toBe(422);
    await submit(staff, restrict, restriction);
    const catalogUnavailable = lang === 'pl'
      ? 'Zweryfikowany katalog gry jest niedostępny. Lista pokazuje wyłącznie ograniczenia portalu i nie nadaje zbywalności.'
      : 'The verified Game catalog is unavailable. This list shows Platform restrictions only; it grants no transferability.';
    await expect(staff.getByText(catalogUnavailable, { exact: true })).toBeVisible();
    await expect(staff.getByText(restriction.production_key, { exact: false })).toBeVisible();
    expect((await db.query(
      'SELECT world_id,minimum_value,denied FROM bazaar_item_restrictions WHERE id=$1', [request.fields.id],
    )).rows).toEqual([{ world_id: restriction.world, minimum_value: '100', denied: true }]);
  } finally {
    await context.close();
    await db.end();
  }
});

test('code issuer submits the translated form and player redemption is idempotent', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const issuerContext = await browser.newContext({ ...info.project.use });
  const db = pool();
  try {
    const issue = `/${lang}/admin/store/codes`;
    const redeem = `/${lang}/shop/codes/redeem`;
    const player = await signInPlayer(page, lang);
    expect((await page.goto(issue)).status()).toBe(403);
    const issuer = await staffActor(issuerContext, lang, db, ['store.codes.generate']);
    expect((await issuer.goto(issue)).status()).toBe(200);
    const heading = lang === 'pl' ? 'Generowanie kodów' : 'Issue gift codes';
    const button = lang === 'pl' ? 'Wygeneruj kody' : 'Issue codes';
    await expect(issuer).toHaveTitle(heading);
    await expect(issuer.getByRole('heading', { level: 1 })).toHaveText(heading);
    await expect(issuer.getByRole('button', { name: button })).toBeVisible();
    const labels = lang === 'pl'
      ? ['Rodzaj', 'Wartość kodu', 'Liczba kodów', 'Limit użyć kodu', 'Limit użyć na konto', 'Ważny od (czas Unix w sekundach)', 'Ważny do (czas Unix w sekundach)', 'Uzasadnienie']
      : ['Kind', 'Amount per code', 'Number of codes', 'Redemptions per code', 'Redemptions per account', 'Valid from (Unix seconds)', 'Valid until (Unix seconds)', 'Reason for issuance'];
    const [kindLabel, ...inputLabels] = labels;
    const kind = issuer.locator('select[name="kind"]');
    await expect(kind).toBeVisible();
    // The wrapped select contributes its option text to getByLabel().
    expect(await kind.evaluate(select => Array.from(select.labels[0].childNodes)
      .filter(node => node.nodeType === Node.TEXT_NODE)
      .map(node => node.textContent).join('').trim())).toBe(kindLabel);
    for (const label of inputLabels) {
      await expect(issuer.getByLabel(label, { exact: true })).toBeVisible();
    }
    await expect(issuer.locator('option[value="coins"]')).toHaveText(lang === 'pl' ? 'Monety' : 'Coins');
    await expect(issuer.locator('option[value="premium_days"]')).toHaveText(lang === 'pl' ? 'Dni premium' : 'Premium days');
    const clock = Math.floor(Date.now() / 1000);
    const values = {
      amount: '37', count: '1', valid_from: String(clock - 60), valid_until: String(clock + 3600),
      note: 'Disposable browser gift code issuance',
    };
    const { url, fields } = await readForm(issuer.locator('main form'));
    expect((await postAs(issuer, url, { ...fields, ...values, count: '0' })).status()).toBe(422);
    const issued = issuer.waitForNavigation();
    await submit(issuer, 'main form', values);
    const result = await issued;
    expect(result.status()).toBe(200);
    const { batch_id: batch, codes } = await result.json();
    expect(codes.length).toBe(1);

    await page.goto(`/${lang}/shop/coins`);
    const token = await page.locator('main input[name="_csrf"]').inputValue();
    const headers = { 'x-csrf-token': token, origin: new URL(page.url()).origin };
    const data = { code: codes[0], operation: uuid7() };
    await csrfRejected(page, redeem);
    const first = await page.request.post(redeem, { headers, data });
    expect(first.status()).toBe(200);
    const receipt = await first.json();
    expect(receipt).toMatchObject({ amount: 37 });
    const replay = await page.request.post(redeem, { headers, data });
    expect(replay.status()).toBe(200);
    expect(await replay.json()).toEqual(receipt);
    const spent = await page.request.post(redeem, { headers, data: { ...data, operation: uuid7() } });
    expect(spent.status()).toBe(403);
    await page.goto(`/${lang}/shop/history`);
    await expect(page.locator('main dl dd')).toHaveText(['37', '0']);
    expect((await db.query(
      'SELECT count(*)::int AS count FROM code_redemptions r JOIN gift_codes c ON c.id=r.code_id WHERE c.batch_id=$1 AND r.account_id=$2',
      [batch, player.id],
    )).rows[0].count).toBe(1);
  } finally {
    await issuerContext.close();
    await db.end();
  }
});

test('payment staff reconciles without changing state and approves one full refund dispatch', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const requesterContext = await browser.newContext({ ...info.project.use });
  const reviewerContext = await browser.newContext({ ...info.project.use });
  const db = pool();
  try {
    const player = await signInPlayer(page, lang);
    const requester = await staffActor(requesterContext, lang, db, ['payments.reconcile']);
    const reviewer = await staffActor(reviewerContext, lang, db, ['payments.reconcile']);
    await openCommerce(requester, lang, ['reconcile', 'refund']);
    // Fixture for a settled payment which has not delivered wallet value.
    const intent = await settledPayment(db, player.id);
    await requester.goto(commerce);
    const reconcile = await readForm(requester.locator(formFor('reconcile')));
    await submit(requester, formFor('reconcile'), { target: intent, reason: 'Review disposable test payment' });
    const resolution = (await db.query('SELECT result FROM commerce_operations WHERE id=$1', [reconcile.fields.request_id])).rows[0].result;
    expect(resolution).toMatchObject({ resolution_code: 'reviewed_no_payment_state_change', state: 'succeeded' });

    const refund = await readForm(requester.locator(formFor('refund')));
    const reason = 'Request full refund of disposable payment';
    expect((await postAs(requester, refund.url, { ...refund.fields, target: uuid7(), reason })).status()).toBe(422);
    await submit(requester, formFor('refund'), { target: intent, reason });
    const id = refund.fields.request_id;
    const approval = formFor(`approvals/${id}`);
    await expect(requester.locator(approval).getByRole('button')).toHaveCount(0);
    expect((await db.query('SELECT state FROM commerce_refunds WHERE operation_id=$1', [id])).rows).toEqual([]);
    await reviewer.goto(commerce);
    const review = await readForm(reviewer.locator(approval));
    await submit(reviewer, approval);
    await expect(reviewer.locator(approval)).toHaveCount(0);
    expect((await postAs(reviewer, review.url, review.fields)).status()).toBe(303);
    await expect.poll(async () => (await db.query(
      'SELECT state,provider_reference FROM commerce_refunds WHERE operation_id=$1', [id],
    )).rows, { timeout: 30_000 }).toEqual([{ state: 'done', provider_reference: `refund:${intent}` }]);
    expect((await db.query('SELECT state FROM payment_intents WHERE id=$1', [intent])).rows).toEqual([{ state: 'succeeded' }]);
    expect((await db.query("SELECT count(*)::int AS count FROM outbox WHERE kind='commerce.refund' AND idempotency_key=$1", [intent])).rows[0].count).toBe(1);
  } finally {
    await requesterContext.close();
    await reviewerContext.close();
    await db.end();
  }
});

test('premium staff grants after independent approval, then revokes the entitlement', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const requesterContext = await browser.newContext({ ...info.project.use });
  const reviewerContext = await browser.newContext({ ...info.project.use });
  const db = pool();
  try {
    const player = await signInPlayer(page, lang);
    const requester = await staffActor(requesterContext, lang, db, ['products.premium.manage']);
    const reviewer = await staffActor(reviewerContext, lang, db, ['products.premium.manage']);
    await openCommerce(requester, lang, ['grant', 'revoke']);
    const grant = await readForm(requester.locator(formFor('grant')));
    const mutation = { target: player.id, days: '7', reason: 'Disposable player premium grant' };
    const invalid = await postAs(requester, grant.url, { ...grant.fields, ...mutation, days: '367' });
    expect(invalid.status()).toBe(422);
    await submit(requester, formFor('grant'), mutation);
    const id = grant.fields.request_id;
    const approval = formFor(`approvals/${id}`);
    await expect(requester.locator(approval).getByRole('button')).toHaveCount(0);
    expect((await db.query('SELECT event_type FROM premium_ledger WHERE request_id=$1', [id])).rows).toEqual([]);
    await reviewer.goto(commerce);
    await submit(reviewer, approval);
    await expect(reviewer.locator(approval)).toHaveCount(0);
    expect((await db.query('SELECT event_type,duration_days FROM premium_ledger WHERE request_id=$1', [id])).rows)
      .toEqual([{ event_type: 'GRANT', duration_days: 7 }]);

    const panel = `/${lang}/panel/premium`;
    await page.goto(panel);
    await expect(page.getByText(lang === 'pl' ? 'Aktywne' : 'Active', { exact: true })).toBeVisible();
    await requester.goto(commerce);
    const revoke = await readForm(requester.locator(formFor('revoke')));
    await submit(requester, formFor('revoke'), { target: player.id, reason: 'Revoke disposable premium grant' });
    await requester.waitForURL(new RegExp(`${commerce}$`));
    expect((await db.query('SELECT event_type FROM premium_ledger WHERE request_id=$1', [revoke.fields.request_id])).rows)
      .toEqual([{ event_type: 'REVOKE' }]);
    await page.reload();
    await expect(page.getByText(lang === 'pl' ? 'Odebrane' : 'Revoked', { exact: true })).toBeVisible();
  } finally {
    await requesterContext.close();
    await reviewerContext.close();
    await db.end();
  }
});

test('Bazaar staff reviews reports, suspends a listing, and requires independent invalidation approval', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const path = `/admin/${lang}/bazaar`;
  const requesterContext = await browser.newContext({ ...info.project.use });
  const reviewerContext = await browser.newContext({ ...info.project.use });
  const db = pool();
  const form = action => `form:has(input[name="action"][value="${action}"])`;
  try {
    const seller = await signInPlayer(page, lang);
    const reporter = await seedPlayer(lang);
    const listing = uuid7();
    // Seed a draft for staff moderation. Activating an item listing requires
    // a verified Game binding, which this fixture does not supply.
    await db.query(
      `INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at)
       VALUES($1,$2,'item','auction','coins',100,false,'draft',now()+interval '2 days')`,
      [listing, seller.id],
    );
    const staff = await staffActor(requesterContext, lang, db, ['marketplace.manage']);
    const reviewer = await staffActor(reviewerContext, lang, db, ['marketplace.manage']);

    for (const [decision, state] of [['true', 'accepted'], ['false', 'dismissed']]) {
      const report = uuid7();
      await db.query(
        "INSERT INTO bazaar_reports(id,listing_id,reporter,category,detail) VALUES($1,$2,$3,'fraud','Disposable suspected fraud report')",
        [report, listing, reporter.id],
      );
      await staff.goto(path);
      const review = staff.locator(`${form('review')}:has(input[name="id"][value="${report}"])`);
      const request = await readForm(review);
      const accepted = lang === 'pl' ? 'Uznaj zgłoszenie' : 'Accept report';
      const dismissed = lang === 'pl' ? 'Odrzuć zgłoszenie' : 'Dismiss report';
      await expect(review.locator('option')).toHaveText([accepted, dismissed]);
      await csrfRejected(staff, request.url, { ...request.fields, accepted: decision });
      expect((await postAs(staff, request.url, { ...request.fields, accepted: 'invalid' })).status()).toBe(422);
      await review.locator('select[name="accepted"]').selectOption(decision);
      await review.getByRole('button').click();
      await expect(review).toHaveCount(0);
      expect((await db.query('SELECT state FROM bazaar_reports WHERE id=$1', [report])).rows).toEqual([{ state }]);
    }

    const reason = 'Suspend a disposable listing during review';
    const suspend = await readForm(staff.locator(form('suspend')));
    expect((await postAs(staff, suspend.url, { ...suspend.fields, listing: 'invalid', reason })).status()).toBe(422);
    await submit(staff, form('suspend'), { listing, reason });
    expect((await db.query('SELECT revision FROM bazaar_moderation_holds WHERE listing_id=$1', [listing])).rows)
      .toEqual([{ revision: '1' }]);
    const invalidation = await readForm(staff.locator(form('request')));
    const id = invalidation.fields.id;
    await submit(staff, form('request'), { listing, reason: 'Request independent invalidation review' });
    await expect(staff.getByText(id, { exact: true })).toBeVisible();
    const self = await readForm(staff.locator(form('approve')));
    expect((await postAs(staff, self.url, { ...self.fields, id })).status()).toBe(403);

    await reviewer.goto(path);
    await submit(reviewer, form('approve'), { id });
    await expect(reviewer.getByText(id, { exact: true })).toHaveCount(0);
    expect((await db.query(
      'SELECT approved_by IS NOT NULL AS approved FROM bazaar_invalidation_requests WHERE id=$1', [id],
    )).rows).toEqual([{ approved: true }]);
    expect((await db.query('SELECT kind FROM bazaar_moderation_outbox WHERE id=$1 AND listing_id=$2', [id, listing])).rows)
      .toEqual([{ kind: 'reconcile_invalidation' }]);
  } finally {
    await requesterContext.close();
    await reviewerContext.close();
    await db.end();
  }
});
