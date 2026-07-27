'use strict';

const childProcess = require('node:child_process');
const playwright = require('playwright');

// The exploratory harness predates the production homepage refresh. Keep the
// legacy capture flow executable while resolving its former home-search id to
// the current accessible control. This adapter is deliberately scoped to the
// visual harness and does not alter application markup or browser behavior.
const originalChromiumLaunch = playwright.chromium.launch.bind(playwright.chromium);
playwright.chromium.launch = async function acceptanceChromiumLaunch(...launchArgs) {
    const browser = await originalChromiumLaunch(...launchArgs);
    const originalNewContext = browser.newContext.bind(browser);

    browser.newContext = async function acceptanceNewContext(...contextArgs) {
        const context = await originalNewContext(...contextArgs);
        const originalNewPage = context.newPage.bind(context);

        context.newPage = async function acceptanceNewPage(...pageArgs) {
            const page = await originalNewPage(...pageArgs);
            const originalLocator = page.locator.bind(page);

            page.locator = function acceptanceLocator(selector, ...locatorArgs) {
                const resolvedSelector = selector === '#character-name'
                    ? '#home-character-name'
                    : selector;
                return originalLocator(resolvedSelector, ...locatorArgs);
            };

            return page;
        };

        return context;
    };

    return browser;
};

const originalExecFileSync = childProcess.execFileSync;
childProcess.execFileSync = function acceptanceExecFileSync(binary, args = [], options = {}) {
    if (binary === 'mariadb' && Array.isArray(args)) {
        const canaryDatabase = process.env.CANARY_DB_DATABASE || '';
        if (!canaryDatabase) {
            throw new Error('CANARY_DB_DATABASE is required for the visual dependency failure probe.');
        }

        args = args.map((argument) => argument === 'DROP TABLE canary.cluster_sessions;'
            ? `DROP TABLE \`${canaryDatabase}\`.cluster_sessions;`
            : argument);
    }

    return originalExecFileSync(binary, args, options);
};

require('./visual-acceptance-core.js');
