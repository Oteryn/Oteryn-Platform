# syntax=docker/dockerfile:1.7

ARG NODE_IMAGE=node:22-bookworm-slim
FROM ${NODE_IMAGE}

ENV DEBIAN_FRONTEND=noninteractive \
    PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \
    PATH=/opt/oteryn-playwright/node_modules/.bin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

WORKDIR /opt/oteryn-playwright
COPY package.json ./package.json

RUN npm install --no-audit --no-fund --package-lock=false \
    && node -e 'const installed = require("./node_modules/@playwright/test/package.json").version; const expected = require("./package.json").devDependencies["@playwright/test"]; if (installed !== expected) { console.error(`Playwright version mismatch: installed ${installed}, expected ${expected}`); process.exit(1); }' \
    && mkdir -p "$PLAYWRIGHT_BROWSERS_PATH" \
    && npx playwright install --with-deps chromium \
    && npx playwright --version \
    && chmod -R a+rX /opt/oteryn-playwright "$PLAYWRIGHT_BROWSERS_PATH" \
    && npm cache clean --force \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
CMD ["node", "--version"]
