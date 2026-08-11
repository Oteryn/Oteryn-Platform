<?php

namespace App\Wiki\Content;

use App\Wiki\Domain\WikiCategoryTranslationInput;
use JsonException;
use League\CommonMark\Environment\Environment;
use League\CommonMark\Extension\CommonMark\CommonMarkCoreExtension;
use League\CommonMark\Extension\CommonMark\Node\Inline\Link;
use League\CommonMark\Extension\Strikethrough\StrikethroughExtension;
use League\CommonMark\Extension\Table\TableExtension;
use League\CommonMark\Parser\MarkdownParser;
use LogicException;

final class WikiExpectedContentValidator
{
    /**
     * @return array{
     *     inventory_version: string,
     *     catalog_version: string,
     *     catalog_source_git_blob_sha: string,
     *     categories: int,
     *     articles: int,
     *     category_translations: int,
     *     article_translations: int,
     *     source_references: int,
     *     internal_links: int,
     *     editorial_media_tokens: int
     * }
     */
    public function validateCatalog(WikiLaunchContentCatalog $catalog): array
    {
        $this->validateMachineReadableInventory();
        $catalogSourceGitBlobSha = $this->validateReviewedCatalogSource();
        $summary = $this->validate(
            WikiLaunchContentCatalog::VERSION,
            $catalog->categories(),
            $catalog->articles(),
        );

        return [
            ...$summary,
            'catalog_source_git_blob_sha' => $catalogSourceGitBlobSha,
        ];
    }

    /**
     * @param  list<WikiLaunchCategory>  $categories
     * @param  list<WikiLaunchArticle>  $articles
     * @return array{
     *     inventory_version: string,
     *     catalog_version: string,
     *     categories: int,
     *     articles: int,
     *     category_translations: int,
     *     article_translations: int,
     *     source_references: int,
     *     internal_links: int,
     *     editorial_media_tokens: int
     * }
     */
    public function validate(string $catalogVersion, array $categories, array $articles): array
    {
        if ($catalogVersion !== WikiExpectedContentInventory::CATALOG_VERSION) {
            throw new LogicException(sprintf(
                'Wiki launch catalog version %s does not match expected inventory catalog version %s.',
                $catalogVersion,
                WikiExpectedContentInventory::CATALOG_VERSION,
            ));
        }

        $categoryTranslations = $this->validateCategories($categories);
        [$articleTranslations, $sourceReferences, $internalLinks, $editorialMediaTokens] =
            $this->validateArticles($articles);

        if ($editorialMediaTokens !== WikiExpectedContentInventory::EXPECTED_EDITORIAL_MEDIA_TOKENS) {
            throw new LogicException(sprintf(
                'Wiki launch content contains %d editorial media token(s); expected %d for inventory %s (%s).',
                $editorialMediaTokens,
                WikiExpectedContentInventory::EXPECTED_EDITORIAL_MEDIA_TOKENS,
                WikiExpectedContentInventory::VERSION,
                WikiExpectedContentInventory::MEDIA_FALLBACK_POLICY,
            ));
        }

        return [
            'inventory_version' => WikiExpectedContentInventory::VERSION,
            'catalog_version' => $catalogVersion,
            'categories' => count($categories),
            'articles' => count($articles),
            'category_translations' => $categoryTranslations,
            'article_translations' => $articleTranslations,
            'source_references' => $sourceReferences,
            'internal_links' => $internalLinks,
            'editorial_media_tokens' => $editorialMediaTokens,
        ];
    }

