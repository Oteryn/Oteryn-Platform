import { basename } from 'node:path';

// Browser assertions can capture recovery codes, token URLs and session fields.
// Keep those assertion values out of terminal and CI output.
export default class MutationReporter {
  onTestEnd(test, result) {
    const name = test.titlePath().filter(Boolean).join(' / ');
    process.stdout.write(`${result.status} ${name}\n`);
    for (const error of result.errors) {
      const location = error.location;
      if (location) {
        process.stdout.write(`  ${basename(location.file)}:${location.line}:${location.column}\n`);
      }
      const message = (error.message || '').replace(/\x1b\[[0-9;]+m/g, '');
      for (const line of message.split('\n')) {
        if (/^\s*(Expected|Received): (\d{3}|true|false|null|undefined)$/.test(line)) {
          process.stdout.write(`${line}\n`);
        }
      }
    }
  }

  onError(error) {
    const location = error.location;
    const source = location ? ` at ${basename(location.file)}:${location.line}:${location.column}` : '';
    process.stdout.write(`Matrix runner error${source}\n`);
  }
}
