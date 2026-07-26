<?php

namespace App\Wiki\Content;

use InvalidArgumentException;

final readonly class WikiLaunchArticle
{
    /**
     * @param  non-empty-list<string>  $categoryKeys
     * @param  non-empty-list<WikiLaunchTranslation>  $translations
     * @param  non-empty-list<string>  $sourceReferences
     */
    public function __construct(
        public string $contentType,
        public bool $featured,
        public int $sortOrder,
        public array $categoryKeys,
        public array $translations,
        public array $sourceReferences,
    ) {
        if ($sortOrder < 0) {
            throw new InvalidArgumentException('Wiki launch article sort order cannot be negative.');
        }
    }

    public function translation(string $locale): WikiLaunchTranslation
    {
        foreach ($this->translations as $translation) {
            if ($translation->locale === $locale) {
                return $translation;
            }
        }

        throw new InvalidArgumentException("Wiki launch article has no {$locale} translation.");
    }
}
