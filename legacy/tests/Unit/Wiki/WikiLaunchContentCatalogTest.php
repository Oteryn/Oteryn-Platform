<?php

namespace Tests\Unit\Wiki;

use App\Wiki\Content\WikiLaunchContentCatalog;
use Tests\TestCase;

final class WikiLaunchContentCatalogTest extends TestCase
{
    public function test_catalog_contains_the_complete_bilingual_source_backed_launch_set(): void
    {
        $catalog = new WikiLaunchContentCatalog;
        $categories = $catalog->categories();
        $articles = $catalog->articles();

        self::assertCount(4, $categories);
        self::assertCount(13, $articles);
        self::assertSame(
            [
                'Download and installation',
                'Creating an account',
                'Creating a character',
                'First login',
                'Server information',
                'Server rates',
                'Vocations',
                'PvP and game rules',
                'Account security and MFA',
                'Frequently asked questions',
                'Known issues',
                'Discord and support',
                'Report a bug',
            ],
            array_map(
                static fn ($article): string => $article->translation('en')->title,
                $articles,
            ),
        );

        $categoryKeys = [];
        $localizedCategorySlugs = [];

        foreach ($categories as $category) {
            self::assertNotContains($category->key, $categoryKeys);
            $categoryKeys[] = $category->key;
            self::assertSame(['en', 'pl'], array_column($category->translations, 'locale'));

            foreach ($category->translations as $translation) {
                $localizedSlug = $translation->locale.'/'.$translation->slug;
                self::assertNotContains($localizedSlug, $localizedCategorySlugs);
                $localizedCategorySlugs[] = $localizedSlug;
            }
        }

        $localizedArticleSlugs = [];

        foreach ($articles as $article) {
            self::assertSame(['en', 'pl'], array_column($article->translations, 'locale'));
            self::assertNotSame([], $article->sourceReferences);

            foreach ($article->categoryKeys as $categoryKey) {
                self::assertContains($categoryKey, $categoryKeys);
            }

            foreach ($article->sourceReferences as $sourceReference) {
                self::assertFileExists(base_path($sourceReference));
            }

            foreach ($article->translations as $translation) {
                $localizedSlug = $translation->locale.'/'.$translation->slug;
                self::assertNotContains($localizedSlug, $localizedArticleSlugs);
                $localizedArticleSlugs[] = $localizedSlug;
                self::assertDoesNotMatchRegularExpression(
                    '/!\[[^\]]*\]\((?:https?:)?\/\//i',
                    $translation->sourceMarkdown,
                );
                self::assertDoesNotMatchRegularExpression(
                    '/<\/?[A-Za-z][^>]*>/',
                    $translation->sourceMarkdown,
                );
            }
        }
    }

    public function test_unknown_gameplay_values_remain_explicit_in_the_reviewed_copy(): void
    {
        $catalog = new WikiLaunchContentCatalog;
        $articles = $catalog->articles();
        $bySlug = [];

        foreach ($articles as $article) {
            $bySlug[$article->translation('en')->slug] = $article;
        }

        self::assertStringContainsString(
            'no approved numeric',
            strtolower($bySlug['server-rates']->translation('en')->sourceMarkdown),
        );
        self::assertStringContainsString(
            'does not assign unapproved',
            strtolower($bySlug['vocations']->translation('en')->sourceMarkdown),
        );
        self::assertStringContainsString(
            'no invented penalties',
            strtolower($bySlug['pvp-and-game-rules']->translation('en')->sourceMarkdown),
        );
        self::assertStringContainsString(
            'does not embed a Discord invitation',
            $bySlug['discord-and-support']->translation('en')->sourceMarkdown,
        );
        self::assertStringContainsString(
            'does not claim that Platform web credentials are already accepted',
            $bySlug['first-login']->translation('en')->sourceMarkdown,
        );
    }
}
