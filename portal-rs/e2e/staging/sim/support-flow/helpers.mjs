import { createHmac, randomBytes } from 'node:crypto';
import { setTimeout as pause } from 'node:timers/promises';
import { expect } from '@playwright/test';

const origin = 'https://staging.oteryn.com';
const accessHeaders = {
  'CF-Access-Client-Id': process.env.STAGING_CF_ACCESS_CLIENT_ID,
  'CF-Access-Client-Secret': process.env.STAGING_CF_ACCESS_CLIENT_SECRET,
};
const mailList = '/__mailpit/api/v1/messages?limit=100';
const identities = new WeakMap();

export const runId = () => randomBytes(7).toString('hex');
export const pathOf = page => new URL(page.url()).pathname;
export const formFor = (page, action) => page.locator(`main form[action$="${action}"]`);
export const adminForm = (page, action) => page.locator(`main form:has(input[name=action][value="${action}"])`);

export class MailUnavailable extends Error {}
export class SharedBudget extends Error {}
class AccountLost extends Error {}
class MailMissing extends Error {}

export async function ready(request) {
  await expect.poll(async () => {
    try {
      const response = await request.get('/readyz', { headers: accessHeaders, maxRedirects: 0, timeout: 8000 });
      return response.status();
    } catch {
      return 'connection failed';
    }
  }, { message: 'Staging GET /readyz returns 200', timeout: 300_000, intervals: [5000] }).toBe(200);
}

export async function attachAccessRouting(context) {
  await context.route('**/*', async route => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.protocol !== 'https:' || url.host !== 'staging.oteryn.com') {
      await route.continue();
      return;
    }
    // Forward one hop. Headers supplied to route.continue would carry over
    // to redirects. This keeps Access credentials out of browser requests.
    let response;
    try {
      try {
        response = await route.fetch({
          headers: { ...request.headers(), ...accessHeaders },
          maxRedirects: 0,
          timeout: 20000,
        });
      } catch {
        await route.abort('failed');
        return;
      }
      await route.fulfill({ response });
    } catch (error) {
      // Navigation can cancel a pending subresource while the proxy is fetching it.
      if (!/Route is already handled|Target page, context or browser has been closed/.test(error.message)) throw error;
    } finally {
      await response?.dispose().catch(() => {});
    }
  });
}

export async function secondContext(browser, info) {
  const { baseURL, locale, viewport, proxy, serviceWorkers } = info.project.use;
  const context = await browser.newContext({ baseURL, locale, viewport, proxy, serviceWorkers });
  await attachAccessRouting(context);
  return context;
}

export async function visit(page, path, status = 200) {
  const response = await page.goto(path, { waitUntil: 'domcontentloaded' });
  if (status === 200 && /\/login$/.test(pathOf(page)) && !/\/login$/.test(path)) {
    const identity = identities.get(page.context());
    if (identity) {
      if (!(await login(page, identity.language, identity))) {
        throw new AccountLost('A previously authenticated account no longer accepts its test password.');
      }
      if (identity.staff && pathOf(page).endsWith('/mfa/challenge')) {
        await completeMfa(page, identity.language, identity.email);
      }
      return visitAfterLogin(page, path);
    }
  }
  expect(response.status(), `GET ${path.split('?')[0]}`).toBe(status);
  return response;
}

async function visitAfterLogin(page, path) {
  const response = await page.goto(path, { waitUntil: 'domcontentloaded' });
  expect(response.status(), `GET ${path.split('?')[0]} after signing in again`).toBe(200);
  expect(pathOf(page)).not.toMatch(/\/login$/);
  return response;
}

export async function onceAfterRedeploy(info, page, run) {
  for (let attempt = 0; attempt < 2; attempt++) {
    try {
      return await run();
    } catch (error) {
      if (!(error instanceof AccountLost) || attempt === 1) throw error;
      identities.delete(page.context());
      await page.context().clearCookies();
      info.annotations.push({ type: 'staging redeploy', description: error.message });
    }
  }
}

export async function click(form, fields = {}) {
  for (const [name, value] of Object.entries(fields)) {
    const field = form.locator(`[name="${name}"]`);
    if (typeof value === 'boolean') await field.setChecked(value);
    else if (await field.evaluate(element => element.tagName === 'SELECT')) await field.selectOption(value);
    else await field.fill(value);
  }
  await form.locator('button').first().click();
}

