<?php

declare(strict_types=1);

use App\Cms\Editorial\EditorialContentType;
use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Models\EditorialTranslation;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\NewsPost;
use Illuminate\Contracts\Console\Kernel;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$englishTitle = implode(' ', array_map(
    static fn (int $index): string => sprintf('EnglishSegment%02d', $index),
    range(1, 11),
));
$polishTitle = implode(' ', array_map(
    static fn (int $index): string => sprintf('PolskiSegment%02d', $index),
    range(1, 11),
));
$englishBody = 'English managed content boundary '.implode(' ', array_map(
    static fn (int $index): string => sprintf('readable-content-segment-%03d', $index),
    range(1, 90),
));
$polishBody = 'Polska granica długiej treści '.implode(' ', array_map(
    static fn (int $index): string => sprintf('czytelny-segment-treści-%03d', $index),
    range(1, 100),
));
$publishedAt = now()->subMinute();

$news = NewsPost::query()->updateOrCreate(
    ['slug' => 'acceptance-long-localized-news'],
    [
        'title' => $englishTitle,
        'body' => $englishBody,
        'published_at' => $publishedAt,
    ],
);
$news->refresh();

$page = ManagedPage::query()->updateOrCreate(
    ['slug' => 'acceptance-long-localized-page'],
    [
        'title' => $englishTitle,
        'body' => $englishBody,
        'published_at' => $publishedAt,
    ],
);
$page->refresh();

$terms = ManagedPage::query()->updateOrCreate(
    ['slug' => EditorialPageKey::Terms->managedPageSlug()],
    [
        'title' => $englishTitle,
        'body' => $englishBody,
        'legal_version' => '2026.1',
        'legal_effective_date' => '2026-07-01',
        'published_at' => $publishedAt,
    ],
);
$terms->refresh();

foreach ([
    [EditorialContentType::NewsPost, $news],
    [EditorialContentType::ManagedPage, $page],
    [EditorialContentType::ManagedPage, $terms],
] as [$type, $source]) {
    EditorialTranslation::query()->updateOrCreate(
        [
            'content_type' => $type->value,
            'content_id' => $source->getKey(),
            'locale' => 'pl',
        ],
        [
            'title' => $polishTitle,
            'body' => $polishBody,
            'action_label' => null,
            'source_updated_at' => $source->updated_at,
            'published_at' => $publishedAt,
        ],
    );
}

fwrite(STDOUT, json_encode([
    'news_slug' => $news->slug,
    'page_slug' => $page->slug,
    'english_title' => $englishTitle,
    'polish_title' => $polishTitle,
    'english_body' => $englishBody,
    'polish_body' => $polishBody,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE).PHP_EOL);
