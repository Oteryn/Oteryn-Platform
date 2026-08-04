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
let redisServiceNeedsRestart = false;

function restoreRedisService() {
    if (!redisServiceNeedsRestart) {
        return;
    }

    try {
        const containerIds = String(originalExecFileSync(
            'docker',
            ['ps', '-aq', '--filter', 'ancestor=redis:7.4-alpine'],
            { encoding: 'utf8' },
        ))
            .split('\n')
            .map((value) => value.trim())
            .filter(Boolean);
        const containerId = containerIds[0];

        if (!containerId) {
            throw new Error('The Redis 7.4 service container could not be identified.');
        }

        originalExecFileSync('docker', ['start', containerId], { stdio: 'ignore' });

        for (let attempt = 1; attempt <= 30; attempt += 1) {
            try {
                const response = String(originalExecFileSync(
                    'redis-cli',
                    ['ping'],
                    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] },
                )).trim();

                if (response === 'PONG') {
                    redisServiceNeedsRestart = false;
                    return;
                }
            } catch {
                // Retry until the restarted service accepts connections.
            }

            Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1000);
        }

        throw new Error('The restarted Redis service did not become ready within 30 seconds.');
    } catch (error) {
        process.stderr.write(`Failed to restore Redis after visual failure-state capture: ${error.stack || error}\n`);
        process.exitCode = 1;
    }
}

process.on('exit', restoreRedisService);

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

    if (
        binary === 'redis-cli'
        && Array.isArray(args)
        && args[0] === 'shutdown'
        && args[1] === 'nosave'
    ) {
        redisServiceNeedsRestart = true;
    }

    return originalExecFileSync(binary, args, options);
};

require('./visual-acceptance-core.js');
