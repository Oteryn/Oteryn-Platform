'use strict';

const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { chromium } = require('playwright');

const baseUrl = process.env.ACCEPTANCE_BASE_URL || 'http://127.0.0.1:8080';
const outputDir = process.env.ACCEPTANCE_OUTPUT_DIR || '/tmp/oteryn-visual-acceptance';
const screenshotDir = path.join(outputDir, 'screenshots');
const validationSha = process.env.VALIDATION_SHA || 'UNKNOWN';

const regularEmail = 'visual.user@example.test';
const regularPassword = 'Acceptance-User-9!Pass';
const adminEmail = 'visual.admin@example.test';
const adminPassword = 'Acceptance-Admin-9!Pass';
const adminRecoveryCodes = ['ADMIN-00001', 'ADMIN-00002', 'ADMIN-00003', 'ADMIN-00004'];

const viewports = {
    desktop: { width: 1440, height: 1000 },
    tablet: { width: 768, height: 1024 },
    mobile: { width: 390, height: 844 },
};

fs.mkdirSync(screenshotDir, { recursive: true });

const results = [];

function safeName(value) {
    return value.replace(/[^a-zA-Z0-9._-]+/g, '-');
}

function base32Decode(input) {
    const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
    const normalized = input.toUpperCase().replace(/=+$/g, '').replace(/\s+/g, '');
    let bits = '';

    for (const character of normalized) {
        const value = alphabet.indexOf(character);
        if (value < 0) {
            throw new Error(`Invalid base32 character: ${character}`);
        }
        bits += value.toString(2).padStart(5, '0');
    }

    const bytes = [];
    for (let offset = 0; offset + 8 <= bits.length; offset += 8) {
        bytes.push(Number.parseInt(bits.slice(offset, offset + 8), 2));
    }

    return Buffer.from(bytes);
}

function currentTotp(secret) {
    const counter = Math.floor(Date.now() / 1000 / 30);
    const counterBuffer = Buffer.alloc(8);
    counterBuffer.writeBigUInt64BE(BigInt(counter));
    const digest = crypto.createHmac('sha1', base32Decode(secret)).update(counterBuffer).digest();
    const offset = digest[digest.length - 1] & 0x0f;
    const binary = ((digest[offset] & 0x7f) << 24)
        | ((digest[offset + 1] & 0xff) << 16)
        | ((digest[offset + 2] & 0xff) << 8)
        | (digest[offset + 3] & 0xff);

    return String(binary % 1_000_000).padStart(6, '0');
}

