<?php

namespace App\Wiki\Application\Rendering;

interface WikiMarkdownRenderer
{
    public function render(string $sourceMarkdown): RenderedWikiMarkdown;
}
