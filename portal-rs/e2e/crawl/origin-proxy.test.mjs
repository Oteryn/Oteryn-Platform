import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import https from 'node:https';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { chromium, request } from '@playwright/test';
import { originProxySettings, startOriginProxy } from './origin-proxy.mjs';

async function listen(server) {
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  return `https://localhost:${server.address().port}`;
}

test('Chromium and its API clients cannot follow redirects outside the fixture', { timeout: 30_000 }, async () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'crawl-origin-'));
  let browser, api, proxy, fixture, outsideServer;
  try {
    const key = path.join(directory, 'key.pem');
    const cert = path.join(directory, 'cert.pem');
    execFileSync('openssl', ['req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256',
      '-nodes', '-keyout', key, '-out', cert, '-days', '1', '-subj', '/CN=localhost'], { stdio: 'ignore' });
    const tls = { key: fs.readFileSync(key), cert: fs.readFileSync(cert) };
    let outsideConnections = 0;
    outsideServer = https.createServer(tls, (_request, response) => {
      response.end('outside');
    });
    outsideServer.on('connection', () => { outsideConnections += 1; });
    const outside = await listen(outsideServer);
    fixture = https.createServer(tls, (incoming, response) => {
      if (incoming.url === '/same-origin') response.writeHead(302, { Location: '/page' });
      if (incoming.url === '/first-hop') response.writeHead(307, { Location: '/second-hop' });
      if (incoming.url === '/second-hop') response.writeHead(307, { Location: outside });
      response.end('fixture');
    });
    const origin = await listen(fixture);
    proxy = await startOriginProxy(origin);
    const options = { baseURL: origin, proxy: originProxySettings(proxy.url), ignoreHTTPSErrors: true };
    browser = await chromium.launch({
      executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium',
      args: ['--no-sandbox'],
    });
    const context = await browser.newContext(options);
    const page = await context.newPage();
    assert.equal((await page.goto('/same-origin')).url(), `${origin}/page`);
    await assert.rejects(page.goto(outside), /ERR_TUNNEL_CONNECTION_FAILED/);
    await assert.rejects(page.goto('/first-hop'), /ERR_TUNNEL_CONNECTION_FAILED/);

    api = await request.newContext(options);
    for (const client of [context.request, api]) {
      assert.equal((await client.get('/same-origin')).url(), `${origin}/page`);
      const rejected = await client.post('/first-hop', { form: { probe: 'local-sentinel' } });
      assert.equal(rejected.status(), 403);
    }
    assert.equal(outsideConnections, 0);
  } finally {
    await api?.dispose();
    await browser?.close();
    proxy?.close();
    for (const server of [fixture, outsideServer]) {
      if (!server?.listening) continue;
      server.closeAllConnections();
      await new Promise(resolve => server.close(resolve));
    }
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
