<?php

namespace App\Wiki\Application\Rendering;

final readonly class RenderedWikiMarkdown
{
    /**
     * @param  list<WikiTableOfContentsItem>  $tableOfContents
     */
    public function __construct(
        public string $html,
        public array $tableOfContents,
    ) {}
}