async function collectDomMetrics(page) {
    return page.evaluate(() => {
        const parseRgb = (value) => {
            const match = value.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/i);
            if (!match) {
                return null;
            }
            return {
                r: Number(match[1]),
                g: Number(match[2]),
                b: Number(match[3]),
                a: match[4] === undefined ? 1 : Number(match[4]),
            };
        };

        const channel = (value) => {
            const normalized = value / 255;
            return normalized <= 0.04045
                ? normalized / 12.92
                : ((normalized + 0.055) / 1.055) ** 2.4;
        };

        const luminance = (rgb) => 0.2126 * channel(rgb.r) + 0.7152 * channel(rgb.g) + 0.0722 * channel(rgb.b);
        const ratio = (foreground, background) => {
            const light = Math.max(luminance(foreground), luminance(background));
            const dark = Math.min(luminance(foreground), luminance(background));
            return (light + 0.05) / (dark + 0.05);
        };

        const opaqueBackground = (element) => {
            let current = element;
            while (current instanceof Element) {
                const candidate = parseRgb(getComputedStyle(current).backgroundColor);
                if (candidate && candidate.a > 0.98) {
                    return candidate;
                }
                current = current.parentElement;
            }
            return { r: 255, g: 255, b: 255, a: 1 };
        };

        const contrastSelectors = ['body', 'a', '.muted', '.notice', 'button', 'input', 'textarea', 'select'];
        const contrastSamples = [];
        for (const selector of contrastSelectors) {
            const element = document.querySelector(selector);
            if (!(element instanceof Element)) {
                continue;
            }
            const foreground = parseRgb(getComputedStyle(element).color);
            const background = opaqueBackground(element);
            if (!foreground) {
                continue;
            }
            contrastSamples.push({
                selector,
                ratio: Number(ratio(foreground, background).toFixed(2)),
                foreground: getComputedStyle(element).color,
                background: `rgb(${background.r}, ${background.g}, ${background.b})`,
            });
        }

        const controls = Array.from(document.querySelectorAll('input:not([type="hidden"]), select, textarea'));
        const unlabeledControls = controls
            .filter((control) => {
                const id = control.getAttribute('id');
                const explicit = id ? document.querySelector(`label[for="${CSS.escape(id)}"]`) : null;
                const wrapped = control.closest('label');
                const ariaLabel = control.getAttribute('aria-label');
                const ariaLabelledBy = control.getAttribute('aria-labelledby');
                return !explicit && !wrapped && !ariaLabel && !ariaLabelledBy;
            })
            .map((control) => ({
                tag: control.tagName.toLowerCase(),
                id: control.getAttribute('id'),
                name: control.getAttribute('name'),
                type: control.getAttribute('type'),
            }));

        const tables = Array.from(document.querySelectorAll('table')).map((table) => ({
            hasCaption: Boolean(table.querySelector('caption')),
            headerCells: table.querySelectorAll('th').length,
            hasThead: Boolean(table.querySelector('thead')),
            rows: table.querySelectorAll('tbody tr').length,
            scrollWidth: table.scrollWidth,
            clientWidth: table.clientWidth,
        }));

        const root = document.documentElement;
        const bodyText = document.body?.innerText || '';
        const rawTechnicalMessage = /(stack trace|SQLSTATE\[|\/vendor\/|Illuminate\\|PDOException|RedisException|Symfony\\Component\\HttpKernel)/i.test(bodyText);
        const links = Array.from(document.querySelectorAll('a'));
        const underlinedLinks = links.filter((link) => getComputedStyle(link).textDecorationLine.includes('underline')).length;

        return {
            htmlLang: document.documentElement.getAttribute('lang') || '',
            title: document.title,
            headingSequence: Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6')).map((heading) => ({
                level: Number(heading.tagName.slice(1)),
                text: (heading.textContent || '').trim().slice(0, 160),
            })),
            landmarks: {
                main: document.querySelectorAll('main').length,
                nav: document.querySelectorAll('nav').length,
                header: document.querySelectorAll('header').length,
            },
            controls: controls.length,
            unlabeledControls,
            tables,
            linkCount: links.length,
            buttonCount: document.querySelectorAll('button').length,
            underlinedLinks,
            horizontalOverflow: root.scrollWidth > root.clientWidth + 1,
            documentScrollWidth: root.scrollWidth,
            documentClientWidth: root.clientWidth,
            rawTechnicalMessage,
            contrastSamples,
            lowContrastSamples: contrastSamples.filter((sample) => sample.ratio < 4.5),
            statusRegions: document.querySelectorAll('[role="status"]').length,
            alertRegions: document.querySelectorAll('[role="alert"]').length,
        };
    });
}

async function collectKeyboardMetrics(page) {
    const sequence = [];
    await page.evaluate(() => {
        if (document.activeElement instanceof HTMLElement) {
            document.activeElement.blur();
        }
    });

    for (let index = 0; index < 6; index += 1) {
        await page.keyboard.press('Tab');
        sequence.push(await page.evaluate(() => {
            const element = document.activeElement;
            if (!(element instanceof HTMLElement)) {
                return null;
            }
            const style = getComputedStyle(element);
            const outlineWidth = Number.parseFloat(style.outlineWidth || '0');
            const visibleFocus = (style.outlineStyle !== 'none' && outlineWidth > 0)
                || (style.boxShadow !== 'none' && style.boxShadow !== '');
            return {
                tag: element.tagName.toLowerCase(),
                id: element.id || null,
                name: element.getAttribute('name'),
                text: (element.innerText || element.getAttribute('aria-label') || element.getAttribute('value') || '').trim().slice(0, 120),
                visibleFocus,
                outlineStyle: style.outlineStyle,
                outlineWidth: style.outlineWidth,
                boxShadow: style.boxShadow,
            };
        }));
    }

    return {
        sequence: sequence.filter(Boolean),
        firstFocusableVisible: sequence.some((entry) => entry && entry.visibleFocus),
    };
}

