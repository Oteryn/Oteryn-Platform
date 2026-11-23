import { spawn } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { test as base, expect } from '../fixtures.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';
import { login, pool, submit } from '../helpers.mjs';
import { gameJson, gamePeer, nil, seededAccount, seededPlayer, sourceTime } from '../outbox-helpers.mjs';

const test = base.extend({
  creationHarness: async ({}, use, info) => {
    const binary = process.env.PORTAL_E2E_BINARY;
    if (!binary) throw new Error('Use run.mjs with the local launch environment');
    const scratch = await fs.mkdtemp(path.join(os.tmpdir(), 'portal-gap01-'));
    const wrapper = path.join(scratch, 'portal.mjs');
    const output = path.join(scratch, 'harness');
    const port = 18800 + 4 * info.parallelIndex;
    const flags = {
      OTERYN_CHARACTER_CREATE_WEB_ENABLED: 'true',
      OTERYN_CHARACTER_COMMANDS_ENABLED: 'true',
      OTERYN_CHARACTER_COMMAND_CREATE_ENABLED: 'true',
      OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY: 'oteryn-game-e2e',
      OTERYN_ACCOUNT_OVERVIEW_ENABLED: 'true',
    };
    await fs.writeFile(wrapper, [
      '#!' + process.execPath,
      "import { spawn } from 'node:child_process';",
      'const child = spawn(' + JSON.stringify(binary) + ', process.argv.slice(2), {',
      '  env: { ...process.env, ...' + JSON.stringify(flags) + ' }, stdio: "inherit",',
      '});',
      'for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => child.kill(signal));',
      'child.on("exit", code => process.exit(code ?? 1));',
    ].join('\n'), { mode: 0o700 });
    const child = spawn(process.execPath, ['harness.mjs', '--outbox'], {
      cwd: fileURLToPath(new URL('../', import.meta.url)),
      env: {
        ...process.env,
        OTERYN_ENV: 'testing',
        PORTAL_E2E_BINARY: wrapper,
        PORTAL_E2E_OUT: output,
        PORTAL_E2E_KEEP_DATABASE: '0',
        PORTAL_E2E_HTTP_PORT: String(port),
        PORTAL_E2E_INTERNAL_PORT: String(port + 1),
        PORTAL_E2E_ADMISSION_PORT: String(port + 2),
        PORTAL_E2E_GATEWAY_PORT: String(port + 3),
        PORTAL_CHARACTER_COMMANDS_ENABLED: 'true',
        PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY: flags.OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY,
        OTERYN_LCFA_ENABLED: 'true',
        OTERYN_LCFA_SOURCE_AUTHORITY: flags.OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY,
      },
      stdio: ['ignore', 'pipe', 'inherit'],
    });
    const stopped = once(child, 'close');
    const reports = createInterface({ input: child.stdout });
    try {
      const [line] = await Promise.race([
        once(reports, 'line'),
        stopped.then(([code]) => { throw new Error('Creation harness exited before readiness (' + code + ')'); }),
      ]);
      expect(JSON.parse(line).status).toBe('ready');
      const env = JSON.parse(await fs.readFile(path.join(output, 'secrets', 'env.json'), 'utf8'));
      const prior = Object.fromEntries(Object.keys(env).map(key => [key, process.env[key]]));
      Object.assign(process.env, env);
      try {
        await use(env.PORTAL_E2E_ORIGIN);
      } finally {
        for (const [key, value] of Object.entries(prior)) {
          if (value === undefined) delete process.env[key];
          else process.env[key] = value;
        }
      }
    } finally {
      reports.close();
      child.kill('SIGTERM');
      await stopped;
      await fs.rm(scratch, { recursive: true, force: true });
    }
  },
  baseURL: async ({ creationHarness }, use) => use(creationHarness),
});

test.use({ trace: 'off', screenshot: 'off' });

