<?php

namespace App\Wiki\Infrastructure\Rendering;

use League\CommonMark\Extension\CommonMark\Node\Inline\Link;
use League\CommonMark\Node\Node;
use League\CommonMark\Renderer\ChildNodeRendererInterface;
use League\CommonMark\Renderer\NodeRendererInterface;
use League\CommonMark\Util\HtmlElement;
use LogicException;

final class SafeWikiLinkRenderer implements NodeRendererInterface
{
    public function render(Node $node, ChildNodeRendererInterface $childRenderer): HtmlElement|string
    {
        if (! $node instanceof Link) {
            throw new LogicException('SafeWikiLinkRenderer can only render links.');
        }

        $contents = $childRenderer->renderNodes($node->children());
        $url = $node->getUrl();
        if (! $this->isAllowed($url)) {
            return $contents;
        }

        /** @var array<string, string> $attributes */
        $attributes = ['href' => $url];
        if (str_starts_with($url, 'https://')) {
            $attributes['rel'] = 'noopener noreferrer';
        }

        $title = $node->getTitle();
        if ($title !== null && $title !== '') {
            $attributes['title'] = $title;
        }

        return new HtmlElement('a', $attributes, $contents);
    }

    private function isAllowed(string $url): bool
    {
        if (
            $url === ''
            || preg_match('/[\x00-\x20\\\\]/', $url) === 1
            || str_starts_with($url, '//')
        ) {
            return false;
        }

        if (preg_match('/\A#[a-zA-Z0-9][a-zA-Z0-9._:-]*\z/D', $url) === 1) {
            return true;
        }

        if (str_starts_with($url, '/')) {
            return true;
        }

        $parts = parse_url($url);

        return is_array($parts)
            && ($parts['scheme'] ?? null) === 'https'
            && is_string($parts['host'] ?? null)
            && $parts['host'] !== ''
            && ! isset($parts['user'])
            && ! isset($parts['pass']);
    }
}
