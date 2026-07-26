<?php

namespace App\Wiki\Infrastructure\Rendering;

use App\Wiki\Application\Media\WikiMediaAltText;
use App\Wiki\Application\Media\WikiMediaRenderContext;
use App\Wiki\Application\Media\WikiMediaSyntax;
use League\CommonMark\Extension\CommonMark\Node\Inline\Image;
use League\CommonMark\Node\Node;
use League\CommonMark\Renderer\ChildNodeRendererInterface;
use League\CommonMark\Renderer\NodeRendererInterface;
use League\CommonMark\Util\HtmlElement;
use LogicException;

final readonly class WikiMediaImageRenderer implements NodeRendererInterface
{
    public function __construct(private ?WikiMediaRenderContext $context) {}

    public function render(Node $node, ChildNodeRendererInterface $childRenderer): HtmlElement
    {
        if (! $node instanceof Image) {
            throw new LogicException('WikiMediaImageRenderer can only render images.');
        }

        $altText = WikiMediaAltText::normalized($node);
        $mediaId = WikiMediaSyntax::mediaId($node->getUrl());
        $url = $mediaId === null ? null : $this->context?->urlFor($mediaId);

        if ($url === null || $altText === null) {
            return new HtmlElement(
                'span',
                ['class' => 'wiki-image-placeholder'],
                htmlspecialchars($altText ?? '', ENT_NOQUOTES | ENT_SUBSTITUTE | ENT_HTML5, 'UTF-8'),
            );
        }

        return new HtmlElement('img', [
            'class' => 'wiki-editorial-image',
            'src' => $url,
            'alt' => $altText,
            'loading' => 'lazy',
            'decoding' => 'async',
        ], '', true);
    }
}
