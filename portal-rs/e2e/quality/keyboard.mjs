import { expect } from '@playwright/test';

async function focusState(page) {
  return page.evaluate(() => {
    const element = document.activeElement;
    const style = getComputedStyle(element);
    const bounds = element.getBoundingClientRect();
    const previous = window.portalQualityFocusStyles?.get(element);
    const visible = element.matches(':focus-visible') && (
      (style.outlineStyle !== 'none' && Number.parseFloat(style.outlineWidth) > 0) ||
      (previous && [style.boxShadow, style.backgroundColor, style.borderColor, style.textDecorationLine]
        .some((value, i) => value !== previous[i]))
    );
    const link = element.getAttribute('href');
    return {
      index: [...document.querySelectorAll('*')].indexOf(element),
      target: element.tagName.toLowerCase() + (element.id ? '#' + element.id : '') +
        (link ? ' ' + new URL(link, location.href).pathname : ''),
      visible: Boolean(visible && bounds.width > 0 && bounds.height > 0 &&
        bounds.right > 0 && bounds.left < innerWidth && bounds.bottom > 0 && bounds.top < innerHeight),
      inHeader: !!element.closest('header'),
      inScope: !!(element.form || element.closest('form, header, nav')),
    };
  });
}

const soft = expect.configure({ soft: true, timeout: 1000 });

// Chromium exposes a single active input while Tab traverses its native date fields.
const nativeTabSteps = { date: 3, 'datetime-local': 8, month: 2, time: 5, week: 2 };

async function tabStops(page) {
  return page.evaluate(nativeTabSteps => {
    window.portalQualityFocusStyles = new WeakMap();
    return [...document.querySelectorAll('*')].flatMap((element, index) => {
      if (!element.matches('a[href], button, input, select, textarea, summary, iframe, [contenteditable], [tabindex]') ||
          element.matches(':disabled,input[type=hidden]') || element.closest('[inert]') ||
          !element.getClientRects().length ||
          !element.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) return [];
      const radio = element.matches('input[type=radio][name]') && element.name;
      if (radio) {
        const controls = element.form ? [...element.form.elements] : [...document.querySelectorAll('input[type=radio]')];
        const group = controls.filter(candidate => candidate.name === radio &&
          candidate.matches('input[type=radio]:enabled') &&
          candidate.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true }));
        if (element !== (group.find(candidate => candidate.checked) ?? group[0])) return [];
      }
      if (element.tabIndex < 0 && !element.matches('summary,[contenteditable=true]')) return [];
      const style = getComputedStyle(element);
      window.portalQualityFocusStyles.set(element,
        [style.boxShadow, style.backgroundColor, style.borderColor, style.textDecorationLine]);
      const internalSteps = element instanceof HTMLInputElement ? nativeTabSteps[element.type] ?? 0 : 0;
      return [{ index, inHeader: !!element.closest('header'), inDialog: !!element.closest('dialog'), inScope: !!(element.form || element.closest('form, header, nav')), tabIndex: element.tabIndex, internalSteps }];
    });
  }, nativeTabSteps);
}

async function tabUntil(page, locator, bound) {
  for (let index = 0; index <= bound; index++) {
    if (await locator.evaluate(element => element === document.activeElement)) return true;
    await page.keyboard.press('Tab');
  }
  return false;
}

async function tabPastControl(page, stop, label) {
  await page.keyboard.press('Tab');
  for (let step = 0; step < stop.internalSteps; step++) {
    const focus = await focusState(page);
    if (focus.index !== stop.index) break;
    soft(focus.visible, `${label}: native input focus indicator`).toBe(true);
    await page.keyboard.press('Tab');
  }
}

async function assertAccountDialog(page, header, toggle, label, bound, mobile) {
  const account = header.locator('[data-account-link]');
  if (await account.isVisible()) {
    const reached = await tabUntil(page, account, bound + (await tabStops(page)).length);
    soft(reached, `${label}: account menu is keyboard reachable`).toBe(true);
    if (!reached) return false;
    await page.keyboard.press('Enter');
    const dialog = page.locator('#podglad-konta');
    await soft(dialog).toHaveAttribute('open', '');
    const dialogStops = (await tabStops(page)).filter(stop => stop.inDialog);
    for (const stop of dialogStops) {
      const focus = await focusState(page);
      soft(focus.index, `${label}: account dialog order`).toBe(stop.index);
      soft(focus.visible, `${label}: account dialog focus indicator`).toBe(true);
      await tabPastControl(page, stop, label);
    }
    await page.keyboard.press('Escape');
    await soft(dialog).not.toHaveAttribute('open', '');
    await soft(mobile ? toggle : account, `${label}: account dialog restores focus`).toBeFocused();
  }
  return true;
}

