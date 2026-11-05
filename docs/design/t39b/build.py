"""Generate static review artifacts from paired design-only translation catalogs."""
import json
from html import escape
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'portal-rs/templates/mockups/t39b'
CATALOG = Path(__file__).with_name('copy.json')


def render():
    catalogs = json.loads(CATALOG.read_text())
    assert catalogs['en'].keys() == catalogs['pl'].keys()
    for locale, catalog in catalogs.items():
        def t(key):
            return escape(catalog[key])
        for screen in ('home', 'account', 'bazaar', 'atlas', 'journal', 'statistics'):
            nav = ''.join(f'<a href="{name}-{locale}.html"' + (' aria-current="page"' if name == screen else '') + f'>{t(name)}</a>' for name in ('home', 'account', 'bazaar', 'atlas', 'journal', 'statistics'))
            other = 'en' if locale == 'pl' else 'pl'
            cards = ''.join(f'<section class="op-card op-stack"><p class="op-eyebrow">{t(screen+".tag"+str(i))}</p><h2>{t(screen+".title"+str(i))}</h2><p>{t(screen+".body"+str(i))}</p></section>' for i in range(1, 4))
            extra = ''
            if screen == 'atlas':
                extra = f'<figure class="op-frame"><strong>{t("map")}</strong><figcaption>{t("map.caption")}</figcaption></figure>'
            if screen in ('journal', 'statistics'):
                rows = ''.join(f'<tr><th scope="row">2026-11-0{i}</th><td>{i * 1200:,}</td></tr>' for i in range(1, 4))
                extra = f'<div class="op-table-wrap" tabindex="0" role="region" aria-label="{t("table")}"><table class="op-table"><caption>{t("table.caption")}</caption><thead><tr><th scope="col">{t("date")}</th><th scope="col">{t("experience")}</th></tr></thead><tbody>{rows}</tbody></table></div>'
            html = f'''<!doctype html>
<html lang="{locale}" data-theme="dark">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{t(screen)} · Oteryn</title>
<link rel="stylesheet" href="../../../static/css/tokens.css">
<link rel="stylesheet" href="../../../static/css/components.css">
<link rel="stylesheet" href="review.css"></head>
<body class="op-theme">
<header class="op-stack"><a href="#main">{t('skip')}</a><div class="op-row"><strong>OTERYN</strong><span class="op-badge">{t('draft')}</span><a href="{screen}-{other}.html" lang="{other}" hreflang="{other}">{'English' if other == 'en' else 'Polski'}</a></div>
<nav class="op-tabs" aria-label="{t('navigation')}">{nav}</nav></header>
<main id="main" tabindex="-1" class="op-stack"><p class="op-eyebrow">{t(screen+'.eyebrow')}</p><h1>{t(screen+'.heading')}</h1><p class="intro">{t(screen+'.intro')}</p>
<p class="op-alert op-alert--warning">{t('disabled')}</p>
<div class="layout"><div class="op-stack">{cards}{extra}</div>
<aside class="op-panel op-stack" aria-labelledby="review-title"><h2 id="review-title">{t('review')}</h2><p>{t(screen+'.review')}</p><p class="op-muted">{t('fixture')}</p><button class="op-button" type="button" disabled aria-describedby="action-note">{t(screen+'.action')}</button><p id="action-note">{t('action.note')}</p></aside></div>
<section class="op-stack" aria-labelledby="states"><h2 id="states">{t('states')}</h2>
<details class="op-disclosure"><summary>{t('success')}</summary><div class="op-panel"><p>{t(screen+'.success')}</p></div></details>
<details class="op-disclosure"><summary>{t('refusal')}</summary><div class="op-panel"><p>{t(screen+'.refusal')}</p></div></details>
<details class="op-disclosure"><summary>{t('invalid')}</summary><div class="op-panel op-stack"><div class="op-field"><label for="invalid-field">{t(screen+'.field')}</label><input class="op-input" id="invalid-field" value="?" aria-invalid="true" aria-describedby="invalid-help" readonly><p id="invalid-help" class="op-error">{t(screen+'.invalid')}</p></div></div></details></section>
</main><footer class="op-stack"><p>{t('footer')}</p><a href="../../../../docs/design/t39b/README.md">{t('notes')}</a></footer>
</body></html>
'''
            (OUT / f'{screen}-{locale}.html').write_text(html)


if __name__ == '__main__':
    render()
