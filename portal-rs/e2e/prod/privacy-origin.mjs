function assertOrigin(url, origin) {
  if (new URL(url).origin !== origin) throw new Error('Seeded Privacy refused a foreign origin');
}

export function privacyFormAction(pageUrl, action, origin) {
  assertOrigin(pageUrl, origin);
  const target = new URL(action || pageUrl, pageUrl);
  assertOrigin(target, origin);
  return target.href;
}

export async function guardPrivacyOrigin(page, session, origin) {
  let foreignNavigation = false;
  let interceptionError = false;
  session.on('Fetch.requestPaused', async ({ requestId, request, resourceType }) => {
    try {
      if (new URL(request.url).origin === origin) {
        await session.send('Fetch.continueRequest', { requestId });
        return;
      }
      if (resourceType === 'Document' || !['GET', 'HEAD'].includes(request.method)) foreignNavigation = true;
      await session.send('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' });
    } catch {
      interceptionError = true;
    }
  });
  // Playwright routing only receives the initial request of a redirect chain.
  await session.send('Fetch.enable', { patterns: [{ urlPattern: 'http*' }] });
  return () => {
    if (foreignNavigation) throw new Error('Seeded Privacy refused a foreign origin');
    if (interceptionError) throw new Error('Seeded Privacy could not verify requests');
    assertOrigin(page.url(), origin);
  };
}
