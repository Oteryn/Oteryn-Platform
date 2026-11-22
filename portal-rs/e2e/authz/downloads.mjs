import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

const bundle = new URL('../../crates/downloads/tests/fixtures/origin-v1/stable/', import.meta.url);
const policyHash = createHash('sha256').update(readFileSync(new URL('policy-v1.json', bundle))).digest('hex');
const cases = [
  ['/updates/{channel}/metadata/{filename}', '/updates/stable/metadata/1.root.json', 'metadata/1.root.json'],
  ['/updates/{channel}/policy-v1.json', '/updates/stable/policy-v1.json', 'policy-v1.json'],
  ['/updates/{channel}/targets/{*target}', `/updates/stable/targets/${policyHash}.channels/stable/policy-v1.json`, 'policy-v1.json'],
];
export const downloadRoutes = cases.map(([path]) => ['GET', path]);

export async function exerciseDownloads(request, routes) {
  const [{ available }] = await query(`
    SELECT EXISTS(
      SELECT 1 FROM client_channels c JOIN client_releases r ON r.id=c.active_publication_id WHERE c.name='stable'
    ) AS available
  `);
  if (!available) {
    const [{ account_id: publisher }] = await query(`
      WITH publisher AS (
        INSERT INTO accounts(id,email,password_hash,status,locale)
        VALUES(uuidv7(),'authz-download-publisher@example.invalid','disabled-operator-fixture','active','en')
        RETURNING id
      )
      INSERT INTO account_roles(account_id,role_id)
      SELECT publisher.id,roles.id FROM publisher CROSS JOIN roles WHERE roles.key='platform_admin'
      RETURNING account_id
    `);
    if (process.env.OTERYN_ENV !== 'testing' || !process.env.PORTAL_E2E_BINARY) {
      throw new Error('The signed download fixture requires the disposable portal binary');
    }
    try {
      execFileSync(process.env.PORTAL_E2E_BINARY, [
        'downloads-publish', '--channel', 'stable', '--actor', publisher, '--dir', fileURLToPath(bundle),
      ], {
        env: {
          ...process.env, OTERYN_DOWNLOADS_ENABLED: 'true',
          OTERYN_DOWNLOADS_ROOT_STABLE: fileURLToPath(new URL('metadata/1.root.json', bundle)),
        },
        stdio: 'ignore',
      });
    } catch {
      throw new Error('The signed stable download fixture could not be published');
    }
  }
  for (const [path, target, file] of cases) {
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === 'GET' && r.path === path)
      .registrations[0];
    expect(guard.kind).toBe('anonymous');
    await test.step(`GET ${path}`, async () => {
      const response = await request.get(target, { maxRedirects: 0 });
      try {
        expect(response.status(), source).toBe(200);
        expect((await response.body()).equals(readFileSync(new URL(file, bundle))), 'published signed bytes match').toBe(true);
      } finally {
        await response.dispose();
      }
    });
  }
}
