# syntax=docker/dockerfile:1.7

# One-shot trusted-main repair for the existing local Synology Playwright cache.
# The base already contains the PHP 8.5 runtime and browser system dependencies.
# Refresh only the exact repository Playwright package + browser binaries; do not
# repeat `playwright install --with-deps`, which is prohibitively expensive on NAS.
ARG BASE_IMAGE=oteryn-playwright-php85
FROM ${BASE_IMAGE}

ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright
WORKDIR /opt/oteryn-playwright

COPY scripts/acceptance/package.json ./package.json
RUN npm install --no-audit --no-fund --package-lock=false \
    && node -e 'const installed = require("./node_modules/@playwright/test/package.json").version; const expected = require("./package.json").devDependencies["@playwright/test"]; if (installed !== expected) { console.error(`Playwright version mismatch: installed ${installed}, expected ${expected}`); process.exit(1); }' \
    && npx playwright install chromium firefox webkit \
    && npx playwright --version \
    && chmod -R a+rX /opt/oteryn-playwright "$PLAYWRIGHT_BROWSERS_PATH" \
    && npm cache clean --force

COPY scripts/acceptance/run-playwright-ci.sh /usr/local/bin/oteryn-playwright-ci
RUN chmod 0755 /usr/local/bin/oteryn-playwright-ci

WORKDIR /workspace
ENTRYPOINT ["/usr/local/bin/oteryn-playwright-ci"]
CMD ["--runtime-smoke"]
