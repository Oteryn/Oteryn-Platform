import crypto from 'node:crypto';
import pg from 'pg';
import {test, expect} from '../fixtures.mjs';
import {password, submit} from '../helpers.mjs';
import {uuid7} from '../support/bazaar-game.mjs';
import {GamePublishers} from '../game-publishers.mjs';

test('Game confirms character creation and publishes the public profile, presence and death', async ({page, browser}, info) => {
  const report = process.env.PORTAL_E2E_GAME_DATA_REPORT;
  if (!report) throw new Error('Set PORTAL_E2E_GAME_DATA_REPORT to the report from harness.mjs --game-data');
  const game = new GamePublishers(report);
  const lang = info.project.metadata.locale;
  const account = {email: `game-data-${crypto.randomUUID()}@example.invalid`, password: password(), locale: lang};
  const {accountId} = game.seed('account', account);
  const {worldId} = game.seed('world', {
    worldId: '01934f10-7c02-7001-805b-3b1122334401', name: 'Publisher World',
  });
  const surname = Array.from(crypto.randomBytes(9), byte => String.fromCharCode(97 + byte % 26)).join('');
  const name = `Publisher ${surname[0].toUpperCase()}${surname.slice(1)}`;
  const characterId = uuid7();
  const open = async (screen, suffix) => {
    const response = await screen.goto(`${game.origin}/${lang}/${suffix}`);
    expect(response.status(), suffix).toBe(200);
  };

  await open(page, 'login');
  await submit(page, 'main form', {email: account.email, password: account.password});
  await expect(page).toHaveURL(`${game.origin}/${lang}/`);
  await open(page, 'account/characters/new');
  await page.selectOption('select[name=world]', worldId);
  await page.locator('input[name=sex][value=Male]').check();
  await page.locator('input[name=vocation][value=Knight]').check();
  await submit(page, 'main form[method=post]', {name});
  await expect(page).toHaveURL(/\/account\/characters\/requests\/[a-f0-9-]+$/);
  await expect(page.locator('main [role=status]')).toHaveText(lang === 'pl'
    ? 'Trwa przetwarzanie prośby. Bez wyniku z gry rezultat pozostaje nieznany. Odśwież stronę, aby sprawdzić ponownie.'
    : 'Your request is being processed. Without a result from Game, its outcome is still unknown. Refresh this page to check again.');
  const requestPath = new URL(page.url()).pathname;
  const character = await game.completeCreation(requestPath.split('/').at(-1), characterId, {
    account_id: accountId, target_world_id: worldId, requested_name: name,
    requested_sex: 'Male', requested_vocation: 'Knight',
  });
  expect((await page.reload()).status()).toBe(200);
  await expect(page.locator('main [role=status]')).toHaveText(lang === 'pl'
    ? 'Gra potwierdziła utworzenie postaci. Aktualizacja listy postaci na koncie może chwilę potrwać.'
    : 'Game confirmed that your character was created. The account list may take a moment to update.');

  await game.portfolio(character);
  await open(page, `account/characters/${encodeURIComponent(name)}/profile`);
  await submit(page, 'main form', {hidden: false, show_status: true, show_deaths: true});
  await expect(page.locator('[name=show_deaths]')).toBeChecked();
  await open(page, 'account/privacy');
  await submit(page, 'main form', {public_status_visible: true});
  await expect(page.locator('[name=public_status_visible]')).toBeChecked();

  const reader = await browser.newContext({locale: lang, viewport: info.project.use.viewport, ignoreHTTPSErrors: true});
  try {
    const publicPage = await reader.newPage();
    const deathId = uuid7();
    await game.publicCharacter(character, deathId);
    // The owner preference forms can outlast the initial LCFA freshness cut.
    await game.portfolioWatermark();
    await open(publicPage, `characters/${encodeURIComponent(name)}`);
    await expect(publicPage.locator('main').getByRole('heading', {name, exact: true})).toBeVisible();
    await expect(publicPage.locator('main')).toContainText('412');
    await open(publicPage, 'online');
    await expect(publicPage.locator('main').getByRole('listitem').filter({hasText: name})).toContainText('412');
    // The composed binary admits the V2 contract but gates public serving
    // while G-4 deaths is a candidate. Verify the persisted event and the gate.
    const db = new pg.Client({connectionString: game.env.OTERYN_DATABASE_URL});
    await db.connect();
    try {
      const {rows} = await db.query('SELECT deaths FROM public_deaths_v2 WHERE subject=$1', [characterId]);
      expect(rows).toEqual([{deaths: [expect.objectContaining({
        death_occurrence_id: deathId, level: '413', pvp: false,
        killers: [{kind: 'creature', creature_key: 'oteryn:monster.dragon', name: 'Dragon'}],
      })]}]);
    } finally {
      await db.end();
    }
    const deaths = await publicPage.goto(`${game.origin}/${lang}/deaths`);
    expect(deaths.status(), 'candidate deaths contract is gated in the composed binary').toBe(503);
    expect(deaths.headers()['cache-control']).toContain('no-store');
    await expect(publicPage.locator('main article')).toHaveCount(0);
  } finally {
    await reader.close();
  }
});
