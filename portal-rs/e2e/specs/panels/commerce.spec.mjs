import {createHash, randomBytes} from 'node:crypto';
import {expect, test} from '../../fixtures.mjs';
import {csrfRejected, pool} from '../../helpers.mjs';
import {uuid7} from '../../staff-commerce-helpers.mjs';
import {query, unique} from '../../staff-rbac-helpers.mjs';

const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
const digest = value => createHash('sha256').update(value).digest();

async function actor(context, lang, staff = false) {
  const session = randomBytes(32).toString('hex');
  const csrf = randomBytes(32).toString('hex');
  const grant = randomBytes(32).toString('hex');
  const [{id, password_hash}] = await query(
    `INSERT INTO accounts(email,password_hash,status,email_verified_at,locale)
     VALUES ($1,'commerce-test-only','active',now(),$2) RETURNING id,password_hash`,
    [`${unique()}@example.test`, lang],
  );
  if (staff) {
    await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'", [id]);
    const [{generation}] = await query(
      'INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true) RETURNING generation',
      [id, randomBytes(32)],
    );
    await query(
      `INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at)
       VALUES($1,$2,$3,$4,'grant',clock_timestamp()+interval '15 minutes')`,
      [digest(grant), id, generation, digest(password_hash)],
    );
  }
  await query(
    `INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorization_generation)
     SELECT $1,id,now()+interval '15 minutes',now()+interval '1 hour','commerce test',$3,$4,native_security_generation
     FROM accounts WHERE id=$2`,
    [digest(session), id, csrf, JSON.stringify(staff ? {mfa_grant: grant} : {})],
  );
  await context.addCookies([{name: '__Host-oteryn_session', value: session, url: origin, secure: true, httpOnly: true}]);
  return {id, csrf};
}

async function open(page, path, status = 200) {
  expect((await page.goto(path, {waitUntil: 'domcontentloaded'})).status()).toBe(status);
}
async function width(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(page.viewportSize().width);
}
async function post(page, path, form, status) {
  const response = await page.request.post(path, {form, headers: {origin}, maxRedirects: 0});
  expect(response.status()).toBe(status);
}

test('store orders and delivery are scoped to the signed-in buyer', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const {id: buyer} = await actor(page.context(), lang);
  const sellerContext = await browser.newContext({baseURL: origin, ignoreHTTPSErrors: true, locale: lang});
  const order = uuid7();
  const request = uuid7();
  const operation = uuid7();
  const line = uuid7();
  const revision = digest(order).toString('hex');
  const offer = `test.order.${order}`;
  const db = pool();
  const conn = await db.connect();
  try {
    await conn.query('BEGIN');
    await conn.query('INSERT INTO wallet_accounts(account_id) VALUES($1)', [buyer]);
    await conn.query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'store_purchase',$2,17)", [operation, buyer]);
    await conn.query("INSERT INTO store_purchase_requests(purchase_request_id,account_id,request_bytes,request_digest,result,answer_bytes) VALUES($1,$2,'{}',$3,'ACCEPTED','{}')", [request, buyer, revision]);
    await conn.query("INSERT INTO store_price_list_imports(catalog_revision,canonical_bytes,imported_by) VALUES($1,'{}',$2)", [revision, buyer]);
    await conn.query("INSERT INTO store_offers(catalog_revision,offer_key,currency,unit_price,max_quantity,purchasable,canonical_bytes) VALUES($1,$2,'COINS',17,1,true,'{}')", [revision, offer]);
    await conn.query("INSERT INTO store_orders(order_id,purchase_request_id,account_id,character_id,world_id,wallet_operation_id,wallet_idempotency_key,currency,total_price) VALUES($1,$2,$3,$4,$5,$6,$7,'COINS',17)", [order, request, buyer, uuid7(), uuid7(), operation, order]);
    await conn.query('INSERT INTO store_order_lines(line_id,order_id,catalog_revision,offer_key,quantity,unit_price) VALUES($1,$2,$3,$4,1,17)', [line, order, revision, offer]);
    await conn.query("INSERT INTO store_fulfilment_units(entitlement_id,line_id,unit_index,account_id,product_id,product_version,unit_kind,grant_key,grant_revision,quantity) VALUES($1,$2,0,$3,'premium.test','1','PREMIUM_TIME','test','1',1)", [uuid7(), line, buyer]);
    await conn.query('COMMIT');
    const base = `/${lang}/account/store-orders`;
    await open(page, '/account');
    await expect(page.locator(`a[href="${base}"]`).first()).toBeVisible();
    await open(page, base);
    await expect(page.getByRole('link', {name: offer})).toBeVisible();
    await width(page);
    await page.getByRole('link', {name: offer}).click();
    await expect(page.locator('main')).toContainText('premium.test');
    await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Oczekuje na dostawę' : 'Awaiting delivery');
    await expect(page.locator('main')).toContainText(/\d{4}-\d{2}-\d{2} .*UTC/);
    await width(page);
    await open(page, `${base}/bad-id`, 422);
    await open(page, `${base}?before=bad-id`, 422);

    await actor(sellerContext, lang);
    const other = await sellerContext.newPage();
    await open(other, base);
    await expect(other.locator('main')).not.toContainText(offer);
    await open(other, `${base}/${order}`, 404);
    await expect(other.locator('main')).not.toContainText(offer);
  } finally {
    await conn.query('ROLLBACK');
    conn.release();
    await db.end();
    await sellerContext.close();
  }
});

