import { run } from '../api/harness.mjs';

// From e2e, with the disposable PostgreSQL admin URL and portal binary set:
// node character-create-nav/run.mjs --project=pl-1440 --project=en-390
const flags = [
  'OTERYN_CHARACTER_CREATE_WEB_ENABLED',
  'OTERYN_CHARACTER_COMMANDS_ENABLED',
  'OTERYN_CHARACTER_COMMAND_CREATE_ENABLED',
];

let result = 0;
for (const disabled of [undefined, ...flags]) {
  console.log(disabled ? `Character creation with ${disabled}=false` : 'Character creation with all route flags enabled');
  await run({
    config: 'character-create-nav/playwright.config.mjs',
    args: ['--grep', disabled ? '@route-disabled' : '@route-enabled', ...process.argv.slice(2)],
    portalOptions: {
      ...Object.fromEntries(flags.map(flag => [flag, flag === disabled ? 'false' : 'true'])),
      OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY: 'oteryn-game-e2e',
    },
  });
  if (process.exitCode === 130 || process.exitCode === 143) break;
  result ||= process.exitCode ?? 0;
  process.exitCode = result;
}
