import { expect } from '../fixtures.mjs';
import { loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';
import { seedAccount, seedEntity } from '../security/auth-fixtures.mjs';
import { login } from '../helpers.mjs';

export async function withFixtureCredentials(operation) {
  try {
    await operation();
  } catch {
    // Playwright action errors can include the supplied password or code.
    throw new Error('Browser authentication with fixture credentials failed');
  }
}

export async function signIn(page, lang, role) {
  const account = await seedAccount(lang);
  if (role) {
    const assigned = await query(
      'INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2 RETURNING account_id',
      [account.id, role],
    );
    expect(assigned).toEqual([{ account_id: account.id }]);
    const seeded = await seedEntity('account-mfa', { accountId: account.id, password: account.password });
    const code = seeded.recoveryCodes[0];
    expect(Boolean(code), 'seeded staff recovery code exists').toBe(true);
    await withFixtureCredentials(() => loginWithRecoveryCode(page, lang, account, code));
  } else {
    await withFixtureCredentials(() => login(page, lang, account));
  }
  await expect(page).toHaveURL(`/${lang}/`);
  return account;
}

export function snapshot(button) {
  return button.evaluate(submitter => {
    const form = submitter.form;
    const action = submitter.getAttribute('formaction') ?? form.getAttribute('action');
    const target = new URL(action || location.href, document.baseURI);
    if (target.origin !== location.origin) throw new Error('Expected a same-origin form action');
    return {
      url: target.href,
      lang: navigator.language,
      fields: [...new FormData(form, submitter)],
    };
  });
}

export async function replay(page, { url, lang, fields }) {
  const origin = new URL(page.url()).origin;
  if (new URL(url).origin !== origin) throw new Error('Expected a same-origin form replay');
  try {
    return await page.request.post(url, {
      data: new URLSearchParams(fields).toString(),
      headers: {
        'accept-language': lang,
        'content-type': 'application/x-www-form-urlencoded',
        origin,
      },
      maxRedirects: 0,
    });
  } catch {
    throw new Error('Browser form replay failed at the transport layer');
  }
}

function isFormNavigation(request, url) {
  return request.url() === url && request.method() === 'POST' && request.isNavigationRequest();
}

export async function submitForm(page, button, url) {
  const [response] = await Promise.all([
    page.waitForResponse(reply => isFormNavigation(reply.request(), url)),
    button.click(),
  ]);
  return response;
}

export function parallelSubmit(page, button, submission) {
  const duplicate = page.waitForRequest(request => isFormNavigation(request, submission.url))
    .then(() => replay(page, submission));
  return Promise.all([submitForm(page, button, submission.url), duplicate]);
}

export function expectRedirect(response, status, target) {
  expect(response.status()).toBe(status);
  expect(response.headers().location).toBe(target);
}

export async function reloadGet(page) {
  const response = await page.reload();
  expect(response.request().method()).toBe('GET');
  expect(response.status()).toBe(200);
}

export async function expectCsrfRefusal(response, lang) {
  expect(response.status()).toBe(403);
  expect(response.headers()['content-language']).toBe(lang);
  expect(await response.text()).toBe(
    lang === 'pl' ? 'Nie masz uprawnień do wyświetlenia tej strony.' : 'You do not have permission to view this page.',
  );
}
