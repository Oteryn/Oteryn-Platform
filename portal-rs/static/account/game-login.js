'use strict';

const latestGameLogin = document.querySelector('[data-latest-game-login]');
if (latestGameLogin) {
  fetch(latestGameLogin.dataset.latestGameLogin, {
    mode: 'same-origin',
    cache: 'no-store',
    redirect: 'error',
    headers: {Accept: 'application/json'},
  }).then(async response => {
    if (!response.ok) return;
    const {entered_at} = await response.json();
    if (!Number.isSafeInteger(entered_at) || entered_at < 0) return;
    const iso = new Date(entered_at).toISOString();
    const time = document.createElement('time');
    time.dateTime = iso;
    time.textContent = iso.slice(0, 19).replace('T', ' ') + ' UTC';
    latestGameLogin.replaceChildren(time);
  }).catch(() => {
    return;
  });
}
