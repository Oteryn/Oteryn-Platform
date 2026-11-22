import { spawnSync } from 'node:child_process';
import { randomUUID, createHash } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, password } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';

const sharedMessage = { pl: 'Zbyt wiele żądań', en: 'Too many requests' };
const reportMessage = {
  pl: 'Zbyt wiele zgłoszeń. Spróbuj ponownie za minutę.',
  en: 'Too many reports. Try again in one minute.',
};
const inviteMessage = {
  pl: 'Jeśli adres spełnia warunki, zaproszenie trafi do kolejki. Dane kont nie są ujawniane.',
  en: 'If the address is eligible, the invitation is queued. No account details are disclosed.',
};
const world = '019a0000-0000-7000-8000-00000000e001';
const uuidPath = '[a-f0-9-]{36}';

async function seedAccount(lang) {
  const account = { email: `abuse-${randomUUID()}@example.invalid`, password: password() };
  const result = spawnSync(process.env.PORTAL_E2E_BINARY, [
    'seed', 'account', JSON.stringify({ ...account, locale: lang }),
  ], {
    encoding: 'utf8',
    env: { ...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: process.env.PORTAL_E2E_DATABASE_URL },
  });
  expect(result.status, 'local account seed exit status').toBe(0);
  return { ...account, id: JSON.parse(result.stdout).accountId };
}

