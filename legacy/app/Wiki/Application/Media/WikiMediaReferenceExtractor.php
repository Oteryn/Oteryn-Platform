<?php

namespace App\Wiki\Application\Media;

use League\CommonMark\Environment\Environment;
use League\CommonMark\Extension\CommonMark\CommonMarkCoreExtension;
use League\CommonMark\Extension\CommonMark\Node\Inline\Image;
use League\CommonMark\Parser\MarkdownParser;

final class WikiMediaReferenceExtractor
{
    /**
     * @return list<int>
     */
    public function extractValidated(string $sourceMarkdown): array
    {
        $environment = new Environment([
            'html_input' => 'strip',
            'allow_unsafe_links' => false,
            'max_nesting_level' => 20,
            'max_delimiters_per_line' => 200,
        ]);
        $environment->addExtension(new CommonMarkCoreExtension);
        $document = (new MarkdownParser($environment))->parse($sourceMarkdown);
        $walker = $document->walker();
        $identifiers = [];

        while (($event = $walker->next()) !== null) {
            $node = $event->getNode();
            if (! $event->isEntering() || ! $node instanceof Image) {
                continue;
            }

            $mediaId = WikiMediaSyntax::mediaId($node->getUrl());
            if ($mediaId === null) {
                throw new InvalidWikiMediaSyntax(
                    'Wiki images must use the exact local target wiki-media:<positive-decimal-id>.',
                );
            }

            if (WikiMediaAltText::normalized($node) === null) {
                throw new InvalidWikiMediaSyntax(
                    'Wiki image alternative text must contain 1 to 500 plain-text characters.',
                );
            }

            $identifiers[$mediaId] = $mediaId;
        }

        ksort($identifiers, SORT_NUMERIC);

        return array_values($identifiers);
    }
}
