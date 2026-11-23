import crypto from "node:crypto";
import {spawnSync} from 'node:child_process';
import {mkdtempSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {expect} from '@playwright/test';
import {submit} from './helpers.mjs';
import {staffState} from './staff-rbac-helpers.mjs';
export async function staffContext(browser,info) {
 return browser.newContext({baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',ignoreHTTPSErrors:true,storageState:await staffState(browser,info)});
}
export async function token(page,lang) {
 await page.goto(`/${lang}/shop/coins`);
 return page.locator('main input[name="_csrf"]').first().inputValue();
}
export async function paidTopup(page,lang,db) {
 await page.goto(`/${lang}/shop/coins`);
 const operation=await page.locator('main form[action$="/shop/coins"] [name=operation]').first().inputValue();
 await submit(page,'main form[action$="/shop/coins"]');
 // Actual bank checkout form is the inline Askama template in store/src/web.rs.
 await submit(page,'main form[action*="/shop/test-checkout/"]');
 const row=(await db.query('SELECT payment_intent_id,coins FROM store_web_topups WHERE operation_id=$1',[operation])).rows[0];
 expect(row,'checkout creates payment intent').toBeTruthy();
 await expect.poll(async()=>Number((await db.query('SELECT count(*)::int n FROM wallet_entries WHERE operation_id=$1',[row.payment_intent_id])).rows[0].n),{message:'bank checkout credits wallet',timeout:30000}).toBe(1);
 return row;
}
export function uuid7() {
 const hex=Date.now().toString(16).padStart(12,'0');
 return `${hex.slice(0,8)}-${hex.slice(8)}-7000-8000-${crypto.randomUUID().replaceAll('-','').slice(-12)}`;
}

export function catalogCommand(args, account) {
  const result = spawnSync(process.env.PORTAL_E2E_BINARY, ['catalog', ...args, '--actor', account], {
    encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'],
  });
  expect(result.status, 'catalog CLI succeeded').toBe(0);
  return result.stdout.trim();
}

export function importGameCatalog(account, offer, price) {
  const directory = mkdtempSync(join(tmpdir(), 'portal-commerce-catalog-'));
  try {
    const path = join(directory, 'catalog.json');
    writeFileSync(path, JSON.stringify({
      schema: 'OTERYN_STORE_RUNTIME_CATALOGUE/v1',
      source: {fixture: offer},
      offers: [{key: offer, category: 'items', title: 'Exercise sword', price,
        currency: 'TIBIA_COIN', item: {key: 'oteryn:item.tibia.i28552', revision: 'definition-r1', charges: 1},
        source_line: 1}],
    }));
    return catalogCommand(['import', path], account);
  } finally {
    rmSync(directory, {recursive: true});
  }
}
