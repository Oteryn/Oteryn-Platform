(() => {
  const status = document.querySelector('[data-outcome="pending"]');
  const check = document.querySelector('[data-character-request-check]');
  if (!status || !check) return;

  const url = new URL(check.href, window.location.href);
  if (url.origin !== window.location.origin) return;
  let attempts = 0;
  const startedAt = Date.now();

  async function refresh() {
    if (Date.now() - startedAt >= 300000) return;
    attempts += 1;
    try {
      const response = await fetch(url.href, {
        credentials: 'same-origin',
        cache: 'no-store',
        signal: AbortSignal.timeout(5000),
        headers: { Accept: 'text/html' },
      });
      if (response.redirected || [401, 403, 404, 429].includes(response.status)) return;
      if (response.ok) {
        const page = new DOMParser().parseFromString(await response.text(), 'text/html');
        const outcome = page.querySelector('[data-outcome]')?.dataset.outcome;
        if (outcome === 'created' || outcome === 'rejected') {
          window.location.replace(url.href);
          return;
        }
      }
    } catch {
      // A failed read proves no outcome. The existing request remains available for manual checks.
    }
    if (attempts < 60 && Date.now() - startedAt < 300000) window.setTimeout(refresh, 5000);
  }

  window.setTimeout(refresh, 5000);
})();
