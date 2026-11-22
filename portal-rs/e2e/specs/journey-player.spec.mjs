import crypto from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { TOTP, Secret } from 'otpauth';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, mailLink, password, pool, seedPasswordRange, submit } from '../helpers.mjs';
import { post, uuid7 } from '../game-sim-helpers.mjs';
import { follow, localized, newPlayer, signIn, topUp, verifyPlayer, visibleLink } from '../journey-player-helpers.mjs';

test('a new player goes from discovering Oteryn to managing their account', async ({ page, browser }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const path = suffix => `/${lang}/${suffix}`;
  const t = (pl, en) => localized(lang, pl, en);
  const main = page.locator('main');
  let account;
  let totp;
  let recoveryCodes;
  let accountId;
  let opponentCharacter;
  let characterRequest;
  const worldId = '0192a000-0000-7000-8000-000000000012';
  let lastTotpWindow;

  async function challenge({ invalidFirst = false, recoveryCode } = {}) {
    await expect(page).toHaveURL(/\/mfa\/challenge/);
    const form = 'main form[action$="/mfa/challenge"]';
    if (invalidFirst) {
      await submit(page, form, { code: 'invalid-code' });
      await expect(main.getByRole('alert')).toHaveText(t(
        'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.',
        'The request could not be verified. Check your details and try again.'));
    }
    if (!recoveryCode) {
      await expect.poll(() => Math.floor(Date.now() / 30_000), { timeout: 35_000 }).toBeGreaterThan(lastTotpWindow);
      lastTotpWindow = Math.floor(Date.now() / 30_000);
    }
    await submit(page, form, { code: recoveryCode ?? totp.generate() });
    await expect(page).toHaveURL(path(''));
  }

  await test.step('Arrive anonymously and open the news archive', async () => {
    expect((await page.goto(path(''))).status()).toBe(200);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(page.locator('.hero-news h1')).toHaveText(t('Aktualności', 'News'));
    await follow(page, path('news'), 200);
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Aktualności', 'News'));
  });

  await test.step('Require a login for private account and support pages', async () => {
    for (const suffix of ['account/characters/new', 'account/privacy/export', 'account/privacy/delete', 'support/tickets']) {
      const response = await page.request.get(path(suffix), { maxRedirects: 0 });
      expect(response.status(), suffix).toBe(303);
      expect(response.headers().location).toMatch(/\/login(?:\?|$)/);
    }
  });

  await test.step('Check registered worlds, then check highscores and the wiki', async () => {
    await follow(page, path('worlds'));
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Status światów', 'World status'));
    await expect(main).toContainText('Local Bazaar World');
    await follow(page, path('rankings/experience'), 503, t('Ranking doświadczenia', 'Experience ranking'));
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Rankingi', 'Rankings'));
    await follow(page, path('wiki'), 503, 'Wiki');
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t(
      'Dane świata są obecnie niedostępne.', 'World data is currently unavailable.'));
  });

  await test.step('Read the getting-started guide and check client information', async () => {
    await follow(page, path('#jak-zaczac'));
    await expect(page.locator('#start-title')).toHaveText(t('Jak zacząć grę', 'How to start playing'));
    await follow(page, path('server-information'));
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Ta strona nie została jeszcze opublikowana.', 'This page has not been published yet.'));
    await follow(page, path('#jak-zaczac'));
  });

  await test.step('Fix the registration validation error and accept the legal terms', async () => {
    account = await newPlayer(page, lang, { showErrors: true });
    await expect(main.getByRole('status')).toHaveText(t(
      'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
      'Check your inbox for an e-mail with the next steps.'));
  });

  await test.step('Open the verification email and verify the account', async () => {
    await verifyPlayer(page, lang, account.email);
  });

  await test.step('Reject a wrong password, then sign in and see the account', async () => {
    await follow(page, path('login'));
    await signIn(page, { ...account, password: 'Incorrect local password' });
    await expect(main.getByRole('alert')).toBeVisible();
    await signIn(page, account);
    await expect(page).toHaveURL(path(''));
    await follow(page, '/account');
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Przegląd konta', 'Account overview'));
    await expect(main).toContainText(t('Aktywne', 'Active'));
    const db = pool();
    try {
      accountId = (await db.query('SELECT id FROM accounts WHERE email=$1', [account.email])).rows[0].id;
    } finally {
      await db.end();
    }
  });

  await test.step('Enroll an authenticator, save recovery codes and sign in again with TOTP', async () => {
    await follow(page, path('mfa'), 200, t('Uwierzytelnianie dwuskładnikowe', 'Two-factor authentication'));
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Bezpieczeństwo konta', 'Account security'));
    await submit(page, 'main form[action$="/mfa/enroll"]', { password: account.password });
    const secret = (await main.locator('.op-prose').innerText()).trim();
    totp = new TOTP({ secret: Secret.fromBase32(secret), algorithm: 'SHA1', digits: 6, period: 30 });
    lastTotpWindow = Math.floor(Date.now() / 30_000);
    await submit(page, 'main form[action$="/mfa/confirm"]', { password: account.password, code: totp.generate() });
    await expect(main.locator('textarea[readonly]').first()).toBeVisible();
    recoveryCodes = await main.locator('textarea[readonly]').allTextContents();
    await expect(main).toContainText(t('Zapisz te kody odzyskiwania', 'Save these recovery codes privately'));
    await follow(page, path('login'));
    await signIn(page, account);
    await challenge({ invalidFirst: true });
  });

  await test.step('Sign out and use a saved recovery code to sign in again', async () => {
    await page.waitForLoadState('load');
    const mobile = page.locator('#portal-menu-toggle:visible[aria-expanded=false]');
    if (await mobile.count()) await mobile.click();
    await page.locator('[data-account-link]:visible').click();
    await csrfRejected(page, path('logout'));
    await page.locator('dialog[open] form[action$="/logout"]').getByRole('button').click();
    await follow(page, path('login'));
    await signIn(page, account);
    await challenge({ recoveryCode: recoveryCodes[3] });
  });

  await test.step('Submit a character request and read the pending Game status', async () => {
    await follow(page, path('account/characters/new'));
    await follow(page, path('account/characters/suggest'));
    const form = main.locator('form[method=post]');
    await expect(form.locator('[name=name]')).toHaveValue(/^[A-Za-z]+(?: [A-Za-z]+)*$/);
    const world = form.locator('select[name=world]');
    await world.selectOption({ label: 'Local Bazaar World' });
    await form.locator('input[name=sex]').first().check();
    await form.locator('input[name=vocation]').first().check();
    const name = await form.locator('[name=name]').inputValue();
    await submit(page, 'main form[method=post]', { name: '12345' });
    await expect(main.getByRole('status')).toHaveText(t(
      'Sprawdź reguły imion i wybierz prawidłowy świat, płeć oraz profesję przed ponownym wysłaniem.',
      'Check the name rules and choose a valid world, sex and vocation before submitting again.'));
    await expect(world).toHaveValue(worldId);
    await submit(page, 'main form[method=post]', { name });
    await expect(page).toHaveURL(/\/characters\/requests\/[a-f0-9-]+$/);
    characterRequest = new URL(page.url()).pathname;
    await expect(main.getByRole('status')).toHaveText(t(
      'Trwa przetwarzanie prośby. Bez wyniku z gry rezultat pozostaje nieznany. Odśwież stronę, aby sprawdzić ponownie.',
      'Your request is being processed. Without a result from Game, its outcome is still unknown. Refresh this page to check again.'));
  });

  await test.step('Check the public online and deaths projections', async () => {
    await follow(page, path('online'), 503);
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Postacie online', 'Online characters'));
    await follow(page, path('deaths'), 503);
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Ostatnie zgony', 'Recent deaths'));
  });

  await test.step('Cancel and reject test checkouts, then make a successful top-up', async () => {
    await follow(page, path('shop'));
    for (const action of ['cancel', 'reject']) {
      await topUp(page, lang, action);
      await expect(main.getByRole('status')).toHaveText(t(
        'Płatność odrzucona lub anulowana. Nie przyznano coinów.',
        'Payment rejected or cancelled. No coins were credited.'));
      await expect(main.locator('dd').first()).toHaveText('0');
    }
    await topUp(page, lang, 'pay');
    await expect.poll(async () => {
      await page.reload();
      return main.locator('dd').first().innerText();
    }, { timeout: 45_000 }).toBe('1234');
    await follow(page, path('shop/history'));
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Saldo i historia', 'Balance and history'));
    await expect(main).toContainText('1234 / 0');
  });

  await test.step('Check Premium status and the store catalogue', async () => {
    await follow(page, path('panel/premium'));
    await expect(main.getByRole('heading', { level: 1 })).toHaveText('Premium');
    await expect(main).toContainText(t('Brak premium', 'No premium'));
    await expect(main.locator('form')).toHaveCount(0);
    await follow(page, path('shop'));
    await expect(main.getByRole('status')).toHaveText(t(
      'Operacja jest niedostępna. Powodzenie nie zostało potwierdzone.',
      'This operation is unavailable. No success has been confirmed.'));
    await csrfRejected(page, path('shop'), {
      operation: uuid7(), character: uuid7(), world: uuid7(),
      revision: 'missing', offer: 'missing', price: '1', quantity: '1',
    });
    await expect(main.locator('form[action$="/shop"]')).toHaveCount(0);
  });

  await test.step('List a Game item, sell it to a second player, and see both sides of the sale', async () => {
    const context = await browser.newContext({
      baseURL: new URL(page.url()).origin, ignoreHTTPSErrors: true,
      locale: lang, viewport: info.project.use.viewport,
    });
    const buyer = await context.newPage();
    buyer.setDefaultTimeout(30_000);
    const db = pool();
    try {
      await buyer.bringToFront();
      expect((await buyer.goto(path(''))).status()).toBe(200);
      await follow(buyer, '#jak-zaczac');
      const opponent = await newPlayer(buyer, lang);
      await verifyPlayer(buyer, lang, opponent.email);
      await follow(buyer, path('login'));
      await signIn(buyer, opponent);
      await expect(buyer).toHaveURL(path(''));
      await follow(buyer, '/account');
      await follow(buyer, path('shop/history'));
      await topUp(buyer, lang, 'pay');
      await expect.poll(async () => {
        await buyer.reload();
        return buyer.locator('main dd').first().innerText();
      }, { timeout: 45_000 }).toBe('1234');

      const buyerId = (await db.query('SELECT id FROM accounts WHERE email=$1', [opponent.email])).rows[0].id;
      expect((await buyer.request.get(characterRequest)).status()).toBe(404);
      const asset = uuid7();
      const sellerCharacter = uuid7();
      const destination = uuid7();
      const seeded = await page.request.post('/__bazaar_game/seed', { data: {
        seller: accountId, buyer: buyerId, kind: 'item', asset_id: asset,
        seller_character_id: sellerCharacter, buyer_character_id: destination,
      } });
      expect(seeded.status()).toBe(201);
      opponentCharacter = `Fixture ${destination.slice(-12)}`;
      await follow(buyer, '/account');
      const nameLink = buyer.locator('main').getByRole('link', { name: opponentCharacter, exact: true });
      await expect(nameLink).toBeVisible();
      const publicProfile = await nameLink.getAttribute('href');
      expect(publicProfile).toBe(path(`characters/${encodeURIComponent(opponentCharacter)}`));
      await follow(buyer, publicProfile, 503, opponentCharacter);

      await page.bringToFront();
      await follow(page, path('account/sudo'));
      await submit(page, 'main form', { current_password: account.password });
      await follow(page, path('account/bazaar'));
      await follow(page, path('account/bazaar/sell'));
      await main.locator('[name=kind]').selectOption('item');
      await main.locator('[name=currency]').selectOption('COINS');
      await submit(page, 'main form', {
        world_id: worldId, seller_character_id: sellerCharacter, asset_id: asset,
        expected_revision: '7', quantity: '1', definition_key: 'fixture-sword', definition_revision: 'v1',
        starting_bid: '100', duration_days: '1', publish_snapshot: true,
      });
      await expect(page).toHaveURL(/\/bazaar\/[a-f0-9-]+$/);
      const lot = new URL(page.url()).pathname;
      const listingId = lot.split('/').at(-1);
      const card = main.locator(`[data-listing="${listingId}"]`);
      await expect.poll(async () => {
        await page.reload();
        return card.getAttribute('data-state');
      }, { timeout: 45_000 }).toBe('active');

      await buyer.bringToFront();
      await follow(buyer, path('bazaar'));
      await follow(buyer, lot);
      const foreignCancel = await post(buyer, path(`account/bazaar/${listingId}/control`), {
        request_id: uuid7(), action: 'cancel',
      });
      expect(foreignCancel.status()).toBe(403);
      await expect(buyer.locator(`[data-listing="${listingId}"]`)).toHaveAttribute('data-state', 'active');
      await submit(buyer, 'main form[action$="/bids"]', { buyer_character_id: destination, maximum: '200' });
      await expect(buyer).toHaveURL(lot);
      expect((await db.query('SELECT amount FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2',
        [listingId, buyerId])).rows[0].amount).toBe('200');
      // Advance only this auction's clock after the buyer's funded browser bid.
      await db.query("UPDATE bazaar_listings SET ends_at=clock_timestamp()-interval '1 second' WHERE id=$1", [listingId]);
      const completed = t('Sprzedaż zakończona', 'Sale completed');
      await expect.poll(async () => {
        await page.reload();
        return card.getAttribute('data-state');
      }, { timeout: 45_000 }).toBe('completed');
      await expect(card).toContainText(completed);
      await buyer.reload();
      await expect(buyer.locator('main')).toContainText(completed);
      const receipt = await page.request.get(`/__bazaar_game/state?asset=${asset}`);
      expect(receipt.ok()).toBe(true);
      expect((await receipt.json()).asset.inbox).toBe(buyerId);
      await follow(page, path('shop'));
      await follow(page, path('shop/history'));
      await expect(main.locator('dd').first()).toHaveText('1324');
      await follow(buyer, path('shop'));
      await follow(buyer, path('shop/history'));
      await expect(buyer.locator('main dd').first()).toHaveText('1134');
    } finally {
      await context.close();
      await db.end();
    }
  });

  await test.step('Open party finder, puzzles and collection', async () => {
    await follow(page, path('party'), 200, t('Znajdź drużynę', 'Find a party'));
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Szukam drużyny', 'Looking for a team'));
    const filters = main.locator('form[method=get]');
    await filters.locator('[name=world]').fill(worldId);
    await filters.locator('[name=purpose]').selectOption('hunt');
    await filters.locator('[name=level]').fill('20');
    await filters.getByRole('button').click();
    await expect(page).toHaveURL(url => url.searchParams.get('level') === '20');
    await expect(main).toContainText(t('Brak ogłoszeń', 'No listings'));
    const now = Math.floor(Date.now() / 1000);
    const refusal = page.waitForResponse(response =>
      new URL(response.url()).pathname === path('party') && response.request().method() === 'POST' && response.request().isNavigationRequest());
    await submit(page, 'main form[method=post]', {
      character: uuid7(), min_level: '10', max_level: '40', starts_at: String(now + 60),
      expires_at: String(now + 3600), note: 'Looking for a hunt on the local fixture world.',
    });
    expect((await refusal).status()).toBe(403);
    await expect(main.getByRole('heading', { level: 1 })).toHaveText(t('Brak dostępu', 'Access denied'));
    await follow(page, path('puzzles'), 200, t('Codzienne zagadki', 'Daily puzzles'));
    await expect(main).toContainText(t('Dzisiejsze zagadki są niedostępne.', "Today's puzzles are unavailable."));
    await follow(page, path('collection'), 200, t('Kolekcja', 'Collection'));
    await expect(main).toContainText(t(
      'Kolekcja jest niedostępna. Wymagana jest weryfikacja aktualnego prywatnego postępu i właściciela postaci.',
      'Collection is unavailable. Current private progress and ownership must be verified.'));
  });

  await test.step('Create a support ticket and add a reply', async () => {
    await follow(page, path('support/tickets'));
    await follow(page, path('support/tickets/create'));
    const subject = 'Journey support request';
    await submit(page, 'main form', { subject, body: 'A local test player asks for help using the client.' });
    await expect(page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
    await expect(main).toContainText(subject);
    const reply = 'The local test player added a clarification.';
    await submit(page, 'main form[action$="/reply"]', { body: reply });
    await expect(main).toContainText(reply);
  });

  await test.step('Submit a private report about another player', async () => {
    await follow(page, path('support/reports'));
    await follow(page, path('support/reports/create'));
    await submit(page, 'main form', {
      target_reference: opponentCharacter,
      evidence_summary: 'Synthetic conduct report for this local browser journey.',
    });
    await expect(page).toHaveURL(/\/support\/reports\/[a-f0-9-]+$/);
    await expect(main).toContainText(opponentCharacter);
  });

  await test.step('Change the password and prove the old one is refused', async () => {
    await follow(page, path('password/change'));
    const next = password();
    await seedPasswordRange(next);
    await submit(page, 'main form', {
      current_password: account.password, password: next, password_confirmation: next,
    });
    await expect(page).toHaveURL(/\/login$/);
    await signIn(page, account);
    await expect(main.getByRole('alert')).toBeVisible();
    account.password = next;
    await signIn(page, account);
    await challenge({ recoveryCode: recoveryCodes[0] });
  });

  await test.step('Change email and confirm it from the new mailbox', async () => {
    await follow(page, path('account/recovery-key'));
    await submit(page, 'main form', { current_password: account.password });
    const key = (await main.locator('.op-code').innerText()).trim();
    await submit(page, 'main form', { current_password: account.password, recovery_key: key });
    await follow(page, path('account/security/email'), 200, t('Zmień adres e-mail', 'Change email address'));
    const email = `journey-changed-${crypto.randomUUID()}@local.test`;
    await submit(page, 'main form', {
      email, email_confirmation: email, current_password: account.password, proof: key, recovery: true,
    });
    const link = await mailLink(page, email, /https:\/\/localhost:18443\/email-change\/confirm\/[a-f0-9]{64}/, 75_000);
    expect((await page.goto(link)).status()).toBe(200);
    await submit(page);
    await expect(page).toHaveURL(/\/login$/);
    account.email = email;
    await signIn(page, account);
    await challenge({ recoveryCode: recoveryCodes[1] });
  });

  await test.step('Confirm the password and inspect the downloaded personal data', async () => {
    await follow(page, path('account/sudo'), 200, t('Potwierdź hasło', 'Confirm your password'));
    await submit(page, 'main form', { current_password: account.password });
    const exportLink = await visibleLink(page, path('account/privacy/export'));
    await expect(exportLink).toHaveText(t('Pobierz dane konta', 'Download account data'));
    await follow(page, path('account/privacy/export'));
    const [file] = await Promise.all([
      page.waitForEvent('download'),
      main.getByRole('button', { name: t('Pobierz dane konta', 'Download account data') }).click(),
    ]);
    expect(file.suggestedFilename()).toBe('account-data.json');
    const data = JSON.parse(await readFile(await file.path(), 'utf8'));
    expect(data.schema_version).toBe(1);
    expect(data.account).toMatchObject({ id: accountId, email: account.email, status: 'active', locale: lang });
    const serialized = JSON.stringify(data);
    expect(serialized.includes(account.password), 'export omits the password').toBe(false);
    expect(recoveryCodes.some(code => serialized.includes(code)), 'export omits MFA recovery codes').toBe(false);
  });

  await test.step('Request account closure, read the grace period, and cancel', async () => {
    await follow(page, path('account/privacy/delete'), 200, t('Zamknij konto', 'Close account'));
    const explanation = t('Poproś o usunięcie konta po okresie karencji.',
      'Request account deletion after the grace period.');
    await expect(main.getByRole('status')).toContainText(explanation);
    const confirmation = main.locator('[name=confirmation]');
    await confirmation.fill('WRONG');
    expect(await confirmation.evaluate(input => input.validity.patternMismatch)).toBe(true);
    await submit(page, 'main form', { current_password: account.password, confirmation: 'TERMINATE' });
    await expect(page).toHaveURL(/\/login$/);
    await signIn(page, account);
    await challenge({ recoveryCode: recoveryCodes[2] });
    await follow(page, path('account/sudo'));
    await submit(page, 'main form', { current_password: account.password });
    await follow(page, path('account/privacy/delete'));
    await expect(main.getByRole('status')).toHaveText(t(
      'Usunięcie konta jest zaplanowane. Możesz je anulować przed zakończeniem.',
      'Deletion is scheduled. You can cancel before finalization.'));
    await submit(page, 'main form[action$="/cancel"]', { current_password: account.password });
    await expect(main.getByRole('status')).toContainText(explanation);
  });

  await test.step('Close the account again and verify login fails after Game finalization', async () => {
    await submit(page, 'main form', { current_password: account.password, confirmation: 'TERMINATE' });
    await expect(page).toHaveURL(/\/login$/);
    const db = pool();
    try {
      // Advance only the grace period. Game still decides and publishes the receipt.
      const changed = await db.query(
        `UPDATE account_terminations SET created_at=now()-interval '15 days', scheduled_for=now()-interval '1 day'
         WHERE account_id=$1 AND cancelled_at IS NULL AND finalized_at IS NULL`, [accountId]);
      expect(changed.rowCount).toBe(1);
      await expect.poll(async () =>
        (await db.query('SELECT status FROM accounts WHERE id=$1', [accountId])).rows[0].status,
      { timeout: 90_000 }).toBe('closed');
    } finally {
      await db.end();
    }
    await signIn(page, account);
    await expect(page).toHaveURL(/\/login$/);
    await expect(main.getByRole('alert')).toBeVisible();
  });
});
