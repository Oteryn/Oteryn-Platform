<?php

declare(strict_types=1);

use App\Wiki\Domain\WikiArticleStatus;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$englishTitle = 'ScaleLexicon '.implode(' ', array_map(
    static fn (int $index): string => sprintf('EnglishWikiSegment%02d', $index),
    range(1, 8),
));
$polishTitle = 'ScaleLexicon '.implode(' ', array_map(
    static fn (int $index): string => sprintf('PolskiSegmentWiki%02d', $index),
    range(1, 9),
));
$englishSummary = 'ScaleLexicon English Wiki summary '.implode(' ', array_map(
    static fn (int $index): string => sprintf('summary-segment-%02d', $index),
    range(1, 24),
));
$polishSummary = 'ScaleLexicon polskie podsumowanie Wiki '.implode(' ', array_map(
    static fn (int $index): string => sprintf('segment-podsumowania-%02d', $index),
    range(1, 24),
));
$englishBody = 'ScaleLexicon English Wiki body '.implode(' ', array_map(
    static fn (int $index): string => sprintf('wiki-body-segment-%03d', $index),
    range(1, 100),
));
$polishBody = 'ScaleLexicon polska treść Wiki '.implode(' ', array_map(
    static fn (int $index): string => sprintf('segment-treści-wiki-%03d', $index),
    range(1, 100),
));
$now = now();

$articleIds = DB::table('wiki_article_translations')
    ->where('slug', 'like', 'content-scale-wiki-%')
    ->pluck('article_id')
    ->map(static fn (mixed $id): int => (int) $id)
    ->all();
$categoryIds = DB::table('wiki_categories')
    ->where('key', 'content-scale-wiki')
    ->pluck('id')
    ->map(static fn (mixed $id): int => (int) $id)
    ->all();

DB::transaction(static function () use ($articleIds, $categoryIds): void {
    if ($articleIds !== []) {
        DB::table('wiki_revisions')->whereIn('article_id', $articleIds)->delete();
        DB::table('wiki_article_category')->whereIn('article_id', $articleIds)->delete();
        DB::table('wiki_article_translations')->whereIn('article_id', $articleIds)->delete();
        DB::table('wiki_articles')->whereIn('id', $articleIds)->delete();
    }
    if ($categoryIds !== []) {
        DB::table('wiki_article_category')->whereIn('category_id', $categoryIds)->delete();
        DB::table('wiki_category_translations')->whereIn('category_id', $categoryIds)->delete();
        DB::table('wiki_categories')->whereIn('id', $categoryIds)->delete();
    }
});

$categoryId = DB::table('wiki_categories')->insertGetId([
    'key' => 'content-scale-wiki',
    'sort_order' => 900,
    'visible' => true,
    'lock_version' => 1,
    'created_at' => $now,
    'updated_at' => $now,
]);
DB::table('wiki_category_translations')->insert([
    [
        'category_id' => $categoryId,
        'locale' => 'en',
        'name' => 'ScaleLexicon Guides',
        'slug' => 'content-scale-wiki-en',
        'description' => 'ScaleLexicon bounded public collection.',
        'created_at' => $now,
        'updated_at' => $now,
    ],
    [
        'category_id' => $categoryId,
        'locale' => 'pl',
        'name' => 'Poradniki ScaleLexicon',
        'slug' => 'content-scale-wiki-pl',
        'description' => 'ScaleLexicon ograniczona kolekcja publiczna.',
        'created_at' => $now,
        'updated_at' => $now,
    ],
]);

$scaleIds = [];
for ($index = 1; $index <= 26; $index++) {
    $articleId = DB::table('wiki_articles')->insertGetId([
        'content_type' => 'guide',
        'status' => WikiArticleStatus::PUBLISHED->value,
        'is_featured' => false,
        'sort_order' => 100 + $index,
        'published_at' => $now->copy()->subMinutes($index + 2),
        'lock_version' => 1,
        'created_at' => $now,
        'updated_at' => $now->copy()->subSeconds($index),
    ]);
    $scaleIds[] = $articleId;

    DB::table('wiki_article_translations')->insert([
        [
            'article_id' => $articleId,
            'locale' => 'en',
            'title' => sprintf('ScaleLexicon Article %03d', $index),
            'slug' => sprintf('content-scale-wiki-en-%03d', $index),
            'summary' => sprintf('ScaleLexicon bounded search result %03d.', $index),
            'source_markdown' => sprintf('ScaleLexicon bounded Wiki body %03d.', $index),
            'created_at' => $now,
            'updated_at' => $now,
        ],
        [
            'article_id' => $articleId,
            'locale' => 'pl',
            'title' => sprintf('ScaleLexicon Artykuł %03d', $index),
            'slug' => sprintf('content-scale-wiki-pl-%03d', $index),
            'summary' => sprintf('ScaleLexicon ograniczony wynik wyszukiwania %03d.', $index),
            'source_markdown' => sprintf('ScaleLexicon ograniczona treść Wiki %03d.', $index),
            'created_at' => $now,
            'updated_at' => $now,
        ],
    ]);
    DB::table('wiki_article_category')->insert([
        'article_id' => $articleId,
        'category_id' => $categoryId,
        'sort_order' => $index,
    ]);
}

$articleId = DB::table('wiki_articles')->insertGetId([
    'content_type' => 'guide',
    'status' => WikiArticleStatus::PUBLISHED->value,
    'is_featured' => true,
    'sort_order' => 1,
    'published_at' => $now->copy()->subMinute(),
    'lock_version' => 1,
    'created_at' => $now,
    'updated_at' => $now->copy()->addSecond(),
]);
DB::table('wiki_article_translations')->insert([
    [
        'article_id' => $articleId,
        'locale' => 'en',
        'title' => $englishTitle,
        'slug' => 'content-scale-wiki-en-long',
        'summary' => $englishSummary,
        'source_markdown' => "# ScaleLexicon long article\n\n{$englishBody}",
        'created_at' => $now,
        'updated_at' => $now,
    ],
    [
        'article_id' => $articleId,
        'locale' => 'pl',
        'title' => $polishTitle,
        'slug' => 'content-scale-wiki-pl-long',
        'summary' => $polishSummary,
        'source_markdown' => "# Długi artykuł ScaleLexicon\n\n{$polishBody}",
        'created_at' => $now,
        'updated_at' => $now,
    ],
]);
DB::table('wiki_article_category')->insert([
    'article_id' => $articleId,
    'category_id' => $categoryId,
    'sort_order' => 1,
]);

fwrite(STDOUT, json_encode([
    'wiki_article_id' => $articleId,
    'wiki_query' => 'ScaleLexicon',
    'wiki_english_slug' => 'content-scale-wiki-en-long',
    'wiki_polish_slug' => 'content-scale-wiki-pl-long',
    'wiki_english_title' => $englishTitle,
    'wiki_polish_title' => $polishTitle,
    'wiki_english_summary' => $englishSummary,
    'wiki_polish_summary' => $polishSummary,
    'wiki_english_body' => $englishBody,
    'wiki_polish_body' => $polishBody,
    'wiki_admin_page_two_ids' => array_slice($scaleIds, 0, 2),
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE).PHP_EOL);
