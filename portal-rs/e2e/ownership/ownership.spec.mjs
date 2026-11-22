import {createHash, randomBytes, randomUUID} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {test, expect} from '../fixtures.mjs';
import {login, password, pool, submit} from '../helpers.mjs';
import {action, isolatedAdmin, loginWithRecoveryCode, query} from '../staff-rbac-helpers.mjs';
import {listing, uuid7} from '../game-sim-helpers.mjs';

const ownership = test.extend({
  players: [async ({browser}, use, info) => {
    const lang = info.project.metadata.locale;
    const baseURL = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
    if (!['localhost', '127.0.0.1', '[::1]'].includes(new URL(baseURL).hostname)) {
      throw new Error('Ownership fixtures require the local portal');
    }
    const actors = [];
    try {
      for (const letter of ['a', 'b']) {
        const account = await seedAccount(letter, lang);
        const context = await browser.newContext({
          baseURL, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
        });
        const page = await context.newPage();
        actors.push({...account, page, context});
        await login(page, lang, account);
        expect(await page.locator('form[action$="/logout"]').count()).toBeGreaterThan(0);
      }
      await use({a: actors[0], b: actors[1], lang});
    } finally {
      await Promise.all(actors.map(({context}) => context.close()));
    }
  }, {scope: 'worker'}],
  operator: [async ({browser}, use, info) => {
    const operator = await isolatedAdmin(browser, info);
    try {
      await use(operator.page);
    } finally {
      await operator.context.close();
    }
  }, {scope: 'worker'}],
});

async function seed(entity, args) {
  const db = pool();
  const databaseURL = db.options.connectionString;
  await db.end();
  const binary = process.env.PORTAL_E2E_BINARY;
  if (!binary) throw new Error('Set PORTAL_E2E_BINARY to the local launcher binary');
  const result = spawnSync(binary, ['seed', entity, JSON.stringify(args)], {
    env: {...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: databaseURL},
    encoding: 'utf8',
  });
  expect(result.status, `seed ${entity}`).toBe(0);
  return JSON.parse(result.stdout);
}

async function seedAccount(letter, locale) {
  const account = {email: `ownership-${letter}-${randomUUID()}@example.invalid`, password: password()};
  return {...account, id: (await seed('account', {...account, locale})).accountId};
}

async function view(page, path) {
  const response = await page.goto(path);
  expect(response.status(), path).toBe(200);
}

async function csrf(page, source) {
  await view(page, source);
  return page.locator('main [name="_csrf"]').first().inputValue();
}