async function withPlayers(browser, info, exercise) {
  const lang = info.project.metadata.locale;
  await query('SELECT 1'); // Validate the disposable database before invoking the seed CLI.
  const contexts = [];
  try {
    const players = [];
    for (let i = 0; i < 2; i++) {
      const account = await seedAccount(lang);
      const context = await browser.newContext({
        baseURL: process.env.PORTAL_E2E_ORIGIN, ignoreHTTPSErrors: true,
        locale: lang, viewport: info.project.use.viewport,
        extraHTTPHeaders: { 'Accept-Language': lang },
      });
      contexts.push(context);
      const page = await context.newPage();
      await login(page, lang, account);
      players.push({ page, id: account.id });
    }
    await exercise(players, lang);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
}

async function formFields(page, route, selector = 'main form[method=post]') {
  expect((await page.goto(route)).status(), route).toBe(200);
  const form = page.locator(selector).first();
  await expect(form).toBeVisible();
  return form.evaluate(node => Object.fromEntries(new FormData(node)));
}

async function post(page, route, form, source) {
  return page.request.post(route, {
    form, headers: {
      origin: new URL(page.url()).origin, accept: 'text/html',
      ...(source && { 'x-e2e-abuse-source': source }),
    }, maxRedirects: 0,
  });
}

function redirect(response, status, location) {
  expect(response.status(), new URL(response.url()).pathname).toBe(status);
  expect(response.headers().location).toMatch(location);
  return response.headers().location;
}

async function denied(response, lang, { message = sharedMessage[lang], retry } = {}) {
  expect(response.status()).toBe(429);
  const html = await response.text();
  expect(html).toContain(message);
  const header = response.headers()['retry-after'];
  if (retry === 'minute-bucket') {
    expect(header).toMatch(/^[1-9][0-9]?$/);
    expect(Number(header)).toBeLessThanOrEqual(60);
  } else {
    expect(header).toBe(retry);
  }
}

async function partyQuotaDenied(response) {
  expect(response.status()).toBe(429);
  expect(await response.text()).toBe('');
  expect(response.headers()['retry-after']).toBeUndefined();
}

async function startMinute() {
  // Leave enough of the production window to exercise the exact boundary.
  await expect.poll(async () => Number((await query(
    'SELECT extract(second FROM clock_timestamp()) AS seconds',
  ))[0].seconds), { timeout: 65_000, intervals: [500] }).toBeLessThan(10);
}

function bucketHash(policy, account, source) {
  const digest = createHash('sha256');
  for (const part of [policy, account, source]) {
    const length = Buffer.alloc(8);
    length.writeBigUInt64BE(BigInt(Buffer.byteLength(part)));
    digest.update(length).update(part);
  }
  return digest.digest();
}

async function expireBucket(policy, account, budget) {
  const rows = await query(
    `UPDATE web_rate_limits SET window_start = window_start - interval '1 minute'
     WHERE limiter_name=$1 AND key_hash=$2 AND window_start=date_trunc('minute', clock_timestamp())
     RETURNING count::int AS count`, [policy, bucketHash(policy, account, '127.0.0.1')],
  );
  expect(rows.map(row => row.count)).toEqual([budget + 1]);
}

async function verifySecondaryBucket(policy, account) {
  expect(await query(
    `SELECT count::int AS count FROM web_rate_limits
     WHERE limiter_name=$1 AND key_hash=$2 AND window_start=date_trunc('minute', clock_timestamp())`,
    [policy, bucketHash(policy, account, '127.0.0.2')],
  )).toEqual([{ count: 1 }]);
}

const createTicket = async (player, lang, suffix) => {
  const route = `/${lang}/support/tickets`;
  const fields = await formFields(player.page, `${route}/create`);
  return post(player.page, route, { ...fields, subject: `Browser ticket ${suffix}`, body: 'Local abuse qualification message.' });
};
const ticketLocation = lang => new RegExp(`^/${lang}/support/tickets/${uuidPath}$`);

test('comment-only profile updates accept thirty, refuse thirty-one and recover', async ({ browser }, info) => {
  await withPlayers(browser, info, async ([player], lang) => {
    await partyCharacters([player]);
    const route = `/${lang}/account/characters/${encodeURIComponent('Aldric Vale')}/profile`;
    const form = await formFields(player.page, route);
    const save = index => post(player.page, route, { ...form, comment: `Profile comment ${index}` });
    const saved = () => query(
      'SELECT comment FROM character_profiles WHERE account_id=$1 AND character_id=$2',
      [player.id, player.character],
    );
    for (let i = 1; i <= 30; i++) redirect(await save(i), 303, route);
    expect(await saved()).toEqual([{ comment: 'Profile comment 30' }]);
    await denied(await save(31), lang);
    expect(await saved()).toEqual([{ comment: 'Profile comment 30' }]);
    expect(await query(
      `UPDATE character_profile_events SET created_at=clock_timestamp()-interval '1 minute 1 second'
       WHERE account_id=$1 AND event_type='character.profile_preferences_updated' RETURNING account_id`, [player.id],
    )).toHaveLength(30);
    redirect(await save(31), 303, route);
    expect(await saved()).toEqual([{ comment: 'Profile comment 31' }]);
  });
});

test('ticket creation allows six, isolates a second account and resets', async ({ browser }, info) => {
  await withPlayers(browser, info, async ([a, b], lang) => {
    const route = `/${lang}/support/tickets`;
    const forms = new Map();
    for (const player of [a, b]) forms.set(player.id, await formFields(player.page, `${route}/create`));
    const keys = await query('SELECT uuidv7() AS key FROM generate_series(1,13)');
    const create = (player, suffix, source) => post(player.page, route, {
      ...forms.get(player.id), request_key: keys[suffix - 1].key,
      subject: `Browser ticket ${suffix}`, body: 'Local abuse qualification message.',
    }, source);
    await startMinute();
    for (let i = 1; i <= 6; i++) redirect(await create(a, i), 302, ticketLocation(lang));
    await denied(await create(a, 7), lang, { retry: 'minute-bucket' });
    redirect(await create(a, 13, 'secondary'), 302, ticketLocation(lang));
    await verifySecondaryBucket('support-ticket-create', a.id);
    redirect(await create(b, 12), 302, ticketLocation(lang));
    await denied(await create(a, 7), lang, { retry: 'minute-bucket' });
    await expireBucket('support-ticket-create', a.id, 6);
    const seventh = redirect(await create(a, 7), 302, ticketLocation(lang));
    for (let i = 8; i <= 9; i++) redirect(await create(a, i), 302, ticketLocation(lang));

    const atCapacity = await create(a, 11);
    redirect(atCapacity, 302, `${route}/create`);
    expect(atCapacity.headers()['retry-after']).toBeUndefined();
    expect((await a.page.goto(`${route}/create`)).status()).toBe(200);
    await expect(a.page.locator('main [role=alert]')).toHaveText(lang === 'pl'
      ? 'Nie udało się wykonać żądania. Odśwież zgłoszenie i spróbuj ponownie.'
      : 'The request could not be completed. Reload the ticket and try again.');
    const draft = a.page.locator('main form[method=post]').first();
    await expect(draft.locator('[name=subject]')).toHaveValue('Browser ticket 11');
    await expect(draft.locator('[name=body]')).toHaveValue('Local abuse qualification message.');
    await expect(draft.locator('[name=request_key]')).toHaveValue(keys[10].key);
    const restored = await draft.evaluate(node => Object.fromEntries(new FormData(node)));
    redirect(await create(b, 11), 302, ticketLocation(lang));
    const close = await formFields(a.page, seventh, 'main form[action$="/status"]');
    redirect(await post(a.page, `${seventh}/status`, { ...close, status: 'closed' }), 302, seventh);
    redirect(await post(a.page, route, restored), 302, ticketLocation(lang));
    const counts = await query('SELECT account_id, count(*)::int AS count FROM tickets WHERE account_id=ANY($1) GROUP BY account_id', [[a.id, b.id]]);
    expect(Object.fromEntries(counts.map(row => [row.account_id, row.count]))).toEqual({ [a.id]: 11, [b.id]: 2 });
  });
});

test('ticket replies allow twelve, isolate a second account and reset', async ({ browser }, info) => {
  await withPlayers(browser, info, async ([a, b], lang) => {
    const route = `/${lang}/support/tickets`;
    const existing = await query(
      `WITH inserted AS (
         INSERT INTO tickets(account_id,request_key,category,subject,last_message_at)
         SELECT $1,uuidv7(),'other','Existing ticket ' || n,clock_timestamp()
         FROM generate_series(1,10) AS n RETURNING id
       )
       INSERT INTO ticket_messages(ticket_id,author_account_id,author_kind,internal,body)
       SELECT id,$1,'user',false,'Existing ticket message.' FROM inserted RETURNING ticket_id`, [a.id],
    );
    const selector = 'main form[action$="/reply"]';
    const prepared = [];
    for (const { ticket_id: id } of existing) {
      const ticket = `${route}/${id}`;
      prepared.push({ id, ticket, fields: await formFields(a.page, ticket, selector) });
    }
    const bTicket = redirect(await createTicket(b, lang, 'B'), 302, ticketLocation(lang));
    const bFields = await formFields(b.page, bTicket, selector);
    const reply = (player, ticket, fields, index, source) =>
      post(player.page, `${ticket}/reply`, { ...fields, body: `Local browser reply ${index}.` }, source);
    const changeStatus = (ticket, fields, status, offset) =>
      post(a.page, `${ticket}/status`, {
        _csrf: fields._csrf, lock_version: String(Number(fields.lock_version) + offset), status,
      });

    // Reuse four tickets after closing and reopening them to stay within the
    // ten-open-ticket cap. Each successful mutation advances the lock version.
    const reopened = prepared.slice(0, 4).map(({ ticket, fields }) => ({
      ticket, fields: { ...fields, lock_version: String(Number(fields.lock_version) + 3) },
    }));
    const [eleventh, twelfth, refused, secondary] = reopened;

    await startMinute();
    for (const [i, { ticket, fields }] of prepared.entries()) {
      redirect(await reply(a, ticket, fields, i + 1), 302, ticket);
    }
    for (const { ticket, fields } of prepared.slice(0, 4)) {
      redirect(await changeStatus(ticket, fields, 'closed', 1), 302, ticket);
      redirect(await changeStatus(ticket, fields, 'open', 2), 302, ticket);
    }
    redirect(await reply(a, eleventh.ticket, eleventh.fields, 11), 302, eleventh.ticket);
    redirect(await reply(a, twelfth.ticket, twelfth.fields, 12), 302, twelfth.ticket);
    await denied(await reply(a, refused.ticket, refused.fields, 13), lang, { retry: 'minute-bucket' });
    redirect(await reply(a, secondary.ticket, secondary.fields, 14, 'secondary'), 302, secondary.ticket);
    redirect(await reply(b, bTicket, bFields, 1), 302, bTicket);
    await denied(await reply(a, refused.ticket, refused.fields, 13), lang, { retry: 'minute-bucket' });
    await expireBucket('support-ticket-reply', a.id, 12);
    redirect(await reply(a, refused.ticket, refused.fields, 13), 302, refused.ticket);
    await verifySecondaryBucket('support-ticket-reply', a.id);

    const bId = bTicket.split('/').at(-1);
    const counts = await query(
      'SELECT ticket_id, count(*)::int AS count FROM ticket_messages WHERE ticket_id=ANY($1) GROUP BY ticket_id',
      [[...prepared.map(({ id }) => id), bId]],
    );
    expect(Object.fromEntries(counts.map(row => [row.ticket_id, row.count]))).toEqual({
      ...Object.fromEntries(prepared.map(({ id }, i) => [id, i < 4 ? 3 : 2])),
      [bId]: 2,
    });
  });
});

test('ticket status changes allow twelve per account and source and reset', async ({ browser }, info) => {
  await withPlayers(browser, info, async ([a, b], lang) => {
    const aTicket = redirect(await createTicket(a, lang, 'status A'), 302, ticketLocation(lang));
    const bTicket = redirect(await createTicket(b, lang, 'status B'), 302, ticketLocation(lang));
    const selector = 'main form[action$="/status"]';
    const aFields = await formFields(a.page, aTicket, selector);
    const bFields = await formFields(b.page, bTicket, selector);
    const change = (player, ticket, fields, version, status, source) =>
      post(player.page, `${ticket}/status`, { ...fields, lock_version: String(version), status }, source);

    const initial = Number(aFields.lock_version);
    expect(initial).toBe(1);
    await startMinute();
    for (let i = 0; i < 12; i++) {
      redirect(await change(a, aTicket, aFields, initial + i, i % 2 ? 'open' : 'closed'), 302, aTicket);
    }
    const version = initial + 12;
    await denied(await change(a, aTicket, aFields, version, 'closed'), lang, { retry: 'minute-bucket' });
    redirect(await change(a, aTicket, aFields, version, 'closed', 'secondary'), 302, aTicket);
    await verifySecondaryBucket('support-ticket-status', a.id);
    redirect(await change(b, bTicket, bFields, Number(bFields.lock_version), 'closed'), 302, bTicket);
    await denied(await change(a, aTicket, aFields, version + 1, 'open'), lang, { retry: 'minute-bucket' });
    await expireBucket('support-ticket-status', a.id, 12);
    redirect(await change(a, aTicket, aFields, version + 1, 'open'), 302, aTicket);
    expect(await query(
      'SELECT status, lock_version::int FROM tickets WHERE id=$1', [aTicket.split('/').at(-1)],
    )).toEqual([{ status: 'open', lock_version: version + 2 }]);
  });
});

test('player report submissions allow three, isolate a second account and reset', async ({ browser }, info) => {
  await withPlayers(browser, info, async ([a, b], lang) => {
    const route = `/${lang}/support/reports`;
    const location = new RegExp(`^${route}/${uuidPath}$`);
    const submit = async (player, index) => {
      const fields = await formFields(player.page, `${route}/create`);
      return post(player.page, route, {
        ...fields, target_reference: `Local report target ${index}`, evidence_summary: 'Synthetic local evidence.',
      });
    };
    for (let i = 1; i <= 3; i++) redirect(await submit(a, i), 302, location);
    await denied(await submit(a, 4), lang, { message: reportMessage[lang], retry: '60' });
    redirect(await submit(b, 4), 302, location);
    await denied(await submit(a, 4), lang, { message: reportMessage[lang], retry: '60' });
    expect(await query(
      `UPDATE player_report_submission_limits SET window_started_at=clock_timestamp()-interval '61 seconds'
       WHERE account_id=$1 RETURNING attempts`, [a.id],
    )).toEqual([{ attempts: 3 }]);
    redirect(await submit(a, 4), 302, location);
    expect(await query('SELECT attempts FROM player_report_submission_limits WHERE account_id=$1', [a.id])).toEqual([{ attempts: 1 }]);
    expect(await query('SELECT count(*)::int AS count FROM player_reports WHERE reporter_account_id=$1', [a.id])).toEqual([{ count: 4 }]);
    redirect(await submit(a, 5), 302, location);
    const pending = await submit(a, 6);
    redirect(pending, 302, `${route}/create`);
    expect(pending.headers()['retry-after']).toBeUndefined();
    expect((await a.page.goto(`${route}/create`)).status()).toBe(200);
    await expect(a.page.locator('main [role=alert]')).toHaveText(lang === 'pl'
      ? 'Osiągnięto limit oczekujących zgłoszeń. Poczekaj na rozpatrzenie wcześniejszych zgłoszeń.'
      : 'Your pending report limit has been reached. Wait for the existing reports to be reviewed.');
    redirect(await submit(b, 6), 302, location);
    expect((await query(
      `UPDATE player_reports SET status='closed', public_outcome='Closed for qualification'
       WHERE reporter_account_id=$1 AND target_reference='Local report target 1'
       RETURNING id`, [a.id],
    )).length).toBe(1);
    expect(await query(
      `UPDATE player_report_submission_limits SET window_started_at=clock_timestamp()-interval '61 seconds'
       WHERE account_id=$1 RETURNING attempts`, [a.id],
    )).toEqual([{ attempts: 3 }]);
    redirect(await submit(a, 6), 302, location);
    expect(await query(
      "SELECT count(*)::int AS count FROM player_reports WHERE reporter_account_id=$1 AND status='submitted'", [a.id],
    )).toEqual([{ count: 5 }]);
  });
});

test('new email invitations suppress a 26th enqueue, isolate an inviter and reset', async ({ browser }, info) => {
  await withPlayers(browser, info, async ([a, b], lang) => {
    await query('UPDATE referral_campaign_settings SET enabled=true WHERE singleton');
    const route = `/${lang}/account/referrals`;
    const selector = 'form:has(input[name=kind][value=new])';
    const fields = new Map();
    for (const player of [a, b]) fields.set(player.id, await formFields(player.page, route, selector));
    const recipient = index => `invite-${a.id}-${index}@example.invalid`;
    const invite = async (player, index) => {
      redirect(await post(player.page, route, { ...fields.get(player.id), email: recipient(index) }), 303, route);
    };
    const count = async player => (await query(
      'SELECT count(*)::int AS count FROM referral_email_attempts WHERE inviter_id=$1', [player.id],
    ))[0].count;
    await invite(a, 1);
    await a.page.goto(route);
    await expect(a.page.locator('main')).toContainText(inviteMessage[lang]);
    for (let i = 2; i <= 25; i++) await invite(a, i);
    expect(await count(a)).toBe(25);
    await invite(a, 26);
    await a.page.goto(route);
    await expect(a.page.locator('main')).toContainText(inviteMessage[lang]);
    expect(await count(a)).toBe(25);
    await invite(b, 26);
    expect(await count(b)).toBe(1);
    await invite(a, 26);
    expect(await count(a)).toBe(25);
    expect((await query(
      `UPDATE referral_email_attempts SET created_at=clock_timestamp()-interval '24 hours 1 second'
       WHERE inviter_id=$1 RETURNING id`, [a.id],
    )).length).toBe(25);
    await invite(a, 26);
    expect(await count(a)).toBe(26);

    await invite(a, 1);
    expect(await count(a)).toBe(26);
    expect((await query(
      `UPDATE referral_email_invitations SET created_at=clock_timestamp()-interval '14 days 1 second'
       WHERE inviter_id=$1 AND recipient=$2 RETURNING id`, [a.id, recipient(1)],
    )).length).toBe(1);
    await invite(a, 1);
    expect(await count(a)).toBe(27);
  });
});

test('Bazaar bid admission allows twenty requests, isolates a bidder and resets', async ({ browser }, info) => {
  await withPlayers(browser, info, async ([a, b], lang) => {
    const [fixture] = await query('SELECT uuidv7() AS listing, uuidv7() AS character');
    const route = `/${lang}/account/bazaar/${fixture.listing}/bids`;
    for (const player of [a, b]) {
      const fields = await formFields(player.page, `/${lang}/support/tickets/create`);
      player.csrf = fields._csrf;
    }
    const bid = async (player, source) => {
      const [{ key }] = await query('SELECT uuidv7() AS key');
      return post(player.page, route, { _csrf: player.csrf, request_id: key, buyer_character_id: fixture.character, maximum: '100' }, source);
    };
    await csrfRejected(a.page, route, { request_id: fixture.listing, buyer_character_id: fixture.character, maximum: '100' });
    await startMinute();
    // A syntactically valid bid reaches the real handler and finds no listing.
    // No Game publication or funded auction is necessary for this admission test.
    for (let i = 0; i < 20; i++) expect((await bid(a)).status()).toBe(404);
    await denied(await bid(a), lang, { retry: 'minute-bucket' });
    expect((await bid(a, 'secondary')).status()).toBe(404);
    await verifySecondaryBucket('marketplace-bid', a.id);
    expect((await bid(b)).status()).toBe(404);
    await denied(await bid(a), lang, { retry: 'minute-bucket' });
    await expireBucket('marketplace-bid', a.id, 20);
    expect((await bid(a)).status()).toBe(404);
  });
});

async function partyCharacters(players) {
  await query("INSERT INTO world_status(id,name) VALUES($1,'Abuse Browser World') ON CONFLICT(id) DO NOTHING", [world]);
  await query('UPDATE public_projection_state SET highest_epoch=1 WHERE id=1');
  for (const player of players) {
    const [{ id: character }] = await query('SELECT uuidv7() AS id');
    const name = 'Aldric Vale';
    player.character = character;
    // Same persisted ownership and public-profile inputs as the party crate's
    // production-authority test. The portal resolves and locks both proofs.
    await query(`INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,
      projection_revision,source_observed_at,content_digest)
      VALUES($1,'oteryn-game-e2e','1','1',0,decode(repeat('00',32),'hex'))`, [player.id]);
    await query(`INSERT INTO account_characters(account_id,character_id,world_id,name,availability)
      VALUES($1,$2,$3,$4,'AVAILABLE')`, [player.id, character, world, name]);
    await query(`INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,
      from_projection_revision,to_projection_revision,projection_epoch,recovery_generation)
      VALUES($1,$2,$3,1,1,1,1,1)`, [character, player.id, world]);
    await query(`INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload)
      VALUES('character_profile',$1,1,1,0,$2::jsonb)`,
    [character, JSON.stringify({ profile: [{ world_id: world, name, level: '120', vocation: 2 }] })]);
  }
  await query(`UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',
    source_authority='oteryn-game-e2e',complete_through=extract(epoch FROM statement_timestamp())::bigint,
    observed_at=extract(epoch FROM statement_timestamp())::bigint WHERE id=1`);
  await query(`INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload)
    VALUES('character_profile','',true,1,1,extract(epoch FROM statement_timestamp())::bigint,'{}')
    ON CONFLICT(family,subject,is_watermark) DO UPDATE SET watermark=EXCLUDED.watermark`);
}

test('party caps five active and ten hourly posts, isolates an owner and resets', async ({ browser }, info) => {
  await withPlayers(browser, info, async ([a, b], lang) => {
    const route = `/${lang}/party`;
    for (const player of [a, b]) {
      const fields = await formFields(player.page, `/${lang}/support/tickets/create`);
      player.csrf = fields._csrf;
    }
    await partyCharacters([a, b]);
    const create = player => {
      const now = Math.floor(Date.now() / 1000);
      return post(player.page, route, {
        _csrf: player.csrf, character: player.character, purpose: 'hunt', min_level: '1',
        max_level: '200', profession: 'any', note: 'Local party limit qualification',
        starts_at: String(now + 60), expires_at: String(now + 3600),
      });
    };
    const active = player => query("SELECT id FROM pd_party_posts WHERE account_id=$1 AND status='active' ORDER BY id", [player.id]);
    const remove = async id => redirect(await post(a.page, `${route}/${id}/remove`, { _csrf: a.csrf }), 303, route);

    for (let i = 0; i < 5; i++) redirect(await create(a), 303, route);
    expect(await active(a)).toHaveLength(5);
    await partyQuotaDenied(await create(a));
    redirect(await create(b), 303, route);
    await partyQuotaDenied(await create(a));

    for (const { id } of await active(a)) await remove(id);
    for (let i = 0; i < 5; i++) {
      redirect(await create(a), 303, route);
      const [{ id }] = await active(a);
      await remove(id);
    }
    expect(await active(a)).toEqual([]);
    await partyQuotaDenied(await create(a));
    redirect(await create(b), 303, route);
    await partyQuotaDenied(await create(a));
    expect((await query(
      `UPDATE pd_party_posts SET created_at=clock_timestamp()-interval '1 hour 1 second'
       WHERE account_id=$1 AND status='removed' RETURNING id`, [a.id],
    )).length).toBe(10);
    redirect(await create(a), 303, route);
    expect(await active(a)).toHaveLength(1);
    expect(await active(b)).toHaveLength(2);
  });
});

test('party reports accept the twentieth and refuse the twenty-first under each cap', async ({ browser }, info) => {
  await withPlayers(browser, info, async ([a, b], lang) => {
    const route = `/${lang}/party`;
    const fields = await formFields(a.page, `/${lang}/support/tickets/create`);
    // These reports count toward the hourly cap only.
    const reports = await query(
      `WITH posts AS (
         INSERT INTO pd_party_posts(account_id,character_id,character_name,world_id,purpose,
           min_level,max_level,profession,note,starts_at,expires_at,status)
         SELECT $1,uuidv7(),'Party Owner',$2,'hunt',1,200,'any','Prior party post',
           now()-interval '45 minutes',now()-interval '5 minutes','expired'
         FROM generate_series(1,19) RETURNING id
       )
       INSERT INTO pd_party_reports(post_id,account_id,reason,status,created_at)
       SELECT id,$3,'Prior report','resolved',now()-interval '10 minutes' FROM posts RETURNING id`,
      [b.id, world, a.id],
    );
    expect(reports).toHaveLength(19);
    const targets = await query(
      `INSERT INTO pd_party_posts(account_id,character_id,character_name,world_id,purpose,
         min_level,max_level,profession,note,starts_at,expires_at)
       SELECT $1,uuidv7(),'Party Owner',$2,'hunt',1,200,'any','Current party post',
         now(),now()+interval '30 minutes' FROM generate_series(1,4) RETURNING id`, [b.id, world],
    );
    const report = ({ id }) => post(a.page, `${route}/${id}/report`, {
      _csrf: fields._csrf, reason: 'Browser qualification report',
    });
    const saved = ({ id }) => query(
      'SELECT status FROM pd_party_reports WHERE account_id=$1 AND post_id=$2', [a.id, id],
    );

    redirect(await report(targets[0]), 303, route);
    expect(await saved(targets[0])).toEqual([{ status: 'open' }]);
    await partyQuotaDenied(await report(targets[1]));
    expect(await saved(targets[1])).toEqual([]);

    // Age all twenty reports out of the hourly window, then make only the
    // nineteen seeded reports pending to test the second cap independently.
    expect(await query(
      `UPDATE pd_party_reports
       SET created_at=now()-interval '1 hour 1 second',
           status=CASE WHEN id=ANY($2::uuid[]) THEN 'open' ELSE 'resolved' END
       WHERE account_id=$1 RETURNING id`, [a.id, reports.map(row => row.id)],
    )).toHaveLength(20);
    redirect(await report(targets[2]), 303, route);
    expect(await saved(targets[2])).toEqual([{ status: 'open' }]);
    await partyQuotaDenied(await report(targets[3]));
    expect(await saved(targets[3])).toEqual([]);

    expect(await query(
      "UPDATE pd_party_reports SET status='resolved' WHERE account_id=$1 AND status='open' RETURNING id",
      [a.id],
    )).toHaveLength(20);
    redirect(await report(targets[3]), 303, route);
    expect(await saved(targets[3])).toEqual([{ status: 'open' }]);
  });
});

test('returning invitations suppress the 21st enqueue in 30 days and reset', async ({ browser }, info) => {
  await withPlayers(browser, info, async ([a, b], lang) => {
    await query('UPDATE referral_campaign_settings SET enabled=true WHERE singleton');
    await query("UPDATE portal_referral_coverage SET covered_since=clock_timestamp()-interval '13 months' WHERE id");
    const candidates = await query(
      `WITH targets AS (
         INSERT INTO accounts(email,password_hash,status,locale,email_verified_at,created_at)
         SELECT $1 || '-' || n || '@example.invalid', password_hash, 'active', locale,
                clock_timestamp()-interval '13 months', clock_timestamp()-interval '13 months'
         FROM accounts CROSS JOIN generate_series(1,22) AS n WHERE id=$2
         RETURNING id,email
       )
       SELECT id,email FROM targets ORDER BY email`, [`returning-${a.id}`, a.id],
    );
    // These inactive targets never log in. Seed the same historical activity
    // that the referral service's PostgreSQL tests use for an eligible buddy.
    await query(
      `WITH gate AS MATERIALIZED (SELECT set_config('oteryn.referrals_enabled','true',true))
       INSERT INTO referral_activity(account_id,created_at,last_activity)
       SELECT id,created_at,created_at FROM gate CROSS JOIN accounts WHERE id=ANY($1)
       ON CONFLICT(account_id) DO UPDATE SET created_at=EXCLUDED.created_at,last_activity=EXCLUDED.last_activity`,
      [candidates.map(target => target.id)],
    );
    const route = `/${lang}/account/referrals`;
    const selector = 'form:has(input[name=kind][value=returning])';
    const fields = new Map();
    for (const player of [a, b]) fields.set(player.id, await formFields(player.page, route, selector));
    const invite = async (player, index) => {
      const response = await post(player.page, route, { ...fields.get(player.id), email: candidates[index].email });
      redirect(response, 303, route);
      expect(response.headers()['retry-after']).toBeUndefined();
    };
    const count = async player => (await query(
      'SELECT count(*)::int AS count FROM referral_email_attempts WHERE inviter_id=$1', [player.id],
    ))[0].count;
    for (let i = 0; i < 20; i++) await invite(a, i);
    expect(await count(a)).toBe(20);
    await invite(a, 20);
    await a.page.goto(route);
    await expect(a.page.locator('main [role=status]')).toHaveText(inviteMessage[lang]);
    expect(await count(a)).toBe(20);
    await invite(b, 21);
    expect(await count(b)).toBe(1);
    await invite(a, 20);
    expect(await count(a)).toBe(20);
    expect((await query(
      `UPDATE referral_email_attempts SET created_at=clock_timestamp()-interval '30 days 1 second'
       WHERE inviter_id=$1 RETURNING id`, [a.id],
    )).length).toBe(20);
    await invite(a, 20);
    expect(await count(a)).toBe(21);
  });
});
