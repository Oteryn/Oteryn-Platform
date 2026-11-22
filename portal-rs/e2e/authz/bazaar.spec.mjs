import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';
import { actorDefinitions, csrfPage, openActor, sessionHash } from './actors.mjs';
import { assertRefusalState, database, rateKey } from './database.mjs';

const { routes, roles } = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));
const actors = actorDefinitions(roles);
const sell = '/{locale}/account/bazaar/sell';
const bid = '/{locale}/account/bazaar/{listing}/bids';
const uuid = async () => (await query('SELECT uuidv7() AS id'))[0].id;

function localProcesses() {
  if (process.env.OTERYN_ENV !== 'testing') throw new Error('Bazaar authorization requires the local harness');
  const pid = Number(process.env.PORTAL_E2E_AUTHZ_BAZAAR_LAUNCHER_PID);
  if (!Number.isSafeInteger(pid) || pid <= 1) throw new Error('Provide the disposable local.mjs launcher PID');
  const command = readFileSync(`/proc/${pid}/cmdline`, 'utf8').split('\0');
  if (!command.some(arg => /(?:^|\/)local\.mjs$/.test(arg))) throw new Error('Expected the local.mjs launcher');
  const children = execFileSync('ps', ['--ppid', String(pid), '-o', 'pid=,args='], { encoding: 'utf8' });
  const worker = /^\s*(\d+)\s+\S*\boteryn-portal worker\s*$/m.exec(children);
  if (!worker) throw new Error('Expected the disposable launcher worker');
  const workerPid = Number(worker[1]);
  const environment = readFileSync(`/proc/${workerPid}/environ`, 'utf8').split('\0');
  if (!process.env.OTERYN_DATABASE_URL
      || !environment.includes(`OTERYN_DATABASE_URL=${process.env.OTERYN_DATABASE_URL}`)) {
    throw new Error('The launcher worker must belong to this disposable test database');
  }
  return [pid, workerPid];
}

async function paused(producers, run) {
  const stopped = [];
  try {
    for (const pid of producers) {
      process.kill(pid, 'SIGSTOP');
      stopped.push(pid);
      await expect.poll(() => readFileSync(`/proc/${pid}/status`, 'utf8')).toMatch(/^State:\s+T\b/m);
    }
    return await run();
  } finally {
    for (const pid of stopped.reverse()) process.kill(pid, 'SIGCONT');
  }
}

async function stagedAuction(request, seller, buyer) {
  const [{ id: world }] = await query('SELECT id FROM world_status ORDER BY id LIMIT 1');
  const [{ asset, character, destination, operation }] = await query(
    'SELECT uuidv7() AS asset,uuidv7() AS character,uuidv7() AS destination,uuidv7() AS operation');
  const seeded = await request.post('/__bazaar_game/seed', {
    data: { seller, buyer, kind: 'item', asset_id: asset, seller_character_id: character, buyer_character_id: destination },
    maxRedirects: 0,
  });
  try {
    expect(seeded.status(), 'local Game producer seeded custody and the destination').toBe(201);
  } finally {
    await seeded.dispose();
  }
  return { destination, form: {
    request_id: operation, kind: 'item', world_id: world, seller_character_id: character, asset_id: asset,
    expected_revision: '7', quantity: '1', definition_key: 'fixture-sword', definition_revision: 'v1',
    currency: 'COINS', starting_bid: '10000', duration_days: '1', publish_snapshot: 'true',
  } };
}

