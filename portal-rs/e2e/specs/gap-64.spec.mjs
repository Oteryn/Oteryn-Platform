import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import https from 'node:https';
import path from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, password } from '../helpers.mjs';
import { uuid7 } from '../staff-commerce-helpers.mjs';

const fixtures = JSON.parse(fs.readFileSync(
  new URL('../../crates/minigames/src/league/fixtures/snapshots.json', import.meta.url),
));
const fixture = name => JSON.parse(fixtures.vectors.find(vector => vector.name === name).wire);
const seconds = () => Math.floor(Date.now() / 1000);

function publish(snapshot) {
  const endpoint = new URL('/internal/v1/game-public/world-event-outcome', process.env.OTERYN_LEAGUE_BASE_URL);
  expect(endpoint.origin).toBe('https://127.0.0.1:18446');
  const dir = path.dirname(process.env.OTERYN_LEAGUE_RECOVERY_FILE);
  const body = JSON.stringify(snapshot);
  const options = {
    method: 'POST',
    key: fs.readFileSync(path.join(dir, 'league-game.key')),
    cert: fs.readFileSync(path.join(dir, 'league-game.pem')),
    ca: fs.readFileSync(path.join(dir, '..', 'ca.pem')),
    minVersion: 'TLSv1.3',
    maxVersion: 'TLSv1.3',
    ALPNProtocols: ['http/1.1'],
    headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) },
  };
  return new Promise((resolve, reject) => {
    const request = https.request(endpoint, options, response => {
      response.resume();
      response.on('end', () => resolve(response.statusCode));
    });
    request.on('error', reject);
    request.setTimeout(5000, () => request.destroy(new Error('Fixture publication timed out')));
    request.end(body);
  });
}

async function accepted(snapshot) {
  await expect.poll(async () => {
    const status = await publish(snapshot);
    expect([200, 429, 503]).toContain(status);
    return status;
  }, { timeout: 30_000, intervals: [1000, 2000] }).toBe(200);
}

test('player predicts a verified event and sees the final score alongside guild standings', async ({ page, request }, info) => {
  test.setTimeout(150_000);
  const lang = info.project.metadata.locale;
  const route = `/${lang}/league`;
  const action = `${route}/prediction`;
  const control = process.env.PORTAL_E2E_LEAGUE_CONTROL;
  expect((await request.post(`${control}/resume`)).ok()).toBeTruthy();
  await expect.poll(async () => (await request.get(route)).status(), { timeout: 45_000 }).toBe(200);

  const account = { email: `league-${uuid7()}@example.invalid`, password: password() };
  const binary = process.env.PORTAL_E2E_BINARY ||
    path.resolve(process.env.CARGO_TARGET_DIR || '../target', 'debug/oteryn-portal');
  const seed = spawnSync(binary, ['seed', 'account', JSON.stringify({ ...account, locale: lang })], {
    env: { ...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: process.env.PORTAL_E2E_DATABASE_URL },
    stdio: 'ignore',
  });
  expect(seed.status).toBe(0);
  await login(page, lang, account);

  const snapshot = fixture('advance_schedule');
  const event = snapshot.event[0];
  const lock = seconds() + 40;
  snapshot.event_id = uuid7();
  snapshot.source_observed_at = String(seconds());
  event.occurrence_id = uuid7();
  event.lock_at = String(lock);
  event.scheduled_start = String(lock + 1);
  event.scheduled_end = String(lock + 2);
  await accepted(snapshot);
  await expect.poll(async () => {
    const response = await page.request.get(route);
    return response.ok() && (await response.text()).includes(`name="event" value="${snapshot.event_id}"`);
  }, { timeout: 24_000, intervals: [1000, 2000] }).toBeTruthy();

  expect((await page.goto(route)).status()).toBe(200);
  const article = page.locator(`[data-event="${snapshot.event_id}"]`);
  const form = article.locator('form');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const post = form => page.request.post(action, {
    form,
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
  await csrfRejected(page, action, { ...fields, choice: 'completed' });
  expect((await request.post(action, { form: { ...fields, choice: 'completed' } })).status()).toBe(403);
  expect((await post({ ...fields, choice: 'unknown' })).status()).toBe(422);
  expect((await post({ ...fields, choice: 'completed', revision: '2' })).status()).toBe(409);

  await form.getByRole('radio', { name: 'completed', exact: true }).check();
  await form.getByRole('button', { name: lang === 'pl' ? 'Zapisz typ' : 'Submit prediction' }).click();
  await expect(article.locator('[data-prediction-choice]')).toContainText('completed');
  await expect(form).toHaveCount(0);
  await expect(article.locator('[data-prediction-score]')).toHaveCount(0);
  expect((await post({ ...fields, choice: 'completed' })).status()).toBe(409);

  await sleep(Math.max(0, (lock + 2) * 1000 - Date.now()));
  expect((await post({ ...fields, choice: 'completed' })).status()).toBe(409);
  for (const [i, state] of ['LOCKED', 'SETTLED', 'CLOSED'].entries()) {
    snapshot.projection_revision = String(i + 2);
    snapshot.source_observed_at = String(seconds());
    event.state = state;
    if (state === 'SETTLED') {
      event.outcome = ['completed'];
      event.decided_at = [snapshot.source_observed_at];
      event.reason_code = 'NATIVE_RESULT';
      event.evidence_sha256 = fixture('settlement').event[0].evidence_sha256;
    }
    if (state === 'CLOSED') event.reason_code = 'FINAL_FREEZE';
    await accepted(snapshot);
  }

  const score = lang === 'pl' ? 'Punkty za typ 1' : 'Prediction points 1';
  await expect.poll(async () => {
    const response = await page.request.get(route);
    return response.ok() && (await response.text()).includes(score);
  }, { timeout: 30_000, intervals: [1000, 2000] }).toBeTruthy();
  await page.reload();
  await expect(article.locator('[data-prediction-score]')).toHaveText(score);
  await expect(page.getByRole('cell', { name: 'Aldric Guild', exact: true })).toBeVisible();
  await expect(page.getByRole('cell', { name: 'Second Guild', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(page.viewportSize().width);
});
