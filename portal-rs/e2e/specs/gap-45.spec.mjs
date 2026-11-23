import { test, expect } from '../fixtures.mjs';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { actor, uuid7 } from '../game-sim-helpers.mjs';
import { disposableDatabaseUrl, login, pool, register } from '../helpers.mjs';

test('hunt journal shows the signed-in player the unavailable state before capture is qualified', async ({ page }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const target = `/${lang}/account/hunts`;

  await page.goto(`/${lang}/account`);
  const journal = page.locator(`header a[href="${target}"]`).first();
  await expect(journal).toBeAttached();
  // The same navigation link is inside a collapsed menu on narrow screens.
  const response = await page.goto(await journal.getAttribute('href'));
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toContain('no-store');
  await expect(page.locator('main h1')).toHaveText(lang === 'pl'
    ? 'Analizator sesji polowania'
    : 'Hunt session analyzer');
  await expect(page.locator('main [role=status]')).toHaveText(lang === 'pl'
    ? 'Niedostępne'
    : 'Unavailable');
  await expect(page.locator('main article')).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth))
    .toBeLessThanOrEqual(info.project.use.viewport.width);

  const invalid = await page.request.get(`${target}/compare?ids=invalid`);
  expect(invalid.status()).toBe(400);
  const otherSession = '019a5000-0000-7000-8000-000000000003';
  const refused = await page.request.get(`${target}/${otherSession}/share`);
  expect(refused.status()).toBe(503);
  expect(refused.headers()['cache-control']).toContain('no-store');
});