for (const actor of actors) {
  test(`Bazaar sell and bid authorization ${actor.key}`, async ({}, { project }) => {
    test.setTimeout(180_000);
    const locale = project.metadata.locale;
    const origin = new URL(process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443');
    const native = new URL(process.env.PORTAL_E2E_BROWSER_NATIVE_ORIGIN || 'https://localhost:18444');
    expect(origin.protocol === 'https:' && origin.hostname === 'localhost').toBe(true);
    expect(native.protocol === 'https:' && native.hostname === origin.hostname && native.port !== origin.port).toBe(true);
    const producers = localProcesses();
    const escrow = (await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'"))[0].id;
    const seller = await openActor(locale, actors.find(row => row.key === 'platform_admin'));
    const db = database();
    let buyer;
    try {
      buyer = await openActor(locale, actor, escrow);
      const token = await csrfPage(buyer.request, buyer.accountId ? `/${locale}/` : `/${locale}/login`);
      const hash = await sessionHash(buyer.request);
      const sudo = await seller.request.post(`/${locale}/account/sudo`, {
        form: { current_password: seller.account.password },
        headers: { origin: origin.origin, 'x-csrf-token': await csrfPage(seller.request, `/${locale}/`) },
        maxRedirects: 0,
      });
      try {
        expect(sudo.status()).toBe(302);
        expect(sudo.headers().location).toBe(`/${locale}/account/security`);
      } finally {
        await sudo.dispose();
      }
      const setup = await stagedAuction(seller.request, seller.accountId, buyer.accountId ?? escrow);
      const sellPath = sell.replace('{locale}', locale);
      const created = await seller.request.post(sellPath, {
        form: setup.form, headers: { origin: origin.origin, 'x-csrf-token': await csrfPage(seller.request, `/${locale}/`) },
        maxRedirects: 0,
      });
      let listing;
      try {
        expect(created.status(), 'MFA and sudo permit listing').toBe(303);
        listing = created.headers().location?.split('/').at(-1);
        expect(Boolean(listing)).toBe(true);
      } finally {
        await created.dispose();
      }
      await expect.poll(async () => (await query('SELECT state FROM bazaar_listings WHERE id=$1', [listing]))[0]?.state,
        { timeout: 45_000 }).toBe('active');

      const rejectedSell = await stagedAuction(seller.request, buyer.accountId ?? escrow, seller.accountId);
      async function verify(method, pattern, path, form, policy, allowed) {
        const row = routes.find(r => r.listener === 'public' && r.method === method && r.path === pattern);
        expect(row.registrations[0].guard.kind).toBe('logged-in');
        const send = async url => buyer.request.post(url, {
          form, headers: { origin: origin.origin, 'x-csrf-token': token }, maxRedirects: 0,
        });
        await test.step(`${method} ${pattern}`, async () => {
          if (allowed) {
            const response = await send(path);
            try {
              expect(response.status()).toBe(303);
              expect(response.headers().location).toBe(`/${locale}/bazaar/${listing}`);
              expect(await query('SELECT 1 FROM bazaar_bids WHERE listing_id=$1 AND account=$2',
                [listing, buyer.accountId])).toHaveLength(1);
            } finally {
              await response.dispose();
            }
            return;
          }
          const rate = buyer.accountId
            ? { policy, key: rateKey(policy, buyer.accountId, '127.0.0.1') } : undefined;
          await paused(producers, async () => {
            const before = await db.snapshot(hash, rate);
            const response = await send(new URL(path, native).href);
            try {
              expect(response.status()).toBe(403);
              assertRefusalState(before, await db.snapshot(hash, rate));
            } finally {
              await response.dispose();
            }
          });
        });
      }
      await verify('POST', sell, sellPath, rejectedSell.form, 'marketplace-listing', false);
      if (buyer.accountId) {
        const operation = await uuid();
        await query('INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING', [buyer.accountId]);
        await query(`
          INSERT INTO wallet_operations(id,kind,account_id,amount,reason) VALUES($1,'adjustment',$2,50000,'Local Bazaar matrix')
        `, [operation, buyer.accountId]);
        await query('INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES(uuidv7(),$1,$2,50000,0)',
          [operation, buyer.accountId]);
      }
      await verify('POST', bid, bid.replace('{locale}', locale).replace('{listing}', listing),
        { request_id: await uuid(), buyer_character_id: setup.destination, maximum: '15000' },
        'marketplace-bid', Boolean(buyer.accountId) && !actor.restricted);
    } finally {
      await buyer?.request.dispose();
      await db.close();
      await seller.request.dispose();
    }
  });
}