    private function validateMachineReadableInventory(): void
    {
        $inventoryPath = base_path('docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json');
        $source = file_get_contents($inventoryPath);

        if (! is_string($source)) {
            throw new LogicException('The machine-readable Wiki expected-content inventory is unavailable.');
        }

        try {
            $document = json_decode($source, true, 512, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new LogicException('The machine-readable Wiki expected-content inventory is invalid JSON.', 0, $exception);
        }

        $expectedDocument = [
            'schema_version' => 1,
            'status' => 'complete',
            'inventory_version' => WikiExpectedContentInventory::VERSION,
            'catalog_version' => WikiExpectedContentInventory::CATALOG_VERSION,
            'catalog_source_git_blob_sha' => WikiExpectedContentInventory::CATALOG_SOURCE_GIT_BLOB_SHA,
            'effective_from' => WikiExpectedContentInventory::EFFECTIVE_FROM,
            'locales' => WikiExpectedContentInventory::LOCALES,
            'expected_records' => [
                'categories' => WikiExpectedContentInventory::CATEGORIES,
                'articles' => WikiExpectedContentInventory::ARTICLES,
            ],
            'internal_paths' => WikiExpectedContentInventory::INTERNAL_PATHS,
            'expected_editorial_media_tokens' => WikiExpectedContentInventory::EXPECTED_EDITORIAL_MEDIA_TOKENS,
            'media_fallback_policy' => WikiExpectedContentInventory::MEDIA_FALLBACK_POLICY,
        ];

        if ($document !== $expectedDocument) {
            throw new LogicException('The machine-readable Wiki expected-content inventory drifted from the reviewed runtime inventory.');
        }
    }

    private function validateReviewedCatalogSource(): string
    {
        $catalogPath = base_path('app/Wiki/Content/WikiLaunchContentCatalog.php');
        $source = file_get_contents($catalogPath);

        if (! is_string($source)) {
            throw new LogicException('The reviewed Wiki launch catalog source file is unavailable.');
        }

        $gitBlobSha = sha1('blob '.strlen($source)."\0".$source);

        if (! hash_equals(WikiExpectedContentInventory::CATALOG_SOURCE_GIT_BLOB_SHA, $gitBlobSha)) {
            throw new LogicException(sprintf(
                'Wiki launch catalog source digest drifted: got %s, expected reviewed blob %s.',
                $gitBlobSha,
                WikiExpectedContentInventory::CATALOG_SOURCE_GIT_BLOB_SHA,
            ));
        }

        return $gitBlobSha;
    }

    /**
     * @param  list<WikiLaunchCategory>  $categories
     */
    private function validateCategories(array $categories): int
    {
        if (count($categories) !== count(WikiExpectedContentInventory::CATEGORIES)) {
            throw new LogicException(sprintf(
                'Wiki launch category count drifted: got %d, expected %d.',
                count($categories),
                count(WikiExpectedContentInventory::CATEGORIES),
            ));
        }

        $seenKeys = [];
        $seenSlugs = [];
        $translationCount = 0;

        foreach ($categories as $category) {
            if (isset($seenKeys[$category->key])) {
                throw new LogicException("Wiki launch category key {$category->key} is duplicated.");
            }

            $seenKeys[$category->key] = true;
            $expected = WikiExpectedContentInventory::CATEGORIES[$category->key] ?? null;

            if ($expected === null) {
                throw new LogicException("Unexpected Wiki launch category key {$category->key}.");
            }

            if ($category->sortOrder !== $expected['sort_order']) {
                throw new LogicException("Wiki launch category {$category->key} sort order drifted.");
            }

            $translations = $this->indexCategoryTranslations($category);

            foreach (WikiExpectedContentInventory::LOCALES as $locale) {
                $translation = $translations[$locale] ?? null;

                if (! $translation instanceof WikiCategoryTranslationInput) {
                    throw new LogicException("Wiki launch category {$category->key} is missing locale {$locale}.");
                }

                if ($translation->slug !== $expected['slugs'][$locale]) {
                    throw new LogicException("Wiki launch category {$category->key} {$locale} slug drifted.");
                }

                $slugIdentity = $locale.':'.$translation->slug;

                if (isset($seenSlugs[$slugIdentity])) {
                    throw new LogicException("Wiki launch category slug {$slugIdentity} is duplicated.");
                }

                $seenSlugs[$slugIdentity] = true;
                $translationCount++;
            }
        }

        $missingKeys = array_diff(array_keys(WikiExpectedContentInventory::CATEGORIES), array_keys($seenKeys));

        if ($missingKeys !== []) {
            throw new LogicException('Wiki launch categories are missing expected keys: '.implode(', ', $missingKeys).'.');
        }

        return $translationCount;
    }

    /**
     * @param  list<WikiLaunchArticle>  $articles
     * @return array{int, int, int, int}
     */
    private function validateArticles(array $articles): array
    {
        if (count($articles) !== count(WikiExpectedContentInventory::ARTICLES)) {
            throw new LogicException(sprintf(
                'Wiki launch article count drifted: got %d, expected %d.',
                count($articles),
                count(WikiExpectedContentInventory::ARTICLES),
            ));
        }

        $seenArticleKeys = [];
        $seenSlugs = [];
        $articleTranslations = 0;
        $sourceReferences = 0;
        $internalLinks = 0;
        $editorialMediaTokens = 0;

        foreach ($articles as $article) {
            $translations = $this->indexArticleTranslations($article);
            $english = $translations['en'] ?? null;

            if (! $english instanceof WikiLaunchTranslation) {
                throw new LogicException('Wiki launch article is missing the required en translation.');
            }

            $articleKey = $english->slug;

            if (isset($seenArticleKeys[$articleKey])) {
                throw new LogicException("Wiki launch article key {$articleKey} is duplicated.");
            }

            $seenArticleKeys[$articleKey] = true;
            $expected = WikiExpectedContentInventory::ARTICLES[$articleKey] ?? null;

            if ($expected === null) {
                throw new LogicException("Unexpected Wiki launch article {$articleKey}.");
            }

            if (
                $article->contentType !== $expected['content_type']
                || $article->featured !== $expected['featured']
                || $article->sortOrder !== $expected['sort_order']
                || $article->categoryKeys !== $expected['category_keys']
            ) {
                throw new LogicException("Wiki launch article {$articleKey} metadata drifted from the expected inventory.");
            }

            foreach ($article->categoryKeys as $categoryKey) {
                if (! array_key_exists($categoryKey, WikiExpectedContentInventory::CATEGORIES)) {
                    throw new LogicException("Wiki launch article {$articleKey} references unknown category {$categoryKey}.");
                }
            }

            foreach (WikiExpectedContentInventory::LOCALES as $locale) {
                $translation = $translations[$locale] ?? null;

                if (! $translation instanceof WikiLaunchTranslation) {
                    throw new LogicException("Wiki launch article {$articleKey} is missing locale {$locale}.");
                }

                if ($translation->slug !== $expected['slugs'][$locale]) {
                    throw new LogicException("Wiki launch article {$articleKey} {$locale} slug drifted.");
                }

                $slugIdentity = $locale.':'.$translation->slug;

                if (isset($seenSlugs[$slugIdentity])) {
                    throw new LogicException("Wiki launch article slug {$slugIdentity} is duplicated.");
                }

                $seenSlugs[$slugIdentity] = true;
                $articleTranslations++;
                $internalLinks += $this->validateMarkdownLinks($translation);
                $mediaMatches = preg_match_all('/wiki-media:\d+/u', $translation->sourceMarkdown);

                if ($mediaMatches === false) {
                    throw new LogicException("Wiki launch article {$translation->slug} contains invalid editorial media syntax.");
                }

                $editorialMediaTokens += $mediaMatches;
            }

            $sourceReferences += $this->validateSourceReferences(
                $articleKey,
                $article->sourceReferences,
                $expected['source_references'],
            );
        }

        $missingKeys = array_diff(array_keys(WikiExpectedContentInventory::ARTICLES), array_keys($seenArticleKeys));

        if ($missingKeys !== []) {
            throw new LogicException('Wiki launch articles are missing expected entries: '.implode(', ', $missingKeys).'.');
        }

        return [$articleTranslations, $sourceReferences, $internalLinks, $editorialMediaTokens];
    }

    /**
     * @return array<string, WikiCategoryTranslationInput>
     */
    private function indexCategoryTranslations(WikiLaunchCategory $category): array
    {
        if (count($category->translations) !== count(WikiExpectedContentInventory::LOCALES)) {
            throw new LogicException("Wiki launch category {$category->key} must have exactly en/pl translations.");
        }

        $indexed = [];

        foreach ($category->translations as $translation) {
            if (isset($indexed[$translation->locale])) {
                throw new LogicException("Wiki launch category {$category->key} locale {$translation->locale} is duplicated.");
            }

            if (! in_array($translation->locale, WikiExpectedContentInventory::LOCALES, true)) {
                throw new LogicException("Wiki launch category {$category->key} has unexpected locale {$translation->locale}.");
            }

            $indexed[$translation->locale] = $translation;
        }

        return $indexed;
    }

    /**
     * @return array<string, WikiLaunchTranslation>
     */
    private function indexArticleTranslations(WikiLaunchArticle $article): array
    {
        if (count($article->translations) !== count(WikiExpectedContentInventory::LOCALES)) {
            throw new LogicException('Each Wiki launch article must have exactly en/pl translations.');
        }

        $indexed = [];

        foreach ($article->translations as $translation) {
            if (isset($indexed[$translation->locale])) {
                throw new LogicException("Wiki launch article locale {$translation->locale} is duplicated.");
            }

            if (! in_array($translation->locale, WikiExpectedContentInventory::LOCALES, true)) {
                throw new LogicException("Wiki launch article has unexpected locale {$translation->locale}.");
            }

            $indexed[$translation->locale] = $translation;
        }

        return $indexed;
    }

    private function validateMarkdownLinks(WikiLaunchTranslation $translation): int
    {
        $environment = new Environment([
            'html_input' => 'strip',
            'allow_unsafe_links' => false,
            'max_nesting_level' => 20,
            'max_delimiters_per_line' => 200,
        ]);
        $environment->addExtension(new CommonMarkCoreExtension);
        $environment->addExtension(new StrikethroughExtension);
        $environment->addExtension(new TableExtension);

        $document = (new MarkdownParser($environment))->parse($translation->sourceMarkdown);
        $targets = [];

        foreach ($document->iterator() as $node) {
            if ($node instanceof Link) {
                $targets[] = $node->getUrl();
            }
        }

        foreach ($targets as $target) {
            if (! str_starts_with($target, '/')) {
                throw new LogicException("Wiki launch article {$translation->slug} contains a non-first-party Markdown link.");
            }

            if (in_array($target, WikiExpectedContentInventory::INTERNAL_PATHS, true)) {
                continue;
            }

            if ($this->isExpectedLocalizedWikiPath($target)) {
                continue;
            }

            throw new LogicException("Wiki launch article {$translation->slug} contains unexpected internal link {$target}.");
        }

        return count($targets);
    }

    private function isExpectedLocalizedWikiPath(string $target): bool
    {
        if (preg_match('#^/(en|pl)/wiki/([a-z0-9-]+)$#u', $target, $matches) !== 1) {
            return false;
        }

        $locale = $matches[1];
        $slug = $matches[2];

        foreach (WikiExpectedContentInventory::ARTICLES as $article) {
            if ($article['slugs'][$locale] === $slug) {
                return true;
            }
        }

        return false;
    }

    /**
     * @param  list<string>  $references
     * @param  list<string>  $expectedReferences
     */
    private function validateSourceReferences(
        string $articleKey,
        array $references,
        array $expectedReferences,
    ): int {
        if ($references !== $expectedReferences) {
            throw new LogicException("Wiki launch article {$articleKey} provenance paths drifted from the expected inventory.");
        }

        if ($references === []) {
            throw new LogicException("Wiki launch article {$articleKey} has no repository source references.");
        }

        $seen = [];

        foreach ($references as $reference) {
            if (
                trim($reference) !== $reference
                || $reference === ''
                || str_starts_with($reference, '/')
                || str_contains($reference, '\\')
                || in_array('..', explode('/', $reference), true)
            ) {
                throw new LogicException("Wiki launch article {$articleKey} has invalid repository source reference {$reference}.");
            }

            if (isset($seen[$reference])) {
                throw new LogicException("Wiki launch article {$articleKey} duplicates repository source reference {$reference}.");
            }

            if (! is_file(base_path($reference))) {
                throw new LogicException("Wiki launch article {$articleKey} references missing repository source {$reference}.");
            }

            $seen[$reference] = true;
        }

        return count($references);
    }
}
