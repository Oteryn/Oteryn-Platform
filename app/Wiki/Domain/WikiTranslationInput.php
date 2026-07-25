<?php

namespace App\Wiki\Domain;

final readonly class WikiTranslationInput
{
    public function __construct(
        public string $locale,
        public string $title,
        public string $slug,
        public string $summary,
        public string $sourceMarkdown,
    ) {
        WikiContentRules::assertArticleTranslation(
            $this->locale,
            $this->title,
            $this->slug,
            $this->summary,
            $this->sourceMarkdown,
        );
    }
}