async function recordPage(page, name, viewportName, expectedStatus, actualStatus, consoleErrors, pageErrors) {
    await page.waitForLoadState('networkidle').catch(() => {});
    const screenshotPath = path.join(screenshotDir, `${safeName(name)}-${viewportName}.png`);
    await page.screenshot({ path: screenshotPath, fullPage: true });

    const dom = await collectDomMetrics(page);
    const keyboard = await collectKeyboardMetrics(page);
    const record = {
        name,
        viewport: viewportName,
        viewportSize: viewports[viewportName],
        url: page.url(),
        expectedStatus,
        actualStatus,
        statusMatches: actualStatus === null || actualStatus === expectedStatus,
        screenshot: path.relative(outputDir, screenshotPath),
        dom,
        keyboard,
        consoleErrors,
        pageErrors,
    };

    results.push(record);
    process.stdout.write(`ACCEPTANCE_RESULT ${JSON.stringify(record)}\n`);
    return record;
}

async function openAndRecord(context, name, viewportName, route, options = {}) {
    const page = await context.newPage();
    const consoleErrors = [];
    const pageErrors = [];
    page.on('console', (message) => {
        if (message.type() === 'error') {
            consoleErrors.push(message.text());
        }
    });
    page.on('pageerror', (error) => pageErrors.push(error.message));

    const response = await page.goto(`${baseUrl}${route}`, { waitUntil: 'networkidle' });
    if (typeof options.prepare === 'function') {
        await options.prepare(page);
    }
    const expectedStatus = options.expectedStatus ?? 200;
    const actualStatus = response ? response.status() : null;
    await recordPage(page, name, viewportName, expectedStatus, actualStatus, consoleErrors, pageErrors);
    await page.close();
}

async function recordExistingPage(page, name, viewportName, expectedStatus = 200, actualStatus = 200) {
    const consoleErrors = [];
    const pageErrors = [];
    page.on('console', (message) => {
        if (message.type() === 'error') {
            consoleErrors.push(message.text());
        }
    });
    page.on('pageerror', (error) => pageErrors.push(error.message));
    return recordPage(page, name, viewportName, expectedStatus, actualStatus, consoleErrors, pageErrors);
}

async function login(page, email, password) {
    await page.goto(`${baseUrl}/login`, { waitUntil: 'networkidle' });
    await page.locator('#email').fill(email);
    await page.locator('#password').fill(password);
    await page.getByRole('button', { name: 'Sign in' }).click();
    await page.waitForLoadState('networkidle');
}

async function submitMfaChallenge(page, code) {
    await page.locator('#code').fill(code);
    await page.getByRole('button', { name: 'Verify and sign in' }).click();
    await page.waitForLoadState('networkidle');
}

async function capturePublicSurfaces(browser, viewportName) {
    const context = await browser.newContext({ viewport: viewports[viewportName] });
    const surfaces = [
        ['home', '/', async (page) => page.locator('#character-name').fill('Acceptance Hero')],
        ['news-list', '/news'],
        ['news-detail', '/news/welcome-to-oteryn'],
        ['online', '/online'],
        ['highscores', '/highscores'],
        ['servers', '/servers'],
        ['character-detail', `/characters/${encodeURIComponent('Acceptance Hero')}`],
        ['guild-detail', `/guilds/${encodeURIComponent('Acceptance Guild')}`],
        ['managed-public-page', '/pages/about-oteryn'],
    ];

    for (const [name, route, prepare] of surfaces) {
        await openAndRecord(context, name, viewportName, route, { prepare });
    }

    await context.close();
}

async function captureGuestIdentitySurfaces(browser, viewportName) {
    const context = await browser.newContext({ viewport: viewports[viewportName] });
    await openAndRecord(context, 'login', viewportName, '/login');
    await openAndRecord(context, 'registration', viewportName, '/register');
    await openAndRecord(context, 'password-recovery', viewportName, '/forgot-password');
    await openAndRecord(
        context,
        'password-reset',
        viewportName,
        '/reset-password/acceptance-token?email=visual.user%40example.test',
    );
    await context.close();
}

