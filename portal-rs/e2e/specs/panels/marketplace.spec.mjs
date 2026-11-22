import {createHash, randomBytes, randomUUID} from 'node:crypto';
import {expect, test} from '../../fixtures.mjs';
import {pool} from '../../helpers.mjs';

const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
const labels = {
  pl: {title: 'Aktywność na Bazarze', matched: 'Dopasowano', unavailable: /Operacje gry.*są niedostępne/},
  en: {title: 'Bazaar activity', matched: 'Matched', unavailable: /Live Game custody.*unavailable/},
};

async function actor(db, context, locale) {
  const {rows: [{id}]} = await db.query(
    `INSERT INTO accounts(email,password_hash,status,email_verified_at,locale)
     VALUES ($1,'marketplace-test-only','active',now(),$2) RETURNING id`,
    [`${randomUUID()}@example.test`, locale],
  );
  const session = randomBytes(32).toString('hex');
  const csrf = randomBytes(32).toString('hex');
  await db.query(
    `INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,authorization_generation)
     SELECT $1,id,now()+interval '15 minutes',now()+interval '1 hour','marketplace test',$3,native_security_generation
     FROM accounts WHERE id=$2`,
    [createHash('sha256').update(session).digest(), id, csrf],
  );
  return {id, csrf, signIn: () => context.addCookies([{
    name: '__Host-oteryn_session', value: session, url: origin, secure: true, httpOnly: true,
  }])};
}

test('players see only their Bazaar listing, bid and settlement records', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const label = labels[lang];
  const route = `/${lang}/account/bazaar/activity`;
  const db = pool();
  const item = randomUUID();
  const character = randomUUID();
  let seller, buyer, stranger;
  try {
    seller = await actor(db, page.context(), lang);
    buyer = await actor(db, page.context(), lang);
    stranger = await actor(db, page.context(), lang);
    await db.query(
      `INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at)
       VALUES ($1,$3,'item','auction','coins',113,false,now()),
              ($2,$3,'character','auction','coins',117,false,now())`,
      [item, character, seller.id],
    );
    await db.query(
      `INSERT INTO bazaar_bids(id,listing_id,account,maximum,idempotency_key,leader,price,ends_at)
       VALUES ($1,$2,$3,347,'panel-fixture',$3,117,now())`,
      [randomUUID(), character, buyer.id],
    );
    await db.query(
      `INSERT INTO bazaar_settlements(id,listing_id,buyer,seller,currency,amount,commission,seller_net,reservation,idempotency_key)
       VALUES ($1,$2,$3,$4,'coins',117,11,106,$5,'panel-fixture')`,
      [randomUUID(), character, buyer.id, seller.id, randomUUID()],
    );
    await db.query(
      `INSERT INTO bazaar_item_sales(id,listing_id,buyer,seller,gross_amount,fee_amount)
       VALUES ($1,$2,$3,$4,113,11)`,
      [randomUUID(), item, buyer.id, seller.id],
    );
  } finally {
    await db.end();
  }

  expect((await page.request.get(route, {maxRedirects: 0})).status()).toBe(303);
  await seller.signIn();
  expect((await page.goto(`/account?lang=${lang}`)).status()).toBe(200);
  const link = page.locator('nav[data-panel="player"]').getByRole('link', {name: label.title});
  await expect(link).toHaveAttribute('href', route);
  await link.click();
  await expect(page.getByRole('heading', {name: label.title})).toBeVisible();
  const records = (section, listing) => page.locator(`[data-activity="${section}"] [data-listing="${listing}"]`);
  await expect(records('listings', item)).toContainText('113');
  await expect(records('listings', character)).toContainText('117');
  await expect(records('settlements', character)).toContainText('117');
  await expect(records('settlements', item)).toContainText(label.matched);
  await expect(records('bids', character)).toHaveCount(0);
  await expect(page.locator('main [role=status]')).toHaveText(label.unavailable);
  await expect(page.locator('main form')).toHaveCount(0);
  await expect(page.locator('main time').first()).toHaveAttribute('datetime', /Z$/);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

  await buyer.signIn();
  expect((await page.goto(route)).status()).toBe(200);
  await expect(records('bids', character)).toContainText('347');
  await expect(records('settlements', character)).toBeVisible();
  await expect(records('listings', character)).toHaveCount(0);
  await records('bids', character).getByRole('link').click();
  await expect(page).toHaveURL(`${origin}${route}?listing=${character}`);
  expect((await page.request.get(`${route}?listing=invalid`)).status()).toBe(400);
  expect((await page.request.get(`${route}?account=${seller.id}`)).status()).toBe(400);
  expect((await page.request.post(route, {form: {}})).status()).toBe(403);
  expect((await page.request.post(route, {form: {_csrf: buyer.csrf}})).status()).toBe(405);

  await stranger.signIn();
  expect((await page.request.get(`${route}?listing=${character}`)).status()).toBe(404);
  expect((await page.goto(route)).status()).toBe(200);
  await expect(page.locator('main [data-listing]')).toHaveCount(0);
});