async function submitObserved(page, form, fields) {
  const action = await form.evaluate(element => new URL(element.action).pathname);
  const startUrl = page.url();
  let challengeLimit;
  const trackChallenge = response => {
    if (new URL(response.url()).pathname.endsWith('/register/abuse/challenge') && response.status() === 429) {
      challengeLimit = response;
    }
  };
  page.on('response', trackChallenge);
  try {
    for (let attempt = 0; attempt < 3; attempt++) {
      challengeLimit = undefined;
      const [response] = await Promise.all([
        page.waitForResponse(event => event.request().method() === 'POST' && new URL(event.url()).pathname === action),
        click(form, fields),
      ]);
      const budgetResponse = challengeLimit ?? (response.status() === 429 ? response : undefined);
      if (!budgetResponse) return response;
      // Other simulations can use the same ingress source budget.
      if (attempt === 2) throw new SharedBudget('Shared staging ingress kept rate-limiting an ordinary submission.');
      await waitForBudget(budgetResponse);
      await page.goto(startUrl, { waitUntil: 'domcontentloaded' });
    }
  } finally {
    page.off('response', trackChallenge);
  }
}

async function waitForBudget(response) {
  const seconds = Number(await response.headerValue('retry-after'));
  await pause((seconds > 0 ? Math.min(seconds, 60) : 60) * 1000 + 1000);
}

export async function captureForm(form) {
  return form.evaluate(element => ({
    path: new URL(element.action).pathname,
    fields: Object.fromEntries(new FormData(element)),
  }));
}

// Use the fields and CSRF token from the real form for browser-equivalent
// replays and for validating limits that HTML controls enforce before submit.
export async function post(page, form, overrides = {}) {
  try {
    const options = {
      form: { ...form.fields, ...overrides },
      headers: { ...accessHeaders, origin },
      maxRedirects: 0,
      timeout: 15000,
    };
    let response = await page.request.post(form.path, options);
    if (!form.path.endsWith('/support/reports') && response.status() === 429) {
      const seconds = Number(response.headers()['retry-after']);
      await pause((seconds > 0 ? Math.min(seconds, 60) : 60) * 1000 + 1000);
      response = await page.request.post(form.path, options);
      if (response.status() === 429) throw new SharedBudget('Shared staging ingress kept rate-limiting an ordinary submission.');
    }
    return response;
  } catch (error) {
    if (error instanceof SharedBudget) throw error;
    throw new Error(`POST ${form.path} failed at the HTTP transport.`);
  }
}

async function mailGet(request, path, timeout = 10_000) {
  try {
    return await request.get(path, { headers: accessHeaders, timeout, maxRedirects: 0 });
  } catch {
    throw new MailUnavailable('Mailpit GET failed at the HTTP transport.');
  }
}

export async function checkMailbox(request, timeout) {
  const list = await mailGet(request, mailList, timeout);
  if ([404, 405].includes(list.status())) {
    throw new MailUnavailable(`GET /__mailpit/api/v1/messages returned HTTP ${list.status()}.`);
  }
  expect(list.status(), 'Mailpit message list').toBe(200);
  return list;
}

async function newestMessage(request, address, since, subjects, timeout) {
  const list = await checkMailbox(request, timeout);
  const { messages } = await list.json();
  await list.dispose();
  return (messages ?? [])
    .sort((a, b) => Date.parse(b.Created) - Date.parse(a.Created))
    .slice(0, 100)
    .find(message => Date.parse(message.Created) >= since
      && message.To?.some(to => to.Address.toLowerCase() === address.toLowerCase())
      && (subjects.length === 0 || subjects.includes(message.Subject)));
}

