import { expect } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { submit, waitForWikiBudget } from '../helpers.mjs';

export async function publishEditorialFixtures(page, locale) {
  const suffix = randomUUID();
  const news = `quality-news-${suffix}`;
  const event = `quality-event-${suffix}`;
  const guide = `quality-guide-${suffix}`;
  const now = Date.now();
  const published = new Date(now - 60_000).toISOString();

  expect((await page.goto('/admin/news/create')).status()).toBe(200);
  await submit(page, 'main form', {
    slug: news,
    title: 'Quality suite news',
    title_pl: 'Aktualności testowe',
    body: 'Published through the staff browser form.',
    body_pl: 'Opublikowano przez formularz administracyjny.',
    published_at: published,
    translation_published_at: published,
  });
  await expect(page).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit$/);
  expect((await page.goto(`/${locale}/news/${news}`)).status()).toBe(200);

  expect((await page.goto('/admin/events/create')).status()).toBe(200);
  await submit(page, 'main form', {
    starts_at: new Date(now - 3_600_000).toISOString().slice(0, 16),
    ends_at: new Date(now + 86_400_000).toISOString().slice(0, 16),
    en_title: 'Quality suite event', pl_title: 'Wydarzenie testowe',
    en_slug: event, pl_slug: event,
    en_summary: 'Public test event.', pl_summary: 'Publiczne wydarzenie testowe.',
    en_body: 'Published through the staff browser form.',
    pl_body: 'Opublikowano przez formularz administracyjny.',
  });
  await expect(page).toHaveURL(/\/admin\/events\/[a-f0-9-]+\/edit$/);
  await page.locator('main form[action$="/status"] select[name=status]').selectOption('published');
  await submit(page, 'main form[action$="/status"]');
  expect((await page.goto(`/${locale}/events/${event}`)).status()).toBe(200);

  expect((await page.goto('/admin/wiki/articles/create')).status()).toBe(200);
  await submit(page, 'main form', {
    en_title: 'Quality suite guide', pl_title: 'Poradnik testowy',
    en_slug: guide, pl_slug: guide,
    en_summary: 'Public test guide.', pl_summary: 'Publiczny poradnik testowy.',
    en_markdown: 'Published through the staff browser form.',
    pl_markdown: 'Opublikowano przez formularz administracyjny.',
    note: 'Quality suite browser fixture.',
  });
  await expect(page).toHaveURL(/\/admin\/wiki\/articles\/[a-f0-9-]+\/edit$/);
  await submit(page, 'main form[action$="/submit-review"]');
  await submit(page, 'main form[action$="/publish"]');
  const target = `/${locale}/wiki/articles/${guide}`;
  await waitForWikiBudget(target);
  const response = await page.goto(target);
  expect(response.status(), 'published Wiki article opens').toBe(200);
  return [`/${locale}/news/${news}`, `/${locale}/events/${event}`, target];
}
