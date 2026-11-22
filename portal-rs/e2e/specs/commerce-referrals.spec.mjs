import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, mailLink, pool, submit } from '../helpers.mjs';
import { postAs, readForm, signInPlayer } from '../commerce-helpers.mjs';

const messages = {
  "en": {
    "title": "Tell a friend",
    "link": "Your unlimited invitation link",
    "email": "E-mail address",
    "send": "Invite a new player",
    "buddy": "Bring back a buddy",
    "hint": "Up to 25 e-mails per 24 hours. Resends start 14 days after the first attempt. Buddies must have no website or game activity for 12 months; up to 20 invitations per 30 days.",
    "rewards-hint": "New accounts receive a 7-day cosmetic for their first 3 characters. Recruiter outfit: 1 paying invitee; addons: 3 and 10. Each 6 premium loyalty points earn you 1 point. A returning buddy's first qualifying premium purchase within 90 days grants you 5 days for 30–89 days purchased, or 10 days for 90 or more.",
    "status": "Invitations",
    "rewards": "Rewards",
    "empty": "No entries yet.",
    "new": "New player",
    "waiting": "Waiting for a paid purchase",
    "pending": "Pending game delivery",
    "starter": "Starter cosmetic: first 3 characters, 7 days",
    "loyalty": "Referral loyalty points",
    "notice": "If the address is eligible, the invitation is queued. No account details are disclosed.",
    "invalid": "Check the e-mail address and try again.",
    "denied": "This invitation cannot be accepted.",
    "accept": "Accept invitation",
    "accept-hint": "Continue to registration or sign in, then open your referrals page. Only eligible new accounts can accept a new-player invitation.",
    "create-link": "Create referral link"
  },
  "pl": {
    "title": "Poleć znajomemu",
    "link": "Twój link do zaproszeń bez limitu",
    "email": "Adres e-mail",
    "send": "Zaproś nowego gracza",
    "buddy": "Zaproś znajomego do powrotu",
    "hint": "Maksymalnie 25 e-maili w ciągu 24 godzin. Ponowne wysłanie od 14. dnia po pierwszej próbie. Powracający znajomy nie może być aktywny na stronie ani w grze przez 12 miesięcy; limit 20 zaproszeń w ciągu 30 dni.",
    "rewards-hint": "Nowe konta otrzymują wygląd kosmetyczny dla pierwszych 3 postaci na 7 dni. Strój rekrutera: 1 polecone płacące konto; dodatki: 3 i 10 kont. Za każde 6 punktów lojalności z premium otrzymasz 1 punkt. Pierwszy zakup premium przez powracającego znajomego w ciągu 90 dni daje Ci 5 dni za zakup 30–89 dni albo 10 dni za zakup co najmniej 90 dni.",
    "status": "Zaproszenia",
    "rewards": "Nagrody",
    "empty": "Brak wpisów.",
    "new": "Nowy gracz",
    "waiting": "Oczekiwanie na płatny zakup",
    "pending": "Oczekuje na dostawę w grze",
    "starter": "Początkowy wygląd: pierwsze 3 postacie, 7 dni",
    "loyalty": "Punkty lojalności za polecenia",
    "notice": "Jeśli adres spełnia warunki, zaproszenie trafi do kolejki. Dane kont nie są ujawniane.",
    "invalid": "Sprawdź adres e-mail i spróbuj ponownie.",
    "denied": "Nie można przyjąć tego zaproszenia.",
    "accept": "Przyjmij zaproszenie",
    "accept-hint": "Przejdź do rejestracji lub zaloguj się, a następnie otwórz stronę poleceń. Zaproszenie nowego gracza mogą przyjąć tylko uprawnione nowe konta.",
    "create-link": "Utwórz link polecający"
  }
};

