import { randomBytes, randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { login, password, pool, submit } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';
import { paidTopup } from '../staff-commerce-helpers.mjs';
import { prepare, sendForm, rejected } from './matrix-helpers.mjs';

test('the Game confirms exactly one paid browser rename and captures one test-provider-funded coin', { tag: '@game-publisher' }, async ({ page, browser }, info) => {
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
    return { accountId, password: account.password };
  };
  try {
    const owner = await authenticate(page);
    const { accountId } = owner;
    const outsiders = [];
    for (const role of [undefined, 'content_editor']) {
      const context = await browser.newContext({
        baseURL: game.origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
      });
      contexts.push(context);
      const screen = await context.newPage();
      const actor = await authenticate(screen, role);
      await screen.goto(`/${lang}/account/privacy`);
      const { fields } = await prepare(screen, 'main form[method=post]');
      outsiders.push({ screen, password: actor.password, token: fields._csrf });
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

    const money = pool();
    try {
      // Fund the account through the browser checkout and its local test provider.
      await paidTopup(page, lang, money);
    } finally {
      await money.end();
    }
    const available = () => query(
      `SELECT coalesce(sum(available_delta),0)::text AS available,
              coalesce(sum(reserved_delta),0)::text AS reserved FROM wallet_entries WHERE account_id=$1`,
      [accountId],
    );
    const before = [{ available: '1234', reserved: '0' }];
    expect(await available()).toEqual(before);
    for (const actor of [{ screen: page, password: owner.password }, ...outsiders]) {
      expect((await actor.screen.goto(`/${lang}/account/sudo`)).status()).toBe(200);
      await submit(actor.screen, form, { current_password: actor.password });
      await expect(actor.screen).toHaveURL(new RegExp(`/${lang}/password/change$`));
    }
    const publishedAt = String(Math.floor(Date.now() / 1000));
    const serviceAuthority = game.env.PORTAL_CHARACTER_SERVICES_PROJECTION_AUTHORITY;
    const servicePath = '/internal/v2/game-auth/character-services';
    await game.publish('character-services', `${servicePath}/state`, {
      contract_version: 2, operation: 'PublishCharacterServiceStateV2', source_authority: serviceAuthority,
      account_id: accountId, character_id: characterId, projection_epoch: '1', projection_revision: '1',
      source_observed_at: publishedAt, character_revision: '8', world_id: worldId, name, sex: null,
      lifecycle: 'ACTIVE', deletion_operation_id: null, deletion_scheduled_at: null, deletion_effective_at: null,
    });
    const refresh = async () => {
      const at = String(Math.floor(Date.now() / 1000));
      const [{ complete_through: lcfa }] = await query('SELECT complete_through FROM account_character_feed WHERE id=1');
      if (Number(lcfa) < Number(at)) {
        await game.publish('lcfa', '/internal/v1/game-auth/native-account-characters/watermark', {
          contract_version: 1, operation: 'PublishProjectionWatermarkV1',
          source_authority: game.env.OTERYN_LCFA_SOURCE_AUTHORITY, projection_epoch: '1',
          complete_through: at, observed_at: at,
        });
      }
      const [{ complete_through: service }] = await query('SELECT complete_through FROM character_service_feed WHERE id=1');
      if (Number(service) < Number(at)) {
        await game.publish('character-services', `${servicePath}/watermark`, {
          contract_version: 2, operation: 'PublishCharacterServiceWatermarkV2', source_authority: serviceAuthority,
          projection_epoch: '1', complete_through: at, observed_at: at,
        });
      }
    };
    await refresh();
    const pathname = `/${lang}/account/characters/${characterId}/services`;
    const status = lang === 'pl' ? 'Zgłoszenie oczekuje na potwierdzenie przez grę.'
      : 'Your request is pending confirmation by the game.';
    const error = lang === 'pl' ? 'Sprawdź żądanie i spróbuj ponownie.' : 'Check the request and try again.';
    const commandRoute = '/internal/v2/game-auth/character-commands';
    const commandAuthority = game.env.PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY;
    const operationState = request => query(
      `SELECT c.id,c.command,c.state,o.financial_state FROM character_service_orders o
       JOIN character_commands c ON c.id=o.operation_id
       WHERE o.account_id=$1 AND o.character_id=$2 AND o.request_id=$3`,
      [accountId, characterId, request],
    );
    const write = async (action, command, extras, samples) => {
      await refresh();
      expect((await page.goto(pathname)).status()).toBe(200);
      const form = `main form:has([name=action][value=${action}])`;
      const prepared = await prepare(page, form, { confirm: 'yes', ...extras });
      const snapshot = () => operationState(prepared.fields.request_id);
      const beforeConfirmation = () => Promise.all([snapshot(), available()]);
      const unchanged = [[], before];
      expect(await beforeConfirmation()).toEqual(unchanged);
      await rejected({ page, ...prepared, snapshot: beforeConfirmation, baseline: unchanged });
      for (const { screen, token } of outsiders) {
        expect((await sendForm(screen, prepared.url, { ...prepared.fields, _csrf: token })).status()).toBe(403);
        expect(await beforeConfirmation()).toEqual(unchanged);
      }
      for (const patch of samples) {
        const response = await sendForm(page, prepared.url, { ...prepared.fields, ...patch });
        expect(response.status()).toBe(400);
        expect(await response.text()).toContain(error);
        expect(await beforeConfirmation()).toEqual(unchanged);
      }
      await refresh();
      const outcome = page.waitForResponse(response => response.url() === prepared.url
        && response.request().method() === 'POST');
      await submit(page, form, { confirm: true, ...extras });
      expect((await outcome).status()).toBe(202);
      await expect(page.locator('body')).toContainText(status);
      const pending = [{ id: expect.any(String), command, state: 'PENDING', financial_state: 'RESERVED' }];
      expect(await snapshot()).toEqual(pending);
      expect((await sendForm(page, prepared.url, prepared.fields)).status()).toBe(202);
      expect(await snapshot()).toEqual(pending);
      expect(await available()).toEqual([{ available: '1233', reserved: '1' }]);
      const [{ id }] = await snapshot();
      const intent = await game.post('character-commands', `${commandRoute}/read`, {
        contract_version: 2, operation: 'ReadCharacterCommandIntentV2', source_authority: commandAuthority,
        issuer_authority: 'OTERYN_PLATFORM_CHARACTER_AUTHORITY', command, operation_id: id,
      });
      expect(intent).toMatchObject({ operation_id: id, character_id: characterId, account_id: accountId });
      return { id, intent, snapshot, command };
    };
    const acknowledge = async (request, version, effect) => {
      const decided = String(Math.floor(Date.now() / 1000));
      await game.publish('character-commands', `${commandRoute}/receipt`, {
        contract_version: 2, operation: 'PublishCharacterCommandReceiptV2', source_authority: commandAuthority,
        command: request.command, operation_id: request.id, outcome: 'COMMITTED', result_code: 'CHAR_CMD_OK',
        character_id: characterId, account_id: accountId,
        character_revision: String(Number(request.intent.expected_character_revision) + 1),
        projection_epoch: '1', projection_revision: String(version),
        ...effect, recovery_generation: '1', decided_at: decided,
      });
      expect(await request.snapshot()).toEqual([
        { id: request.id, command: request.command, state: 'TERMINAL', financial_state: 'SETTLED' },
      ]);
    };
    const requestedName = `${name} Renamed`;
    const request = await write('rename', 'RenameCharacter', { requested_name: requestedName }, [
      { requested_name: '' }, { requested_name: 'x'.repeat(30) }, { accepted_price: 'invalid' }, { requested_name: 'X' },
    ]);
    expect(request.intent.requested_name).toBe(requestedName);
    await acknowledge(request, 2, { previous_name: name, name: requestedName });
    expect(await available()).toEqual([{ available: '1233', reserved: '0' }]);
    const [{ settlements }] = await query(
      `SELECT count(*)::int AS settlements FROM character_service_orders o
       JOIN wallet_entries w ON w.operation_id=o.settlement_id AND w.account_id=o.account_id
       WHERE o.operation_id=$1 AND w.available_delta=0 AND w.reserved_delta=-1`, [request.id],
    );
    expect(settlements).toBe(1);
    const at = String(Math.floor(Date.now() / 1000));
    await game.publish('character-services', `${servicePath}/state`, {
      contract_version: 2, operation: 'PublishCharacterServiceStateV2', source_authority: serviceAuthority,
      account_id: accountId, character_id: characterId, projection_epoch: '1', projection_revision: '2',
      source_observed_at: at, character_revision: '9', world_id: worldId, name: requestedName, sex: null,
      lifecycle: 'ACTIVE', deletion_operation_id: null, deletion_scheduled_at: null, deletion_effective_at: null,
    });
    await game.publish('lcfa', '/internal/v1/game-auth/native-account-characters', {
      contract_version: 1, operation: 'PublishAccountCharactersV1', source_authority: game.env.OTERYN_LCFA_SOURCE_AUTHORITY,
      account_id: accountId, projection_epoch: '1', projection_revision: '2', source_observed_at: at,
      characters: [{ character_id: characterId, world_id: worldId, name: requestedName, availability: 'AVAILABLE' }],
    });
    await refresh();
    expect((await page.goto(pathname)).status()).toBe(200);
    await expect(page.locator('main h2').first()).toHaveText(requestedName);
    const confirmation = page.locator('main li').filter({ hasText: request.id });
    await expect(confirmation).toHaveText(`${request.id}: ${lang === 'pl' ? 'Potwierdzone przez grę' : 'Confirmed by the game'}`);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});
