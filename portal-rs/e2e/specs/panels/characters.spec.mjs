import { escapeRegex } from '../../regex.mjs';
import crypto from 'node:crypto';
import fs from 'node:fs';
import https from 'node:https';
import path from 'node:path';
import {test, expect} from '../../fixtures.mjs';
import {uuid7} from '../../game-sim-helpers.mjs';
import {csrfRejected, login, pool, submit} from '../../helpers.mjs';
import {otherBrowser, seedAccount} from '../../security/auth-fixtures.mjs';

const authority = process.env.OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY;
const stamp = () => String(Math.floor(Date.now() / 1000));
const transferFixture = new URL('../../../crates/characters/src/commands/fixtures/transfer-intent.json', import.meta.url);
const receiptFixture = new URL('../../../crates/characters/src/commands/fixtures/create-receipt.json', import.meta.url);
const outfitFixture = new URL('../../../../docs/contracts/fixtures/public-game-families-v2/valid-character-outfit.json', import.meta.url);

async function gamePost(route, payload, identity) {
  const origin = new URL(process.env.PORTAL_ACCOUNT_CLOSURE_BASE_URL);
  expect(origin.hostname).toBe('127.0.0.1');
  const directory = path.dirname(process.env.OTERYN_INTERNAL_TLS_CERT_FILE);
  const raw = JSON.stringify(payload);

  return new Promise((resolve, reject) => {
    const request = https.request(new URL(route, origin), {
      method: 'POST',
      ca: fs.readFileSync(process.env.OTERYN_INTERNAL_TLS_CLIENT_CA_FILE),
      cert: fs.readFileSync(path.join(directory, identity + '.pem')),
      key: fs.readFileSync(path.join(directory, identity + '-key.pem')),
      minVersion: 'TLSv1.3',
      maxVersion: 'TLSv1.3',
      ALPNProtocols: ['http/1.1'],
      headers: {'content-type': 'application/json', 'content-length': Buffer.byteLength(raw)},
      timeout: 5000,
    }, response => {
      let body = '';
      response.setEncoding('utf8');
      response.on('data', chunk => { body += chunk; });
      response.on('end', () => resolve({status: response.statusCode, body}));
    });
    request.on('error', reject);
    request.on('timeout', () => request.destroy(new Error('Local Game exchange timed out')));
    request.end(raw);
  });
}

async function publishCreation(operation, account, character, world, name) {
  const peer = 'panel-character-commands';
  const result = await gamePost('/internal/v1/game-auth/character-commands/read', {
    contract_version: 1, operation: 'ReadCharacterCommandIntentV1', source_authority: authority,
    issuer_authority: 'OTERYN_PLATFORM_CHARACTER_AUTHORITY', command: 'CreateCharacter',
    operation_id: operation,
  }, peer);
  expect(result.status).toBe(200);
  expect(JSON.parse(result.body)).toMatchObject({
    operation_id: operation, account_id: account, target_world_id: world, requested_name: name,
  });
  const receipt = JSON.parse(fs.readFileSync(receiptFixture, 'utf8'));
  Object.assign(receipt, {
    source_authority: authority, operation_id: operation, account_id: account, character_id: character,
    world_id: world, name, projection_epoch: '3', projection_revision: '1', decided_at: stamp(),
  });
  expect((await gamePost('/internal/v1/game-auth/character-commands/receipt', receipt, peer)).status).toBe(200);
}

async function publishPortfolio(account, character, world, name) {
  const time = stamp();
  expect((await gamePost('/internal/v1/game-auth/native-account-characters', {
    contract_version: 1, operation: 'PublishAccountCharactersV1', source_authority: authority,
    account_id: account, projection_epoch: '3', projection_revision: '1', source_observed_at: time,
    characters: [{character_id: character, world_id: world, name, availability: 'AVAILABLE'}],
  }, 'closure-portfolio')).status).toBe(200);
  await publishPortfolioWatermark();
}

async function publishPortfolioWatermark() {
  const time = stamp();
  expect((await gamePost('/internal/v1/game-auth/native-account-characters/watermark', {
    contract_version: 1, operation: 'PublishProjectionWatermarkV1', source_authority: authority,
    projection_epoch: '3', complete_through: time, observed_at: time,
  }, 'closure-portfolio')).status).toBe(200);
}

