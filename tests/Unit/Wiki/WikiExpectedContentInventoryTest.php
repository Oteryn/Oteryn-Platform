<?php

namespace Tests\Unit\Wiki;

use App\Wiki\Content\WikiExpectedContentInventory;
use App\Wiki\Content\WikiExpectedContentValidator;
use App\Wiki\Content\WikiLaunchArticle;
use App\Wiki\Content\WikiLaunchCategory;
use App\Wiki\Content\WikiLaunchContentCatalog;
use App\Wiki\Content\WikiLaunchTranslation;
use LogicException;
use Tests\TestCase;

final class WikiExpectedContentInventoryTest extends TestCase
{
    public function test_reviewed_launch_catalog_matches_the_authoritative_expected_inventory(): void
    {
        $validator = new WikiExpectedContentValidator;
        $summary = $validator->validateCatalog(new WikiLaunchContentCatalog);

        self::assertSame(WikiExpectedContentInventory::VERSION, $summary['inventory_version']);
        self::assertSame(WikiLaunchContentCatalog::VERSION, $summary['catalog_version']);
        self::assertSame(4, $summary['categories']);
        self::assertSame(13, $summary['articles']);
        self::assertSame(8, $summary['category_translations']);
        self::assertSame(26, $summary['article_translations']);
        self::assertGreaterThan(0, $summary['source_references']);
        self::assertGreaterThan(0, $summary['internal_links']);
        self::assertSame(0, $summary['editorial_media_tokens']);
        self::assertCount(4, WikiExpectedContentInventory::CATEGORIES);
        self::assertCount(13, WikiExpectedContentInventory::ARTICLES);
        self::assertSame(['en', 'pl'], WikiExpectedContentInventory::LOCALES);
    }

    public function test_missing_expected_article_fails_closed(): void
    {
        $catalog = new WikiLaunchContentCatalog;
        $articles = $catalog->articles();
        array_pop($articles);

        $this->expectException(LogicException::class);
        $this->expectExceptionMessage('article count drifted');

        (new WikiExpectedContentValidator)->validate(
            WikiLaunchContentCatalog::VERSION,
            $catalog->categories(),
            $articles,
        );
    }

    public function test_article_metadata_or_category_drift_fails_closed(): void
    {
        $catalog = new WikiLaunchContentCatalog;
        $articles = $catalog->articles();
        $original = $articles[0];
        $articles[0] = new WikiLaunchArticle(
            $original->contentType,
            ! $original->featured,
            $original->sortOrder,
            ['support'],
            $original->translations,
            $original->sourceReferences,
        );

        $this->expectException(LogicException::class);
        $this->expectExceptionMessage('metadata drifted');

        (new WikiExpectedContentValidator)->validate(
            WikiLaunchContentCatalog::VERSION,
            $catalog->categories(),
            $articles,
        );
    }

    public function test_locale_asymmetry_fails_closed(): void
    {
        $catalog = new WikiLaunchContentCatalog;
        $articles = $catalog->articles();
        $original = $articles[0];
        $articles[0] = new WikiLaunchArticle(
            $original->contentType,
            $original->featured,
            $original->sortOrder,
            $original->categoryKeys,
            [$original->translation('en')],
            $original->sourceReferences,
        );

        $this->expectException(LogicException::class);
        $this->expectExceptionMessage('exactly en/pl translations');

        (new WikiExpectedContentValidator)->validate(
            WikiLaunchContentCatalog::VERSION,
            $catalog->categories(),
            $articles,
        );
    }

    public function test_missing_repository_source_reference_fails_closed(): void
    {
        $catalog = new WikiLaunchContentCatalog;
        $articles = $catalog->articles();
        $original = $articles[0];
        $articles[0] = new WikiLaunchArticle(
            $original->contentType,
            $original->featured,
            $original->sortOrder,
            $original->categoryKeys,
            $original->translations,
            ['docs/does-not-exist/wiki-source.md'],
        );

        $this->expectException(LogicException::class);
        $this->expectExceptionMessage('references missing repository source');

        (new WikiExpectedContentValidator)->validate(
            WikiLaunchContentCatalog::VERSION,
            $catalog->categories(),
            $articles,
        );
    }

    public function test_unexpected_or_external_markdown_link_fails_closed(): void
    {
        $catalog = new WikiLaunchContentCatalog;
        $articles = $catalog->articles();
        $original = $articles[0];
        $english = $original->translation('en');
        $polish = $original->translation('pl');
        $articles[0] = new WikiLaunchArticle(
            $original->contentType,
            $original->featured,
            $original->sortOrder,
            $original->categoryKeys,
            [
                new WikiLaunchTranslation(
                    $english->locale,
                    $english->title,
                    $english->slug,
                    $english->summary,
                    $english->sourceMarkdown."\n\n[Unexpected](https://example.com/wiki)",
                ),
                $polish,
            ],
            $original->sourceReferences,
        );

        $this->expectException(LogicException::class);
        $this->expectExceptionMessage('non-first-party Markdown link');

        (new WikiExpectedContentValidator)->validate(
            WikiLaunchContentCatalog::VERSION,
            $catalog->categories(),
            $articles,
        );
    }

    public function test_duplicate_category_identity_fails_closed_without_changing_count(): void
    {
        $catalog = new WikiLaunchContentCatalog;
        $categories = $catalog->categories();
        $first = $categories[0];
        $categories[1] = new WikiLaunchCategory(
            $first->key,
            $first->sortOrder,
            $first->translations,
        );

        $this->expectException(LogicException::class);
        $this->expectExceptionMessage('category key getting-started is duplicated');

        (new WikiExpectedContentValidator)->validate(
            WikiLaunchContentCatalog::VERSION,
            $categories,
            $catalog->articles(),
        );
    }
}
