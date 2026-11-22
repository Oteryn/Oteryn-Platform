export function fixtureDatabase() {
  const connectionString = process.env.PORTAL_E2E_DATABASE_URL || process.env.OTERYN_DATABASE_URL;
  const database = URL.parse(connectionString);
  const addressOptions = ['host', 'hostaddr', 'port', 'dbname', 'database'];
  if (!database
      || !['postgres:', 'postgresql:'].includes(database.protocol)
      || !['localhost', '127.0.0.1', '[::1]'].includes(database.hostname)
      || !database.pathname.startsWith('/portal_e2e_')
      || addressOptions.some(option => database.searchParams.has(option))) {
    throw new Error('Staff fixtures require a local portal_e2e_ database');
  }
  return database;
}
