import { request } from '@playwright/test';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { mailList, requestTo, stagingOrigin, stagingProxy } from './transport.mjs';

export default async function setup() {
  delete process.env.OTERYN_BLACKBOX_MAIL_BLOCKER;
  const api = await request.newContext({ baseURL: stagingOrigin, proxy: stagingProxy });
  try {
    const response = await requestTo(api, 'GET', mailList);
    const status = response.status();
    if (status === 404 || status === 405) {
      process.env.OTERYN_BLACKBOX_MAIL_BLOCKER = `GET ${mailList} returned HTTP ${status}`;
    }
    console.log(`GET ${mailList} returned HTTP ${status}`);
  } catch (error) {
    console.log(error.message);
  } finally {
    await api.dispose();
  }

  const directory = await mkdtemp(join(tmpdir(), 'oteryn-staging-registration-'));
  process.env.OTERYN_BLACKBOX_SLOTS = directory;
  return () => rm(directory, { recursive: true, force: true });
}