export async function newestMail(request, address, since, { subject, includes } = {}) {
  const deadline = Date.now() + 60_000;
  const subjects = subject ? [subject].flat() : [];
  while (deadline - Date.now() > 1000) {
    const remaining = () => Math.min(10_000, Math.max(1000, deadline - Date.now()));
    const newest = await newestMessage(request, address, since, subjects, remaining());
    if (newest) {
      const response = await mailGet(request, `/__mailpit/api/v1/message/${encodeURIComponent(newest.ID)}`, remaining());
      expect(response.status(), 'Mailpit message detail').toBe(200);
      const message = await response.json();
      if (!includes || message.Text?.includes(includes)) return message;
    }
    await pause(Math.min(2500, Math.max(0, deadline - Date.now())));
  }
  throw new MailMissing('No matching message for the test recipient within 60 seconds.');
}

function mailLink(message, kind) {
  const source = `${message.Text ?? ''}\n${message.HTML ?? ''}`.replaceAll('&amp;', '&');
  for (const candidate of source.match(/https?:\/\/[^\s"'<>]+/g) ?? []) {
    if (!URL.canParse(candidate)) continue;
    const link = new URL(candidate);
    if (!['staging.oteryn.com', 'localhost', '127.0.0.1'].includes(link.hostname)) continue;
    const valid = kind === 'verify'
      ? /^\/(en|pl)\/verify-email$/.test(link.pathname) && /^[a-f0-9]+$/.test(link.searchParams.get('token'))
      : /^\/(en|pl)\/reset-password\/[A-Za-z0-9_-]+$/.test(link.pathname);
    if (valid) return new URL(link.pathname + link.search, origin).href;
  }
  throw new Error(`The ${kind} email has no staging confirmation link.`);
}

async function openMailLink(page, message, kind) {
  let status;
  try {
    status = (await page.goto(mailLink(message, kind), { waitUntil: 'domcontentloaded' })).status();
  } catch {
    throw new Error(`Could not navigate to the ${kind} confirmation from email.`);
  }
  expect(status, `${kind} confirmation opens`).toBe(200);
}

const freshPassword = () => `N7!a-${randomBytes(28).toString('base64url')}`;

async function login(page, language, { email, password }) {
  await visit(page, `/${language}/login`);
  await submitObserved(page, formFor(page, '/login'), { email, password });
  return !pathOf(page).endsWith('/login');
}

async function captureFormFailure(page, info, name, status) {
  if (status < 500) return;
  const panel = page.locator('main, h1').first();
  if (await panel.isVisible()) {
    await panel.screenshot({ path: info.outputPath(`${name}-${status}.png`) });
  }
}

export async function register(page, language, run, number, info) {
  const email = `sim-support-flow-${run}-${number}@example.test`;
  const password = freshPassword();
  const opened = await page.goto(`/${language}/register`, { waitUntil: 'domcontentloaded' });
  await captureFormFailure(page, info, 'registration-get', opened.status());
  expect(opened.status(), 'GET registration opens the form').toBe(200);
  const date = Date.now() - 2000;
  const response = await submitObserved(page, formFor(page, '/register'), {
    email, password, password_confirmation: password, terms: true, privacy: true,
  });
  // Keep the expected redirect assertion even when staging serves its refusal page.
  if (response.status() === 503) {
    const unavailable = language === 'pl'
      ? 'Rejestracja jest obecnie niedostępna. Spróbuj ponownie później.'
      : 'Registration is currently unavailable. Please try again later.';
    await expect(page.getByText(unavailable, { exact: true })).toBeVisible();
  }
  await captureFormFailure(page, info, 'registration-post', response.status());
  expect(response.status(), 'POST registration reaches the inbox confirmation').toBe(303);
  await expect(page).toHaveURL(`/${language}/register/check-inbox`);
  const mail = await newestMail(page.request, email, date);
  await openMailLink(page, mail, 'verify');
  const verify = page.locator('main form[action*="/verify-email?"]');
  if (await verify.count()) await click(verify);
  const identity = { email, password, language };
  expect(await login(page, language, identity), 'the verified player can sign in').toBe(true);
  identities.set(page.context(), identity);
  return identity;
}

function totp(secret) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  const key = [];
  let pending = 0;
  let bits = 0;
  for (const char of secret.toUpperCase().replace(/[\s=]/g, '')) {
    const value = alphabet.indexOf(char);
    if (value === -1) throw new Error('Invalid authenticator enrollment key.');
    pending = (pending << 5) | value;
    bits += 5;
    if (bits >= 8) {
      bits -= 8;
      key.push((pending >> bits) & 255);
    }
  }
  const counter = Buffer.alloc(8);
  counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
  const hash = createHmac('sha1', Buffer.from(key)).update(counter).digest();
  return String((hash.readUInt32BE(hash.at(-1) & 15) & 0x7fffffff) % 1_000_000).padStart(6, '0');
}

async function completeMfa(page, language, email, secret) {
  await expect(page).toHaveURL(`/${language}/mfa/challenge`);
  let code;
  if (secret) {
    // Enrollment may consume the current TOTP counter.
    await pause(30_000 - Date.now() % 30_000 + 300);
    code = totp(secret);
  } else {
    const since = Date.now() - 2000;
    await click(formFor(page, '/mfa/email'));
    const subject = language === 'pl' ? 'Twój kod weryfikacyjny Oteryn' : 'Your Oteryn verification code';
    const mail = await newestMail(page.request, email, since, { subject });
    code = mail.Text?.match(/^\s*(\d{6})\s*$/m)?.[1];
    if (!code) throw new Error('The MFA email has no verification code.');
  }
  await click(formFor(page, '/mfa/challenge'), { code });
  expect(pathOf(page), 'staff completes the verification challenge').not.toMatch(/\/mfa\/challenge$/);
}

export async function staffSignIn(page, language, role, info) {
  if (!['support', 'moderator'].includes(role)) throw new Error('This simulation owns two staff roles.');
  const email = `staff-${role}@example.invalid`;
  const password = freshPassword();
  // Delivery uses the seeded account locale. A new request within 60 seconds
  // of the previous one can return the normal notice without issuing another link.
  const subject = ['Zresetuj hasło do Oteryn', 'Reset your Oteryn password'];
  const previous = await newestMessage(page.request, email, Date.now() - 61_000, subject);
  if (previous) await pause(Math.max(0, Date.parse(previous.Created) + 61_000 - Date.now()));
  await visit(page, `/${language}/forgot-password`);
  const since = Date.now() - 2000;
  const requested = await submitObserved(page, formFor(page, '/forgot-password'), { email });
  expect(requested.status(), 'public password-reset request returns to the form').toBe(302);
  let mail;
  try {
    mail = await newestMail(page.request, email, since, { subject });
  } catch (error) {
    if (!(error instanceof MailMissing)) throw error;
    throw new MailUnavailable(`No password reset email for the seeded ${role} account within 60 seconds.`);
  }
  await openMailLink(page, mail, 'reset');
  const resetLanguage = pathOf(page).split('/')[1];
  const reset = await submitObserved(page, formFor(page, '/reset-password'), { email, password, password_confirmation: password });
  await captureFormFailure(page, info, `${role}-password-reset`, reset.status());
  expect(reset.status(), 'password reset accepts the new password').toBe(302);
  expect(await reset.headerValue('location') === `/${resetLanguage}/login`, 'successful password reset redirects to sign-in').toBe(true);
  expect(await login(page, language, { email, password }), 'staff can sign in after public password reset').toBe(true);
  if (pathOf(page).endsWith('/mfa/challenge')) {
    await completeMfa(page, language, email);
  } else {
    await visit(page, `/${language}/mfa`);
    const enroll = formFor(page, '/mfa/enroll');
    if (await enroll.count()) {
      await click(enroll, { password });
      const secret = await page.locator('main .op-prose').textContent();
      await click(formFor(page, '/mfa/confirm'), { password, code: totp(secret) });
      await expect(page.locator('main textarea[readonly]').first()).toBeVisible();
      expect(await login(page, language, { email, password }), 'staff signs in after MFA enrollment').toBe(true);
      await completeMfa(page, language, email, secret);
    }
  }
  identities.set(page.context(), { email, password, language, staff: true });
}

export async function playerId(page, language, player) {
  await visit(page, `/${language}/account/sudo`);
  const form = page.locator('main form[method=post]');
  await click(form, { current_password: player.password });
  await visit(page, `/${language}/account/privacy/export`);
  const exported = await post(page, await captureForm(form));
  expect(exported.status(), 'the player can read their own account data').toBe(200);
  const { account } = await exported.json();
  expect(account.id).toMatch(/^[a-f0-9-]{36}$/);
  return account.id;
}
