import { expect } from '../fixtures.mjs';
import { loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';
import { seedAccount, seedEntity } from '../security/auth-fixtures.mjs';
import { login } from '../helpers.mjs';

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
    await loginWithRecoveryCode(page, lang, account, code);
  } else {
    await login(page, lang, account);
  }
  await expect(page).toHaveURL(`/${lang}/`);
  return account;
}

export function snapshot(button) {
  return button.evaluate(submitter => {
    const form = submitter.form;
    const action = submitter.getAttribute('formaction') ?? form.getAttribute('action');
    return {
      url: new URL(action || location.href, document.baseURI).href,
      lang: navigator.language,
      fields: [...new FormData(form, submitter)],
    };
  });
}

export function replay(page, { url, lang, fields }) {
  return page.request.post(url, {
    data: new URLSearchParams(fields).toString(),
    headers: {
      'accept-language': lang,
      'content-type': 'application/x-www-form-urlencoded',
      origin: new URL(url).origin,
    },
    maxRedirects: 0,
  });
}

export async function submitForm(page, button, url) {
  const [response] = await Promise.all([
    page.waitForResponse(reply =>
      reply.url() === url && reply.request().method() === 'POST' &&
      reply.request().isNavigationRequest()
    ),
    button.click(),
  ]);
  return response;
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
