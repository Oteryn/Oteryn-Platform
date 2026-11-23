// Runs in the page. Return measurements and structural signatures, with no
// form values, account records, raw markup or URLs.
export function measure() {
  const selectors = {
    header: 'body > header', nav: 'body > header nav, body > main > nav',
    footer: 'body > footer', headerLink: 'body > header a, body > header summary',
    footerLink: 'body > footer a', logo: 'body > header img, body > footer .brand img',
    active: 'header [aria-current], footer [aria-current], main nav [aria-current]',
    main: 'main', h1: 'main h1', h2: 'main h2', h3: 'main h3', body: 'body, main p',
    small: 'main small, main .op-hint, main time', link: 'main a:not(.op-button)',
    primary: '.op-button--primary', secondary: '.op-button--secondary, .op-button:not([class*="op-button--"])',
    danger: '.op-button--danger, .op-button--destructive', button: 'main button:not(.op-button)',
    input: 'input:not([type=hidden],[type=checkbox],[type=radio],[type=submit])',
    select: 'select', textarea: 'textarea', table: 'table', tableHeader: 'th',
    tableOdd: 'tbody tr:nth-child(odd)', tableEven: 'tbody tr:nth-child(even)', tableCell: 'td',
    alert: '[role=alert], .op-alert, .op-error', status: '[role=status], .op-notice, .op-success',
    card: 'main .op-card, main .op-panel', badge: '[class*="badge"]', pagination: '[class*="pagination"]',
    breadcrumb: '[class*="breadcrumb"]', empty: '[class*="empty"]',
  };
  const styleKeys = [
    'backgroundColor', 'color', 'fontFamily', 'fontSize', 'fontWeight', 'lineHeight',
    'borderTop', 'borderRadius', 'padding', 'boxShadow', 'outline', 'outlineOffset', 'textDecorationLine',
  ];
  const paletteKeys = {
    colors: ['color', 'backgroundColor', 'borderTopColor', 'borderRightColor',
      'borderBottomColor', 'borderLeftColor', 'outlineColor', 'fill', 'stroke'],
    families: ['fontFamily'], sizes: ['fontSize'], radii: ['borderRadius'],
    shadows: ['boxShadow', 'textShadow'],
    spacing: ['marginTop', 'marginRight', 'marginBottom', 'marginLeft',
      'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft', 'rowGap', 'columnGap'],
  };
  const style = el => getComputedStyle(el);
  const visible = el => {
    if (!el.checkVisibility({ contentVisibilityAuto: true, visibilityProperty: true })
      || el.closest('[inert], [hidden]')) return false;
    for (let node = el; node; node = node.parentElement) {
      const closed = node.parentElement;
      if (closed?.matches('details:not([open])')
        && node !== closed.querySelector(':scope > summary')) return false;
    }
    return !!el.getClientRects().length;
  };
  const matches = selector => [...document.querySelectorAll(selector)].filter(visible);
  const clip = el => style(el).clipPath !== 'none' || style(el).clip !== 'auto';
  const round = number => Math.round(number * 10) / 10;
  const box = el => {
    const { x, y, width, height } = el.getBoundingClientRect();
    return [x, y + scrollY, width, height].map(round);
  };
  const values = el => Object.fromEntries(styleKeys.map(key => [key, style(el)[key]]));
  const label = el => [el.localName, ...[...el.classList].filter(token => /^[a-z0-9_-]{2,35}$/i.test(token)).slice(0, 3)].join('.');
  const regionOf = el => ['header', 'footer', 'main form', 'main nav', 'main'].find(region => el.closest(region)) || 'body';
  const identity = value => value.trim().replace(/\s+/g, ' ').toLocaleLowerCase();
  const fingerprint = {};
  const geometry = new Set(['header', 'nav', 'footer', 'logo', 'main', 'active', 'breadcrumb']);
  for (const [part, selector] of Object.entries(selectors)) {
    const elements = matches(selector);
    const distinct = new Map();
    for (const el of elements) {
      const css = values(el);
      const key = JSON.stringify(css);
      const row = distinct.get(key) || { css, count: 0 };
      row.count++;
      if (geometry.has(part)) (row.boxes ??= []).push(box(el));
      if (['input', 'select', 'textarea', 'button', 'primary', 'secondary', 'danger'].includes(part)) {
        (row.heights ??= new Set()).add(box(el)[3]);
      }
      distinct.set(key, row);
    }
    fingerprint[part] = [...distinct.values()].map(row => row.heights
      ? { ...row, heights: [...row.heights].sort((a, b) => a - b) } : row);
  }
  const main = document.querySelector('main');
  const head = document.querySelector('body > header');
  const content = main ? {
    maxWidth: style(main).maxWidth,
    gutters: [round(main.getBoundingClientRect().left), round(innerWidth - main.getBoundingClientRect().right)],
    gapAfterHeader: head ? round(main.getBoundingClientRect().top - head.getBoundingClientRect().bottom) : null,
  } : null;
  const all = matches('body, body *').filter(el => !clip(el));
  const palette = Object.fromEntries(Object.entries(paletteKeys).map(([key, props]) =>
    [key, [...new Set(all.flatMap(el => props.map(prop => style(el)[prop])).filter(Boolean))].sort()]));
  const problems = new Map();
  function note(kind, component, expected, actual) {
    const issue = { kind, component, expected, actual };
    const id = JSON.stringify(issue);
    problems.set(id, { ...issue, count: (problems.get(id)?.count ?? 0) + 1 });
  }

  // One CSS pixel for overflow and intersections. AA ratios use the computed
  // solid backdrop. A gradient, image or group opacity needs screenshot review.
  const pixel = 1;
  const parse = color => {
    const parts = color.match(/^rgba?\(([\d., ]+)\)$/)?.[1].split(',').map(Number);
    return parts && parts.every(Number.isFinite) ? [parts[0], parts[1], parts[2], parts[3] ?? 1] : null;
  };
  const blend = (front, back) => {
    const alpha = front[3] + back[3] * (1 - front[3]);
    return [...front.slice(0, 3).map((value, i) => alpha
      ? (value * front[3] + back[i] * back[3] * (1 - front[3])) / alpha : 0), alpha];
  };
  const background = el => {
    let color = [0, 0, 0, 0];
    for (let node = el; node; node = node.parentElement) {
      const s = style(node);
      if (Number(s.opacity) !== 1 || (color[3] < 1 && s.backgroundImage !== 'none')) return null;
      const backdrop = parse(s.backgroundColor);
      if (!backdrop) return null;
      color = blend(color, backdrop);
    }
    const canvas = style(document.documentElement).colorScheme === 'dark' ? 18 : 255;
    return blend(color, [canvas, canvas, canvas, 1]);
  };
  const luminance = color => color.slice(0, 3).reduce((sum, v, i) => {
    const channel = v / 255;
    return sum + [0.2126, 0.7152, 0.0722][i] * (channel <= 0.04045
      ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4);
  }, 0);
  let unknownContrast = 0;
  const controlSelector = 'a[href],button,summary,input:not([type=hidden]),select,textarea,[role=button]';
  for (const el of all) {
    const s = style(el);
    const directText = [...el.childNodes].some(node => node.nodeType === Node.TEXT_NODE && node.textContent.trim());
    const isControl = el.matches(controlSelector) && !el.matches(':disabled') && !el.closest('[aria-disabled=true]');
    const rect = el.getBoundingClientRect();
    const navigationFragment = el.matches('a[href^="#"]') && /skip/i.test(el.className);
    if (navigationFragment) continue;
    const range = document.createRange();
    if (directText || isControl) range.selectNodeContents(el);
    const textBoxes = directText || isControl ? [...range.getClientRects()].filter(r => r.width > 0 && r.height > 0) : [];
    if (innerWidth === 390 && (directText || isControl || el.matches('img,svg,table'))
      && rect.width > 0 && (rect.left < -pixel || rect.right > innerWidth + pixel)) {
      note('viewport', label(el), 'within the 390px viewport', `left ${round(rect.left)}, right ${round(rect.right)}`);
    }
    if ((directText || isControl) && !el.matches('input, textarea, select') && el.clientWidth > 0
      && (el.scrollWidth > el.clientWidth + pixel
        || (['hidden', 'clip'].includes(s.overflowY) && textBoxes.some(text => text.bottom > rect.bottom + pixel)))) {
      note('clipping', label(el), 'text fits its container', `scroll ${el.scrollWidth}px / client ${el.clientWidth}px; overflow ${s.overflowX}/${s.overflowY}`);
    }
    const hit = el.matches('input[type=checkbox], input[type=radio]') && el.labels?.length
      ? el.labels[0].getBoundingClientRect() : rect;
    if (innerWidth === 390 && isControl && (hit.width < 39.5 || hit.height < 39.5)) {
      note('tap target', `${regionOf(el)} / ${label(el)}`, 'at least 40×40px (0.5px rounding tolerance)',
        `${round(hit.width)}×${round(hit.height)}px`);
    }
    if (directText || el.matches('input:not([type=checkbox],[type=radio],[type=hidden]),select,textarea')) {
      const foreground = parse(s.color);
      const behind = background(el);
      if (!foreground || !behind) {
        unknownContrast++;
      } else {
        const a = luminance(blend(foreground, behind)) + 0.05;
        const b = luminance(behind) + 0.05;
        const actual = Math.max(a, b) / Math.min(a, b);
        const size = parseFloat(s.fontSize);
        const threshold = (size >= 24 || (size >= 18.667 && Number(s.fontWeight) >= 700)) ? 3 : 4.5;
        if (actual < threshold) note('contrast', label(el), `AA ${threshold}:1`,
          `${actual.toFixed(2)}:1 (${s.color} on rgb(${behind.slice(0, 3).map(Math.round).join(', ')}))`);
      }
    }
    if (el instanceof SVGUseElement) {
      if (!el.closest('svg') || el.closest('svg').getBoundingClientRect().width === 0) continue;
      const symbol = el.getBBox();
      if (!symbol.width && !symbol.height) note('image', label(el), 'visible SVG symbol has geometry', 'zero-size symbol');
    }
    if (el instanceof HTMLImageElement && !el.matches('.skip')) {
      if (!el.complete || !el.naturalWidth) note('image', label(el), 'loaded image', 'no intrinsic width');
      else if (['fill', 'none'].includes(s.objectFit) && rect.width && rect.height) {
        const delta = Math.abs((rect.width / rect.height) / (el.naturalWidth / el.naturalHeight) - 1);
        if (delta > 0.02) note('image', label(el), 'within 2% of intrinsic aspect ratio',
          `${round(rect.width)}×${round(rect.height)} vs ${el.naturalWidth}×${el.naturalHeight}`);
      }
    }
  }

  for (const region of matches('body > header, main nav, main form')) {
    const controls = [...region.querySelectorAll(controlSelector)].filter(visible);
    for (const [index, a] of controls.entries()) {
      const x = a.getBoundingClientRect();
      for (const b of controls.slice(index + 1)) {
        if (a.contains(b) || b.contains(a)) continue;
        const y = b.getBoundingClientRect();
        if (Math.min(x.right, y.right) - Math.max(x.left, y.left) > pixel
          && Math.min(x.bottom, y.bottom) - Math.max(x.top, y.top) > pixel) {
          note('overlap', label(region), 'distinct controls do not intersect', `${label(a)} intersects ${label(b)}`);
        }
      }
    }
  }

  const current = new URL(location.href);
  const samePage = element => {
    const target = new URL(element.href);
    return target.origin === current.origin
      && target.pathname.replace(/\/$/, '') === current.pathname.replace(/\/$/, '')
      && (!target.search || target.search === current.search) && !target.hash;
  };
  const navLinks = [...document.querySelectorAll('header nav a[href], header .om-brand, footer a[href], main nav a[href], .account-grid a[href]')];
  const matchingNav = navLinks.filter(samePage);
  const h1 = matches('main h1');
  const h1Text = h1.map(el => identity(el.textContent));
  if (!document.title.endsWith(' · Oteryn')) note('title', 'document title', '<Page> · Oteryn', 'suffix missing');
  if (h1.length !== 1) note('heading', 'main h1', 'one visible h1', String(h1.length));
  if (matchingNav.length && !matchingNav.some(el => el.getAttribute('aria-current') === 'page')) {
    note('active nav', 'navigation', 'an exact-destination link has aria-current=page', 'no matching link marked active');
  }
  for (const crumb of matches('[class*="breadcrumb"]')) {
    const links = [...crumb.querySelectorAll('a[href]')];
    const currentCrumb = crumb.querySelector('[aria-current="page"]');
    if (currentCrumb?.matches('a[href]') && !samePage(currentCrumb)) {
      note('breadcrumb', 'navigation trail', 'the current crumb points to this page', 'destination differs');
    }
    if (links.some(el => !el.getAttribute('href')?.trim())) {
      note('breadcrumb', 'navigation trail', 'each parent crumb has a destination', 'a parent href is empty');
    }
  }
  const activeNav = navLinks.filter(el => el.getAttribute('aria-current') === 'page' && samePage(el));
  if (activeNav.length && h1.length === 1 && !activeNav.some(el => identity(el.textContent) === h1Text[0])) {
    note('heading label', 'navigation / main h1', 'heading matches an exact active link label', 'labels differ');
  }
  for (const el of matches('main table')) {
    if (!el.querySelector('tbody tr, caption') && !el.parentElement.querySelector('[role=status], .op-empty, [class*="empty"]')) {
      note('empty table', label(el), 'empty result has an explanation', 'no data row, caption or nearby status');
    }
  }

  const signature = matches('header .om-top-link, footer .foot-group a, main button, main nav a').map(el => {
    const line = parseFloat(style(el).lineHeight) || 1.2 * parseFloat(style(el).fontSize);
    const range = document.createRange();
    range.selectNodeContents(el);
    const lines = [...range.getClientRects()]
      .filter(rect => rect.height > 0 && rect.width > 0)
      .map(rect => rect.top + rect.height / 2).sort((a, b) => a - b);
    const rows = lines.filter((value, index) => index === 0 || value - lines[index - 1] > line / 2).length;
    return [label(el), rows];
  });
  const dateFormats = matches('main time, main td').map(el => el.textContent.trim())
    .filter(text => /^\d{1,4}[.-/]\d{1,2}[.-/]\d{1,4}(?:[ T][\d:]+(?: ?\w+)?)?$/.test(text))
    .map(text => text.replace(/\d/g, '#'));
  const rawTitle = document.title;
  const rawHeadings = h1.map(el => el.textContent.trim());
  const navLabels = [...new Set(activeNav.map(el => el.textContent.trim()))];
  return {
    components: fingerprint, content, palette, signature,
    dateFormats: [...new Set(dateFormats)].sort(), rawTitle, rawHeadings, navLabels,
    favicon: document.querySelector('link[rel~="icon"]')?.getAttribute('href') || '/favicon.ico',
    theme: [document.documentElement.dataset.theme ?? null, style(document.body).colorScheme,
      style(document.body).backgroundColor, style(document.body).color],
    unknownContrast, problems: [...problems.values()],
  };
}