async function publishPublicReadModels(db, character, world, name) {
  const source = process.env.PORTAL_PUBLIC_PROJECTIONS_AUTHORITY;
  const {rows: [state]} = await db.query('SELECT highest_epoch::text FROM public_projection_state WHERE id=1');
  const epoch = state.highest_epoch ?? '1';
  const time = stamp();
  const publish = async (version, endpoint, operation, fields) => {
    const result = await gamePost(`/internal/v${version}/game-public/${endpoint}`, {
      contract_version: version, operation, source_authority: source, ...fields,
    }, 'panel-public-projections');
    expect(result.status, `Game publication accepted: ${operation}`).toBe(200);
  };
  const snapshot = {
    character_id: character, projection_epoch: epoch, projection_revision: '1', source_observed_at: time,
  };
  const watermark = {projection_epoch: epoch, complete_through: time, observed_at: time};
  await publish(1, 'character-profile', 'PublishCharacterProfileV1', {
    ...snapshot,
    profile: [{world_id: world, name, vocation: 4, level: '412', guild: [], guild_visible: false}],
  });
  await publish(1, 'watermark', 'PublishPublicProjectionWatermarkV1', {
    family: 'character_profile', ...watermark,
  });
  const {outfit} = JSON.parse(fs.readFileSync(outfitFixture, 'utf8'));
  Object.assign(outfit[0], {world_id: world, character_revision: '1'});
  await publish(2, 'character-outfit', 'PublishCharacterOutfitV2', {...snapshot, outfit});
  await publish(2, 'watermark', 'PublishPublicProjectionWatermarkV2', {
    family: 'character_outfit', ...watermark,
  });
}

async function expectLoginRedirect(page, target, locale) {
  const response = await page.request.get(target, {maxRedirects: 0});
  expect(response.status()).toBe(303);
  expect(response.headers().location).toMatch(new RegExp(`^/${escapeRegex(locale)}/login`));
}

async function signedInPlayer(browser, info) {
  const locale = info.project.metadata.locale;
  const account = await seedAccount(locale);
  const context = await otherBrowser(browser, info);
  try {
    const page = await context.newPage();
    await login(page, locale, account);
    return {page, context, id: account.id};
  } catch (error) {
    await context.close();
    throw error;
  }
}