async function check(page, path, {method = 'GET', status = 404, form, data, token, headers = {}, privateValues, snapshot, location, contains}) {
  await test.step(`${method} ${path}`, async () => {
    const before = await snapshot();
    const response = await page.request.fetch(path, {
      method, form: form && token ? {...form, _csrf: token} : form, data, maxRedirects: 0,
      headers: token ? {origin: new URL(page.url()).origin, ...headers, 'x-csrf-token': token} : headers,
    });
    let html = await response.text();
    // The shared shell translates the requested pathname in its locale links,
    // even on refusals. Check for the ID everywhere outside those exact links.
    const pathname = new URL(path, page.url()).pathname;
    for (const locale of ['pl', 'en']) {
      const localized = pathname.replace(/^\/(pl|en)\//, `/${locale}/`);
      html = html.replaceAll(`href="${localized}" lang="${locale}" hreflang="${locale}"`, '');
    }
    expect(response.status(), 'ownership refusal status').toBe(status);
    expect(response.headers().location).toBe(location);
    for (const value of [].concat(contains ?? [])) {
      expect(html.includes(value), 'expected content in response').toBe(true);
    }
    for (const value of privateValues) {
      expect(html.includes(value), 'foreign data in response').toBe(false);
    }
    expect(await snapshot(), 'persisted resource and effects').toEqual(before);
  });
}

ownership('support ticket detail, read acknowledgement, reply and status', async ({players: {a, b, lang}, operator}) => {
  const root = `/${lang}/support/tickets`;
  const subject = `Private ticket ${randomUUID()}`;
  const body = `Private ticket message ${randomUUID()}`;
  await view(a.page, `${root}/create`);
  await submit(a.page, 'main form', {subject, body});
  await expect(a.page).toHaveURL(/support\/tickets\/[0-9a-f-]+$/);
  await expect(a.page.locator('main')).toContainText(body);
  const path = new URL(a.page.url()).pathname;
  const id = path.split('/').at(-1);
  const replyBody = `Public staff reply ${randomUUID()}`;
  await view(operator, `/admin/support/tickets/${id}`);
  await action(operator, 'form:has(input[value=reply])', {body: replyBody});
  await expect(operator.locator('main')).toContainText(replyBody);
  const [ticket] = await query('SELECT account_id, lock_version, unread_reply FROM tickets WHERE id=$1', [id]);
  expect(ticket).toMatchObject({account_id: a.id, unread_reply: true});
  const [{id: reply}] = await query(
    "SELECT id FROM ticket_messages WHERE ticket_id=$1 AND author_kind='staff' AND NOT internal AND body=$2",
    [id, replyBody],
  );
  const snapshot = () => query(`
    SELECT (SELECT to_jsonb(t) FROM tickets t WHERE id=$1) AS ticket,
           (SELECT jsonb_agg(m ORDER BY id) FROM ticket_messages m WHERE ticket_id=$1) AS messages
  `, [id]);
  const token = await csrf(b.page, `${root}/create`);
  const options = {privateValues: [id, subject, body, reply, replyBody], snapshot};
  await check(b.page, path, options);
  await check(b.page, `${path}/read`, {...options, method: 'POST', token, form: {reply}});
  await check(b.page, `${path}/reply`, {
    ...options, method: 'POST', token, form: {lock_version: ticket.lock_version, body: 'Foreign reply attempt'},
  });
  for (const method of ['POST', 'PUT']) {
    await check(b.page, `${path}/status`, {
      ...options, method, token, form: {lock_version: ticket.lock_version, status: 'closed'},
    });
  }
  expect((await snapshot())[0].ticket.unread_reply).toBe(true);
  await view(a.page, path);
  const acknowledge = a.page.locator(`main form[action="${path}/read"]`);
  await expect(acknowledge.locator('[name=reply]')).toHaveValue(reply);
  await submit(a.page, `main form[action="${path}/read"]`);
  await expect(a.page).toHaveURL(new RegExp(`${path}$`));
  expect((await snapshot())[0].ticket.unread_reply).toBe(false);
});

ownership('player report detail is private to its reporter', async ({players: {a, b, lang}}) => {
  const target = `Private report target ${randomUUID()}`;
  const evidence = `Private evidence ${randomUUID()}`;
  await view(a.page, `/${lang}/support/reports/create`);
  await submit(a.page, 'main form', {target_reference: target, evidence_summary: evidence});
  await expect(a.page).toHaveURL(/support\/reports\/[0-9a-f-]+$/);
  await expect(a.page.locator('main')).toContainText(target);
  const path = new URL(a.page.url()).pathname;
  const id = path.split('/').at(-1);
  const snapshot = () => query('SELECT * FROM player_reports WHERE id=$1', [id]);
  expect((await snapshot()).map((report) => report.reporter_account_id)).toEqual([a.id]);
  await check(b.page, path, {privateValues: [id, target, evidence], snapshot});
});

ownership('character creation request is private to its applicant', async ({players: {a, b, lang}}) => {
  const [{id: world}] = await query("INSERT INTO world_status(id,name) VALUES(uuidv7(),'Ownership Test World') RETURNING id");
  await view(a.page, `/${lang}/account/characters/new`);
  const form = a.page.locator('main form[method=post]').first();
  await form.locator('[name=world]').selectOption(world);
  await form.locator('[name=sex]').first().check();
  await form.locator('[name=vocation]').first().check();
  const suffix = [...randomUUID().slice(0, 8)].map((digit) => 'abcdefghijklmnop'[parseInt(digit, 16)]).join('');
  const name = `Ownership ${suffix}`;
  await submit(a.page, 'main form[method=post]', {name});
  await expect(a.page).toHaveURL(/characters\/requests\/[0-9a-f-]+$/);
  const path = new URL(a.page.url()).pathname;
  const id = path.split('/').at(-1);
  const snapshot = () => query('SELECT account_id,state,receipt_json FROM character_commands WHERE id=$1', [id]);
  expect(await snapshot()).toEqual([{account_id: a.id, state: 'PENDING', receipt_json: null}]);
  await check(b.page, path, {privateValues: [a.email, id, name], snapshot});
});

ownership('test checkout page, status and all three settlement actions', async ({players: {a, b, lang}}) => {
  const coins = `/${lang}/shop/coins`;
  await view(a.page, coins);
  const fields = await a.page.locator('main form[action$="/shop/coins"]').first()
    .evaluate((form) => Object.fromEntries(new FormData(form)));
  const {operation, package: packageKey} = fields;
  expect(Boolean(operation && packageKey)).toBe(true);
  await submit(a.page, 'main form[action$="/shop/coins"]');
  await expect(a.page).toHaveURL(/shop\/test-checkout\/[0-9a-f-]+$/);
  const path = new URL(a.page.url()).pathname;
  const id = path.split('/').at(-1);
  await expect(a.page.locator('main button[name=action]')).toHaveCount(3);
  const snapshot = () => query(`
    SELECT (SELECT md5(to_jsonb(p)::text) FROM payment_intents p WHERE id=$1) AS payment,
           (SELECT md5(to_jsonb(t)::text) FROM store_web_topups t WHERE payment_intent_id=$1) AS topup,
           (SELECT jsonb_agg(e.id ORDER BY e.id) FROM payment_events e WHERE intent_id=$1) AS payment_events,
           (SELECT jsonb_agg(i ORDER BY id) FROM invoices i WHERE intent_id=$1) AS invoices,
           (SELECT jsonb_agg(e ORDER BY id) FROM wallet_entries e WHERE operation_id=$1) AS entries,
           (SELECT jsonb_agg(o.id ORDER BY o.id) FROM outbox o WHERE idempotency_key=$1::uuid::text) AS outbox
  `, [id]);
  const [state] = await snapshot();
  expect(Boolean(state.payment && state.topup)).toBe(true);
  const token = await csrf(b.page, coins);
  const options = {privateValues: [id, a.email], snapshot};
  await check(b.page, path, options);
  await check(b.page, `${coins}?checkout=${id}`, options);
  for (const action of ['pay', 'reject', 'cancel']) {
    await check(b.page, path, {...options, method: 'POST', token, form: {action}});
  }
  const replay = {operation, package: packageKey};
  const ownerToken = await csrf(a.page, coins);
  await check(a.page, coins, {
    method: 'POST', status: 303, location: path, token: ownerToken, form: replay, privateValues: [], snapshot,
  });
  await check(b.page, coins, {...options, method: 'POST', status: 409, token, form: replay});
});

ownership('session list and both single-session revocation methods are account-scoped', async ({players: {a, b, lang}}) => {
  const root = `/${lang}/account/sessions`;
  const [{session_id: id}] = await query('SELECT session_id FROM web_sessions WHERE account_id=$1', [a.id]);
  await view(a.page, root);
  await expect(a.page.locator(`main form[action="${root}/${id}"]`)).toHaveCount(1);
  const token = await csrf(b.page, root);
  await expect(b.page.locator(`main form[action="${root}/${id}"]`)).toHaveCount(0);
  const snapshot = () => query(`
    SELECT (SELECT md5(to_jsonb(s)::text) FROM web_sessions s WHERE session_id=$1) AS session,
           (SELECT jsonb_agg(e ORDER BY id) FROM security_events e WHERE account_id IN ($2,$3)) AS events
  `, [id, a.id, b.id]);
  for (const method of ['POST', 'DELETE']) {
    await check(b.page, `${root}/${id}`, {
      method, status: 302, token, form: {}, privateValues: [id, a.email], snapshot, location: root,
    });
    await view(b.page, root);
    await expect(b.page.locator('main [role=status]')).toHaveText(lang === 'pl'
      ? 'Wybrana sesja jest nieaktywna lub niedostępna.'
      : 'The selected session is inactive or unavailable.');
  }
  await view(a.page, root);
});

ownership('passkey picker and remove action are account-scoped', async ({players: {a, b, lang}}) => {
  const root = `/${lang}/account/passkeys`;
  await view(a.page, root);
  const cdp = await a.context.newCDPSession(a.page);
  await cdp.send('WebAuthn.enable');
  const {authenticatorId} = await cdp.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2', transport: 'internal', hasResidentKey: true,
      hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true,
    },
  });
  try {
    await a.page.locator('#passkey-password').fill(a.password);
    const [registration] = await Promise.all([
      a.page.waitForResponse((response) => new URL(response.url()).pathname === `${root}/register/finish`),
      a.page.locator('[data-passkey="register"]').click(),
    ]);
    expect(registration.status(), 'WebAuthn registration succeeded').toBe(200);
    await expect(a.page.locator('[data-passkey="authenticate"]')).toHaveCount(1);
  } finally {
    await cdp.send('WebAuthn.removeVirtualAuthenticator', {authenticatorId});
    await cdp.detach();
  }
  const [{id}] = await query('SELECT id FROM account_passkeys WHERE account_id=$1', [a.id]);
  await login(a.page, lang, a);
  await view(a.page, root);
  const selector = `[data-remove="${id}"]`;
  await expect(a.page.locator(selector)).toHaveCount(1);
  await view(b.page, root);
  await expect(b.page.locator(selector)).toHaveCount(0);
  const token = await b.page.locator('#passkeys').getAttribute('data-csrf');
  expect(token?.length).toBe(64);
  const snapshot = () => query(`
    SELECT (SELECT md5(to_jsonb(k)::text) FROM account_passkeys k WHERE id=$1) AS passkey,
           (SELECT jsonb_agg(session_id ORDER BY session_id)
            FROM web_sessions WHERE account_id IN ($2,$3)) AS sessions
  `, [id, a.id, b.id]);
  await check(b.page, `${root}/${id}/remove`, {
    method: 'POST', status: 403, token, data: {password: b.password},
    privateValues: [id, a.email], snapshot,
  });
});

