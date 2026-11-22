import {test, expect} from '../fixtures.mjs';
import {submit} from '../helpers.mjs';
import {listing, post as postFromPage} from '../game-sim-helpers.mjs';
import {query, seededAccount, seededLogin, seededMfaAccount, seededStaff, seededSudo} from '../staff-rbac-helpers.mjs';

test('staff suspend and approve listing invalidation with audited reasons', async ({page, browser}, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const route = `/admin/${lang}/bazaar`;
  const contexts = [];
  const newPage = async () => {
    const context = await browser.newContext({
      baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
    });
    contexts.push(context);
    return context.newPage();
  };
  try {
    const sellerPage = await newPage();
    const buyerPage = await newPage();
    const seller = await seededMfaAccount(lang);
    const buyer = await seededAccount(lang);
    await seededLogin(buyerPage, lang, buyer);
    await seededLogin(sellerPage, lang, seller);
    await seededSudo(sellerPage, lang, seller);

    const {id} = await listing({page: sellerPage, id: seller.id}, {page: buyerPage, id: buyer.id}, lang);
    expect((await sellerPage.request.get(route, {maxRedirects: 0})).status()).toBe(403);

    const reportRoute = `/${lang}/account/bazaar/${id}/report`;
    expect((await buyerPage.goto(reportRoute)).status()).toBe(200);
    const report = await buyerPage.locator('main form').evaluate(node => Object.fromEntries(new FormData(node)));
    const refused = await postFromPage(buyerPage, reportRoute, {
      ...report, category: 'other', detail: 'Please review the details of this listing',
    });
    expect(refused.status()).toBe(403);
    expect(await query('SELECT id FROM bazaar_reports WHERE listing_id=$1', [id])).toEqual([]);

    const requester = await seededStaff(page, lang, 'moderator');
    const secondPage = await newPage();
    const approver = await seededStaff(secondPage, lang, 'moderator');
    expect((await page.goto(route)).status()).toBe(200);
    const form = action => `form:has(input[name=action][value="${action}"])`;
    const fields = async (screen, action) => screen.locator(form(action)).evaluate(node => Object.fromEntries(new FormData(node)));
    const post = (screen, data) => screen.request.post(route, {
      form: data, headers: {origin: new URL(screen.url()).origin}, maxRedirects: 0,
    });
    // The public report producer remains disabled. Seed a pending Platform report
    // for the active listing and its eligible, verified third-party reporter.
    const seeded = await query(
      `INSERT INTO bazaar_reports (id, listing_id, reporter, category, detail)
       SELECT $1, listing.id, reporter.id, 'other', 'Fixture report for staff review'
       FROM bazaar_listings listing JOIN accounts reporter ON reporter.id=$3
       WHERE listing.id=$2 AND listing.state='active' AND listing.owner<>reporter.id
         AND reporter.status='active' AND reporter.email_verified_at IS NOT NULL
       RETURNING id, reporter`,
      [report.id, id, buyer.id],
    );
    expect(seeded).toEqual([{id: report.id, reporter: buyer.id}]);
    expect(requester).not.toBe(buyer.id);
    expect((await page.reload()).status()).toBe(200);
    const review = `${form('review')}:has(input[name=id][value="${report.id}"])`;
    const reviewForm = page.locator(review);
    await expect(reviewForm).toHaveCount(1);
    const reviewFields = await reviewForm.evaluate(node => Object.fromEntries(new FormData(node)));
    expect((await post(page, {...reviewFields, accepted: 'true', reason: ' '})).status()).toBe(422);
    await reviewForm.locator('select[name=accepted]').selectOption('true');
    const decisionReason = 'Confirmed the issue with the listing details';
    await submit(page, review, {reason: decisionReason});
    await expect(reviewForm).toHaveCount(0);
    expect(await query(
      'SELECT state, reviewed_by FROM bazaar_reports WHERE id=$1', [report.id],
    )).toEqual([{state: 'accepted', reviewed_by: requester}]);
    expect(await query(
      "SELECT actor, before->'report' AS before, after->'report' AS after, after->>'reason' AS reason FROM audit_log WHERE action='bazaar.report.review' AND target=$1",
      [report.id],
    )).toEqual([{
      actor: requester,
      before: {state: 'pending', reviewed_by: null},
      after: {state: 'accepted', reviewed_by: requester},
      reason: decisionReason,
    }]);

    const suspendReason = 'Suspend listing for moderation review';
    expect((await post(page, {...await fields(page, 'suspend'), listing: id, reason: ' '})).status()).toBe(422);
    expect((await post(page, {...await fields(page, 'suspend'), listing: id, reason: suspendReason.padEnd(501)})).status()).toBe(422);
    await submit(page, form('suspend'), {listing: id, reason: suspendReason});
    await expect(page.locator('main').getByText(id, {exact: true})).toBeVisible();
    const requestId = (await fields(page, 'request')).id;
    const requestReason = 'Request invalidation after listing review';
    await submit(page, form('request'), {listing: id, reason: requestReason});
    const entry = screen => screen.locator(`[data-invalidation="${requestId}"]`);
    await expect(entry(page)).toHaveAttribute('data-request-state', 'pending');
    const approvalReason = 'Independent approval after listing review';
    expect((await post(page, {...await fields(page, 'approve'), id: requestId, reason: approvalReason})).status()).toBe(403);

    expect((await secondPage.goto(route)).status()).toBe(200);
    expect((await post(secondPage, {...await fields(secondPage, 'approve'), id: requestId, reason: ' '})).status()).toBe(422);
    const originalApproval = {...await fields(secondPage, 'approve'), id: requestId, reason: approvalReason};
    await submit(secondPage, form('approve'), {id: requestId, reason: approvalReason});
    await expect(entry(secondPage)).toHaveAttribute('data-request-state', 'approved');
    await expect(entry(secondPage)).toContainText(lang === 'pl' ? 'Zatwierdzono przez zespół' : 'Staff approved');
    await expect.poll(async () => {
      await secondPage.reload();
      return entry(secondPage).getAttribute('data-listing-state');
    }, {timeout: 45_000}).toBe('cancelled');
    await expect(entry(secondPage)).toContainText(lang === 'pl' ? 'Anulowana' : 'Cancelled');
    const outbox = () => query('SELECT * FROM bazaar_moderation_outbox WHERE id=$1', [requestId]);
    const recordedOutbox = await outbox();
    expect((await post(secondPage, originalApproval)).status()).toBe(303);
    expect(await outbox()).toEqual(recordedOutbox);

    const audit = await query(
      "SELECT actor, action, before, after FROM audit_log WHERE target=ANY($1) AND action IN ('bazaar.suspend','bazaar.invalidation.request','bazaar.invalidation.approve') ORDER BY created_at,id",
      [[id, requestId]],
    );
    expect(audit.map(row => [row.actor, row.action, row.after.reason])).toEqual([
      [requester, 'bazaar.suspend', suspendReason],
      [requester, 'bazaar.invalidation.request', requestReason],
      [approver, 'bazaar.invalidation.approve', approvalReason],
    ]);
    expect(audit[0].before.hold).toBeNull();
    expect(audit[2].before.invalidation[0].state).toBe('pending');
    expect(audit[2].after.invalidation[0].state).toBe('approved');
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});