test('character creation follows the receipt and the refreshed account roster', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const langIsPl = lang === 'pl';
  const createPath = '/' + lang + '/account/characters/new';
  const form = 'main form[method=post]';
  const name = 'Aldric Stone';
  const world = uuid7();
  const account = await seededPlayer(page, lang);
  const db = pool();
  try {
    await db.query('INSERT INTO world_status(id, name) VALUES ($1, $2)', [world, 'Qualification World']);
    const commands = () => db.query(
      'SELECT id, state FROM character_commands WHERE account_id=$1 AND command=$2',
      [account.id, 'CreateCharacter'],
    ).then(result => result.rows);
    async function create(requestedName) {
      await page.locator(form + ' select[name=world]').selectOption(world);
      await page.locator(form + ' input[name=sex][value=Female]').check();
      await page.locator(form + ' input[name=vocation][value=Druid]').check();
      await submit(page, form, { name: requestedName });
    }

    expect((await page.goto('/account')).status()).toBe(200);
    await page.locator('section[aria-labelledby=characters] a[href="' + createPath + '"]').click();
    await expect(page.locator(form)).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

    await create('Game Master');
    await expect(page.locator('main [role=status]')).toContainText(langIsPl ? 'zastrzeżone' : 'reserved');
    expect(await commands()).toEqual([]);

    await create(name);
    await expect(page).toHaveURL(/\/characters\/requests\/[a-f0-9-]+$/);
    const statusPath = new URL(page.url()).pathname;
    const operation = statusPath.split('/').at(-1);
    await expect(page.locator('main [role=status]')).toContainText(langIsPl ? 'rezultat pozostaje nieznany' : 'outcome is still unknown');
    expect(await commands()).toEqual([{ id: operation, state: 'PENDING' }]);

    const foreign = await browser.newContext({
      baseURL: new URL(page.url()).origin,
      ignoreHTTPSErrors: true,
      locale: lang,
      viewport: info.project.use.viewport,
    });
    try {
      const other = await foreign.newPage();
      await login(other, lang, await seededAccount(lang));
      expect((await other.goto(statusPath)).status()).toBe(404);
    } finally {
      await foreign.close();
    }

    await page.goto(createPath);
    await create('ALDRIC STONE');
    await expect(page.locator('main [role=status]')).toContainText(langIsPl ? 'imię jest niedostępne' : 'name is not available');
    expect(await commands()).toEqual([{ id: operation, state: 'PENDING' }]);

    const command = gamePeer('character-commands');
    const commandPath = '/internal/v1/game-auth/character-commands';
    const envelope = (operation, authority = process.env.PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY) => ({
      contract_version: 1, operation, source_authority: authority,
    });
    const pending = await gameJson(command, commandPath + '/pending', {
      ...envelope('ListPendingCharacterCommandsV1'),
      after_source_revision: '0', after_operation_id: nil, max_entries: 32,
    });
    expect(pending.pending).toEqual([
      { command: 'CreateCharacter', operation_id: operation, source_revision: expect.any(String) },
    ]);
    const intent = await gameJson(command, commandPath + '/read', {
      ...envelope('ReadCharacterCommandIntentV1'),
      issuer_authority: 'OTERYN_PLATFORM_CHARACTER_AUTHORITY', command: 'CreateCharacter', operation_id: operation,
    });
    expect(intent).toMatchObject({
      contract_version: 2, account_id: account.id, target_world_id: world,
      requested_name: name, requested_sex: 'Female', requested_vocation: 'Druid',
    });

    // The local peer supplies contract evidence through mTLS for this fixture only.
    const character = uuid7();
    expect(await gameJson(command, commandPath + '/receipt', {
      ...envelope('PublishCharacterCommandReceiptV1'),
      command: 'CreateCharacter', operation_id: operation, outcome: 'COMMITTED', result_code: 'CHAR_CMD_OK',
      character_id: character, account_id: account.id, world_id: world, name,
      character_revision: '1', projection_epoch: '1', projection_revision: '1',
      recovery_generation: '1', decided_at: sourceTime(),
    })).toEqual({ contract_version: 1, result: 'accepted' });

    await page.goto('/account');
    await page.locator('main a[href="' + statusPath + '"]').click();
    await expect(page.locator('main [role=status]')).toContainText(langIsPl ? 'może chwilę potrwać' : 'may take a moment');
    await expect(page.locator('main')).toContainText(langIsPl ? 'lista postaci jest niedostępna' : 'roster is unavailable');
    const rosterLink = page.getByRole('link', { name: langIsPl ? 'Otwórz listę postaci' : 'Open character roster' });
    await rosterLink.click();
    await expect(page.locator('main a[href^="/characters/"]').filter({ hasText: name })).toHaveCount(0);

    const lcfa = gamePeer('lcfa');
    const portfolio = '/internal/v1/game-auth/native-account-characters';
    const authority = process.env.OTERYN_LCFA_SOURCE_AUTHORITY;
    const at = sourceTime();
    await gameJson(lcfa, portfolio, {
      ...envelope('PublishAccountCharactersV1', authority),
      account_id: account.id, projection_epoch: '1', projection_revision: '1', source_observed_at: at,
      characters: [{ character_id: character, world_id: world, name, availability: 'AVAILABLE' }],
    });
    await gameJson(lcfa, portfolio + '/watermark', {
      ...envelope('PublishProjectionWatermarkV1', authority),
      projection_epoch: '1', complete_through: at, observed_at: at,
    });

    await page.goto(statusPath);
    await expect(page.locator('main [role=status]')).toContainText(langIsPl ? 'jest już na liście' : 'now in the account roster');
    await rosterLink.click();
    await expect(page.locator('main a[href^="/characters/"]').filter({ hasText: name })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

    await page.goto(createPath);
    await create(name);
    await expect(page.locator('main [role=status]')).toContainText(langIsPl ? 'imię jest niedostępne' : 'name is not available');
    expect(await commands()).toEqual([{ id: operation, state: 'TERMINAL' }]);
  } finally {
    await db.end();
  }
});
