<?php

namespace App\Wiki\Infrastructure\Rendering;

use App\Wiki\Application\Rendering\RenderedWikiMarkdown;
use App\Wiki\Application\Rendering\WikiMarkdownRenderer;
use League\CommonMark\Environment\Environment;
use League\CommonMark\Extension\CommonMark\CommonMarkCoreExtension;
use League\CommonMark\Extension\CommonMark\Node\Block\Heading;
use League\CommonMark\Extension\CommonMark\Node\Inline\Image;
use League\CommonMark\Extension\CommonMark\Node\Inline\Link;
use League\CommonMark\Extension\Strikethrough\StrikethroughExtension;
use League\CommonMark\Extension\Table\TableExtension;
use League\CommonMark\MarkdownConverter;

final class CommonMarkWikiRenderer implements WikiMarkdownRenderer
{
    public function render(string $sourceMarkdown): RenderedWikiMarkdown
    {
        $headings = new WikiHeadingRenderer;
        $environment = new Environment([
            'html_input' => 'strip',
            'allow_unsafe_links' => false,
            'max_nesting_level' => 20,
            'max_delimiters_per_line' => 200,
            'renderer' => [
                'soft_break' => "\n",
            ],
            'table' => [
                'wrap' => [
                    'enabled' => true,
                    'tag' => 'div',
                    'attributes' => ['class' => 'wiki-table-scroll'],
                ],
            ],
        ]);
        $environment->addExtension(new CommonMarkCoreExtension);
        $environment->addExtension(new StrikethroughExtension);
        $environment->addExtension(new TableExtension);
        $environment->addRenderer(Heading::class, $headings, 100);
        $environment->addRenderer(Link::class, new SafeWikiLinkRenderer, 100);
        $environment->addRenderer(Image::class, new BlockedWikiImageRenderer, 100);

        $html = (string) (new MarkdownConverter($environment))->convert($sourceMarkdown);

        return new RenderedWikiMarkdown($html, $headings->tableOfContents());
    }
}
