<?php

namespace App\Wiki\Application\Media;

use App\EditorialMedia\Domain\EditorialMediaConsumer;
use App\EditorialMedia\Infrastructure\Models\EditorialMediaReference;
use Illuminate\Support\Facades\URL;

final readonly class WikiMediaRenderContextFactory
{
    public function __construct(private WikiMediaReferenceExtractor $extractor) {}

    public function public(
        int $translationId,
        string $locale,
        string $sourceMarkdown,
    ): WikiMediaRenderContext {
        $urls = [];

        foreach ($this->referencedMediaIds($translationId, $sourceMarkdown) as $mediaId) {
            $urls[$mediaId] = route('wiki.media', [
                'locale' => $locale,
                'editorialMedia' => $mediaId,
            ]);
        }

        return new WikiMediaRenderContext($urls);
    }

    public function preview(
        int $articleId,
        int $translationId,
        string $locale,
        string $sourceMarkdown,
    ): WikiMediaRenderContext {
        $urls = [];
        $expiresAt = now()->addMinutes(10);

        foreach ($this->referencedMediaIds($translationId, $sourceMarkdown) as $mediaId) {
            $urls[$mediaId] = URL::temporarySignedRoute(
                'admin.wiki.media.preview',
                $expiresAt,
                [
                    'article' => $articleId,
                    'locale' => $locale,
                    'translation' => $translationId,
                    'editorialMedia' => $mediaId,
                ],
            );
        }

        return new WikiMediaRenderContext($urls);
    }

    /**
     * @return list<int>
     */
    private function referencedMediaIds(int $translationId, string $sourceMarkdown): array
    {
        try {
            $sourceMediaIds = $this->extractor->extractValidated($sourceMarkdown);
        } catch (InvalidWikiMediaSyntax) {
            return [];
        }

        if ($sourceMediaIds === []) {
            return [];
        }

        $usages = array_map(
            static fn (int $mediaId): string => WikiMediaSyntax::usage($mediaId),
            $sourceMediaIds,
        );
        $referencedMediaIds = EditorialMediaReference::query()
            ->where('consumer', EditorialMediaConsumer::WIKI->value)
            ->where('consumer_id', WikiMediaSyntax::consumerId($translationId))
            ->whereIn('usage', $usages)
            ->whereIn('media_id', $sourceMediaIds)
            ->orderBy('media_id')
            ->get()
            ->map(static fn (EditorialMediaReference $reference): int => $reference->media_id)
            ->all();

        return array_values(array_intersect($sourceMediaIds, $referencedMediaIds));
    }
}
