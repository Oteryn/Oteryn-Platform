<?php

namespace App\Wiki\Content;

use App\Wiki\Domain\WikiCategoryTranslationInput;
use InvalidArgumentException;

final readonly class WikiLaunchCategory
{
    /**
     * @param  non-empty-list<WikiCategoryTranslationInput>  $translations
     */
    public function __construct(
        public string $key,
        public int $sortOrder,
        public array $translations,
    ) {
        if ($sortOrder < 0) {
            throw new InvalidArgumentException('Wiki launch category sort order cannot be negative.');
        }
    }
}
