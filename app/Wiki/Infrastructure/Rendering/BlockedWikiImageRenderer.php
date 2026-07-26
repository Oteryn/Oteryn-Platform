<?php

namespace App\Wiki\Infrastructure\Rendering;

use League\CommonMark\Extension\CommonMark\Node\Inline\Image;
use League\CommonMark\Node\Node;
use League\CommonMark\Renderer\ChildNodeRendererInterface;
use League\CommonMark\Renderer\NodeRendererInterface;
use League\CommonMark\Util\HtmlElement;

final class BlockedWikiImageRenderer implements NodeRendererInterface
{
    public function render(Node $node, ChildNodeRendererInterface $childRenderer): HtmlElement
    {
        Image::assertInstanceOf($node);

        return new HtmlElement(
            'span',
            ['class' => 'wiki-image-placeholder'],
            $childRenderer->renderNodes($node->children()),
        );
    }
}
