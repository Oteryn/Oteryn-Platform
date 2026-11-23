// Never serialize Playwright request steps, config, or raw transport failures.
function redact(value) {
  let result = String(value);
  for (const name of ['STAGING_CF_ACCESS_CLIENT_ID', 'STAGING_CF_ACCESS_CLIENT_SECRET']) {
    const secret = process.env[name];
    if (secret) result = result.replaceAll(secret, '[redacted]');
  }
  return result.replace(/(\/reset-password\/)[a-f0-9]{64}/gi, '$1[token]')
    .replace(/(token=)[a-f0-9]{64}/gi, '$1[token]');
}

export default class StagingReporter {
  onBegin(_config, suite) {
    console.log(`Staging black-box suite, ${suite.allTests().length} cases`);
  }

  onStdOut(chunk) {
    process.stdout.write(redact(chunk));
  }

  onTestEnd(test, result) {
    const project = test.parent.project()?.name;
    const blocker = test.annotations.find(({ type }) => type === 'skip')?.description;
    console.log(`[${project}] ${result.status.toUpperCase()} ${test.title}${blocker ? ` (${blocker})` : ''}`);
    for (const error of result.errors) {
      console.log(redact(error.message || error.value || 'Unknown test error'));
      if (error.location) console.log(redact(`  at ${error.location.file}:${error.location.line}`));
    }
    for (const attachment of result.attachments.filter(({ name }) => name === 'screenshot')) {
      console.log(redact(`  screenshot ${attachment.path}`));
    }
  }

  onError(error) {
    console.error(redact(error.message || error.value || 'Unknown Playwright error'));
  }
}
