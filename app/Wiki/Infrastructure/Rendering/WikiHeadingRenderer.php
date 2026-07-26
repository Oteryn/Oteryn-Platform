<?php

namespace App\Wiki\Infrastructure\Rendering;

use App\Wiki\Application\Rendering\WikiTableOfContentsItem;
use Illuminate\Support\Str;
use League\CommonMark\Extension\CommonMark\Node\Block\Heading;
use League\CommonMark\Node\Inline\Newline;
use League\CommonMark\Node\Node;
use League\CommonMark\Node\NodeIterator;
use League\CommonMark\Node\StringContainerInterface;
use League\CommonMark\Renderer\ChildNodeRendererInterface;
use League\CommonMark\Renderer\NodeRendererInterface;
use League\CommonMark\Util\HtmlElement;
use LogicException;

final class WikiHeadingRenderer implements NodeRendererInterface
{
    /** @var list<WikiTableOfContentsItem> */
    private array $items = [];

    /** @var array<string, int> */
    private array $slugCounts = [];

    public function render(Node $node, ChildNodeRendererInterface $childRenderer): HtmlElement
    {
        if (! $node instanceof Heading) {
            throw new LogicException('WikiHeadingRenderer can only render headings.');
        }

        $title = $this->plainText($node);
        $baseSlug = Str::slug($title);
        if ($baseSlug === '') {
            $baseSlug = 'section';
        }

        $count = ($this->slugCounts[$baseSlug] ?? 0) + 1;
        $this->slugCounts[$baseSlug] = $count;
        $id = $count === 1 ? $baseSlug : $baseSlug.'-'.$count;
        $level = min(4, max(2, $node->getLevel() + 1));

        $this->items[] = new WikiTableOfContentsItem($level, $title, $id);

        return new HtmlElement(
            'h'.$level,
            ['id' => $id, 'class' => 'wiki-heading'],
            $childRenderer->renderNodes($node->children()),
        );
    }

    /** @return list<WikiTableOfContentsItem> */
    public function tableOfContents(): array
    {
        return $this->items;
    }

    private function plainText(Heading $heading): string
    {
        $text = '';

        foreach (new NodeIterator($heading) as $node) {
            if ($node instanceof StringContainerInterface) {
                $text .= $node->getLiteral();
            } elseif ($node instanceof Newline) {
                $text .= ' ';
            }
        }

        return trim(preg_replace('/\s+/u', ' ', $text) ?? $text);
    }
}
