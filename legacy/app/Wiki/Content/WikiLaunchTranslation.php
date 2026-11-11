<?php

namespace App\Wiki\Content;

use App\Wiki\Domain\WikiTranslationInput;

final readonly class WikiLaunchTranslation
{
    public function __construct(
        public string $locale,
        public string $title,
        public string $slug,
        public string $summary,
        public string $sourceMarkdown,
    ) {
        new WikiTranslationInput($locale, $title, $slug, $summary, $sourceMarkdown);
    }

    public function toInput(): WikiTranslationInput
    {
        return new WikiTranslationInput(
            $this->locale,
            $this->title,
            $this->slug,
            $this->summary,
            $this->sourceMarkdown,
        );
    }
}
