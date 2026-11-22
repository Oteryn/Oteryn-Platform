import assert from 'node:assert/strict';
import { once } from 'node:events';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { requestGate } from './request-gate.mjs';

test('the smoke gate fetches distinct pagination queries and caches by method', async () => {
  const scratch = await mkdtemp(join(tmpdir(), 'portal-live-cache-'));
  const budget = join(scratch, 'budget.json');
  const requests = [];
  const server = createServer((req, res) => {
    requests.push([req.method, req.url]);
    res.writeHead(req.url === '/en/events?page=2' ? 500 : 200).end();
  });
  try {
    await writeFile(budget, '[0,0]');
    server.listen(0, '127.0.0.1');
    await once(server, 'listening');
    const origin = `http://127.0.0.1:${server.address().port}`;
    const gate = requestGate(new Set([origin]), 'en', 'portal-smoke-regression', budget);
    const first = new URL('/en/events?page=1', origin);
    const second = new URL('/en/events?page=2', origin);
    assert.equal((await gate(first)).status, 200);
    assert.equal((await gate(second)).status, 500);
    assert.equal((await gate(first, 'HEAD')).status, 200);
    assert.equal((await gate(second)).status, 500);
    assert.deepEqual(requests, [
      ['GET', '/en/events?page=1'],
      ['GET', '/en/events?page=2'],
      ['HEAD', '/en/events?page=1'],
    ]);
  } finally {
    await new Promise(resolve => server.close(resolve));
    await rm(scratch, { recursive: true, force: true });
  }
});