async function captureValidationStates(browser, viewportName) {
    const context = await browser.newContext({ viewport: viewports[viewportName] });

    const loginPage = await context.newPage();
    await loginPage.goto(`${baseUrl}/login`, { waitUntil: 'networkidle' });
    await loginPage.locator('#email').fill(regularEmail);
    await loginPage.locator('#password').fill('Incorrect-Password-9!');
    await loginPage.getByRole('button', { name: 'Sign in' }).click();
    await loginPage.waitForLoadState('networkidle');
    await recordExistingPage(loginPage, 'login-validation-error', viewportName);
    await loginPage.close();

    const registerPage = await context.newPage();
    await registerPage.goto(`${baseUrl}/register`, { waitUntil: 'networkidle' });
    await registerPage.locator('#email').fill(regularEmail);
    await registerPage.locator('#password').fill('Mismatch-Password-9!');
    await registerPage.locator('#password_confirmation').fill('Different-Password-9!');
    await registerPage.getByRole('button', { name: 'Register' }).click();
    await registerPage.waitForLoadState('networkidle');
    await recordExistingPage(registerPage, 'registration-validation-error', viewportName);
    await registerPage.close();

    await context.close();
}

async function captureRegularAccountFlow(browser) {
    const desktopContext = await browser.newContext({ viewport: viewports.desktop });
    const desktopPage = await desktopContext.newPage();
    await login(desktopPage, regularEmail, regularPassword);

    await openAndRecord(desktopContext, 'character-creation', 'desktop', '/account/characters/create');
    await openAndRecord(desktopContext, 'password-change', 'desktop', '/password/change');
    await openAndRecord(desktopContext, 'mfa-settings-not-enabled', 'desktop', '/mfa');

    await desktopPage.goto(`${baseUrl}/mfa`, { waitUntil: 'networkidle' });
    await desktopPage.getByRole('button', { name: 'Start MFA enrollment' }).click();
    await desktopPage.waitForLoadState('networkidle');
    await recordExistingPage(desktopPage, 'mfa-enrollment', 'desktop');
    const secret = (await desktopPage.locator('p').filter({ hasText: 'Manual secret:' }).locator('code').textContent() || '').trim();
    if (!secret) {
        throw new Error('MFA enrollment secret was not rendered.');
    }

    const mobileContext = await browser.newContext({ viewport: viewports.mobile });
    const mobilePage = await mobileContext.newPage();
    await login(mobilePage, regularEmail, regularPassword);
    await openAndRecord(mobileContext, 'character-creation', 'mobile', '/account/characters/create');
    await openAndRecord(mobileContext, 'password-change', 'mobile', '/password/change');
    await openAndRecord(mobileContext, 'mfa-enrollment', 'mobile', '/mfa');

    await desktopPage.locator('#current_password').fill(regularPassword);
    await desktopPage.locator('#code').fill(currentTotp(secret));
    await desktopPage.getByRole('button', { name: 'Confirm and enable MFA' }).click();
    await desktopPage.waitForLoadState('networkidle');
    await recordExistingPage(desktopPage, 'mfa-recovery-codes', 'desktop');
    const regularRecoveryCodes = await desktopPage.locator('main li code').allTextContents();
    if (regularRecoveryCodes.length === 0) {
        throw new Error('MFA recovery codes were not rendered after enrollment confirmation.');
    }

    await openAndRecord(desktopContext, 'authorization-denied-403', 'desktop', '/admin', { expectedStatus: 403 });

    await mobileContext.close();
    await desktopContext.close();

    const deniedMobileContext = await browser.newContext({ viewport: viewports.mobile });
    const deniedMobilePage = await deniedMobileContext.newPage();
    await login(deniedMobilePage, regularEmail, regularPassword);
    if (!deniedMobilePage.url().includes('/mfa/challenge')) {
        throw new Error(`Expected regular MFA challenge, got ${deniedMobilePage.url()}`);
    }
    await submitMfaChallenge(deniedMobilePage, regularRecoveryCodes[0]);
    await openAndRecord(deniedMobileContext, 'authorization-denied-403', 'mobile', '/admin', { expectedStatus: 403 });
    await deniedMobileContext.close();
}

