import { randomBytes, randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, password, submit, waitForLoginBudget } from '../helpers.mjs';
import { enrollMfa, query } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';
import { prepare, sendForm, rejected, signInWithRecoveryCode } from './matrix-helpers.mjs';

test('Game-backed recruitment permits owner, reporter and staff actions under separate authority', { tag: '@game-publisher' }, async ({ page, browser }, info) => {
  test.setTimeout(720_000);
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
    const path = `/${lang}/login`;
    expect((await screen.goto(path)).status()).toBe(200);
    await waitForLoginBudget();
    const form = screen.locator('main form[method=post]');
    const { email, password: credential } = account;
    await csrfRejected(screen, path, { email, password: credential });
    await form.locator('[name=email]').fill(email);
    await form.locator('[name=password]').fill(credential);
    const [response] = await Promise.all([
      screen.waitForResponse(candidate => candidate.url() === `${game.origin}${path}`
        && candidate.request().method() === 'POST', { timeout: 120_000 }),
      form.locator('button[type=submit], button:not([type])').click({ timeout: 120_000 }),
    ]);
    expect(response.status()).toBe(302);
    expect(response.headers().location).toBe(`/${lang}/`);
    await expect(screen).toHaveURL(`${game.origin}/${lang}/`);
    return { accountId, ...account };
  };
  try {
    const owner = await authenticate(page);
    const { accountId } = owner;
    const outsiders = [];
    for (const role of [undefined, 'content_editor']) {
      const context = await browser.newContext({
        baseURL: game.origin, locale: lang, viewport: info.project.use.viewport,
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


    const moderatorContext = await browser.newContext({
      baseURL: game.origin, locale: lang,
      viewport: info.project.use.viewport,
    });
    contexts.push(moderatorContext);
    const moderator = await moderatorContext.newPage();
    const staff = await authenticate(moderator, 'support');
    await enrollMfa(moderator, lang, staff);
    await signInWithRecoveryCode(moderator, lang, staff, await moderator.locator('textarea[readonly]').first().inputValue());
    await expect(moderator).toHaveURL(`${game.origin}/${lang}/`);
    await game.publicCharacter(character, uuid7());
    const refresh = async () => {
      const at = String(Math.floor(Date.now() / 1000));
      const [{ complete_through: portfolio }] = await query('SELECT complete_through FROM account_character_feed WHERE id=1');
      if (Number(portfolio) < Number(at)) {
        await game.publish('lcfa', '/internal/v1/game-auth/native-account-characters/watermark', {
          contract_version: 1, operation: 'PublishProjectionWatermarkV1',
          source_authority: game.env.OTERYN_LCFA_SOURCE_AUTHORITY, projection_epoch: '1',
          complete_through: at, observed_at: at,
        });
      }
      const [subject] = await query("SELECT watermark FROM public_snapshots WHERE family='character_profile' AND is_watermark");
      if (Number(subject.watermark) < Number(at)) {
        await game.publish('public-projections', '/internal/v1/game-public/watermark', {
          contract_version: 1, operation: 'PublishPublicProjectionWatermarkV1',
          source_authority: game.env.PORTAL_PUBLIC_PROJECTIONS_AUTHORITY, family: 'character_profile',
          projection_epoch: '1', complete_through: at, observed_at: at,
        });
      }
    };
    await refresh();
    const board = `/${lang}/party`;
    const create = `main form[action="${board}"]`;
    expect((await page.goto(board)).status()).toBe(200);
    const memo = `Local recruitment mutation evidence ${randomUUID()}`;
    const durable = () => query(
      `SELECT id,account_id,character_id,status,note,
         ARRAY(SELECT r.status FROM pd_party_reports r WHERE r.post_id=p.id ORDER BY r.id) AS reports
       FROM pd_party_posts p WHERE p.account_id=$1 ORDER BY p.id`,
      [accountId],
    );
    const extraction = lang === 'pl'
      ? 'Niektóre informacje nie zostały zaakceptowane. Sprawdź dane i spróbuj ponownie.'
      : 'Some information could not be accepted. Check your input and try again.';
    const malformed = async (screen, prepared, samples, baseline) => {
      for (const [patch, status = 422] of samples) {
        const response = await sendForm(screen, prepared.url, { ...prepared.fields, ...patch });
        expect(response.status()).toBe(status);
        expect(await response.text()).toContain(extraction);
        expect(await durable()).toEqual(baseline);
      }
    };
    const cross = async (offer, status, baseline) => {
      for (const { screen, token } of outsiders) {
        expect((await sendForm(screen, offer.url, { ...offer.fields, _csrf: token })).status()).toBe(status);
        expect(await durable()).toEqual(baseline);
      }
    };
    const publish = async (note, exerciseRefusals) => {
      const baseline = await durable();
      await refresh();
      expect((await page.goto(board)).status()).toBe(200);
      const now = Math.floor(Date.now() / 1000);
      const values = { character: characterId, min_level: '300', max_level: '500',
        starts_at: String(now + 3600), expires_at: String(now + 7200) };
      const offer = await prepare(page, create, { ...values, note });
      if (exerciseRefusals) {
        await rejected({ page, ...offer, snapshot: durable, baseline });
        await cross(offer, 403, baseline);
        await malformed(page, offer, [[{ character: '' }, 422], [{ note: 'x'.repeat(501) }],
          [{ min_level: 'text' }, 422], [{ max_level: '100001' }]], baseline);
      }
      const draft = page.locator(create);
      for (const [field, value] of Object.entries({ ...values, note })) {
        await draft.locator(`[name="${field}"]`).fill(value);
      }
      await refresh();
      const [posted] = await Promise.all([
        page.waitForResponse(response => response.url() === offer.url && response.request().method() === 'POST'),
        draft.locator('button[type=submit], button:not([type])').click(),
      ]);
      expect(posted.status()).toBe(303);
      expect(posted.headers().location).toBe(board);
      const result = { id: expect.any(String), account_id: accountId,
        character_id: characterId, status: 'active', note, reports: [] };
      const actual = await durable();
      expect(actual).toEqual([...baseline, result]);
      const created = actual.at(-1);
      const listing = page.locator(`main article:has(form[action="${board}/${created.id}/report"])`);
      await expect(listing).toBeVisible();
      await expect(listing).toContainText(note);
      return created;
    };
    const opened = await publish(memo, true);
    const report = `main form[action="${board}/${opened.id}/report"]`;
    const firstActor = outsiders[0];
    const reporter = firstActor.screen;
    expect((await reporter.goto(board)).status()).toBe(200);
    const reason = 'Synthetic abuse report for a disposable recruitment.';
    const complaint = await prepare(reporter, report, { reason });
    await rejected({ page: reporter, ...complaint, snapshot: durable, baseline: [opened] });
    const ownerToken = (await prepare(page, report)).fields._csrf;
    expect((await sendForm(page, complaint.url, { ...complaint.fields, _csrf: ownerToken })).status()).toBe(403);
    expect(await durable()).toEqual([opened]);
    await malformed(reporter, complaint, [[{ reason: '' }], [{ reason: 'x'.repeat(501) }],
      [{ reason: ' ' }], [{ reason: 'x\u007f' }]], [opened]);
    await submit(reporter, report, { reason });
    await expect(reporter).toHaveURL(`${game.origin}${board}`);
    await expect(reporter.locator('main article').filter({ hasText: memo })).toBeVisible();
    let reportOpen = { ...opened, reports: ['open'] };
    expect(await durable()).toEqual([reportOpen]);
    expect((await sendForm(reporter, complaint.url, complaint.fields)).status()).toBe(303);
    expect(await durable()).toEqual([reportOpen]);

    for (const decision of ['dismiss', 'remove']) {
      const control = `/admin/${lang}/party`;
      expect((await moderator.goto(control)).status()).toBe(200);
      const form = `main form[action="${control}/${opened.id}/moderate"]`;
      const moderation = await prepare(moderator, form, { action: decision, reason });
      await rejected({ page: moderator, ...moderation, snapshot: durable, baseline: [reportOpen] });
      await cross(moderation, 403, [reportOpen]);
      await malformed(moderator, moderation, [[{ reason: '' }], [{ reason: 'x'.repeat(501) }],
        [{ report_ids: 'bad-type' }], [{ action: 'ban' }]], [reportOpen]);
      await moderator.locator(`${form} [name=reason]`).fill(reason);
      await moderator.locator(`${form} button[value=${decision}]`).click();
      await expect(moderator).toHaveURL(`${game.origin}${control}`);
      await expect(moderator.locator(form)).toHaveCount(0);
      const after = { ...reportOpen, status: decision === 'remove' ? 'removed' : 'active',
        reports: reportOpen.reports.map(() => 'resolved') };
      expect(await durable()).toEqual([after]);
      if (decision === 'dismiss') {
        const clerical = outsiders[1].screen;
        expect((await clerical.goto(board)).status()).toBe(200);
        await submit(clerical, report, { reason: 'Independent report for the staff removal.' });
        await expect(clerical).toHaveURL(`${game.origin}${board}`);
        reportOpen = { ...after, reports: ['resolved', 'open'] };
        expect(await durable()).toEqual([reportOpen]);
      }
    }
    const moderated = (await durable())[0];
    expect(moderated.status).toBe('removed');
    const ownerListing = await publish(`Owner removes this second recruitment ${randomUUID()}`, false);
    const remove = `main form[action="${board}/${ownerListing.id}/remove"]`;
    const removal = await prepare(page, remove);
    await rejected({ page, ...removal, snapshot: durable, baseline: [moderated, ownerListing] });
    await cross(removal, 404, [moderated, ownerListing]);
    await submit(page, remove);
    await expect(page.locator(remove)).toHaveCount(0);
    const finished = [moderated, { ...ownerListing, status: 'removed' }];
    expect(await durable()).toEqual(finished);
    expect((await sendForm(page, removal.url, removal.fields)).status()).toBe(404);
    expect(await durable()).toEqual(finished);
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});
