# syntax=docker/dockerfile:1.7

ARG PLAYWRIGHT_IMAGE=mcr.microsoft.com/playwright:v1.62.0-noble@sha256:02bbb2155cd7109e3e9c741941097ed1608cf8b6fa44ee2595896da2bdc1f471
FROM ${PLAYWRIGHT_IMAGE}

ENV DEBIAN_FRONTEND=noninteractive \
    PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 \
    PATH=/opt/oteryn-playwright/node_modules/.bin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

WORKDIR /opt/oteryn-playwright
COPY package.json ./package.json

RUN npm install --no-audit --no-fund --package-lock=false --ignore-scripts --save-exact @playwright/test@1.62.0 \
    && node -e 'const installed = require("./node_modules/@playwright/test/package.json").version; if (installed !== "1.62.0") { console.error(`Playwright version mismatch: installed ${installed}, expected 1.62.0`); process.exit(1); }' \
    && npx playwright --version \
    && chmod -R a+rX /opt/oteryn-playwright "$PLAYWRIGHT_BROWSERS_PATH" \
    && npm cache clean --force

WORKDIR /workspace
CMD ["node", "--version"]