test('player creates a referral link, queues an invitation and accepts the link once', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const l = messages[lang];
  const path = `/${lang}/account/referrals`;
  const guest = await page.request.get(path, { maxRedirects: 0 });
  expect(guest.status()).toBe(303);
  expect(guest.headers().location).toBe(`/${lang}/login`);
  const inviter = await signInPlayer(page, lang);
  const visitor = await browser.newContext({ ...info.project.use });
  const db = pool();
  try {
    expect((await page.goto(path)).status()).toBe(200);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(l.title);
    for (const key of ['hint', 'rewards-hint', 'status', 'rewards']) {
      await expect(page.getByText(l[key], { exact: true })).toBeVisible();
    }
    await expect(page.getByText(l.empty, { exact: true })).toHaveCount(2);
    await expect(page.getByText(`${l.loyalty}: 0`, { exact: true })).toBeVisible();

    const create = page.getByRole('button', { name: l['create-link'] });
    const newInvite = 'form:has(input[name="kind"][value="new"])';
    const returningInvite = 'form:has(input[name="kind"][value="returning"])';
    const invitation = await readForm(page.locator(newInvite));
    await csrfRejected(page, invitation.url, { kind: 'link' });
    await create.click();
    const link = await page.getByLabel(l.link).inputValue();
    expect(new URL(link).pathname).toMatch(new RegExp(`^/${lang}/referrals/[a-f0-9-]+$`));
    await expect(create).toHaveCount(0);

    const invalid = await postAs(page, invitation.url, { ...invitation.fields, email: 'invalid' });
    expect(invalid.status()).toBe(400);
    expect((await invalid.text()).includes(l.invalid), 'localized invalid e-mail message').toBe(true);
    await expect(page.locator(newInvite).getByLabel(l.email)).toBeVisible();
    await expect(page.locator(newInvite).getByRole('button')).toHaveText(l.send);
    const target = `referral-${randomUUID()}@local.test`;
    await submit(page, newInvite, { email: target });
    await expect(page.getByRole('status')).toHaveText(l.notice);
    await mailLink(page, target, /https:\/\/localhost:18443\/(?:pl|en)\/referrals\/[a-f0-9-]+/, 90_000);

    await expect(page.locator(returningInvite).getByLabel(l.email)).toBeVisible();
    await expect(page.locator(returningInvite).getByRole('button')).toHaveText(l.buddy);
    await submit(page, returningInvite, { email: inviter.email });
    await expect(page.getByRole('status')).toHaveText(l.notice);

    await page.goto(link);
    await expect(page.getByText(l['accept-hint'])).toBeVisible();
    await expect(page.getByRole('button', { name: l.accept })).toBeVisible();
    const refused = page.waitForNavigation();
    await submit(page);
    expect((await refused).status()).toBe(403);
    await expect(page.getByRole('status')).toHaveText(l.denied);

    const invitee = await visitor.newPage();
    await invitee.goto(link);
    await submit(invitee);
    await expect(invitee).toHaveURL(new RegExp(`/${lang}/register$`));
    const invited = await signInPlayer(invitee, lang);
    for (let attempt = 0; attempt < 2; attempt++) {
      await invitee.goto(link);
      await submit(invitee);
      await expect(invitee).toHaveURL(new RegExp(`${path}$`));
    }
    await expect(invitee.getByText(`${l.starter}: 3 — ${l.pending}`)).toBeVisible();
    await page.goto(path);
    await expect(page.getByText(`${l.new} — ${l.waiting}`)).toBeVisible();
    expect((await db.query(
      'SELECT inviter_id,kind FROM referral_campaign_invitations WHERE invitee_id=$1', [invited.id],
    )).rows).toEqual([{ inviter_id: inviter.id, kind: 'new' }]);
    expect((await db.query(
      "SELECT count(*)::int AS count FROM referral_campaign_rewards WHERE account_id=$1 AND kind='starter'",
      [invited.id],
    )).rows[0].count).toBe(1);
  } finally {
    await visitor.close();
    await db.end();
  }
});
