import { readFileSync } from 'node:fs';
import https from 'node:https';
import path from 'node:path';
import { expect, test } from '../fixtures.mjs';
import { seedEntity } from '../security/auth-fixtures.mjs';
import { gameJson, sourceTime } from '../outbox-helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { expectRedirect, parallelSubmit, reloadGet, replay, signIn, snapshot, submitForm } from './forms.mjs';

const copy = {
  en: {
    heading: 'Character profile settings',
    conflict: 'The character profile settings have changed. Reload the form and review your changes.',
  },
  pl: {
    heading: 'Ustawienia profilu postaci',
    conflict: 'Ustawienia profilu postaci uległy zmianie. Odśwież formularz i sprawdź swoje zmiany.',
  },
};

async function publishPortfolio(account, character, world, name, epoch) {
  const listener = new URL(`https://${process.env.OTERYN_INTERNAL_TLS_BIND}`);
  if (listener.hostname !== '127.0.0.1') throw new Error('Expected the disposable local mTLS listener');
  const ca = process.env.OTERYN_INTERNAL_TLS_CLIENT_CA_FILE;
  const scratch = path.dirname(ca);
  const options = {
    method: 'POST',
    ca: readFileSync(ca),
    cert: readFileSync(path.join(scratch, 'closure-portfolio.pem')),
    key: readFileSync(path.join(scratch, 'closure-portfolio-key.pem')),
    minVersion: 'TLSv1.3',
    ALPNProtocols: ['http/1.1'],
  };
  const post = (route, body) => new Promise((resolve, reject) => {
    const data = JSON.stringify(body);
    const request = https.request(new URL(route, listener), {
      ...options,
      signal: AbortSignal.timeout(5000),
      headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(data) },
    }, async response => {
      try {
        const chunks = [];
        for await (const chunk of response) chunks.push(chunk);
        resolve({ status: response.statusCode, headers: response.headers, raw: Buffer.concat(chunks).toString() });
      } catch (error) {
        reject(error);
      }
    });
    request.on('error', reject);
    request.end(data);
  });
  const route = '/internal/v1/game-auth/native-account-characters';
  const head = { contract_version: 1, source_authority: process.env.OTERYN_LCFA_SOURCE_AUTHORITY };
  const observed = sourceTime();
  const accepted = { contract_version: 1, result: 'accepted' };
  expect(await gameJson(post, route, {
    ...head, operation: 'PublishAccountCharactersV1', account_id: account,
    projection_epoch: epoch, projection_revision: '1', source_observed_at: observed,
    characters: [{ character_id: character, world_id: world, name, availability: 'AVAILABLE' }],
  })).toEqual(accepted);
  expect(await gameJson(post, `${route}/watermark`, {
    ...head, operation: 'PublishProjectionWatermarkV1', projection_epoch: epoch,
    complete_through: observed, observed_at: observed,
  })).toEqual(accepted);
}

test('profile rejects parallel replay and a competing tab at the same privacy revision', async ({ page, context }, info) => {
  const lang = info.project.metadata.locale;
  const owner = await signIn(page, lang);
  const [{ character, world, epoch }] = await query(
    `SELECT uuidv7() AS character, uuidv7() AS world,
            (COALESCE(highest_epoch::numeric, 0) + 1)::text AS epoch FROM account_character_feed WHERE id=1`,
  );
  await seedEntity('world', { worldId: world, name: `Profile world ${world.slice(-12)}` });
  const name = 'Aldric Vale';
  await publishPortfolio(owner.id, character, world, name, epoch);
  const portfolio = await query(
    'SELECT name FROM account_characters WHERE account_id=$1 AND character_id=$2',
    [owner.id, character],
  );
  expect(portfolio).toEqual([{ name }]);
  const path = `/${lang}/account/characters/${encodeURIComponent(name)}/profile`;
  const button = tab => tab.locator('main form button[type=submit]');
  const firstComment = lang === 'pl' ? 'Pierwszy komentarz' : 'First saved comment';
  const second = await context.newPage();
  const [{ privacy_revision: revision }] = await query('SELECT privacy_revision FROM accounts WHERE id=$1', [owner.id]);
  for (const [tab, comment] of [[page, firstComment], [second, 'Concurrent edit']]) {
    expect((await tab.goto(path)).status()).toBe(200);
    await expect(tab.locator('html')).toHaveAttribute('lang', lang);
    await expect(tab.getByRole('heading', { level: 1 })).toHaveText(copy[lang].heading);
    await expect(tab.locator('main [name=expected_revision]')).toHaveValue(revision);
    await tab.locator('main [name=comment]').fill(comment);
  }

  const original = await snapshot(button(page));
  const [accepted, refused] = (await parallelSubmit(page, button(page), original))
    .sort((a, b) => a.status() - b.status());
  expectRedirect(accepted, 303, path);
  expect(refused.status()).toBe(409);
  expect(await refused.text()).toBe(copy[lang].conflict);
  const stale = await snapshot(button(second));
  expect((await submitForm(second, button(second), stale.url)).status()).toBe(409);
  await expect(second.locator('body')).toHaveText(copy[lang].conflict);
  const unversioned = { ...stale, fields: stale.fields.filter(([key]) => key !== 'expected_revision') };
  const oldForm = await replay(page, unversioned);
  expect(oldForm.status()).toBe(409);
  expect(await oldForm.text()).toBe(copy[lang].conflict);

  expect((await page.goto(path)).status()).toBe(200);
  await reloadGet(page);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[lang].heading);
  await expect(page.locator('main [name=comment]')).toHaveValue(firstComment);
  expect(await query(
    'SELECT character_id, comment FROM character_profiles WHERE account_id=$1',
    [owner.id],
  )).toEqual([{ character_id: character, comment: firstComment }]);
  expect(await query('SELECT privacy_revision FROM accounts WHERE id=$1', [owner.id])).toEqual([
    { privacy_revision: String(BigInt(revision) + 1n) },
  ]);
  expect(await query('SELECT event_type FROM character_profile_events WHERE account_id=$1', [owner.id])).toEqual([
    { event_type: 'character.profile_preferences_updated' },
  ]);
});
