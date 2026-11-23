import { checkMail, expect, founder, pathOf, stagingRequest, test } from './helpers.mjs';

test.afterEach(async ({ page }, info) => {
  if (info.status !== 'failed' || !/^\/(pl|en)\/reset-password$/.test(pathOf(page))) return;
  await page.screenshot({
    path: info.outputPath('reset-result.png'),
    mask: [page.locator('form, input, textarea, [href*="reset-password"]')],
  });
});

test('founder completes public password recovery', async ({ page, request }, info) => {
  test.skip(info.project.name !== 'en-1440', 'A single public recovery check uses the shared founder');
  await checkMail(request);
  const panel = await stagingRequest(request, '/admin');
  test.skip(panel.status() === 303, 'The team journey covers founder authentication when admin is available');
  expect(panel.status(), 'Admin entry point status').toBe(404);
  await founder(page, request, info.project.metadata.lang);
});
