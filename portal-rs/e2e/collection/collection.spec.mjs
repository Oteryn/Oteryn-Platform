import { expect, test } from '@playwright/test';
import { login } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { connect, exchange, fixture, harness, jsonBytes, wire, withUnknown } from '../api/transport.mjs';

const journal = '/internal/v1/playerdata';
const portfolio = '/internal/v1/game-auth/native-account-characters';
const authority = 'oteryn-game-e2e';
const accepted = '{"contract_version":1,"result":"accepted"}';
const characterName = 'Aldric Vale';

async function publishPortfolio(portfolioPeer, account, other) {
  const [{ character, world, snapshot, beginId, endId, progressId, epoch }] = await query(
    `SELECT uuidv7() AS character, uuidv7() AS world, uuidv7() AS snapshot,
            uuidv7() AS "beginId", uuidv7() AS "endId", uuidv7() AS "progressId",
            (COALESCE(highest_epoch::numeric, 0) + 1)::text AS epoch
     FROM account_character_feed WHERE id=1`,
  );
  // Pre-existing custody cut, following the T72 baseline fixture. The current
  // portfolio, item snapshot and progress watermark all enter via real mTLS.
  await query(
    `INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,
      from_projection_revision,to_projection_revision,projection_epoch,recovery_generation,accepted_at)
      VALUES($1,$2,$3,1,1,1,$4,1,now()-interval '1 hour')`,
    [character, account, world, epoch],
  );

  const observed = Date.now() - 2000;
  const seconds = String(Math.floor(observed / 1000));
  const snapshotBody = jsonBytes({
    contract_version: 1, operation: 'PublishAccountCharactersV1', source_authority: authority,
    account_id: account, projection_epoch: epoch, projection_revision: '1',
    source_observed_at: seconds, characters: [{
      character_id: character, world_id: world, name: characterName, availability: 'AVAILABLE',
    }],
  });
  await wire(await exchange(portfolioPeer, 'POST', portfolio, snapshotBody), 200, accepted);
  const otherSnapshot = jsonBytes({ ...JSON.parse(snapshotBody), account_id: other, characters: [] });
  await wire(await exchange(portfolioPeer, 'POST', portfolio, otherSnapshot), 200, accepted);
  const portfolioWatermark = jsonBytes({
    contract_version: 1, operation: 'PublishProjectionWatermarkV1', source_authority: authority,
    projection_epoch: epoch, complete_through: seconds, observed_at: seconds,
  });
  await wire(await exchange(portfolioPeer, 'POST', portfolio + '/watermark', portfolioWatermark), 200, accepted);

  return { account, character, world, snapshot, beginId, endId, progressId, epoch, observed, seconds };
}

async function publishJournal(itemsPeer, progressPeer, subject) {
  const { account, character, world, snapshot, beginId, endId, progressId, epoch, observed, seconds } = subject;
  for (const [kind, id] of [['begin', beginId], ['end', endId]]) {
    const record = JSON.parse(fixture('game-journal-items-v1', `record-items-${kind}.valid.json`));
    Object.assign(record, {
      account_id: account, character_id: character, world_id: world, projection_epoch: epoch,
      record_id: id, occurred_at: seconds, coverage_start_at: seconds,
    });
    record.payload.snapshot_id = snapshot;
    const raw = jsonBytes(record);
    const route = journal + '/journal/records';
    if (kind === 'begin') {
      await wire(await exchange(itemsPeer, 'POST', route, withUnknown(raw)), 400, '');
    }
    await wire(await exchange(itemsPeer, 'POST', route, raw), 200, accepted);
  }

  const payload = JSON.parse(fixture('game-journal-progress-v1', 'payload-bestiary.valid.json'));
  const record = jsonBytes({
    contract_version: 1, operation: 'PublishGameJournalProgressV1', source_authority: authority,
    account_id: account, character_id: character, world_id: world, projection_revision: '1',
    projection_epoch: epoch, recovery_generation: '1', family: 'bestiary', source_revision: '1',
    record_id: progressId, entity_key: payload.race_key, character_revision: '1',
    occurred_at_unix_ms: String(observed), record_kind: 'UPSERT', payload,
  });
  // The valid record needs the candidate catalogue mapping, so T72 refuses it.
  await wire(await exchange(progressPeer, 'POST', journal + '/progress/record', record), 503, '');

  const watermark = jsonBytes({
    contract_version: 1, operation: 'PublishGameJournalProgressWatermarkV1', source_authority: authority,
    account_id: account, projection_epoch: epoch, recovery_generation: '1', family: 'bestiary',
    complete_through: '0', observed_at_unix_ms: String(observed), baseline_complete: false,
  });
  const route = journal + '/progress/watermark';
  await wire(await exchange(itemsPeer, 'POST', route, watermark), 401, '');
  await wire(await exchange(progressPeer, 'POST', route, watermark), 200,
    '{"contract_version":1,"result":"accepted","complete_through":"0"}');
  return [Number(seconds) * 1000, observed];
}