test('activity reports recorded delivery refusal and the latest cancellation request', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const route = `/${lang}/account/bazaar/activity`;
  const db = pool();
  const coins = randomUUID();
  const gold = randomUUID();
  const cancelled = randomUUID();
  const other = randomUUID();
  const refusal = 'CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE';
  const requestedAt = '2026-02-03T14:56:00Z';
  let seller, buyer;
  try {
    seller = await actor(db, page.context(), lang);
    buyer = await actor(db, page.context(), lang);
    await db.query(
      `INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at,state,updated_at)
       VALUES ($1,$5,'character','auction','coins',100,false,now(),'awaiting_delivery','2026-01-01'),
              ($2,$5,'item','auction','gold',100,false,now(),'awaiting_delivery','2026-01-01'),
              ($3,$5,'item','auction','coins',100,false,now(),'active','2026-01-01'),
              ($4,$5,'item','auction','coins',100,false,now(),'active','2026-02-02')`,
      [coins, gold, cancelled, other, seller.id],
    );
    await db.query(
      `INSERT INTO bazaar_authority_assets
        (listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,
         session_hash,quantity,publication_consent,input_binding,refusal_code,cancel_requested)
       SELECT id,id,id,id,id,1,decode(repeat('00',32),'hex'),1,true,'activity-fixture',
              CASE WHEN id = $2 THEN NULL ELSE $3 END,id = $2
       FROM bazaar_listings WHERE id=ANY($1::uuid[])`,
      [[coins, gold, cancelled], cancelled, refusal],
    );
    await db.query(
      `INSERT INTO bazaar_settlements(id,listing_id,buyer,seller,currency,amount,commission,seller_net,reservation,idempotency_key)
       VALUES ($1,$1,$2,$3,'coins',100,10,90,$1,'refused-sale')`,
      [coins, buyer.id, seller.id],
    );
    await db.query(
      `INSERT INTO bazaar_authority_gold_sales(id,listing_id,buyer,seller,gross_amount,fee_amount,seller_amount,hold_id,hold_revision)
       VALUES ($1,$1,$2,$3,100,3,97,$1,1)`,
      [gold, buyer.id, seller.id],
    );
    await db.query(
      `INSERT INTO bazaar_authority_browser_requests(request_id,listing_id,actor,action,created_at)
       VALUES ($1,$3,$4,'cancel',$5),($2,$3,$4,'resume','2026-03-01')`,
      [randomUUID(), randomUUID(), cancelled, seller.id, requestedAt],
    );
  } finally {
    await db.end();
  }

  await buyer.signIn();
  for (const listing of [coins, gold]) {
    expect((await page.goto(`${route}?listing=${listing}`)).status()).toBe(200);
    await expect(page.locator('[data-activity="settlements"] li')).toContainText(
      lang === 'pl' ? 'Dostawa odrzucona' : 'Delivery refused',
    );
    await expect(page.locator('[data-activity="settlements"] li')).not.toContainText(
      lang === 'pl' ? 'Oczekuje na rozliczenie' : 'Settlement pending',
    );
    await expect(page.locator('main')).not.toContainText(refusal);
  }
  expect((await page.request.get(`${route}?listing=${cancelled}`)).status()).toBe(404);

  await seller.signIn();
  expect((await page.goto(route)).status()).toBe(200);
  const newest = page.locator('[data-activity="listings"] li').first();
  await expect(newest).toHaveAttribute('data-listing', cancelled);
  await expect(newest).toContainText(lang === 'pl' ? 'Zlecono anulowanie' : 'Cancellation requested');
  await expect(newest.locator('time')).toHaveAttribute('datetime', requestedAt);
});
