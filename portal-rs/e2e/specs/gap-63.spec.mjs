import { test, expect } from '../fixtures.mjs';
import { csrfRejected, pool } from '../helpers.mjs';
import { messages } from '../public-depth-helpers.mjs';
import { uuid7 } from '../staff-commerce-helpers.mjs';
import { seededAccount, seededLogin, seededStaff } from '../staff-rbac-helpers.mjs';

async function historicalPuzzle(db) {
  // Reuse a historical fixture across projects. Published puzzles are immutable
  // and the repository runner drops the disposable database when it exits.
  const day = "(statement_timestamp() AT TIME ZONE 'UTC')::date - 2";
  const { rows: [existing] } = await db.query(`SELECT id FROM puzzles_daily WHERE day=${day} AND kind='quiz'`);
  if (existing) return existing.id;

  const version = uuid7();
  const id = uuid7();
  const definition = {
    kind: 'quiz',
    world_version: version,
    question: { en: 'Which animal has a bushy tail?', pl: 'Które zwierzę ma puszysty ogon?' },
    choices: [
      { label: { en: 'Fox', pl: 'Lis' }, source: { kind: 'creature', key: 'fox' } },
      { label: { en: 'Frog', pl: 'Żaba' }, source: { kind: 'creature', key: 'frog' } },
    ],
    correct_choice: 0,
  };
  await db.query("INSERT INTO world_versions(id,source_commit,status) VALUES($1,$2,'ready')",
    [version, '63'.repeat(20)]);
  await db.query(`INSERT INTO puzzles_daily(id,day,kind,world_version,definition)
    VALUES($1,${day},'quiz',$2,$3)`, [id, version, definition]);
  return id;
}

