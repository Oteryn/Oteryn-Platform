<?php

namespace App\Wiki\Application\Media;

use League\CommonMark\Extension\CommonMark\Node\Inline\HtmlInline;
use League\CommonMark\Extension\CommonMark\Node\Inline\Image;
use League\CommonMark\Node\Inline\AbstractInline;
use League\CommonMark\Node\Inline\Newline;
use League\CommonMark\Node\NodeIterator;
use League\CommonMark\Node\StringContainerInterface;

final class WikiMediaAltText
{
    public static function normalized(Image $image): ?string
    {
        $altText = '';

        foreach ((new NodeIterator($image)) as $node) {
            if ($node === $image) {
                continue;
            }

            if ($node instanceof HtmlInline) {
                return null;
            }

            if ($node instanceof StringContainerInterface) {
                $altText .= $node->getLiteral();
            } elseif ($node instanceof Newline) {
                $altText .= ' ';
            } elseif ($node instanceof AbstractInline && $node->firstChild() === null) {
                return null;
            }
        }

        $normalized = preg_replace('/\s+/u', ' ', trim($altText));

        return is_string($normalized) && $normalized !== '' && mb_strlen($normalized) <= 500
            ? $normalized
            : null;
    }
}