async function authenticatedAdminContext(browser, viewportName, recoveryCode, captureChallenge) {
    const context = await browser.newContext({ viewport: viewports[viewportName] });
    const page = await context.newPage();
    await login(page, adminEmail, adminPassword);
    if (!page.url().includes('/mfa/challenge')) {
        throw new Error(`Expected admin MFA challenge, got ${page.url()}`);
    }
    if (captureChallenge) {
        await recordExistingPage(page, 'mfa-challenge', viewportName);
    }
    await submitMfaChallenge(page, recoveryCode);
    if (page.url().includes('/mfa/challenge')) {
        throw new Error(`Admin MFA challenge did not complete for ${viewportName}.`);
    }
    await page.close();
    return context;
}

async function captureAdminSurfaces(browser, viewportName, recoveryCode) {
    const context = await authenticatedAdminContext(browser, viewportName, recoveryCode, true);
    const surfaces = [
        ['admin-dashboard', '/admin'],
        ['admin-news-list', '/admin/news'],
        ['admin-news-form', '/admin/news/create'],
        ['admin-managed-pages-list', '/admin/pages'],
        ['admin-managed-page-form', '/admin/pages/create'],
        ['admin-role-management', '/admin/roles'],
        ['admin-audit-log', '/admin/audit'],
        ['mfa-settings-confirmed', '/mfa'],
    ];

    for (const [name, route] of surfaces) {
        await openAndRecord(context, name, viewportName, route);
    }

    await context.close();
}

async function captureTabletSurfaces(browser) {
    const publicContext = await browser.newContext({ viewport: viewports.tablet });
    await openAndRecord(publicContext, 'highscores', 'tablet', '/highscores');
    await openAndRecord(publicContext, 'guild-detail', 'tablet', `/guilds/${encodeURIComponent('Acceptance Guild')}`);
    await openAndRecord(publicContext, 'servers', 'tablet', '/servers');
    await publicContext.close();

    const adminContext = await authenticatedAdminContext(browser, 'tablet', adminRecoveryCodes[2], false);
    await openAndRecord(adminContext, 'admin-role-management', 'tablet', '/admin/roles');
    await openAndRecord(adminContext, 'admin-audit-log', 'tablet', '/admin/audit');
    await adminContext.close();
}

async function captureRepresentativeFailureStates(browser) {
    const desktopContext = await browser.newContext({ viewport: viewports.desktop });
    const mobileContext = await browser.newContext({ viewport: viewports.mobile });

    await openAndRecord(desktopContext, 'not-found-404', 'desktop', '/definitely-not-a-real-route', { expectedStatus: 404 });
    await openAndRecord(mobileContext, 'not-found-404', 'mobile', '/definitely-not-a-real-route', { expectedStatus: 404 });

    execFileSync('php', ['scripts/acceptance/seed.php', 'empty-news'], { stdio: 'inherit' });
    await openAndRecord(desktopContext, 'news-empty-state', 'desktop', '/news');
    await openAndRecord(mobileContext, 'news-empty-state', 'mobile', '/news');

    execFileSync('redis-cli', ['shutdown', 'nosave'], { stdio: 'ignore' });
    await new Promise((resolve) => setTimeout(resolve, 500));
    await openAndRecord(desktopContext, 'servers-runtime-dependency-failure', 'desktop', '/servers');
    await openAndRecord(mobileContext, 'servers-runtime-dependency-failure', 'mobile', '/servers');

    // Game data has no native source yet, so /online already answers the contained unavailable state.
    await openAndRecord(desktopContext, 'online-dependency-failure-503', 'desktop', '/online', { expectedStatus: 503 });
    await openAndRecord(mobileContext, 'online-dependency-failure-503', 'mobile', '/online', { expectedStatus: 503 });

    await desktopContext.close();
    await mobileContext.close();
}