test('owner creates a character, sees Game results and reads their outfit', async ({browser, page}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const owner = await signedInPlayer(browser, info);
  const db = pool();
  let other;
  try {
    other = await signedInPlayer(browser, info);
    const create = `/${lang}/account/characters/new`;
    await expectLoginRedirect(page, create, lang);
    expect((await owner.page.goto('/account')).status()).toBe(200);
    await owner.page.locator(`#characters ~ a[href="${create}"]`).click();
    const form = owner.page.locator('main form[method=post]');
    await expect(form).toBeVisible();
    const world = await form.locator('select[name=world] option').evaluateAll(options => options.find(option => option.value)?.value);
    expect(world).toBeTruthy();
    await form.locator('select[name=world]').selectOption(world);
    await form.locator('input[name=sex]').first().check();
    await form.locator('input[name=vocation]').first().check();

    await submit(owner.page, 'main form[method=post]', {name: '123'});
    await expect(owner.page.locator('[role=status]')).toContainText(lang === 'en' ? 'Check the name rules' : 'Sprawdź reguły imion');
    const suffix = Array.from(crypto.randomBytes(9), byte => String.fromCharCode(97 + byte % 26)).join('');
    const name = 'Panel ' + suffix[0].toUpperCase() + suffix.slice(1);
    const csrf = await form.locator('[name=_csrf]').inputValue();
    await submit(owner.page, 'main form[method=post]', {name});
    await expect(owner.page).toHaveURL(/\/characters\/requests\/[a-f0-9-]+$/);
    const request = new URL(owner.page.url()).pathname;
    const operation = request.split('/').at(-1);
    await expect(owner.page.locator('[role=status]')).toContainText(lang === 'en' ? 'outcome is still unknown' : 'rezultat pozostaje nieznany');
    expect((await other.page.request.get(request)).status()).toBe(404);
    expect((await owner.page.request.get(`/${lang}/account/characters/requests/bad-id`)).status()).toBe(404);

    const character = uuid7();
    await publishCreation(operation, owner.id, character, world, name);
    await owner.page.reload();
    await expect(owner.page.locator('[role=status]')).toContainText(lang === 'en' ? 'Game confirmed' : 'Gra potwierdziła');
    await publishPortfolio(owner.id, character, world, name);
    await publishPublicReadModels(db, character, world, name);

    const overview = await owner.page.goto('/account');
    expect(overview.status()).toBe(200);
    expect(overview.headers()['cache-control']).toContain('no-store');
    const row = owner.page.locator('main li').filter({hasText: name});
    await expect(row).toContainText('412');
    const foreignOverview = await other.page.request.get('/account');
    expect(foreignOverview.status()).toBe(200);
    expect(await foreignOverview.text()).not.toContain(name);
    expect(await owner.page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await expect(owner.page.locator('main')).toContainText(lang === 'en' ? 'Ownership transfer' : 'Przeniesienie własności');
    await expect(owner.page.locator('main')).toContainText(/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC/);
    await expect(owner.page.locator('main form[method=post]')).toHaveCount(0);
    const outfit = `/${lang}/account/characters/${encodeURIComponent(name)}/outfit`;
    await row.locator(`[href="${outfit}"]`).click();
    await expect(owner.page.locator('main')).toContainText('outfit.citizen');
    expect((await other.page.request.get(outfit)).status()).toBe(404);
    await expectLoginRedirect(page, outfit, lang);
    expect((await owner.page.request.get(`/${lang}/account/characters/%22/outfit`)).status()).toBe(400);

    const services = `/${lang}/account/characters/${character}/services`;
    expect((await owner.page.request.get(services)).status()).toBe(404);
    await csrfRejected(owner.page, services, {action: 'rename'});
    expect((await owner.page.request.post(services, {
      form: {_csrf: csrf, action: 'rename'},
      headers: {origin: new URL(owner.page.url()).origin},
      maxRedirects: 0,
    })).status()).toBe(404);
    expect(await owner.page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    const profile = `/${lang}/account/characters/${encodeURIComponent(name)}/profile`;
    expect((await owner.page.goto(profile)).status()).toBe(200);
    expect((await other.page.request.get(profile)).status()).toBe(404);
    const profileForm = owner.page.locator('main form[method=post]');
    const profileCsrf = await profileForm.locator('[name=_csrf]').inputValue();
    const saved = 'Seller comment before transfer';
    await publishPortfolioWatermark();
    await submit(owner.page, 'main form[method=post]', {comment: saved});
    await expect(owner.page.locator('main [name=comment]')).toHaveValue(saved);

    // Seed the already-issued command while leaving its Game receipt pending.
    const transfer = JSON.parse(fs.readFileSync(transferFixture, 'utf8'));
    const transferId = uuid7();
    const decisionId = uuid7();
    const holdTransfer = async hold => {
      const response = await owner.page.request.post('/__bazaar_game/hold-transfer', {
        data: {operation_id: transferId, hold},
      });
      expect(response.status()).toBe(204);
    };
    await holdTransfer(true);
    try {
      const {rows: [{source_revision}]} = await db.query(
        `INSERT INTO character_commands(id,issuer_decision_id,request_key,command,account_id,target_account_id,binding_json,intent_json)
         VALUES($1,$2,$1::uuid::text,'TransferCharacterOwnership',$3,$4,$5,'') RETURNING source_revision::text`,
        [transferId, decisionId, owner.id, other.id, JSON.stringify({
          character_id: character, expected_from_account_id: owner.id, to_account_id: other.id,
        })],
      );
      const now = Number(stamp());
      await db.query(
        "UPDATE character_commands SET intent_json=$1 WHERE source_revision=$2 AND state='PENDING'",
        [JSON.stringify({...transfer, source_revision, operation_id: transferId, issuer_decision_id: decisionId, character_id: character,
          expected_from_account_id: owner.id, to_account_id: other.id,
          issued_at_source: String(now), expires_at_source: String(now + 300),
        }), source_revision],
      );
      await publishPortfolioWatermark();
      expect((await owner.page.request.get(profile)).status()).toBe(404);
      const forged = {comment: 'Seller comment after transfer'};
      await csrfRejected(owner.page, profile, forged);
      const refused = await owner.page.request.post(profile, {
        form: {...forged, _csrf: profileCsrf},
        headers: {origin: new URL(owner.page.url()).origin},
        maxRedirects: 0,
      });
      expect(refused.status()).toBe(404);
      const stored = await db.query(
        'SELECT comment FROM character_profiles WHERE account_id=$1 AND character_id=$2',
        [owner.id, character],
      );
      expect(stored.rows).toEqual([{comment: saved}]);
    } finally {
      await holdTransfer(false);
    }
  } finally {
    await db.end();
    await owner.context.close();
    await other?.context.close();
  }
});