test('staff stage an immutable price diff and explicitly activate it', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const staffContext = await browser.newContext({baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport});
  const offers = `/admin/store-catalog?lang=${lang}`;
  const activation = `/admin/store-activation?lang=${lang}`;
  const [{catalog_revision: previous}] = await query('SELECT catalog_revision FROM store_active_catalog WHERE id=1');
  try {
    const {id} = await actor(staffContext, lang, true);
    const admin = await staffContext.newPage();
    await actor(page.context(), lang);
    const offer = `commerce.${unique()}`;
    const document = Buffer.from(JSON.stringify({contract_version: 1, offers: [{
      offer_key: offer, currency: 'COINS', unit_price: '21', max_quantity: 1, purchasable: true,
      units: [{product_id: 'premium.test', product_version: '1', unit_kind: 'PREMIUM_TIME', grant_key: 'test', grant_revision: '1', quantity: 1}],
    }]}));
    const current = digest(document).toString('hex');
    await query('INSERT INTO store_price_list_imports(catalog_revision,canonical_bytes,imported_by) VALUES($1,$2,$3)', [current, document, id]);
    await query("INSERT INTO store_offers(catalog_revision,offer_key,currency,unit_price,max_quantity,purchasable,canonical_bytes) VALUES($1,$2,'COINS',21,1,true,$3)", [current, offer, document]);
    await query('UPDATE store_active_catalog SET catalog_revision=$1 WHERE id=1', [current]);
    await open(page, offers, 403);
    await open(page, activation, 403);
    await open(admin, '/admin');
    await expect(admin.locator(`nav[data-panel=admin] a[href="${offers}"]`)).toBeVisible();
    await expect(admin.locator(`nav[data-panel=admin] a[href="${activation}"]`)).toBeVisible();
    await open(admin, offers);
    await expect(admin.locator('main')).toContainText(offer);
    await width(admin);
    const stage = admin.locator('main form');
    const csrf = await stage.locator('[name=_csrf]').inputValue();
    const fields = {_csrf: csrf, current, offer, price: '31', reason: 'Update verified test price'};
    await csrfRejected(admin, offers, fields);
    await post(admin, offers, {...fields, price: '-1'}, 422);
    await post(admin, offers, {...fields, reason: 'short'}, 422);
    await post(page, offers, fields, 403);
    await post(admin, offers, fields, 303);
    await post(admin, offers, fields, 303);
    const [{catalog_revision: proposed}] = await query('SELECT catalog_revision FROM store_offers WHERE offer_key=$1 AND unit_price=31', [offer]);
    expect((await query('SELECT catalog_revision FROM store_active_catalog WHERE id=1'))[0].catalog_revision).toBe(current);

    await open(admin, activation);
    const proposal = admin.locator('article').filter({hasText: proposed});
    await expect(proposal).toContainText('21 → 31');
    await width(admin);
    const activate = {_csrf: await proposal.locator('[name=_csrf]').inputValue(), current, revision: proposed, reason: 'Approve verified test prices'};
    await csrfRejected(admin, activation, activate);
    await post(admin, activation, {...activate, revision: 'bad'}, 422);
    await post(page, activation, activate, 403);
    await post(admin, activation, activate, 303);
    await post(admin, activation, activate, 409);
    expect((await query('SELECT catalog_revision FROM store_active_catalog WHERE id=1'))[0].catalog_revision).toBe(proposed);
    const audit = await query("SELECT action,before,after FROM audit_log WHERE actor=$1 AND action IN ('store.catalog.price.staged','store.catalog.activated') ORDER BY created_at", [id]);
    expect(audit.map(row => row.action)).toEqual(['store.catalog.price.staged', 'store.catalog.activated']);
    expect(audit[0].before.unit_price).toBe(21);
    expect(audit[0].after.unit_price).toBe(31);
  } finally {
    await query('UPDATE store_active_catalog SET catalog_revision=$1 WHERE id=1', [previous]);
    await staffContext.close();
  }
});

