import crypto from "node:crypto";
import path from 'node:path';
import {expect} from '@playwright/test';
import {submit} from './helpers.mjs';
export async function staffContext(browser,info) {
 return browser.newContext({baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',ignoreHTTPSErrors:true,storageState:path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY,`staff-${info.project.name}.json`)});
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
