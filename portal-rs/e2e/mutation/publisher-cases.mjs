import { randomBytes, randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { login, password, submit } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';
import { prepare, sendForm, rejected, invalid, useFormMethod } from './matrix-helpers.mjs';

test('Game confirms creation and only the character owner can mutate the profile', { tag: '@game-publisher' }, async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const { GamePublishers } = await import('../game-publishers.mjs');
  const game = new GamePublishers(process.env.PORTAL_E2E_GAME_DATA_REPORT);
  const lang = info.project.metadata.locale;
  const contexts = [];
  const authenticate = async (screen, role) => {
    const account = { email: `profile-matrix-${randomUUID()}@example.invalid`, password: password(), locale: lang };
    const { accountId } = game.seed('account', account);
    if (role) {
      expect(await query(
        'INSERT INTO account_roles(account_id, role_id) SELECT $1, id FROM roles WHERE key=$2 RETURNING role_id',
        [accountId, role],
      )).toHaveLength(1);
    }
    await login(screen, lang, account);
    await expect(screen).toHaveURL(`${game.origin}/${lang}/`);
    return accountId;
  };
  try {
    const accountId = await authenticate(page);
    const outsiders = [];
    for (const role of [undefined, 'content_editor']) {
      const context = await browser.newContext({
        baseURL: game.origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      await authenticate(screen, role);
      await screen.goto(`/${lang}/account/privacy`);
      const { fields } = await prepare(screen, 'main form[method=post]');
      outsiders.push({ screen, token: fields._csrf });
    }

    const name = 'Matrix ' + Array.from(randomBytes(11), (x, i) =>
      String.fromCharCode((i ? 97 : 65) + x % 26)).join('');
    const { worldId } = game.seed('world', {
      worldId: '01934f10-7c02-7001-805b-3b1122334401', name: 'Publisher World',
    });
    const characterId = uuid7();
    const form = 'main form[method=post]';
    expect((await page.goto(`/${lang}/account/characters/new`)).status()).toBe(200);
    await page.selectOption('select[name=world]', worldId);
    await page.locator('input[name=sex][value=Male]').check();
    await page.locator('input[name=vocation][value=Knight]').check();
    await submit(page, form, { name });
    await expect(page).toHaveURL(/\/account\/characters\/requests\/[a-f0-9-]+$/);
    const operationId = new URL(page.url()).pathname.split('/').at(-1);
    const character = await game.completeCreation(operationId, characterId, {
      account_id: accountId, target_world_id: worldId, requested_name: name,
      requested_sex: 'Male', requested_vocation: 'Knight',
    });
    await page.reload();
    await expect(page.getByRole('status')).toHaveText(lang === 'pl'
      ? 'Gra potwierdziła utworzenie postaci. Aktualizacja listy postaci na koncie może chwilę potrwać.'
      : 'Game confirmed that your character was created. The account list may take a moment to update.');
    expect(await query('SELECT state FROM character_commands WHERE id=$1 AND account_id=$2', [operationId, accountId]))
      .toEqual([{ state: 'TERMINAL' }]);
    await game.portfolio(character);
    expect(await query('SELECT name FROM account_characters WHERE account_id=$1 AND character_id=$2', [accountId, characterId]))
      .toEqual([{ name }]);

    const state = () => query(
      'SELECT comment, hidden, show_deaths FROM character_profiles WHERE account_id=$1 AND character_id=$2',
      [accountId, characterId],
    );
    expect(await state()).toEqual([]);
    const refreshWatermark = async () => {
      const observed = String(Math.floor(Date.now() / 1000));
      const [watermark] = await query('SELECT complete_through FROM account_character_feed WHERE id=1');
      if (Number(watermark.complete_through) >= Number(observed)) return;
      await game.publish('lcfa', '/internal/v1/game-auth/native-account-characters/watermark', {
        contract_version: 1, operation: 'PublishProjectionWatermarkV1',
        source_authority: game.env.OTERYN_LCFA_SOURCE_AUTHORITY, projection_epoch: '1',
        complete_through: observed, observed_at: observed,
      });
    };
    for (const prefix of [`/${lang}`, '']) {
      for (const method of ['POST', 'PUT']) {
        await test.step(`${method} ${prefix || 'unprefixed'} profile`, async () => {
          await refreshWatermark();
          const path = `${prefix}/account/characters/${encodeURIComponent(name)}/profile`;
          expect((await page.goto(path)).status()).toBe(200);
          const baseline = await state();
          const comment = `${method} ${prefix || 'unprefixed'} matrix profile`;
          const { url, fields } = await prepare(page, form, { comment, hidden: '0', show_deaths: '1' });
          await rejected({ page, url, fields, snapshot: state, baseline, method });
          await refreshWatermark();
          for (const { screen, token } of outsiders) {
            const denial = await sendForm(screen, url, { ...fields, _csrf: token }, method);
            expect(denial.status()).toBe(404);
            expect(await state()).toEqual(baseline);
          }
          await invalid({
            page, url, fields, method, snapshot: state, baseline,
            message: lang === 'pl' ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
              : 'Some information could not be accepted. Check your input and try again.',
            samples: [
              { name: 'empty boolean', patch: { hidden: '' } },
              { name: 'overlong comment', patch: { comment: 'a'.repeat(501) } },
              { name: 'wrong checkbox type', patch: { show_deaths: 'abc' } },
              { name: 'boolean beyond the maximum', patch: { show_deaths: '2' } },
            ],
          });
          await refreshWatermark();
          await useFormMethod(page, url, method, () => submit(page, form, { comment, hidden: false, show_deaths: true }));
          await expect(page.locator('[name=comment]')).toHaveValue(comment);
          await expect(page.locator('[name=show_deaths]')).toBeChecked();
          expect(await state()).toEqual([{ comment, hidden: false, show_deaths: true }]);
        });
      }
    }
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});
