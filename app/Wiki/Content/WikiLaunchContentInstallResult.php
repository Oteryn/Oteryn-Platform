<?php

namespace App\Wiki\Content;

final readonly class WikiLaunchContentInstallResult
{
    public function __construct(
        public int $createdCategories,
        public int $createdArticles,
    ) {}

    public function changed(): bool
    {
        return $this->createdCategories > 0 || $this->createdArticles > 0;
    }
}
