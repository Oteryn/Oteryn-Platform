<?php

namespace App\Wiki\Application\Rendering;

use App\Wiki\Application\Media\WikiMediaRenderContext;

interface WikiMarkdownRenderer
{
    public function render(
        string $sourceMarkdown,
        ?WikiMediaRenderContext $media = null,
    ): RenderedWikiMarkdown;
}
