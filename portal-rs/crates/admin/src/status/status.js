(() => {
  const slot = document.getElementById('status-notices');
  const locale = document.documentElement.lang === 'pl' ? 'pl-PL' : 'en';
  const formatter = new Intl.DateTimeFormat(locale, {
    year: 'numeric', month: 'short', day: 'numeric', hour: 'numeric',
    minute: '2-digit', timeZoneName: 'short',
  });
  const localTime = (time, timestamp) => {
    time.dateTime = timestamp;
    time.textContent = formatter.format(new Date(timestamp));
    return time;
  };
  for (const time of document.querySelectorAll('time[data-local-time]')) {
    localTime(time, time.dateTime);
  }

  const staffArea = /^\/(admin|console)(\/|$)/.test(location.pathname);
  if (!slot || staffArea) return;
  const path = `/${slot.dataset.language}/status/notices`;
  async function showNotices() {
    const response = await fetch(path, {cache: 'no-store'});
    if (response.status !== 200) return;
    const notice = await response.json();
    const content = document.createElement('div');
    const heading = document.createElement('strong');
    heading.textContent = notice.title;
    const list = document.createElement('ul');
    for (const item of notice.items) {
      const row = document.createElement('li');
      const link = document.createElement('a');
      link.href = item.link;
      link.textContent = item.message;
      row.append(`${item.label} · `, link);
      if (item.starts && item.ends) {
        row.append(' · ', localTime(document.createElement('time'), item.starts),
          ' – ', localTime(document.createElement('time'), item.ends));
      }
      list.append(row);
    }
    content.append(heading, list);

    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'op-button op-button--secondary';
    button.textContent = notice.dismiss;
    button.addEventListener('click', async () => {
      button.disabled = true;
      try {
        const result = await fetch(path, {
          method: 'POST', body: new URLSearchParams({_csrf: notice.csrf, edition: notice.edition}),
        });
        if (result.status === 204) {
          slot.hidden = true;
          document.getElementById('main')?.focus();
        }
      } catch {
        // Keep the notice available if the dismissal request cannot be saved.
        slot.hidden = false;
      } finally {
        button.disabled = false;
      }
    });
    slot.setAttribute('role', 'region');
    slot.setAttribute('aria-label', notice.title);
    slot.replaceChildren(content, button);
    slot.hidden = false;
  }

  showNotices().catch(() => { slot.hidden = true; });
})();