// The shared browser launcher has no G-9 source. Route this spec's hunt requests
// through a loopback Portal configured for the same disposable database and
// authenticated sessions. The Rust PostgreSQL test exercises the ingest boundary.
test('owner reviews completed party hunts and reopens the private link', async ({ browser }, info) => {
  test.setTimeout(220_000);
  const owner = await actor(browser, info);
  const lang = info.project.metadata.locale;
  const base = new URL(owner.page.url()).origin;
  const root = `/${lang}/account/hunts`;
  const source = 'g9.browser.fixture';
  const children = [];
  const db = pool();
  let viewer;

  try {
    viewer = await actor(browser, info);
    const character = uuid7();
    const world = '0192a000-0000-7000-8000-000000000012';
    expect((await owner.page.request.post('/__bazaar_game/seed', {
      data: {
        seller: viewer.id, buyer: owner.id, kind: 'item', asset_id: uuid7(),
        seller_character_id: uuid7(), buyer_character_id: character,
      },
    })).status()).toBe(201);

    const { projection_epoch: epoch, projection_revision: cut } = (await db.query(
      'SELECT projection_epoch,projection_revision FROM account_character_snapshots WHERE account_id=$1',
      [owner.id],
    )).rows[0];
    await db.query(
      `INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,
        from_projection_revision,to_projection_revision,projection_epoch,recovery_generation,accepted_at)
       VALUES($1,$2,$3,9,$4,$4,$5,3,now()-interval '10 minutes')`,
      [character, owner.id, world, cut, epoch],
    );
    const generation = uuid7();
    await db.query(
      `INSERT INTO pd_g9_streams(source_authority,account_id,stream_generation,admitted,recovery_generation,
         highest_seen,stored_through,applied_through)
       VALUES($1,$2,$3,true,3,3,3,3),($1,$4,$5,true,3,0,0,0)`,
      [source, owner.id, generation, viewer.id, uuid7()],
    );

    const contract = readFileSync(new URL('../../../docs/contracts/OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md', import.meta.url), 'utf8');
    const golden = kind => JSON.parse(contract.split('\n').find(line =>
      line.startsWith('{') && line.includes(`"record_kind":"${kind}"`)));
    const end = Math.floor(Date.now() / 1000) - 5;
    const first = uuid7();
    const second = uuid7();
    const segment = uuid7();
    const party = {
      party_id: uuid7(),
      members: [{ member_slot: 0, is_subject: true, active_seconds: '60' },
        { member_slot: 1, is_subject: false, active_seconds: '60' }],
      loot: [{ member_slot: 0, item_key: 'item:ruby', quantity: '1' },
        { member_slot: 1, item_key: 'item:ruby', quantity: '3' }],
    };
    const save = async (kind, revision, changes) => {
      const envelope = golden(kind);
      Object.assign(envelope, { source_authority: source, account_id: owner.id,
        stream_generation: generation, source_revision: String(revision) });
      const record = Object.assign(envelope.record, { character_id: character, character_revision: '9' }, changes);
      const raw = Buffer.from(JSON.stringify(envelope));
      await db.query(
        `INSERT INTO pd_g9_records(source_authority,account_id,stream_generation,source_revision,record_kind,
           session_id,segment_id,business_id,segment_index,character_id,digest,canonical_bytes,record,resolved,expires_at)
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,true,$14)`,
        [source, owner.id, generation, revision, kind, record.session_id, record.segment_id,
          record.event_id ?? record.segment_id, record.segment_index ?? null, character,
          createHash('sha256').update(raw).digest(), raw, record, end + 86400],
      );
    };
    await save('HUNT_SEGMENT', 1, { session_id: first, segment_id: segment, world_id: world,
      started_at: String(end - 60), ended_at: String(end), party });
    await save('RARE_DROP', 2, { session_id: first, segment_id: segment, event_id: uuid7(),
      occurred_at: String(end - 30) });
    await save('HUNT_SEGMENT', 3, { session_id: second, segment_id: uuid7(), world_id: world,
      started_at: String(end - 180), ended_at: String(end - 120), rare_drop_count: 0 });

    const probe = createServer();
    await new Promise(resolve => probe.listen(0, '127.0.0.1', resolve));
    const { port } = probe.address();
    await new Promise((resolve, reject) => probe.close(error => error ? reject(error) : resolve()));
    const listen = `http://127.0.0.1:${port}`;
    const portal = spawn(process.env.PORTAL_E2E_BINARY, ['serve'], {
      env: {
        PATH: process.env.PATH,
        OTERYN_ENV: 'testing', OTERYN_BIND: `127.0.0.1:${port}`,
        OTERYN_PUBLIC_ORIGIN: base, OTERYN_DATABASE_URL: disposableDatabaseUrl(),
        OTERYN_STATIC_DIR: process.env.OTERYN_STATIC_DIR,
        PORTAL_NATIVE_ADMISSION_BIND: '127.0.0.1:0',
        PORTAL_PLAYERDATA_G9_SOURCE_AUTHORITY: source,
        PORTAL_PLAYERDATA_G9_PARTY_PRIVACY: 'accepted', OTERYN_PLAYERDATA_HUNTS_READ: 'true',
      },
      stdio: 'ignore',
    });
    children.push(portal);
    portal.on('error', () => {});
    await expect.poll(async () => {
      try { return (await fetch(`${listen}/healthz`, { signal: AbortSignal.timeout(500) })).status; }
      catch { return 0; }
    }, { timeout: 20_000, message: 'G-9 read fixture must be reachable' }).toBe(200);

    for (const screen of [owner.page, viewer.page]) {
      await screen.route(new RegExp(`/${lang}/account/hunts(?:[/?]|$)`), async route => {
        const url = new URL(route.request().url());
        const response = await route.fetch({
          url: `${listen}${url.pathname}${url.search}`,
          headers: await route.request().allHeaders(), maxRedirects: 0,
        });
        await route.fulfill({ response });
      });
    }
    expect((await owner.page.goto(root)).status()).toBe(200);
    await owner.page.locator('#first').selectOption(first);
    await owner.page.locator('#second').selectOption(second);
    await owner.page.locator('form[action$="/compare"] button').click();
    await expect(owner.page.locator('main article')).toHaveCount(2);

    expect((await owner.page.goto(`${root}/${first}`)).status()).toBe(200);
    await expect(owner.page.locator('main dd').filter({ hasText: /^6000$/ })).toHaveCount(1);
    await expect(owner.page.locator('main li').filter({ hasText: 'item:ruby' })).toHaveCount(1);
    await owner.page.locator('#prices').fill('loot item:ruby=100');
    await owner.page.locator('form:has(#prices) button').click();
    await expect(owner.page.locator('main dd').filter({ hasText: /^1 → 0: 100$/ })).toBeVisible();
    expect(await owner.page.evaluate(() => document.documentElement.scrollWidth))
      .toBeLessThanOrEqual(info.project.use.viewport.width);
    await owner.page.locator('main a[href$="/share"]').click();
    await expect(owner.page).toHaveURL(`${base}${root}/${first}`);
    const refused = await viewer.page.goto(`${root}/${first}/share`);
    expect(refused.status()).toBe(404);
    expect(refused.headers()['cache-control']).toContain('no-store');
  } finally {
    await Promise.all(children.map(child => {
      if (!child.pid || child.exitCode !== null || child.signalCode) return;
      return new Promise(resolve => { child.once('exit', resolve); child.kill('SIGTERM'); });
    }));
    await db.end();
    await owner.context.close();
    await viewer?.context.close();
  }
});