async function assertMenu(page, label, bound) {
  const header = page.locator('#portal-header');
  const toggle = header.locator('#portal-menu-toggle');
  const mobile = await toggle.isVisible();
  if (mobile) {
    const reached = await tabUntil(page, toggle, bound);
    soft(reached, `${label}: mobile menu is keyboard reachable`).toBe(true);
    if (!reached) return;
    soft((await focusState(page)).visible, `${label}: mobile menu toggle focus`).toBe(true);
    await page.keyboard.press('Enter');
    await soft(toggle).toHaveAttribute('aria-expanded', 'true');
    if (!await assertAccountDialog(page, header, toggle, label, bound, true)) return;
    if (await toggle.getAttribute('aria-expanded') === 'false') {
      await page.keyboard.press('Enter');
      await soft(toggle).toHaveAttribute('aria-expanded', 'true');
    }
  }
  for (const button of await header.locator('[data-nav-section]:enabled').all()) {
    const reached = await tabUntil(page, button, bound);
    soft(reached, `${label}: menu section is keyboard reachable`).toBe(true);
    if (!reached) return;
    soft((await focusState(page)).visible, `${label}: menu section focus`).toBe(true);
    const panelId = await button.getAttribute('aria-controls');
    const panel = header.locator(`[id="${panelId}"]`);
    await page.keyboard.press('ArrowDown');
    await soft(button).toHaveAttribute('aria-expanded', 'true');
    const links = await panel.locator('a[href]:visible').all();
    soft(links.length, `${label}: expanded ${panelId} links`).toBeGreaterThan(0);
    for (const link of links) {
      await soft(link, `${label}: keyboard order in ${panelId}`).toBeFocused();
      soft((await focusState(page)).visible, `${label}: link focus in ${panelId}`).toBe(true);
      await page.keyboard.press('ArrowDown');
    }
    await page.keyboard.press('Escape');
    await soft(button).toBeFocused();
    await soft(button).toHaveAttribute('aria-expanded', 'false');
  }
  if (!mobile) await assertAccountDialog(page, header, toggle, label, bound, false);
  if (mobile && await toggle.getAttribute('aria-expanded') === 'true') {
    await page.keyboard.press('Escape');
    await soft(toggle).toBeFocused();
    await soft(toggle).toHaveAttribute('aria-expanded', 'false');
  }
}

async function assertNativeMenu(page, label, bound) {
  const menu = page.locator('details.om-fallback-menu');
  if (!await menu.count()) return;
  const summary = menu.locator(':scope > summary');
  const reached = await tabUntil(page, summary, bound);
  soft(reached, `${label}: main menu disclosure is keyboard reachable`).toBe(true);
  if (!reached) return;
  await page.keyboard.press('Enter');
  await soft(menu).toHaveAttribute('open', '');
  const account = menu.locator('details.om-fallback-account');
  const accountSummary = account.locator(':scope > summary');
  const hasAccount = await accountSummary.count() === 1;
  const accountIndex = hasAccount ? await accountSummary.evaluate(summary =>
    [...document.querySelectorAll('*')].indexOf(summary)) : -1;
  const menuIndex = (await focusState(page)).index;
  let expected = (await tabStops(page)).filter(stop => stop.inHeader && stop.index > menuIndex);
  let openedAccount = false;
  for (let index = 0; index < expected.length; index++) {
    await page.keyboard.press('Tab');
    const focus = await focusState(page);
    const stop = expected[index];
    soft(focus.index, `${label}: open menu follows source order`).toBe(stop.index);
    soft(focus.visible, `${label}: open menu focus indicator for ${focus.target}`).toBe(true);
    if (stop.index === accountIndex && focus.index === accountIndex) {
      await page.keyboard.press('Enter');
      await soft(account).toHaveAttribute('open', '');
      openedAccount = true;
      expected = (await tabStops(page)).filter(candidate => candidate.inHeader && candidate.index > accountIndex);
      index = -1;
    }
  }
  if (hasAccount) soft(openedAccount, `${label}: can open the account disclosure from the keyboard`).toBe(true);
  await page.keyboard.press('Tab');
  soft((await focusState(page)).inHeader, `${label}: Tab leaves the open menu`).toBe(false);
  await resetTabStart(page);
  const returned = await tabUntil(page, summary, bound);
  soft(returned, `${label}: keyboard can return to the main disclosure`).toBe(true);
  if (!returned) return;
  await page.keyboard.press('Space');
  await soft(menu).not.toHaveAttribute('open', '');
}