async function seedProjectedCharacter(account, name) {
  const authority = process.env.OTERYN_LCFA_SOURCE_AUTHORITY;
  expect(Boolean(authority)).toBe(true);
  const [character] = await query(`
    WITH feed AS (
      UPDATE account_character_feed
      SET highest_epoch=COALESCE(highest_epoch,'3'),watermark_epoch=COALESCE(highest_epoch,'3'),
          source_authority=$1,complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,
          observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint
      WHERE id=1 RETURNING highest_epoch,observed_at
    ), owner_snapshot AS (
      INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,
        projection_revision,source_observed_at,content_digest)
      SELECT $2,$1,highest_epoch,'1',observed_at,decode(repeat('00',32),'hex') FROM feed ON CONFLICT (account_id) DO NOTHING
    )
    INSERT INTO account_characters(account_id,character_id,world_id,name,availability)
    VALUES($2,uuidv7(),uuidv7(),$3,'AVAILABLE') RETURNING character_id,world_id
  `, [authority, account, name]);
  return character;
}

ownership('party post removal is restricted to its owner', async ({players: {a, b, lang}}) => {
  const root = `/${lang}/party`;
  const note = `Public party note ${randomUUID()}`;
  const {character_id, world_id} = await seedProjectedCharacter(a.id, 'Party Owner');
  // The local harness has no party character creation authority. Seed the
  // Portal-owned board row and use its real public page and removal action.
  const [{id}] = await query(`
    INSERT INTO pd_party_posts(account_id,character_id,character_name,world_id,purpose,
      min_level,max_level,profession,note,starts_at,expires_at)
    VALUES($1,$3,'Party Owner',$4,'hunt',1,100,'any',$2,now(),now()+interval '1 day')
    RETURNING id
  `, [a.id, note, character_id, world_id]);
  await view(a.page, root);
  await expect(a.page.locator('main')).toContainText(note);
  const action = `${root}/${id}/remove`;
  await expect(a.page.locator(`main form[action="${action}"]`)).toHaveCount(1);
  const token = await csrf(b.page, root);
  await expect(b.page.locator('main')).toContainText(note);
  const snapshot = () => query('SELECT * FROM pd_party_posts WHERE id=$1', [id]);
  await check(b.page, action, {
    method: 'POST', token, form: {}, privateValues: [id, note, a.id], snapshot,
  });
  const before = await snapshot();
  const reason = 'Synthetic report on a public party post';
  await submit(b.page, `main form[action="${root}/${id}/report"]`, {reason});
  await expect(b.page).toHaveURL(new RegExp(`${root}$`));
  expect(await query('SELECT account_id,reason FROM pd_party_reports WHERE post_id=$1', [id]))
    .toEqual([{account_id: b.id, reason}]);
  expect(await snapshot()).toEqual(before);
});