test('season pass imports solved Portal puzzles once and reports Game availability', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const viewport = info.project.use.viewport;
  const origin = new URL(info.project.use.baseURL).origin;
  const pass = `/${lang}/pass`;
  const operatorPath = `${pass}/operator`;
  const labels = messages('templates/pass', lang);
  const season = uuid7();
  const challenge = uuid7();
  const journal = uuid7();
  const reward = uuid7();
  const futureSeason = uuid7();
  const player = await seededAccount(lang);
  const answer = uuid7();
  const db = pool();
  const staffContext = await browser.newContext({ baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport });
  try {
    await db.query(`INSERT INTO pass_seasons(id,starts_at,ends_at,price)
      VALUES($1,statement_timestamp()-interval '3 days',statement_timestamp()+interval '39 days',50)`, [season]);
    await db.query(`INSERT INTO pass_challenges(id,season_id,source,metric,target,xp)
      VALUES($1,$2,'portal','daily_puzzles.solved',1,5),($3,$2,'journal','bestiary.complete',7,2)`,
    [challenge, season, journal]);
    await db.query(`INSERT INTO pass_rewards(id,season_id,track,xp,grant_key,grant_revision)
      VALUES($1,$2,'free',5,'season.frame','1')`, [reward, season]);

    await seededLogin(page, lang, player);
    await page.goto(`/${lang}/news`);
    if (viewport.width <= 960) await page.locator('details.om-fallback-menu > summary').click();
    const link = page.locator(`#portal-navigation a[href="${pass}"]`);
    await link.locator('xpath=ancestor::details[contains(@class,"om-section")]').locator('summary').click();
    await link.click();
    await expect(page).toHaveURL(origin + pass);
    await expect(page.locator('#season-points')).toHaveText('0');
    const gameChallenge = page.locator('article').filter({ hasText: labels.journal });
    await expect(gameChallenge).toContainText(labels['data-unavailable']);
    await expect(gameChallenge).not.toContainText('0 / 7');
    const rewardCard = page.locator('article').filter({ hasText: 'season.frame' });
    await expect(rewardCard).toContainText(labels['points-required']);
    expect((await page.request.get(operatorPath)).status()).toBe(403);

    const progress = `${pass}/${season}/progress`;
    const form = page.locator(`form[action="${progress}"]`);
    const csrf = await form.locator('[name=_csrf]').inputValue();
    const post = (path, fields) => page.request.post(path, {
      form: { _csrf: csrf, ...fields }, headers: { origin }, maxRedirects: 0,
    });
    await csrfRejected(page, progress, { challenge });
    expect((await post(progress, { challenge: 'invalid' })).status()).toBe(422);
    expect((await post(progress, { challenge, xp: '99' })).status()).toBe(422);
    expect((await post(progress, { challenge: journal })).status()).toBe(403);
    expect((await post(`${pass}/${uuid7()}/progress`, { challenge })).status()).toBe(403);

    const puzzle = await historicalPuzzle(db);
    await db.query(`INSERT INTO puzzles_answers(id,account_id,puzzle_id,choice,correct,created_at)
      VALUES($1,$2,$3,0,true,statement_timestamp()-interval '2 days')`, [answer, player.id, puzzle]);
    await form.getByRole('button', { name: labels['sync-puzzles'] }).click();
    await expect(page.locator('#season-points')).toHaveText('5');
    await expect(page.locator('main')).toContainText(labels['challenge-complete']);
    await expect(rewardCard).toContainText(labels['reward-earned']);
    await expect(rewardCard).toContainText(`${labels['game-delivery']}: ${labels['data-unavailable']}`);
    await expect(rewardCard.locator('form')).toHaveCount(0);
    expect((await post(progress, { challenge })).status()).toBe(303);
    expect((await post(`${pass}/${season}/claim`, { reward, operation: uuid7() })).status()).toBe(404);
    expect((await db.query("SELECT event_id,amount FROM pass_events WHERE season_id=$1", [season])).rows)
      .toEqual([{ event_id: answer, amount: '1' }]);
    expect((await db.query("SELECT 1 FROM pass_claims WHERE season_id=$1", [season])).rowCount).toBe(0);

    const staff = await staffContext.newPage();
    await seededStaff(staff, lang, 'platform_admin');
    await staff.goto(pass);
    await staff.getByRole('link', { name: labels.operations }).click();
    await expect(staff).toHaveURL(origin + operatorPath);
    await expect(staff.locator('main')).toContainText(season);
    await expect(staff.locator('main')).toContainText(labels['data-unavailable']);
    const publish = staff.locator(`form[action="${operatorPath}"]`);
    const staffCsrf = await publish.locator('[name=_csrf]').inputValue();
    const day = 24 * 60 * 60 * 1000;
    const today = Date.now();
    const definition = {
      id: futureSeason,
      starts_at: new Date(today + 45 * day).toISOString(),
      ends_at: new Date(today + 87 * day).toISOString(),
      price: 50,
      challenges: [{ id: uuid7(), source: 'portal', metric: 'daily_puzzles.solved', target: 1, xp: 5 }],
      rewards: [{ id: uuid7(), paid: false, xp: 5, grant_key: 'season.frame', grant_revision: '1' }],
    };
    await csrfRejected(staff, operatorPath, { definition: JSON.stringify(definition) });
    await publish.getByLabel(labels.definition).fill(JSON.stringify(definition));
    await publish.getByRole('button', { name: labels['publish-season'] }).click();
    await expect(staff.locator('main')).toContainText(futureSeason);
    await expect(staff.locator('main')).toContainText(labels['season-scheduled']);
    const replay = await staff.request.post(operatorPath, {
      form: { _csrf: staffCsrf, definition: JSON.stringify(definition) },
      headers: { origin },
      maxRedirects: 0,
    });
    expect(replay.status()).toBe(409);
    for (const screen of [page, staff]) {
      expect(await screen.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(viewport.width);
    }
  } finally {
    await staffContext.close();
    try {
      const seasons = [[season, futureSeason]];
      await db.query('DELETE FROM pass_progress WHERE season_id=ANY($1::uuid[])', seasons);
      await db.query('DELETE FROM pass_rewards WHERE season_id=ANY($1::uuid[])', seasons);
      await db.query('DELETE FROM pass_challenges WHERE season_id=ANY($1::uuid[])', seasons);
      await db.query('DELETE FROM pass_seasons WHERE id=ANY($1::uuid[])', seasons);
      await db.query('DELETE FROM puzzles_answers WHERE id=$1', [answer]);
    } finally {
      await db.end();
    }
  }
});

test('season pass labels have Polish and English parity', () => {
  const [pl, en] = ['pl', 'en'].map(lang => messages('templates/pass', lang));
  expect(Object.keys(pl).sort()).toEqual(Object.keys(en).sort());
  for (const key of ['sync-puzzles', 'data-unavailable', 'reward-earned', 'operations', 'publish-season']) {
    expect(pl[key]).toBeTruthy();
    expect(en[key]).toBeTruthy();
  }
});
