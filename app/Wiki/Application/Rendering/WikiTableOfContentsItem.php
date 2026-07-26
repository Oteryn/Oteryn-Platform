<?php

namespace App\Wiki\Application\Rendering;

final readonly class WikiTableOfContentsItem
{
    public function __construct(
        public int $level,
        public string $title,
        public string $id,
    ) {}
}