function sourceLabels(lang) {
  return lang === 'pl'
    ? [
        ['Posiadane przedmioty', 'Ostatnia aktualizacja ekwipunku'],
        ['Postęp bestiariusza', 'Ostatnia aktualizacja danych bestiariusza konta'],
      ]
    : [
        ['Inventory ownership', 'Last inventory update'],
        ['Bestiary progress', 'Last account bestiary feed update'],
      ];
}

test('collection shows only the owner the journal update times @read-enabled', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const pl = lang === 'pl';
  const index = `/${lang}/collection`;
  const accounts = harness().seed;
  const foreign = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN, ignoreHTTPSErrors: true,
  });
  const portfolioPeer = await connect('internal', 'lcfa');
  const itemsPeer = await connect('internal', 'journal-items');
  const progressPeer = await connect('internal', 'journal-progress');
  try {
    const visitor = await foreign.newPage();
    const anonymous = await page.request.get(index, { maxRedirects: 0 });
    expect(anonymous.status()).toBe(303);
    expect(anonymous.headers().location).toBe(`/${lang}/login`);
    expect(anonymous.headers()['cache-control']).toBe('private, no-store');
    await login(page, lang, { email: accounts.coinAccount1.email, password: process.env.PORTAL_API_LOGIN_PASSWORD });
    await login(visitor, lang, { email: accounts.coinAccount2.email, password: process.env.PORTAL_API_OTHER_LOGIN_PASSWORD });
    const subject = await publishPortfolio(portfolioPeer, accounts.coinAccount1.accountId, accounts.coinAccount2.accountId);
    const detail = `${index}/${subject.character}`;
    expect((await page.goto(detail)).status()).toBe(200);
    for (const [title, update] of sourceLabels(lang)) {
      const section = page.getByRole('region', { name: title });
      const unknown = pl ? 'Brak informacji' : 'Unknown';
      await expect(section.getByText(`${update} ${unknown}`, { exact: true })).toBeVisible();
    }

    const times = await publishJournal(itemsPeer, progressPeer, subject);

    const overview = await page.goto(index);
    expect(overview.status()).toBe(200);
    expect(overview.headers()['cache-control']).toBe('private, no-store');
    const character = page.getByRole('link', { name: characterName, exact: true });
    await expect(character).toHaveAttribute('href', detail);
    const [response] = await Promise.all([page.waitForResponse(r => new URL(r.url()).pathname === detail), character.click()]);
    expect(response.status()).toBe(200);
    expect(response.headers()['cache-control']).toBe('private, no-store');
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    for (const [i, [title, update]] of sourceLabels(lang).entries()) {
      const millis = times[i];
      const section = page.getByRole('region', { name: title });
      await expect(section.getByText(pl ? 'Niedostępne' : 'Unavailable', { exact: true })).toBeVisible();
      const timestamp = new Date(millis).toISOString().replace(/\.000Z$/, 'Z').replace(/(\.\d*?[1-9])0+Z$/, '$1Z');
      await expect(section.getByText(`${update} ${timestamp}`, { exact: true })).toBeVisible();
    }

    expect((await visitor.goto(index)).status()).toBe(200);
    await expect(visitor.getByRole('link', { name: characterName, exact: true })).toHaveCount(0);
    const refusal = await visitor.goto(detail);
    expect(refusal.status()).toBe(403);
    expect(refusal.headers()['cache-control']).toBe('private, no-store');
    await expect(visitor.getByText(new Date(times[0]).toISOString().replace('.000Z', 'Z'), { exact: false })).toHaveCount(0);

    for (const badId of ['malformed', '00000000-0000-4000-8000-000000000000']) {
      const invalid = await page.goto(`${index}/${badId}`);
      expect(invalid.status()).toBe(400);
      expect(invalid.headers()['cache-control']).toBe('private, no-store');
    }
  } finally {
    await Promise.all([foreign.close(), portfolioPeer.dispose(), itemsPeer.dispose(), progressPeer.dispose()]);
  }
});

test('populated journal evidence is hidden when the read flag is unset @read-disabled', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const index = `/${lang}/collection`;
  const { coinAccount1: owner, coinAccount2: other } = harness().seed;
  const portfolioPeer = await connect('internal', 'lcfa');
  const itemsPeer = await connect('internal', 'journal-items');
  const progressPeer = await connect('internal', 'journal-progress');
  try {
    await login(page, lang, { email: owner.email, password: process.env.PORTAL_API_LOGIN_PASSWORD });
    const subject = await publishPortfolio(portfolioPeer, owner.accountId, other.accountId);
    await publishJournal(itemsPeer, progressPeer, subject);

    const overview = await page.goto(index);
    expect(overview.status()).toBe(200);
    expect(overview.headers()['cache-control']).toBe('private, no-store');
    await expect(page.getByRole('link', { name: characterName, exact: true })).toHaveCount(0);

    const detail = await page.goto(`${index}/${subject.character}`);
    expect(detail.status()).toBe(503);
    expect(detail.headers()['cache-control']).toBe('private, no-store');
    for (const [title, update] of sourceLabels(lang)) {
      await expect(page.getByRole('region', { name: title })).toHaveCount(0);
      await expect(page.getByText(update, { exact: false })).toHaveCount(0);
    }
  } finally {
    await Promise.all([portfolioPeer.dispose(), itemsPeer.dispose(), progressPeer.dispose()]);
  }
});