ownership('notification preferences are scoped by the authenticated account', async ({players: {a, b, lang}}) => {
  const path = `/${lang}/account/notifications`;
  const selection = '[name=support_email]';
  await view(a.page, path);
  await submit(a.page, 'main form', {support_email: true, support_portal: true});
  await expect(a.page.locator(selection)).toBeChecked();
  const snapshot = () => query('SELECT * FROM account_notification_prefs WHERE account_id=$1 ORDER BY category', [a.id]);
  const original = await snapshot();
  expect(original.find(({category}) => category === 'support')?.email_enabled).toBe(true);
  const token = await csrf(b.page, path);
  await expect(b.page.locator(selection)).not.toBeChecked();
  await check(b.page, path, {
    method: 'POST', status: 422, token, form: {account_id: a.id},
    privateValues: [a.id, a.email], snapshot,
  });
  await submit(b.page, 'main form', {support_email: false, support_portal: false});
  await expect(b.page.locator('[name=support_portal]')).not.toBeChecked();
  expect(await snapshot()).toEqual(original);
});

async function fundWallet(actor, lang) {
  await view(actor.page, `/${lang}/shop/coins`);
  await submit(actor.page, 'main form[action$="/shop/coins"]');
  await expect(actor.page).toHaveURL(/shop\/test-checkout\/[0-9a-f-]+$/);
  const intent = new URL(actor.page.url()).pathname.split('/').at(-1);
  await actor.page.locator('main button[value=pay]').click();
  await expect.poll(async () => (await query(
    'SELECT id FROM wallet_entries WHERE operation_id=$1 AND account_id=$2',
    [intent, actor.id],
  )).length, {timeout: 30000}).toBe(1);
  return intent;
}

ownership('wallet cursor stays account-scoped and transfer operation IDs cannot be reused', async ({players: {a, b, lang}}) => {
  const history = `/${lang}/shop/history`;
  const transfer = `/${lang}/shop/transfer`;
  const {id: recipient} = await seedAccount('recipient', lang);
  const credit = await fundWallet(a, lang);
  await fundWallet(b, lang);
  async function send(actor, amount) {
    await view(actor.page, transfer);
    const id = await actor.page.locator('main [name=operation]').inputValue();
    await submit(actor.page, 'main form', {recipient, amount});
    await expect(actor.page).toHaveURL(new RegExp(`${history}$`));
    await expect(actor.page.locator('main')).toContainText(id);
    return id;
  }
  const foreignEntry = await send(a, '137');
  const ownEntry = await send(b, '83');
  const operation = await send(a, '120');
  await expect(a.page).toHaveURL(new RegExp(`${history}$`));
  await expect(a.page.locator('main')).toContainText(credit);
  await expect(a.page.locator('main')).toContainText(operation);
  const [{id: cursor}] = await query('SELECT id FROM wallet_entries WHERE operation_id=$1 AND account_id=$2', [operation, a.id]);
  const snapshot = () => query(`
    SELECT (SELECT jsonb_agg(t ORDER BY id) FROM wallet_transfers t
            WHERE sender_id IN ($1,$2,$3) OR recipient_id IN ($1,$2,$3)) AS transfers,
           (SELECT jsonb_agg(e ORDER BY id) FROM wallet_entries e WHERE account_id IN ($1,$2,$3)) AS entries,
           (SELECT jsonb_agg(o ORDER BY id) FROM wallet_operations o WHERE account_id IN ($1,$2,$3)) AS operations
  `, [a.id, b.id, recipient]);
  await check(b.page, `${history}?before=${cursor}`, {
    status: 200, contains: [ownEntry, '-83 / 0</p>'],
    privateValues: [credit, cursor, foreignEntry, '-137 / 0</p>'], snapshot,
  });
  const token = await csrf(b.page, transfer);
  await check(b.page, transfer, {
    method: 'POST', status: 409, token, form: {operation, recipient, amount: '120'},
    privateValues: [operation, a.id, credit], snapshot,
  });
});

ownership('private Bazaar listing aliases, seller controls and bid replay binding', async ({players: {a, b, lang}}) => {
  // Draft is Portal-owned orchestration state. No Game ownership or custody
  // attestation is needed to exercise its private reads and seller controls.
  const [{listing_id: id, asset_id: asset}] = await query(`
    WITH listing AS (
      INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at)
      VALUES(uuidv7(),$1,'item','auction','coins',76319,false,now()+interval '1 day') RETURNING id
    )
    INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,
      expected_revision,session_hash,quantity,publication_consent,input_binding)
    SELECT id,uuidv7(),uuidv7(),uuidv7(),uuidv7(),1,decode(repeat('00',32),'hex'),1,true,
      'ownership fixture' FROM listing RETURNING listing_id,asset_id
  `, [a.id]);
  const detail = `/${lang}/bazaar/${id}`;
  const items = `/${lang}/bazaar/items/${id}`;
  const account = `/${lang}/account/bazaar`;
  const control = `${account}/${id}/control`;
  await view(a.page, detail);
  await expect(a.page.locator(`main [data-listing="${id}"]`)).toHaveAttribute('data-state', 'draft');
  await expect(a.page.locator(`main form[action="${control}"] button`)).toHaveCount(2);
  await view(a.page, items);
  await expect(a.page.locator(`main [data-listing="${id}"]`)).toHaveCount(1);
  await view(b.page, account);
  await expect(b.page.locator(`main [data-listing="${id}"]`)).toHaveCount(0);
  const snapshot = () => query(`
    SELECT (SELECT to_jsonb(l) FROM bazaar_listings l WHERE id=$1) AS listing,
           (SELECT md5(to_jsonb(a)::text) FROM bazaar_authority_assets a WHERE listing_id=$1) AS asset,
           (SELECT jsonb_agg(r ORDER BY request_id) FROM bazaar_authority_browser_requests r WHERE listing_id=$1) AS controls,
           (SELECT jsonb_agg(r ORDER BY request_id) FROM bazaar_authority_bid_requests r WHERE listing_id=$1) AS requests,
           (SELECT jsonb_agg(r ORDER BY id) FROM bazaar_bids r WHERE listing_id=$1) AS bids,
           (SELECT jsonb_agg(o.operation_id ORDER BY o.operation_id) FROM bazaar_game_outbox o WHERE listing_id=$1) AS outbox
  `, [id]);
  const options = {privateValues: [id, asset, '76319'], snapshot};
  await check(b.page, detail, options);
  await check(b.page, items, options);
  const token = await csrf(b.page, `${account}/sell`);
  for (const action of ['resume', 'cancel']) {
    const [{request_id}] = await query('SELECT uuidv7() AS request_id');
    await check(b.page, control, {...options, method: 'POST', status: 403, token, form: {action, request_id}});
  }
  // The stored replay binding belongs to A. The handler checks it before
  // listing availability or funding, so the refusal also holds after expiry.
  const [{request_id, character_id}] = await query(`
    INSERT INTO bazaar_authority_bid_requests(request_id,listing_id,buyer,character_id,maximum)
    VALUES(uuidv7(),$1,$2,uuidv7(),12000) RETURNING request_id,character_id
  `, [id, a.id]);
  await check(b.page, `${account}/${id}/bids`, {
    ...options, method: 'POST', status: 409, token,
    form: {request_id, buyer_character_id: character_id, maximum: '12000'},
    privateValues: [...options.privateValues, request_id, character_id],
  });
});

