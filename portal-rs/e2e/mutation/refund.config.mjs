import { defineConfig } from '@playwright/test';
import matrix from './playwright.config.mjs';

export default defineConfig({
  ...matrix,
  grep: /@uncredited-payment/,
  grepInvert: undefined,
});
