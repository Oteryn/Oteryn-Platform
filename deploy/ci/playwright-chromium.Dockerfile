# syntax=docker/dockerfile:1.7

ARG PLAYWRIGHT_IMAGE=mcr.microsoft.com/playwright:v1.62.1-noble@sha256:c091b21d9fae78c76e85cd4356431e9b018402f172a214fc7d7a5e9a7e29d8ac
FROM ${PLAYWRIGHT_IMAGE}

ENV DEBIAN_FRONTEND=noninteractive \
    PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 \
    PATH=/opt/oteryn-playwright/node_modules/.bin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

WORKDIR /opt/oteryn-playwright
COPY package.json ./package.json

RUN npm install --no-audit --no-fund --package-lock=false --ignore-scripts \
    && node -e 'const installed = require("./node_modules/@playwright/test/package.json").version; const expected = require("./package.json").devDependencies["@playwright/test"]; if (installed !== expected) { console.error(`Playwright version mismatch: installed ${installed}, expected ${expected}`); process.exit(1); }' \
    && npx playwright --version \
    && chmod -R a+rX /opt/oteryn-playwright "$PLAYWRIGHT_BROWSERS_PATH" \
    && npm cache clean --force

WORKDIR /workspace
CMD ["node", "--version"]
