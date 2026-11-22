import { waitForLoginBudget } from '../helpers.mjs';
import { postForm } from '../security/auth-fixtures.mjs';

export const loginForm = 'main form[action$="/login"]';
export const challengeForm = 'main form[action$="/mfa/challenge"]';
export const logoutForm = 'form[action$="/logout"]';

export async function submitForm(page, selector, fields) {
  try {
    return await postForm(page, selector, fields);
  } catch {
    // Playwright transport diagnostics can include request headers and form values.
    throw new Error('Portal form exchange failed for ' + selector);
  }
}

export async function signIn(page, account) {
  await waitForLoginBudget();
  return submitForm(page, loginForm, { email: account.email, password: account.password });
}
