(() => {
  'use strict';

  const picker = document.querySelector('[data-wiki-media-picker]');
  if (!(picker instanceof HTMLElement)) {
    return;
  }

  const indexUrl = picker.dataset.indexUrl;
  const search = picker.querySelector('[data-wiki-media-search]');
  const searchButton = picker.querySelector('[data-wiki-media-search-button]');
  const status = picker.querySelector('[data-wiki-media-status]');
  const results = picker.querySelector('[data-wiki-media-results]');
  const more = picker.querySelector('[data-wiki-media-more]');
  let nextPageUrl = null;

  if (
    typeof indexUrl !== 'string'
    || !(search instanceof HTMLInputElement)
    || !(searchButton instanceof HTMLButtonElement)
    || !(status instanceof HTMLElement)
    || !(results instanceof HTMLElement)
    || !(more instanceof HTMLButtonElement)
  ) {
    return;
  }

  function setStatus(message) {
    status.textContent = message;
  }

  function insertMarkdown(targetId, markdown, mediaId) {
    const target = document.getElementById(targetId);
    if (!(target instanceof HTMLTextAreaElement)) {
      setStatus('The selected translation field is unavailable.');
      return;
    }

    const start = target.selectionStart ?? target.value.length;
    const end = target.selectionEnd ?? start;
    const prefix = start > 0 && !target.value.slice(0, start).endsWith('\n') ? '\n\n' : '';
    const suffix = end < target.value.length && !target.value.slice(end).startsWith('\n') ? '\n\n' : '';
    const insertion = `${prefix}${markdown}${suffix}`;
    target.setRangeText(insertion, start, end, 'end');
    target.focus();
    target.dispatchEvent(new Event('input', { bubbles: true }));
    setStatus(`Inserted approved image ${mediaId}. Review and localize its alternative text before saving.`);
  }

  function insertionButton(label, targetId, item) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'button button-secondary';
    button.textContent = label;
    button.addEventListener('click', () => insertMarkdown(targetId, item.markdown, item.id));
    return button;
  }

  function mediaCard(item) {
    const card = document.createElement('article');
    card.className = 'wiki-media-card';

    const image = document.createElement('img');
    image.src = item.thumbnail_url;
    image.alt = item.alt_text;
    image.loading = 'lazy';
    image.decoding = 'async';
    card.append(image);

    const details = document.createElement('div');
    details.className = 'wiki-media-card-body';

    const heading = document.createElement('h3');
    heading.textContent = `Media ${item.id}`;
    details.append(heading);

    const alt = document.createElement('p');
    alt.textContent = item.alt_text;
    details.append(alt);

    const metadata = document.createElement('p');
    metadata.className = 'muted';
    metadata.textContent = `${item.mime_type} | ${item.width}x${item.height}`;
    details.append(metadata);

    const actions = document.createElement('div');
    actions.className = 'action-row';
    actions.append(
      insertionButton('Insert in English', 'translations_en_source_markdown', item),
      insertionButton('Insert in Polish', 'translations_pl_source_markdown', item),
    );
    details.append(actions);
    card.append(details);

    return card;
  }

  async function load(url, append = false) {
    setStatus('Loading approved images...');
    searchButton.disabled = true;
    more.disabled = true;

    try {
      const response = await fetch(url, {
        credentials: 'same-origin',
        headers: { Accept: 'application/json' },
      });
      if (!response.ok) {
        throw new Error(`Unexpected response ${response.status}`);
      }

      const payload = await response.json();
      const items = Array.isArray(payload.items) ? payload.items : [];
      if (!append) {
        results.replaceChildren();
      }
      for (const item of items) {
        results.append(mediaCard(item));
      }

      nextPageUrl = typeof payload.next_page_url === 'string' ? payload.next_page_url : null;
      more.hidden = nextPageUrl === null;
      setStatus(
        results.childElementCount === 0
          ? 'No approved images match this search.'
          : `${results.childElementCount} approved image${results.childElementCount === 1 ? '' : 's'} available.`,
      );
    } catch {
      setStatus('The approved image library is temporarily unavailable.');
      nextPageUrl = null;
      more.hidden = true;
    } finally {
      searchButton.disabled = false;
      more.disabled = false;
    }
  }

  function searchMedia() {
    const url = new URL(indexUrl, window.location.origin);
    const query = search.value.trim();
    if (query !== '') {
      url.searchParams.set('q', query);
    }
    load(url.toString());
  }

  searchButton.addEventListener('click', searchMedia);
  search.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      searchMedia();
    }
  });
  more.addEventListener('click', () => {
    if (nextPageUrl !== null) {
      load(nextPageUrl, true);
    }
  });

  load(indexUrl);
})();
