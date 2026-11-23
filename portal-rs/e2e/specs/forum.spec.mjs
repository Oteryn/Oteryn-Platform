import crypto from 'node:crypto';
import fs from 'node:fs';
import {test, expect} from '../fixtures.mjs';
import {mailLink, submit} from '../helpers.mjs';
import {action, query, seededAccount, seededLogin, seededStaff} from '../staff-rbac-helpers.mjs';

const form = (command) => `main form[action*="/actions/${command}"]`;
const postId = (page) => new URL(page.url()).hash.slice('#post-'.length);

async function openForum(page, info) {
  const lang = info.project.metadata.locale;
  await page.goto(`/${lang}/news`);
  if (info.project.use.viewport.width === 390) {
    await page.locator('.om-fallback-menu > summary').click();
  }
  const panel = page.locator('#portal-community-panel');
  await panel.locator('..').locator('summary').click();
  await panel.locator(`a[href="/${lang}/forum"]`).click({timeout: 5000});
  await expect(page).toHaveURL(`/${lang}/forum`);
}

async function responseFor(page, selector, override = {}) {
  const control = page.locator(selector).first();
  const fields = await control.evaluate((element) => Object.fromEntries(new FormData(element)));
  return page.request.post(await control.getAttribute('action'), {
    form: {...fields, ...override},
    headers: {origin: new URL(page.url()).origin},
    maxRedirects: 0,
  });
}

async function fitsViewport(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
    page.viewportSize().width,
  );
}

async function forumAccount(page, lang, handle) {
  const account = await seededAccount(lang);
  await query("UPDATE accounts SET created_at=clock_timestamp()-interval '2 days' WHERE id=$1", [account.id]);
  await seededLogin(page, lang, account);
  await page.goto(`/${lang}/forum/preferences`);
  await submit(page, form('name'), {handle});
  await expect(page.locator('input[name=handle]')).toHaveValue(handle);
  return account;
}

async function createBoard(page, name, visibility, position) {
  await page.goto('/admin/forum');
  await action(page, form('create-board'), {
    name_en: `EN ${name}`, name_pl: `PL ${name}`, visibility, position: String(position),
  });
  await expect(page).toHaveURL(/\/admin\/forum\/boards\/[\da-f-]+/);
  return new URL(page.url()).pathname.split('/').at(-1);
}

async function moderate(page, command, fields, post) {
  const scope = post ? `#post-${post}` : 'main';
  const selector = `form[action*="/actions/${command}"]`;
  const selected = `${scope} ${selector}`;
  await page.locator(`${scope} details:has(> ${selector}) > summary`).click();
  await action(page, selected, {reason: 'Community standards review', ...fields});
}

test('community forum is reachable from the portal', async ({page}, info) => {
  await openForum(page, info);
  await fitsViewport(page);
});

