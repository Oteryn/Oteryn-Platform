import assert from 'node:assert/strict';
import test from 'node:test';
import { fixtureProcessEnv } from './staff-fixture-database.mjs';

test('fixture commands use the validated URL without inherited PostgreSQL settings', () => {
  const database = 'postgresql://127.0.0.1:5433/portal_e2e_staff';
  assert.deepEqual(fixtureProcessEnv({
    PGHOSTADDR: '192.0.2.1',
    PGHOST: 'remote.example.invalid',
    PGPORT: '12345',
    PGDATABASE: 'another_database',
    PGUSER: 'another_user',
    PGSERVICE: 'another_service',
    PGOPTIONS: '-c search_path=another_schema',
    OTERYN_DATABASE_URL: 'postgresql://remote.example.invalid/another_database',
    PORTAL_E2E_DATABASE_URL: database,
    OTERYN_ENV: 'production',
    PATH: '/usr/bin',
  }), {
    OTERYN_DATABASE_URL: database,
    PORTAL_E2E_DATABASE_URL: database,
    OTERYN_ENV: 'testing',
    PATH: '/usr/bin',
  });
});
