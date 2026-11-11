<?php

declare(strict_types=1);

use App\Wiki\Domain\WikiArticleStatus;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$now = now();

DB::transaction(function () use ($now): void {
    DB::table('wiki_revisions')->delete();
    DB::table('wiki_article_category')->delete();
    DB::table('wiki_article_translations')->delete();
    DB::table('wiki_category_translations')->delete();
    DB::table('wiki_articles')->delete();
    DB::table('wiki_categories')->delete();

    $categoryId = DB::table('wiki_categories')->insertGetId([
        'key' => 'getting-started',
        'sort_order' => 10,
        'visible' => true,
        'lock_version' => 1,
        'created_at' => $now,
        'updated_at' => $now,
    ]);
    DB::table('wiki_category_translations')->insert([
        [
            'category_id' => $categoryId,
            'locale' => 'en',
            'name' => 'Getting Started',
            'slug' => 'getting-started',
            'description' => 'Published acceptance guides for new players.',
            'created_at' => $now,
            'updated_at' => $now,
        ],
        [
            'category_id' => $categoryId,
            'locale' => 'pl',
            'name' => 'Pierwsze kroki',
            'slug' => 'pierwsze-kroki',
            'description' => 'Opublikowane poradniki dla nowych graczy.',
            'created_at' => $now,
            'updated_at' => $now,
        ],
    ]);

    $articleId = DB::table('wiki_articles')->insertGetId([
        'content_type' => 'guide',
        'status' => WikiArticleStatus::PUBLISHED->value,
        'is_featured' => true,
        'sort_order' => 10,
        'published_at' => $now->copy()->subMinute(),
        'lock_version' => 1,
        'created_at' => $now,
        'updated_at' => $now,
    ]);
    DB::table('wiki_article_translations')->insert([
        [
            'article_id' => $articleId,
            'locale' => 'en',
            'title' => 'First login',
            'slug' => 'first-login',
            'summary' => 'A deterministic public Wiki acceptance article.',
            'source_markdown' => <<<'MARKDOWN'
# Install the client

Use the approved Download page and verify the published checksum.

## Sign in safely

Never share your account password or recovery codes.

| Check | Expected result |
| --- | --- |
| Client | Approved release |
| Account | Your own identity |
MARKDOWN,
            'created_at' => $now,
            'updated_at' => $now,
        ],
        [
            'article_id' => $articleId,
            'locale' => 'pl',
            'title' => 'Pierwsze logowanie',
            'slug' => 'pierwsze-logowanie',
            'summary' => 'Deterministyczny artykuł testowy publicznego Wiki.',
            'source_markdown' => <<<'MARKDOWN'
# Zainstaluj klienta

Użyj zatwierdzonej strony pobierania i sprawdź opublikowaną sumę kontrolną.

## Zaloguj się bezpiecznie

Nigdy nie udostępniaj hasła ani kodów odzyskiwania.
MARKDOWN,
            'created_at' => $now,
            'updated_at' => $now,
        ],
    ]);
    DB::table('wiki_article_category')->insert([
        'article_id' => $articleId,
        'category_id' => $categoryId,
        'sort_order' => 10,
    ]);
});

fwrite(STDOUT, "acceptance-state: public Wiki seeded\n");
