import {
  captureFailure, expect, mainForm, newEmail, newPassword, open, postForm, requireMail,
  signup, test, tr, verifyPlayer, withSharedQuota,
} from './helpers.mjs';

test('registration rejects mismatched passwords locally in both languages', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const malformed = await withSharedQuota(async () => {
    await open(page, `/${lang}/register`);
    const form = mainForm(page);
    return postForm(form, {
      email: newEmail(), password: newPassword(), password_confirmation: newPassword(),
      terms: await form.locator('[name=terms]').inputValue(),
      privacy: await form.locator('[name=privacy]').inputValue(),
    });
  });
  expect(malformed.status(), 'mismatched registration passwords').toBe(422);
  expect((await malformed.text()).includes(
    tr(lang, 'Popraw poniższe pola.', 'Please correct the fields below.'))).toBe(true);
});

test('valid registration reaches the check-inbox page', async ({ page }, info) => {
  try {
    await signup(page, info.project.metadata.locale);
  } catch (error) {
    if (info.expectedStatus !== 'skipped') await captureFailure(page, info).catch(() => {});
    throw error;
  }
});

test('verification mail activates a freshly registered player', async ({ page, staging }, info) => {
  const lang = info.project.metadata.locale;
  await requireMail(staging.api, info);
  await verifyPlayer(page, lang, await signup(page, lang));
});
