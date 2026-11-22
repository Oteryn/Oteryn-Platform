import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { waitForWikiBudget } from '../helpers.mjs';

const { routes } = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));
const heads = new Map(routes.filter(({ listener, method }) => listener === 'public' && method === 'HEAD')
  .map(route => [route.path, route.registrations[0].source]));

export async function mirrorHead(request, pattern, target, get, expected = get.status()) {
  const source = heads.get(pattern);
  if (!source) throw new Error(`No HEAD registration for ${pattern}`);
  await test.step(`HEAD ${pattern}`, async () => {
    await waitForWikiBudget(target);
    const head = await request.head(target, { maxRedirects: 0 });
    try {
      expect(head.status(), `HEAD ${target} (${source})`).toBe(expected);
      expect((await head.body()).length).toBe(0);
      if (expected === get.status()) expect(head.headers().location).toBe(get.headers().location);
    } finally {
      await head.dispose();
    }
  });
}
