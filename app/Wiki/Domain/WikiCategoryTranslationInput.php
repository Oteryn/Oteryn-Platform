<?php

namespace App\Wiki\Domain;

final readonly class WikiCategoryTranslationInput
{
    public function __construct(
        public string $locale,
        public string $name,
        public string $slug,
        public ?string $description = null,
    ) {
        WikiContentRules::assertCategoryTranslation(
            $this->locale,
            $this->name,
            $this->slug,
            $this->description,
        );
    }
}
