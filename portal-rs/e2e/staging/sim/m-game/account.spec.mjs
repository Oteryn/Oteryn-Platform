import { setTimeout as delay } from 'node:timers/promises';
import {
  check, expect, labels, mailLink, mailStatus, newestMail, register, signIn, test,
} from './helpers.mjs';

test('Fresh player registration and sign-in', async ({ request, screen }, info) => {
  const { lang } = info.project.metadata;
  const t = labels(lang);
  await check(await screen.goto(`/${lang}/login`), 200, info);
  const registration = screen.locator(`main a[href="/${lang}/register"]`);
  await registration.click();
  await expect(screen).toHaveURL(new RegExp(`/${lang}/register$`));
  await expect(screen.locator('main h1')).toHaveText(t('Utwórz konto', 'Create an account'));
  await screen.locator('main form button[type=submit]').click();
  await expect(screen).toHaveURL(new RegExp(`/${lang}/register$`));
  await expect(screen.locator('main input[name=email]:invalid')).toBeVisible();

  let player = await register(screen, lang);
  if (player.response.status() === 429) {
    await delay(65_000);
    await screen.goto(`/${lang}/register`);
    player = await register(screen, lang);
  }
  test.skip(player.response.status() === 429, 'Shared staging registration budget was unavailable after retry.');
  if (player.response.status() !== 303) {
    await screen.waitForLoadState('domcontentloaded');
    try {
      const heading = screen.locator('main h1');
      await heading.waitFor({ state: 'visible', timeout: 5_000 });
      await heading.screenshot({ path: info.outputPath('registration.png'), timeout: 8_000 });
    } catch {
      info.annotations.push({ type: 'evidence', description: 'The registration response did not yield a visible heading screenshot.' });
    }
  }
  await check(player.response, 303, info);
  await expect(screen).toHaveURL(new RegExp(`/${lang}/register/check-inbox$`));
  await expect(screen.locator('main')).toContainText(t(
    'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
    'Check your inbox for an e-mail with the next steps.',
  ));
  await check(await screen.reload(), 200, info);

  await signIn(screen, lang, player);
  await expect(screen).toHaveURL(new RegExp(`/${lang}/login$`));
  await expect(screen.locator('main')).toContainText(t(
    'Podane dane logowania są nieprawidłowe.', 'The provided credentials are invalid.',
  ));

  const availability = await mailStatus(request);
  if ([404, 405].includes(availability)) {
    test.skip(true, `Mailpit returned ${availability}. Verification is blocked.`);
  }
  expect(availability).toBe(200);

  try {
    const message = await newestMail(request, player.email, /weryfik|potwierdź|verif/i, player.since);
    await screen.goto(mailLink(message, /^(?:\/(?:en|pl))?\/verify-email$/));
    await screen.getByRole('button', { name: t('Potwierdź adres e-mail', 'Verify e-mail address') }).click();
    await expect(screen.locator('main')).toContainText(t('Twój adres e-mail został zweryfikowany.', 'Your e-mail has been verified.'));
  } catch {
    throw new Error('The staging verification e-mail could not be confirmed.');
  }
  await signIn(screen, lang, player);
  await expect(screen).toHaveURL(new RegExp(`/${lang}/$`));
  await check(await screen.goto(`/${lang}/account`), 200, info);
  await expect(screen).toHaveURL(new RegExp(`/${lang}/account$`));
});
