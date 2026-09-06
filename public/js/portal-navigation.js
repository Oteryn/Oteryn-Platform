/* Progressive enhancement: the native details menu also works without JavaScript. */
(() => {
    const menu = document.querySelector('.public-body .mobile-nav');
    if (!(menu instanceof HTMLDetailsElement)) return;
    const trigger = menu.querySelector('summary');
    if (!trigger) return;
    const close = (returnFocus = false) => {
        if (!menu.open) return;
        menu.open = false;
        if (returnFocus) trigger.focus();
    };
    document.addEventListener('keydown', (event) => {
        if (event.key === 'Escape' && menu.open) {
            event.preventDefault();
            close(true);
        }
    });
    document.addEventListener('click', (event) => {
        if (!menu.contains(event.target)) close();
    });
    menu.addEventListener('click', (event) => {
        if (!(event.target instanceof Element)) return;
        const link = event.target.closest('a[href]');
        if (link && !event.ctrlKey && !event.metaKey && !event.shiftKey) close();
    });
    const desktop = window.matchMedia('(min-width: 75rem)');
    desktop.addEventListener('change', ({ matches }) => {
        if (matches) close();
    });
})();