function writeSummary() {
    const problematic = {
        statusMismatch: results.filter((record) => !record.statusMatches).map((record) => `${record.name}/${record.viewport}`),
        horizontalOverflow: results.filter((record) => record.dom.horizontalOverflow).map((record) => `${record.name}/${record.viewport}`),
        unlabeledControls: results.filter((record) => record.dom.unlabeledControls.length > 0).map((record) => ({
            surface: `${record.name}/${record.viewport}`,
            controls: record.dom.unlabeledControls,
        })),
        lowContrast: results.filter((record) => record.dom.lowContrastSamples.length > 0).map((record) => ({
            surface: `${record.name}/${record.viewport}`,
            samples: record.dom.lowContrastSamples,
        })),
        focusNotObserved: results.filter((record) => record.keyboard.sequence.length > 0 && !record.keyboard.firstFocusableVisible).map((record) => `${record.name}/${record.viewport}`),
        rawTechnicalMessages: results.filter((record) => record.dom.rawTechnicalMessage).map((record) => `${record.name}/${record.viewport}`),
        browserErrors: results.filter((record) => record.consoleErrors.length > 0 || record.pageErrors.length > 0).map((record) => ({
            surface: `${record.name}/${record.viewport}`,
            consoleErrors: record.consoleErrors,
            pageErrors: record.pageErrors,
        })),
    };

    const payload = {
        classification: 'VISUAL_UX_EVIDENCE_COLLECTED',
        validationSha,
        baseUrl,
        generatedAtUtc: new Date().toISOString(),
        screenshotCount: results.length,
        viewports,
        results,
        problematic,
        environmentNote: 'Exact application SHA and production APP_ENV were used. Browser authentication ran over loopback HTTP with SESSION_SECURE_COOKIE=false solely because php artisan serve does not terminate TLS; secure-cookie production behavior remains covered by the independent Phase 7 staging evidence.',
    };

    fs.writeFileSync(path.join(outputDir, 'visual-acceptance-results.json'), `${JSON.stringify(payload, null, 2)}\n`);

    const summaryLines = [
        '# Visual / UX acceptance browser evidence',
        '',
        `- validation_sha: \`${validationSha}\``,
        `- generated_at_utc: \`${payload.generatedAtUtc}\``,
        `- screenshots: ${results.length}`,
        `- status_mismatches: ${problematic.statusMismatch.length}`,
        `- horizontal_overflow_surfaces: ${problematic.horizontalOverflow.length}`,
        `- unlabeled_control_surfaces: ${problematic.unlabeledControls.length}`,
        `- low_contrast_sample_surfaces: ${problematic.lowContrast.length}`,
        `- focus_not_observed_surfaces: ${problematic.focusNotObserved.length}`,
        `- raw_technical_message_surfaces: ${problematic.rawTechnicalMessages.length}`,
        `- browser_error_surfaces: ${problematic.browserErrors.length}`,
        '',
        '## Horizontal overflow',
        '',
        ...(problematic.horizontalOverflow.length > 0 ? problematic.horizontalOverflow.map((value) => `- ${value}`) : ['- none']),
        '',
        '## Status mismatches',
        '',
        ...(problematic.statusMismatch.length > 0 ? problematic.statusMismatch.map((value) => `- ${value}`) : ['- none']),
        '',
        '## Environment limitation',
        '',
        payload.environmentNote,
    ];
    fs.writeFileSync(path.join(outputDir, 'visual-acceptance-summary.md'), `${summaryLines.join('\n')}\n`);
    process.stdout.write(`ACCEPTANCE_SUMMARY_JSON ${JSON.stringify({
        validationSha,
        screenshotCount: results.length,
        problematic,
    })}\n`);
}

(async () => {
    const browser = await chromium.launch({ headless: true });
    try {
        await capturePublicSurfaces(browser, 'desktop');
        await capturePublicSurfaces(browser, 'mobile');
        await captureGuestIdentitySurfaces(browser, 'desktop');
        await captureGuestIdentitySurfaces(browser, 'mobile');
        await captureValidationStates(browser, 'desktop');
        await captureValidationStates(browser, 'mobile');
        await captureRegularAccountFlow(browser);
        await captureAdminSurfaces(browser, 'desktop', adminRecoveryCodes[0]);
        await captureAdminSurfaces(browser, 'mobile', adminRecoveryCodes[1]);
        await captureTabletSurfaces(browser);
        await captureRepresentativeFailureStates(browser);
        writeSummary();
    } finally {
        await browser.close();
    }
})().catch((error) => {
    console.error(error);
    process.exit(1);
});
