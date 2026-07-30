(() => {
    'use strict';

    function replaceBrokenImage(image) {
        if (!(image instanceof HTMLImageElement)) {
            return;
        }

        if (image.dataset.mediaFallbackHandled === 'true') {
            return;
        }

        image.dataset.mediaFallbackHandled = 'true';

        const kind = image.dataset.mediaFallback;
        const altText = (image.getAttribute('alt') ?? '').trim();
        const fallback = document.createElement('span');
        const fallbackText = altText || (kind === 'admin' ? 'Editorial image' : 'Image unavailable');

        fallback.dataset.mediaFallbackState = 'unavailable';
        fallback.setAttribute('role', 'img');
        fallback.setAttribute('aria-label', fallbackText);

        if (kind === 'admin') {
            fallback.className = 'admin-media-preview-fallback';
            fallback.textContent = `Preview unavailable: ${fallbackText}`;

            const previewLink = image.closest('a');
            if (previewLink !== null) {
                previewLink.replaceWith(fallback);
                return;
            }
        } else {
            fallback.className = 'wiki-image-placeholder';
            fallback.textContent = fallbackText;
        }

        image.replaceWith(fallback);
    }

    document.addEventListener('error', (event) => {
        replaceBrokenImage(event.target);
    }, true);

    function replaceAlreadyFailedImages() {
        document.querySelectorAll('img[data-media-fallback]').forEach((image) => {
            if (image.complete && image.naturalWidth === 0) {
                replaceBrokenImage(image);
            }
        });
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', replaceAlreadyFailedImages, { once: true });
    } else {
        replaceAlreadyFailedImages();
    }
})();
