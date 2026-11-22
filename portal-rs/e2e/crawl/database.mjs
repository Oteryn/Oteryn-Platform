import { randomBytes } from 'node:crypto';

export function crawlDatabase(name, adminURL) {
  if (!/^portal_e2e_[a-f0-9]{12}$/.test(name)) throw new Error('Invalid crawl database name');
  const identifier = `"${name}"`;
  const password = randomBytes(32).toString('hex');
  const url = new URL(`/${name}`, adminURL);
  url.username = name;
  url.password = password;
  let roleRequested = false;
  return {
    url: url.href,
    ownerClause: ` OWNER ${identifier}`,
    get roleRequested() { return roleRequested; },
    async createRole(admin) {
      roleRequested = true;
      try {
        await admin.query(`
          CREATE ROLE ${identifier} LOGIN PASSWORD '${password}'
            NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS
        `);
      } catch (error) {
        if (error.code === '42710') roleRequested = false;
        throw error;
      }
    },
    async dropRole(admin) {
      if (roleRequested) await admin.query(`DROP ROLE IF EXISTS ${identifier}`);
    },
    async restrictAccess(admin) {
      await admin.query(`REVOKE CONNECT, TEMPORARY ON DATABASE ${identifier} FROM PUBLIC`);
    },
  };
}
