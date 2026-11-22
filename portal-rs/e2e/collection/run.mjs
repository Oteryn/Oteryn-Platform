import { run } from '../api/harness.mjs';

const args = process.argv.slice(2);
const readDisabled = args[0] === '--read-disabled';
if (readDisabled) args.shift();

await run({
  config: 'collection/playwright.config.mjs',
  args: ['--grep', readDisabled ? '@read-disabled' : '@read-enabled', ...args],
  portalOptions: {
    OTERYN_COLLECTION_ENABLED: 'true',
    PORTAL_PLAYERDATA_G10_INGEST_ENABLED: 'true',
    PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED: 'true',
    PORTAL_PLAYERDATA_G10_SOURCE_AUTHORITY: 'OTERYN_GAME_JOURNAL',
    PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY: 'oteryn-game-e2e',
    PORTAL_PLAYERDATA_JOURNAL_FAMILIES: 'ITEMS,bestiary',
    // The harness only passes allowlisted environment keys. This flag is
    // absent from that list so the disabled run exercises the factory default.
    ...(readDisabled ? {} : { OTERYN_COLLECTION_JOURNAL_READ_ENABLED: 'true' }),
  },
});