async function resetTabStart(page) {
  await page.evaluate(() => {
    document.body.tabIndex = -1;
    document.body.focus();
    document.body.removeAttribute('tabindex');
  });
}

async function assertDisclosedForms(page, label, bound) {
  for (const summary of await page.locator('main details:not([open]):has(form) > summary:visible').all()) {
    await resetTabStart(page);
    const reached = await tabUntil(page, summary, bound);
    soft(reached, `${label}: form disclosure is keyboard reachable`).toBe(true);
    if (!reached) continue;
    await page.keyboard.press('Enter');
    const details = summary.locator('..');
    await soft(details).toHaveAttribute('open', '');
    const descendants = await details.evaluate(container => [...document.querySelectorAll('*')]
      .flatMap((element, index) => element !== container && container.contains(element) ? [index] : []));
    const summaryIndex = (await focusState(page)).index;
    const stops = (await tabStops(page)).filter(stop =>
      stop.index !== summaryIndex && descendants.includes(stop.index));
    for (const stop of stops) {
      await tabPastControl(page, stop, label);
      const focus = await focusState(page);
      soft(focus.index, `${label}: keyboard order in form disclosure`).toBe(stop.index);
      if (stop.inScope) soft(focus.visible, `${label}: disclosed form control has visible focus`).toBe(true);
    }
    await page.keyboard.press('Tab');
    soft(descendants.includes((await focusState(page)).index), `${label}: Tab leaves the disclosed form`).toBe(false);
    await resetTabStart(page);
    if (await tabUntil(page, summary, bound)) await page.keyboard.press('Space');
    await soft(details).not.toHaveAttribute('open', '');
  }
}

export async function assertKeyboard(page, label) {
  const stops = await tabStops(page);
  const scoped = stops.filter(stop => stop.inScope);
  soft(scoped.filter(stop => stop.tabIndex > 0), `${label}: positive tabindex`).toEqual([]);
  const skip = page.locator('a[href="#main"]').first();
  await page.keyboard.press('Tab');
  await soft(skip, `${label}: first Tab reaches skip link`).toBeFocused();
  soft((await focusState(page)).visible, `${label}: skip link focus`).toBe(true);
  const skipFocused = await skip.evaluate(element => element === document.activeElement).catch(() => false);
  if (skipFocused) {
    await page.keyboard.press('Enter');
    await soft(page.locator('#main'), `${label}: skip link moves focus`).toBeFocused();
  }

  // Reset only the starting position. Every asserted transition is a real key press.
  await resetTabStart(page);
  const reached = [];
  const visited = new Set();
  const invisible = [];
  const native = new Map(stops.map(stop => [stop.index, stop.internalSteps]));
  const bound = stops.reduce((total, stop) => total + 1 + stop.internalSteps, 4);
  let previous;
  let remainingInternalSteps = 0;
  let exited = false;
  for (let step = 0; step < bound; step++) {
    await page.keyboard.press('Tab');
    const focus = await focusState(page);
    if (focus.index === previous && remainingInternalSteps > 0) {
      remainingInternalSteps--;
      if (focus.inScope && !focus.visible) invisible.push(focus.target);
      continue;
    }
    if (focus.target === 'body' || visited.has(focus.index)) {
      exited = focus.target === 'body' || focus.index === visited.values().next().value;
      break;
    }
    previous = focus.index;
    remainingInternalSteps = native.get(focus.index) ?? 0;
    visited.add(focus.index);
    if (focus.inScope) {
      reached.push(focus.index);
      if (!focus.visible) invisible.push(focus.target);
    }
  }
  soft(exited, `${label}: Tab can leave the document sequence`).toBe(true);
  soft(reached, `${label}: menu and form controls follow source order`).toEqual(scoped.map(stop => stop.index));
  soft(invisible, `${label}: focus indicators on menu and form controls`).toEqual([]);

  await resetTabStart(page);
  if (await page.locator('#portal-header.is-enhanced').count()) {
    await assertMenu(page, label, bound);
  } else {
    await assertNativeMenu(page, label, bound);
  }
  await assertDisclosedForms(page, label, bound);
}
