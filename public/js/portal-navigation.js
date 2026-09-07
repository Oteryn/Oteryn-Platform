/* Native details works without JS. Enhancement adds predictable dismissal/focus. */
(() => {
    const disclosures = [...document.querySelectorAll('.nav-group, .mobile-nav')];
    const close = (details, restoreFocus = false) => {
        if (!details.open) return;
        details.open = false;
        if (restoreFocus) details.querySelector('summary')?.focus();
    };
    for (const details of disclosures) {
        details.addEventListener('toggle', () => {
            if (details.open) for (const other of disclosures) if (other !== details) close(other);
        });
        details.addEventListener('focusout', (event) => {
            if (event.relatedTarget && !details.contains(event.relatedTarget)) close(details);
        });
        details.addEventListener('click', (event) => {
            if (event.target instanceof Element && event.target.closest('a[href]')) close(details);
        });
    }
    document.addEventListener('keydown', (event) => {
        if (event.key !== 'Escape') return;
        const open = disclosures.find((details) => details.open && details.contains(document.activeElement))
            ?? disclosures.find((details) => details.open);
        if (open) { event.preventDefault(); close(open, true); }
    });
    document.addEventListener('click', (event) => {
        for (const details of disclosures) if (!details.contains(event.target)) close(details);
    });
    window.matchMedia('(min-width: 80rem)').addEventListener('change', () => {
        for (const details of disclosures) close(details);
    });
})();
