<?php

declare(strict_types=1);

use App\Announcements\Models\SiteAnnouncement;
use App\Cms\Editorial\EditorialContentType;
use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Models\EditorialTranslation;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\NewsPost;
use App\Downloads\Models\ClientRelease;
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
$longFilename = 'oteryn-'.implode('-', array_map(
    static fn (int $index): string => sprintf('artifact-segment-%02d', $index),
    range(1, 14),
)).'.zip';
$publishedAt = now()->subMinute();

for ($index = 1; $index <= 26; $index++) {
    NewsPost::query()->updateOrCreate(
        ['slug' => sprintf('acceptance-scale-news-%03d', $index)],
        [
            'title' => sprintf('Content Scale News %03d', $index),
            'body' => sprintf('Bounded administrator pagination fixture %03d.', $index),
            'published_at' => $publishedAt->copy()->subSeconds($index),
        ],
    );
}

$news = NewsPost::query()->updateOrCreate(
    ['slug' => 'acceptance-long-localized-news'],
    [
        'title' => $englishTitle,
        'body' => $englishBody,
        'published_at' => $publishedAt,
    ],
);
$news->touch();
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

for ($index = 1; $index <= 26; $index++) {
    SiteAnnouncement::query()->updateOrCreate(
        ['title' => sprintf('Content Scale Announcement %03d', $index)],
        [
            'body' => sprintf('Bounded announcement pagination fixture %03d.', $index),
            'severity' => SiteAnnouncement::SEVERITY_INFO,
            'starts_at' => $publishedAt->copy()->subHour(),
            'ends_at' => $publishedAt->copy()->addHour(),
            'publication_state' => SiteAnnouncement::STATE_DRAFT,
            'action_label' => null,
            'action_url' => null,
            'lock_version' => 1,
        ],
    );
}

$announcement = SiteAnnouncement::query()->updateOrCreate(
    ['title' => $englishTitle],
    [
        'body' => $englishBody,
        'severity' => SiteAnnouncement::SEVERITY_WARNING,
        'starts_at' => $publishedAt->copy()->subHour(),
        'ends_at' => $publishedAt->copy()->addHour(),
        'publication_state' => SiteAnnouncement::STATE_PUBLISHED,
        'action_label' => null,
        'action_url' => null,
        'lock_version' => 1,
    ],
);
$announcement->touch();
$announcement->refresh();

EditorialTranslation::query()->updateOrCreate(
    [
        'content_type' => EditorialContentType::SiteAnnouncement->value,
        'content_id' => $announcement->getKey(),
        'locale' => 'pl',
    ],
    [
        'title' => $polishTitle,
        'body' => $polishBody,
        'action_label' => null,
        'source_updated_at' => $announcement->updated_at,
        'published_at' => $publishedAt,
    ],
);

for ($index = 1; $index <= 26; $index++) {
    ClientRelease::query()->updateOrCreate(
        ['version' => sprintf('content-scale-release-%03d', $index)],
        [
            'channel' => 'stable',
            'release_notes' => sprintf('Bounded release pagination fixture %03d.', $index),
            'published_at' => null,
            'is_current' => false,
        ],
    );
}

$release = ClientRelease::query()->updateOrCreate(
    ['version' => 'content-scale-current'],
    [
        'channel' => 'stable',
        'release_notes' => $englishBody,
        'published_at' => $publishedAt,
        'is_current' => true,
    ],
);
$release->touch();
$release->refresh();
$release->artifacts()->updateOrCreate(
    [
        'platform' => 'windows',
        'architecture' => 'x86_64',
    ],
    [
        'artifact_url' => 'https://downloads.example.test/content-scale-current.zip',
        'filename' => $longFilename,
        'size_bytes' => 1572864,
        'sha256' => str_repeat('a', 64),
        'is_enabled' => true,
    ],
);

EditorialTranslation::query()->updateOrCreate(
    [
        'content_type' => EditorialContentType::ClientRelease->value,
        'content_id' => $release->getKey(),
        'locale' => 'pl',
    ],
    [
        'title' => null,
        'body' => $polishBody,
        'action_label' => null,
        'source_updated_at' => $release->updated_at,
        'published_at' => $publishedAt,
    ],
);

fwrite(STDOUT, json_encode([
    'news_id' => $news->id,
    'news_slug' => $news->slug,
    'page_slug' => $page->slug,
    'announcement_id' => $announcement->id,
    'release_id' => $release->id,
    'release_version' => $release->version,
    'artifact_filename' => $longFilename,
    'english_title' => $englishTitle,
    'polish_title' => $polishTitle,
    'english_body' => $englishBody,
    'polish_body' => $polishBody,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE).PHP_EOL);