test('payment review supports scoped search, reconciliation, and four-eyes refund', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const {id: account, csrf: playerCsrf} = await actor(page.context(), lang);
  const opts = {baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport};
  const staff = await browser.newContext(opts);
  const reviewer = await browser.newContext(opts);
  try {
    const {csrf} = await actor(staff, lang, true);
    const {csrf: reviewerCsrf} = await actor(reviewer, lang, true);
    const admin = await staff.newPage();
    const second = await reviewer.newPage();
    const intent = uuid7();
    await query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($1,$2,'test',$3,250,'PLN','succeeded')", [intent, account, `test-${intent}`]);
    const base = `/admin/payment-review?lang=${lang}`;
    await open(page, base, 403);
    await open(admin, '/admin');
    await expect(admin.locator(`nav[data-panel=admin] a[href="${base}"]`)).toBeVisible();
    await open(admin, `${base}&account=${account}`);
    await expect(admin.getByRole('link', {name: intent})).toBeVisible();
    await width(admin);
    await admin.getByRole('link', {name: intent}).click();
    await expect(admin.locator('main')).toContainText('2.50 PLN');
    await expect(admin.locator('main')).toContainText(account);
    await width(admin);
    await open(admin, `${base}&id=invalid`, 422);
    await open(admin, `${base}&id=${uuid7()}`, 404);
    await open(admin, `${base}&queue=true`);
    const chargeback = uuid7();
    const quote = uuid7();
    const order = uuid7();
    const packageId = uuid7();
    const invoice = uuid7();
    await query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($1,$2,'test',$3,100,'PLN','chargeback')",
      [chargeback, account, `test-${chargeback}`]);
    await query("INSERT INTO invoices(id,intent_id,account_id,amount,currency) VALUES($1,$2,$3,100,'PLN')", [invoice, chargeback, account]);
    await query("INSERT INTO coin_purchase_quotes(id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at) VALUES($1,$2,$3,25,100,'PLN',2,'test',now())",
      [quote, account, packageId]);
    await query(`INSERT INTO coin_purchase_orders(id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,receipt_id,review_required,code)
      VALUES($1,$2,$3,$4,$5,25,100,'PLN',2,'test',$6,'chargeback',$7,true,'review')`,
      [order, account, uuid7(), quote, packageId, chargeback, invoice]);
    await query("INSERT INTO coin_purchase_reversals(id,order_id,payer_account_id,kind,provider,provider_event_id,coins,debited,outcome) VALUES($1,$2,$3,'chargeback','test',$4,25,0,'held')",
      [uuid7(), order, account, chargeback]);
    await query("INSERT INTO coin_purchase_holds(account_id,order_id,reason) VALUES($1,$2,'reversal_insufficient_balance')", [account, order]);
    await open(admin, `${base}&queue=true`);
    await admin.getByRole('link', {name: chargeback}).click();
    await expect(admin.locator('main')).toContainText('25 COINS');
    await expect(admin.locator('main')).toContainText(lang === 'pl' ? 'Blokada zakupów' : 'Purchase hold');
    await width(admin);


    const common = {target: intent, reason: 'Verify payment and provider records'};
    const reconcile = '/admin/commerce/reconcile';
    await csrfRejected(admin, reconcile, {...common, request_id: uuid7()});
    await post(page, reconcile, {...common, _csrf: playerCsrf, request_id: uuid7()}, 403);
    await post(admin, reconcile, {...common, target: 'bad', _csrf: csrf, request_id: uuid7()}, 422);
    await post(admin, reconcile, {...common, _csrf: csrf, request_id: uuid7()}, 303);
    await open(admin, `${base}&id=${intent}`);
    await expect(admin.locator('main')).toContainText(lang === 'pl' ? 'Ostatnia zapisana weryfikacja' : 'Last review recorded');

    const refund = '/admin/commerce/refund';
    const request = uuid7();
    await csrfRejected(admin, refund, {...common, request_id: request});
    await post(admin, refund, {...common, reason: '', _csrf: csrf, request_id: request}, 422);
    await post(admin, refund, {...common, _csrf: csrf, request_id: request}, 303);
    const approval = `/admin/commerce/approvals/${request}`;
    await open(second, '/admin/commerce');
    await csrfRejected(second, approval);
    await post(admin, approval, {_csrf: csrf}, 403);
    await post(second, '/admin/commerce/approvals/bad', {_csrf: reviewerCsrf}, 422);
    await post(second, approval, {_csrf: reviewerCsrf}, 303);
    expect((await query('SELECT intent_id FROM commerce_refunds WHERE operation_id=$1', [request]))[0].intent_id).toBe(intent);
    await open(admin, `${base}&id=${intent}`);
    await expect(admin.locator('main')).toContainText(lang === 'pl' ? 'Próby przekazania zwrotu operatorowi' : 'Provider delivery attempts');
  } finally {
    await staff.close();
    await reviewer.close();
  }
});