ownership('character profile slug and projected character reads enforce ownership', async ({players: {a, b, lang}}) => {
  const name = `Owner${randomUUID().slice(0, 8)}`;
  const {character_id: character} = await seedProjectedCharacter(a.id, name);
  const root = `/${lang}/account/characters/${name}`;
  const path = `${root}/profile`;
  const comment = `Private profile ${randomUUID()}`;
  await view(a.page, path);
  await submit(a.page, 'main form', {comment});
  await expect(a.page.locator('[name=comment]')).toHaveValue(comment);
  const snapshot = () => query(`
    SELECT (SELECT jsonb_agg(p ORDER BY id) FROM character_profiles p WHERE character_id=$1) AS profiles,
           (SELECT jsonb_agg(e ORDER BY id) FROM character_profile_events e WHERE account_id IN ($2,$3)) AS events
  `, [character, a.id, b.id]);
  const options = {privateValues: [name, character, comment, a.id], snapshot};
  await check(b.page, path, options);
  const token = await csrf(b.page, `/${lang}/account/notifications`);
  await check(b.page, path, {...options, method: 'POST', token, form: {comment: 'Foreign profile edit'}});
  for (const segment of ['achievements', 'equipment', 'outfit']) {
    await check(b.page, `${root}/${segment}`, options);
  }
});

ownership('payment receipt is private to the billed player', async ({players: {a, b, lang}}) => {
  await view(a.page, `/${lang}/shop/coins`);
  await submit(a.page, 'main form[action$="/shop/coins"]');
  await expect(a.page).toHaveURL(/shop\/test-checkout\/[0-9a-f-]+$/);
  const intent = new URL(a.page.url()).pathname.split('/').at(-1);
  const [{id}] = await query(`
    INSERT INTO invoices(id,intent_id,account_id,amount,currency)
    VALUES(uuidv7(),$1,$2,73199,'PLN') RETURNING id
  `, [intent, a.id]);
  const path = `/${lang}/panel/payments/receipts/${id}`;
  await view(a.page, path);
  await expect(a.page.locator('main')).toContainText('73199');
  await check(b.page, path, {
    privateValues: [id, intent, '73199', a.id],
    snapshot: () => query('SELECT * FROM invoices WHERE id=$1', [id]),
  });
});

ownership('hunt session detail, comparison and disabled sharing preserve private records', async ({players: {a, b, lang}}) => {
  const root = `/${lang}/account/hunts`;
  const place = 'place:private-ownership-hunt';
  const rows = await query(`
    WITH ids AS (
      SELECT uuidv7() AS id,uuidv7() AS character,uuidv7() AS generation,
             floor(extract(epoch FROM clock_timestamp()))::bigint - 120 AS start
      FROM generate_series(1,2)
    ), sessions AS (
      INSERT INTO pd_hunt_sessions(account_id,session_id,stream_generation,character_revision,
        character_id,started_at,ended_at,expires_at,has_party,facts)
      SELECT $1,id,generation,1,character,start,start+60,start+86400,false,
        jsonb_build_object('session_id',id,'character_id',character,'started_at',start,
          'ended_at',start+60,'duration_seconds',60,'experience_gained',72491,'experience_lost',0,
          'places',jsonb_build_object($2::text,60),'loot','{}'::jsonb,'supplies','{}'::jsonb,
          'damage_dealt','[]'::jsonb,'damage_taken','[]'::jsonb,'healing_done',0,
          'healing_received',0,'monsters_killed','{}'::jsonb,'party',NULL)
      FROM ids RETURNING account_id,session_id,stream_generation,character_revision
    )
    INSERT INTO pd_hunt_access(account_id,session_id,stream_generation,character_revision,complete,allowed)
    SELECT account_id,session_id,stream_generation,character_revision,true,true
    FROM sessions RETURNING session_id
  `, [a.id, place]);
  const ids = rows.map(({session_id}) => session_id);
  const path = `${root}/${ids[0]}`;
  const comparison = `${root}/compare?ids=${ids.join(',')}`;
  await view(a.page, path);
  await expect(a.page.locator('main')).toContainText(place);
  await view(a.page, comparison);
  const options = {
    privateValues: [...ids, place, '72491'],
    snapshot: () => query(`
      SELECT to_jsonb(s) AS session,to_jsonb(a) AS access
      FROM pd_hunt_sessions s JOIN pd_hunt_access a USING(account_id,session_id)
      WHERE s.session_id=ANY($1) ORDER BY s.session_id
    `, [ids]),
  };
  await check(b.page, path, options);
  await check(b.page, comparison, options);
  const token = await csrf(b.page, `/${lang}/account/notifications`);
  await check(b.page, `${path}/share`, {...options, status: 503});
  await check(b.page, `${path}/share`, {...options, method: 'POST', status: 503, token, form: {}});
});