test('players publish, quote, subscribe and report while staff manage discussion', async ({page, browser}, info) => {
  test.setTimeout(330_000);
  const lang = info.project.metadata.locale;
  const root = `/${lang}/forum`;
  const unique = crypto.randomBytes(6).toString('hex');
  const title = `Quest planning ${unique}`;
  const writer = `Wayfarer_${unique}`;
  const responder = `Scout_${unique}`;
  const options = {
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
    viewport: info.project.use.viewport,
    locale: lang,
  };
  const staffContext = await browser.newContext(options);
  const team = {context: staffContext, page: await staffContext.newPage()};
  const reader = await browser.newContext(options);
  const replyPage = await reader.newPage();
  try {
    const staffId = await seededStaff(team.page, lang, 'platform_admin');
    await team.page.goto('/admin');
    await team.page.locator('main a[href^="/admin/forum"]').click();
    await expect(team.page).toHaveURL(/\/admin\/forum/);
    const board = await createBoard(team.page, `Adventures ${unique}`, 'public', 123);
    const signed = await createBoard(team.page, `Campfire ${unique}`, 'signed-in', 124);
    const restricted = await createBoard(team.page, `Operations ${unique}`, 'staff', 125);
    await team.page.goto(`/admin/forum/boards/${board}`);
    await action(team.page, form('edit-board'), {position: '122'});
    await expect(team.page.locator(`${form('edit-board')} [name=position]`)).toHaveValue('122');
    await query("UPDATE accounts SET created_at=clock_timestamp()-interval '2 days' WHERE id=$1", [staffId]);
    await team.page.goto('/admin/forum/preferences');
    await submit(team.page, form('name'), {handle: `Steward_${unique}`});
    await team.page.goto(`/admin/forum/boards/${restricted}`);
    await submit(team.page, form('new-thread'), {title: 'Operations notes', body: 'Private staff discussion.'});
    await expect(team.page.locator('main h1')).toHaveText('Operations notes');
    const staffThread = new URL(team.page.url()).pathname.split('/').at(-1);
    expect((await page.request.get(`${root}/threads/${staffThread}`)).status()).toBe(404);
    const boardUrl = `${root}/boards/${board}`;
    expect((await page.request.get(`${root}/boards/${signed}`)).status()).toBe(404);
    expect((await page.request.get(`${root}/boards/${restricted}`)).status()).toBe(404);
    await page.goto(boardUrl);
    await expect(page.locator('main h1')).toHaveText(`${lang.toUpperCase()} Adventures ${unique}`);

    const author = await forumAccount(page, lang, writer);
    const other = await forumAccount(replyPage, lang, responder);
    expect((await page.request.get(`${root}/boards/${signed}`)).status()).toBe(200);
    expect((await page.request.get(`${root}/boards/${restricted}`)).status()).toBe(404);
    await page.goto(boardUrl);
    expect((await responseFor(page, form('new-thread'), {
      _csrf: '', title: 'Unsigned form', body: 'A normal discussion',
    })).status()).toBe(403);
    expect((await responseFor(page, form('new-thread'), {title: '  ', body: 'Field validation'})).status()).toBe(422);
    await submit(page, form('new-thread'), {
      title, body: '**Looking for a party**\n\n[Quest notes](https://example.invalid/guide)\n\n<i>Meet at the gate</i>',
    });
    await expect(page).toHaveURL(/\/forum\/threads\/[\da-f-]+$/);
    const threadUrl = new URL(page.url()).pathname;
    const thread = threadUrl.split('/').at(-1);
    const first = (await page.locator('article.forum-post').getAttribute('id')).slice(5);
    const firstPost = page.locator(`#post-${first}`);
    await expect(firstPost.locator('strong')).toHaveText('Looking for a party');
    await expect(firstPost.locator('a[rel="nofollow ugc"]')).toHaveText('Quest notes');
    await expect(firstPost.locator('.forum-message')).toContainText('<i>Meet at the gate</i>');
    await firstPost.locator('.forum-author').click();
    await expect(page.locator('main h1')).toHaveText(writer);
    await page.goto(threadUrl);
    await page.locator(`#post-${first} details:has(> ${form('edit').replace('main ', '')}) > summary`).click();
    await submit(page, form('edit'), {body: '**Meet at noon**'});
    await expect(page.locator(`#post-${first} strong`)).toHaveText('Meet at noon');
    await expect(page.locator(`#post-${first} .op-badge`)).toContainText(lang === 'pl' ? 'Edytowano' : 'Edited');
    expect((await responseFor(page, form('subscribe'), {page: '0'})).status()).toBe(422);
    await submit(page, form('subscribe'));
    await expect(page.locator(`${form('subscribe')} [name=enabled]`)).toHaveValue('false');

    await replyPage.goto(boardUrl);
    await expect(replyPage.locator('li').filter({hasText: title}).locator('[data-unread]')).toBeVisible();
    await replyPage.bringToFront();
    await replyPage.locator(`main a[href="${threadUrl}"]`).click();
    await expect.poll(async () => (await query(
      'SELECT position FROM forum_read_markers WHERE account_id=$1 AND thread_id=$2', [other.id, thread],
    )).length).toBe(1);
    await replyPage.goto(boardUrl);
    await expect(replyPage.locator('li').filter({hasText: title}).locator('[data-unread]')).toHaveCount(0);
    await replyPage.goto(threadUrl);
    await replyPage.locator(`#post-${first} a[href*="quote="]`).click();
    await expect(replyPage.locator('#reply blockquote')).toContainText('Meet at noon');
    await expect.poll(async () => (await query(
      "SELECT extract(second FROM clock_timestamp())::int AS second",
    ))[0].second, {timeout: 35000, intervals: [1000]}).toBeLessThan(42);
    await submit(replyPage, form('reply'), {body: 'I will bring supplies.'});
    await expect(replyPage).toHaveURL(/#post-[\da-f-]+$/);
    const reply = postId(replyPage);
    await expect(replyPage.locator(`#post-${reply} blockquote`)).toContainText('Meet at noon');
    await submit(replyPage, form('reply'), {body: 'See you at the gate.'});
    await expect(replyPage.locator('main')).toContainText('See you at the gate.');
    expect((await query(
      'SELECT count(*)::int AS n FROM forum_mail_batches WHERE thread_id=$1 AND account_id=$2',
      [thread, author.id],
    ))[0].n).toBe(1);
    await mailLink(page, author.email, new RegExp(`https://localhost:18443/${lang}/forum/threads/${thread}`), 90000);

    await replyPage.goto(threadUrl);
    await submit(replyPage, form('reply'), {body: 'The camp is ready.'});
    const queuedMail = await query(
      `SELECT m.email_id FROM forum_mail_batches m JOIN outbox o ON o.idempotency_key=m.email_id::text
       WHERE m.thread_id=$1 AND m.account_id=$2 AND o.state='pending'`,
      [thread, author.id],
    );
    expect(queuedMail).toHaveLength(1);
    await page.goto(`${root}/preferences`);
    await submit(page, form('email'));
    await expect(page.locator(`${form('email')} [name=enabled]`)).toHaveValue('true');
    await submit(page, form('email'));
    await expect(page.locator(`${form('email')} [name=enabled]`)).toHaveValue('false');
    await query('UPDATE outbox SET run_after=now() WHERE idempotency_key=$1', [queuedMail[0].email_id]);
    await expect.poll(async () => (await query(
      `SELECT o.state,e.delivered_at FROM outbox o JOIN email_outbox e ON e.id::text=o.idempotency_key
       WHERE e.id=$1`, [queuedMail[0].email_id],
    ))[0]).toEqual({state: 'done', delivered_at: null});
    await submit(page, form('email'));
    await replyPage.goto(threadUrl);
    await submit(replyPage, form('reply'), {body: 'Update after opt-out.'});
    await expect(replyPage.locator('main')).toContainText('Update after opt-out.');
    expect((await query(
      'SELECT count(*)::int AS n FROM forum_mail_batches WHERE thread_id=$1 AND account_id=$2',
      [thread, author.id],
    ))[0].n).toBe(2);

    await page.goto(threadUrl);
    const reportForm = `#post-${reply} form[action$="/report"]`;
    const reportFields = {category: 'other', reason: 'Please review this reply.'};
    expect((await responseFor(page, reportForm, {...reportFields, reason: ' '})).status()).toBe(422);
    await page.locator(`#post-${reply} details:has(> form[action$="/report"]) > summary`).click();
    await action(page, reportForm, reportFields);
    await expect(page.locator('main [role=status]')).toBeVisible();
    const report = new URL(page.url()).searchParams.get('report');
    expect(report).toMatch(/^[\da-f-]+$/);
    expect((await responseFor(page, reportForm, {...reportFields, reason: 'Changed details.'})).status()).toBe(409);
    const retry = await responseFor(page, reportForm, reportFields);
    expect(retry.status()).toBe(303);
    expect(new URL(retry.headers().location, page.url()).searchParams.get('report')).toBe(report);
    await team.page.goto('/admin/moderation/reports');
    await team.page.locator(`main a[href="/admin/moderation/reports/${report}"]`).click();
    await expect(team.page.locator('main')).toContainText('Please review this reply.');
    await team.page.locator(`main a[href^="/admin/forum/posts/${reply}"]`).click();
    await expect(team.page).toHaveURL(/\/admin\/forum\/threads\//);
    await fitsViewport(team.page);
    await moderate(team.page, 'post-edit', {body: 'Moderated reply.'}, reply);
    await expect(team.page.locator(`#post-${reply}`)).toContainText('Moderated reply.');
    await moderate(team.page, 'post-delete', {}, reply);
    await replyPage.goto(threadUrl);
    await expect(replyPage.locator(`#post-${reply}`)).toHaveCount(0);
    await moderate(team.page, 'post-restore', {}, reply);
    await replyPage.goto(threadUrl);
    await expect(replyPage.locator(`#post-${reply}`)).toContainText('Moderated reply.');
    await moderate(team.page, 'pin', {});
    await expect(team.page.locator('main > .op-row .op-badge').first()).toContainText(lang === 'pl' ? 'Przypięty' : 'Pinned');

    const replyFields = await replyPage.locator(form('reply')).evaluate((el) => Object.fromEntries(new FormData(el)));
    await moderate(team.page, 'lock', {});
    await expect(team.page.locator('main > .op-row .op-badge').last()).toContainText(lang === 'pl' ? 'Zamknięty' : 'Locked');
    expect((await replyPage.request.post(`${root}/actions/reply`, {
      form: {...replyFields, body: 'Attempt after lock'},
      headers: {origin: new URL(replyPage.url()).origin}, maxRedirects: 0,
    })).status()).toBe(409);
    await moderate(team.page, 'move', {destination: signed});
    await expect(team.page.locator(`main nav a[href^="/admin/forum/boards/${signed}"]`)).toBeVisible();
    await moderate(team.page, 'thread-delete', {});
    expect((await page.request.get(threadUrl)).status()).toBe(404);
    await moderate(team.page, 'thread-restore', {});
    await page.goto(threadUrl);
    await expect(page.locator('main h1')).toHaveText(title);
    await fitsViewport(page);
    // Populate the page boundary using existing discussion data in the disposable database.
    await query(
      `WITH existing_posts AS (
         INSERT INTO forum_posts(thread_id,author_id,position,body)
         SELECT $1,$2,n,'Page boundary discussion '||n FROM generate_series(6,21) n
       )
       UPDATE forum_threads SET last_position=21 WHERE id=$1`,
      [thread, other.id],
    );
    await page.reload();
    const next = page.locator('main a[href$="&page=2"]');
    await next.click();
    await expect(page.locator('main .forum-message')).toContainText('Page boundary discussion 21');
    await fitsViewport(page);
    const auditRows = await query('SELECT action FROM audit_log WHERE target=ANY($1::text[]) AND action LIKE $2', [
      [board, thread, reply], 'forum.%',
    ]);
    expect(auditRows.map((row) => row.action)).toEqual(expect.arrayContaining([
      'forum.board.create', 'forum.board.edit', 'forum.post.edit', 'forum.post.delete', 'forum.post.restore',
      'forum.thread.pin', 'forum.thread.lock', 'forum.thread.move', 'forum.thread.delete', 'forum.thread.restore',
    ]));
  } finally {
    await reader.close();
    await team.context.close();
  }
});

test('a moderator can open the forum with only their assigned actions', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const staff = await seededStaff(page, lang, 'moderator');
  expect((await page.goto('/admin/forum')).status()).toBe(200);
  await expect(page.locator(form('create-board'))).toHaveCount(0);
  const alternate = lang === 'pl' ? 'en' : 'pl';
  await page.goto(`/admin/forum?lang=${alternate}`);
  await expect(page.locator('html')).toHaveAttribute('lang', alternate);
  await expect(page.locator(`footer a[hreflang="${lang}"]`)).toHaveAttribute('href', `/admin/forum?lang=${lang}`);
  await page.goto('/admin/forum/preferences');
  const token = await page.locator(`${form('name')} [name=_csrf]`).inputValue();
  const refused = await page.request.post('/admin/forum/actions/create-board', {
    form: {_csrf: token, name_en: 'Moderation', name_pl: 'Moderacja', visibility: 'public', position: '0'},
    headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
  expect(refused.status()).toBe(403);

  const [{id: thread}] = await query(
    `WITH board AS (
       INSERT INTO forum_boards(name_pl,name_en,position,visibility)
       VALUES('Ogłoszenia','Announcements',200,'public') RETURNING id
     )
     INSERT INTO forum_threads(board_id,author_id,title)
     SELECT id,$1,'Community announcements' FROM board RETURNING id`,
    [staff],
  );
  await page.goto(`/admin/forum/threads/${thread}?lang=${lang}&page=2`);
  const subscription = `/admin/forum/threads/${thread}?lang=${lang}&page=2`;
  for (const attempt of [1, 2]) {
    const response = await responseFor(page, form('subscribe'), {enabled: 'true'});
    expect(response.status(), `attempt ${attempt}`).toBe(303);
    expect(response.headers().location).toBe(subscription);
    await page.goto(subscription);
    await expect(page.locator(`${form('subscribe')} [name=enabled]`)).toHaveValue('false');
  }
  expect((await query(
    "SELECT count(*)::int AS n FROM audit_log WHERE target=$1 AND action='forum.subscription.edit'",
    [thread],
  ))[0].n).toBe(1);
});

test('i18n-parity for the forum and its navigation', async ({page}, info) => {
  const keys = (url) => fs.readFileSync(new URL(url, import.meta.url), 'utf8')
    .split('\n').filter((line) => /^[a-z][\w-]* = /.test(line)).map((line) => line.split(' = ')[0]).sort();
  const english = keys('../../templates/forum/en.ftl');
  expect(english.length).toBeGreaterThan(0);
  expect(keys('../../templates/forum/pl.ftl')).toEqual(english);
  const lang = info.project.metadata.locale;
  await openForum(page, info);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Forum społeczności' : 'Community forum');
});