ownership('character service read, schedule, cancel and rename are owner-only', async ({players: {a, b, lang}}) => {
  const authority = process.env.OTERYN_CHARACTER_SERVICES_PROJECTION_AUTHORITY;
  expect(Boolean(authority), 'character service fixture authority').toBe(true);
  await view(b.page, `/${lang}/account/sudo`);
  await submit(b.page, 'main form', {current_password: b.password});
  const [{sudo}] = await query('SELECT sudo_until>now() AS sudo FROM web_sessions WHERE account_id=$1', [b.id]);
  expect(sudo, 'request reaches ownership with live recent authentication').toBe(true);
  const token = await csrf(b.page, `/${lang}/account/notifications`);

  const name = 'Private Service Character';
  const {character_id: character, world_id: world} = await seedProjectedCharacter(a.id, name);
  const [{epoch, stamp, deletion}] = await query(`
    UPDATE character_service_feed
    SET epoch=(SELECT highest_epoch::numeric FROM account_character_feed WHERE id=1),
        complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,
        observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint
    WHERE id=1 RETURNING epoch::text,observed_at::text AS stamp,uuidv7() AS deletion
  `);
  const state = {
    contract_version: 2, operation: 'PublishCharacterServiceStateV2', source_authority: authority,
    account_id: a.id, character_id: character, projection_epoch: epoch, projection_revision: '1',
    source_observed_at: stamp, character_revision: '1', world_id: world, name, sex: null,
    lifecycle: 'ACTIVE', deletion_operation_id: null, deletion_scheduled_at: null, deletion_effective_at: null,
  };
  await query(`
    INSERT INTO character_service_states(account_id,character_id,epoch,revision,character_revision,snapshot_json)
    VALUES($1,$2,$3,1,'1',$4)
  `, [a.id, character, epoch, JSON.stringify(state)]);
  const path = `/${lang}/account/characters/${character}/services`;
  await view(a.page, path);
  await expect(a.page.locator('main')).toContainText(name);
  for (const action of ['schedule', 'rename']) {
    await expect(a.page.locator(`main form [name=action][value="${action}"]`)).toHaveCount(1);
  }
  const snapshot = () => query(`
    SELECT (SELECT to_jsonb(s) FROM character_service_states s WHERE account_id=$2 AND character_id=$1) AS state,
           (SELECT jsonb_agg(o ORDER BY operation_id) FROM character_service_orders o
            WHERE character_id=$1 OR account_id IN ($2,$3)) AS orders,
           (SELECT jsonb_agg(c ORDER BY id) FROM character_commands c WHERE account_id IN ($2,$3)) AS commands,
           (SELECT jsonb_agg(e ORDER BY id) FROM wallet_entries e WHERE account_id IN ($2,$3)) AS entries
  `, [character, a.id, b.id]);
  const options = {privateValues: [character, world, name, a.id, deletion], snapshot, status: 403};
  await check(b.page, path, options);
  for (const [action, fields] of [
    ['schedule', {}],
    ['rename', {requested_name: 'Another Character', accepted_price: '70'}],
    ['cancel', {deletion_operation_id: deletion}],
  ]) {
    if (action === 'cancel') {
      const pending = {
        ...state, lifecycle: 'DELETION_PENDING', projection_revision: '2', character_revision: '2',
        deletion_operation_id: deletion, deletion_scheduled_at: stamp,
        deletion_effective_at: String(Number(stamp) + 3600),
      };
      await query(`
        UPDATE character_service_states SET snapshot_json=$1,revision=2,character_revision='2'
        WHERE account_id=$2 AND character_id=$3
      `, [JSON.stringify(pending), a.id, character]);
      await view(a.page, path);
      await expect(a.page.locator('main form [name=action][value="cancel"]')).toHaveCount(1);
    }
    const [{request_id}] = await query('SELECT uuidv7() AS request_id');
    await check(b.page, path, {
      ...options, method: 'POST', token, form: {request_id, action, confirm: 'yes', ...fields},
    });
  }
});


ownership('Atlas marker list and deletion enforce the marker owner', async ({players: {a, b}}) => {
  const path = '/atlas/private-markers';
  const label = `Private marker ${randomUUID()}`;
  const headers = (page, token) => ({
    origin: new URL(page.url()).origin, 'x-csrf-token': token,
  });
  async function markers(page) {
    const response = await page.request.get(path);
    expect(response.status()).toBe(200);
    return response.json();
  }
  const aToken = (await markers(a.page)).csrf_token;
  const create = await a.page.request.post(path, {
    data: {x: 100, y: 200, floor: 7, text: label}, headers: headers(a.page, aToken),
  });
  expect(create.status()).toBe(201);
  const {id} = await create.json();
  expect((await markers(a.page)).markers).toContainEqual({id, x: 100, y: 200, floor: 7, label});
  const foreign = await markers(b.page);
  expect(foreign.markers.some((marker) => marker.id === id || marker.label === label)).toBe(false);
  const snapshot = () => query('SELECT * FROM atlas_private_markers WHERE id=$1', [id]);
  expect((await snapshot())[0].account_id).toBe(a.id);
  await check(b.page, `${path}/${id}`, {
    method: 'DELETE', token: foreign.csrf_token, snapshot, privateValues: [id, label],
  });
  expect((await a.page.request.delete(`${path}/${id}`, {
    headers: headers(a.page, aToken),
  })).status()).toBe(204);
  expect(await snapshot()).toEqual([]);
});

const nativeClient = '019a0000-0000-7000-8000-0000000c11e7';
const callback = 'https://localhost:18443/__oauth/callback';

async function nativePayer(page) {
  const verifier = randomBytes(32).toString('base64url');
  const state = randomBytes(24).toString('hex');
  const params = new URLSearchParams({
    response_type: 'code', client_id: nativeClient, redirect_uri: callback,
    scope: 'game:account-services', state,
    code_challenge: createHash('sha256').update(verifier).digest('base64url'),
    code_challenge_method: 'S256',
  });
  await view(page, `/oauth/authorize?${params}`);
  const consent = page.locator('main form').first();
  const form = await consent.evaluate((node) => Object.fromEntries(new FormData(node)));
  const response = await page.request.post(await consent.getAttribute('action') || '/oauth/authorize', {
    form: {...form, decision: 'approve'}, headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
  expect(response.status()).toBe(302);
  const destination = new URL(response.headers().location);
  expect(destination.searchParams.get('state') === state).toBe(true);
  expect(destination.origin + destination.pathname).toBe(callback);
  const token = await page.request.post('/oauth/token', {form: {
    grant_type: 'authorization_code', client_id: nativeClient, redirect_uri: callback,
    code: destination.searchParams.get('code'), code_verifier: verifier,
  }});
  expect(token.status()).toBe(200);
  const {access_token: accessToken} = await token.json();
  expect(Boolean(accessToken)).toBe(true);
  return {authorization: `Bearer ${accessToken}`};
}

ownership('native request status and sandbox checkout require the payer', async ({players: {a, b, lang}}) => {
  await seed('oauth-native-client', {clientId: nativeClient});
  const aHeaders = await nativePayer(a.page);
  const bHeaders = await nativePayer(b.page);
  const api = '/api/v1/native-account/coin-purchases';
  const catalogue = await a.page.request.get(`${api}/catalogue`, {headers: aHeaders});
  expect(catalogue.status()).toBe(200);
  expect((await b.page.request.get(`${api}/catalogue`, {headers: bHeaders})).status()).toBe(200);
  const {packages} = await catalogue.json();
  expect(packages).toHaveLength(1);
  const [offer] = packages;
  const requestId = uuid7();
  const created = await a.page.request.post(`${api}/orders`, {headers: aHeaders, data: {
    version: 1, request_id: requestId, package_id: offer.id, quote_id: offer.quote_id, character_id: null,
  }});
  expect(created.status()).toBe(200);
  const {order_id: order, checkout_url: checkout} = await created.json();
  const checkoutPath = `/native-account/coin-checkout/${order}`;
  expect(new URL(checkout).pathname).toBe(checkoutPath);
  const statusPath = `${api}/requests/${requestId}`;
  const ownerStatus = await a.page.request.get(statusPath, {headers: aHeaders});
  expect(ownerStatus.status()).toBe(200);
  expect(await ownerStatus.json()).toMatchObject({order_id: order, request_id: requestId, state: 'awaiting_payment'});
  await view(a.page, checkoutPath);
  await expect(a.page.locator('main form input[name=action]')).toHaveCount(2);
  const [{payment_intent_id: intent}] = await query(
    'SELECT payment_intent_id FROM coin_purchase_orders WHERE id=$1 AND payer_account_id=$2', [order, a.id],
  );
  const snapshot = () => query(`
    SELECT (SELECT to_jsonb(p) FROM coin_purchase_orders p WHERE id=$1) AS purchase,
           (SELECT md5(to_jsonb(p)::text) FROM payment_intents p WHERE id=$2) AS payment,
           (SELECT jsonb_agg(e.id ORDER BY e.id) FROM payment_events e WHERE intent_id=$2) AS events,
           (SELECT jsonb_agg(i ORDER BY i.id) FROM invoices i WHERE intent_id=$2) AS invoices,
           (SELECT jsonb_agg(e ORDER BY id) FROM wallet_entries e WHERE operation_id=$2) AS entries
  `, [order, intent]);
  const options = {privateValues: [order, requestId, intent], snapshot};
  await check(b.page, statusPath, {...options, headers: bHeaders});
  await check(b.page, checkoutPath, options);
  const token = await csrf(b.page, `/${lang}/shop/coins`);
  for (const decision of ['pay', 'fail']) {
    await check(b.page, checkoutPath, {
      ...options, method: 'POST', token, form: {action: decision},
    });
  }
  const ownerOrigin = new URL(a.page.url()).origin;
  const ownerToken = await a.page.locator('main form:has(input[value=pay]) [name=_csrf]').inputValue();
  expect(ownerToken).toBeTruthy();
  const settle = {...options, method: 'POST', status: 403, form: {action: 'pay'}};
  await check(a.page, checkoutPath, {...settle, headers: {origin: ownerOrigin}});
  await check(a.page, checkoutPath, {
    ...settle, token: ownerToken, headers: {origin: 'https://untrusted.example.invalid'},
  });
  const [settled] = await Promise.all([
    a.page.waitForResponse((response) => response.request().method() === 'POST'
      && new URL(response.url()).pathname === checkoutPath),
    a.page.locator('main form:has(input[name=action][value=pay]) button').click(),
  ]);
  expect(settled.status()).toBe(303);
  expect((await snapshot())[0].purchase.state).toBe('paid');
});

async function enroll(actor, lang, remember = false) {
  const {page} = actor;
  const {recoveryCodes: [recovery]} = await seed('account-mfa', {accountId: actor.id, password: actor.password});
  if (remember) {
    await login(page, lang, actor);
    await expect(page).toHaveURL(/mfa\/challenge/);
    await submit(page, 'form[action$="/mfa/challenge"]', {code: recovery, remember: true});
  } else {
    await loginWithRecoveryCode(page, lang, actor, recovery);
  }
}

async function publishRoster(a, b) {
  const character = uuid7();
  const published = await b.page.request.post('/__bazaar_game/seed', {data: {
    seller: a.id, buyer: b.id, kind: 'item', asset_id: uuid7(),
    seller_character_id: uuid7(), buyer_character_id: character,
  }});
  expect(published.status()).toBe(201);
  let roster;
  await expect.poll(async () => {
    [roster] = await query(
      'SELECT * FROM account_characters WHERE account_id=$1 AND character_id=$2',
      [b.id, character],
    );
    return roster?.account_id;
  }, {message: 'the requested roster revision reached PostgreSQL'}).toBe(b.id);
  return roster;
}

ownership('private item and character-history reads fail closed without G-10 authority', async ({players: {a, b, lang}}) => {
  const {character_id: character, world_id: world, name} = await publishRoster(a, b);
  const snapshot = () => query(`
    SELECT (SELECT to_jsonb(c) FROM account_characters c WHERE account_id=$1 AND character_id=$2) AS character,
           (SELECT jsonb_agg(s ORDER BY family) FROM pd_t76_streams s
            WHERE world_id=$3 AND account_id IN ($1,$4)) AS streams
  `, [b.id, character, world, a.id]);
  const options = {status: 503, privateValues: [b.id, character, world, name], snapshot};
  for (const path of [
    `/${lang}/panel/items/${world}/${character}`,
    `/${lang}/panel/history/${world}/market/${character}`,
    `/${lang}/panel/history/${world}/trades/${character}`,
    `/${lang}/panel/history/${world}/bank`,
  ]) {
    await check(a.page, path, options);
    await check(b.page, path, options);
  }
});

ownership('Atlas bestiary and Collection fail closed with only a verified roster', async ({players: {a, b, lang}}) => {
  const {character_id: character, name} = await publishRoster(a, b);
  const snapshot = () => query(
    'SELECT * FROM account_characters WHERE account_id=$1 AND character_id=$2',
    [b.id, character],
  );
  const options = {status: 503, privateValues: [character, name, b.id], snapshot};
  for (const path of [`/${lang}/atlas/bestiary/${character}`, `/${lang}/collection/${character}`]) {
    await check(b.page, path, options);
    await check(a.page, path, options);
  }
});

ownership('Bazaar promotion form is scoped to the seller', async ({players: {a, b, lang}, operator, request}) => {
  await view(operator, `/admin/${lang}/bazaar`);
  await submit(operator, 'form:has(input[value=fees])', {
    promotion_price: '50', reason: 'Disposable ownership promotion quote',
  });
  expect(await query('SELECT id FROM bazaar_fee_revisions WHERE promotion_price=50')).not.toHaveLength(0);
  await enroll(a, lang);
  await view(a.page, `/${lang}/account/sudo`);
  await submit(a.page, 'main form', {current_password: a.password});
  const {id} = await listing(a, b, lang);
  const path = `/${lang}/account/bazaar/${id}/promote`;
  const guest = await request.get(path, {maxRedirects: 0});
  expect(guest.status()).toBe(303);
  expect(guest.headers().location).toBe(`/${lang}/login`);
  await view(a.page, path);
  const form = await a.page.locator('main form').first().evaluate((node) => {
    const {id, fee_revision, amount} = Object.fromEntries(new FormData(node));
    return {id, fee_revision, amount};
  });
  expect(form.amount).toBe('50');
  const snapshot = () => query(`
    SELECT (SELECT jsonb_agg(p ORDER BY p.id) FROM bazaar_promotions p WHERE listing_id=$1) AS promotions,
           (SELECT jsonb_agg(e ORDER BY e.id) FROM wallet_entries e WHERE operation_id=$2) AS debits
  `, [id, form.id]);
  const options = {status: 403, privateValues: [form.fee_revision, id, form.id], snapshot};
  await check(b.page, path, options);
  const token = await csrf(b.page, `/${lang}/shop/coins`);
  await check(b.page, path, {...options, method: 'POST', token, form});
});

ownership('MFA trusted-device removal requires the device owner', async ({players: {a, b, lang}}) => {
  // The promotion case already authenticated A with MFA. Use separate seeded
  // players here so both actors exercise the real remember-device challenge.
  const aDevice = await seedAccount('device-a', lang);
  const bDevice = await seedAccount('device-b', lang);
  const aState = {...aDevice, page: await a.context.newPage()};
  const bState = {...bDevice, page: await b.context.newPage()};
  try {
    await login(aState.page, lang, aState);
    await enroll(aState, lang, true);
    const path = `/${lang}/mfa`;
    await view(aState.page, path);
    const selector = 'main form[action$="/mfa/devices"]';
    const device = await aState.page.locator(`${selector} [name=device]`).inputValue();
    const snapshot = () => query(`
      SELECT md5(to_jsonb(t)::text) AS device FROM account_mfa_tokens t
      WHERE account_id=$1 AND purpose='trusted' AND encode(token_hash,'hex')=$2
    `, [aState.id, device]);
    expect(await snapshot()).toHaveLength(1);

    await login(bState.page, lang, bState);
    await enroll(bState, lang, true);
    await view(bState.page, path);
    const token = await bState.page.locator(`${selector} [name=_csrf]`).inputValue();
    await check(bState.page, `${path}/devices`, {
      method: 'POST', status: 200, form: {device}, token, snapshot, privateValues: [device],
      contains: lang === 'pl' ? 'Nie udało się zweryfikować żądania.' : 'The request could not be verified.',
    });
    await submit(aState.page, selector);
    await expect(aState.page.locator(selector)).toHaveCount(0);
    expect(await snapshot()).toEqual([]);
  } finally {
    await aState.page.close();
    await bState.page.close();
  }
});
