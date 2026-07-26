<?php

namespace App\Wiki\Application\Media;

use App\EditorialMedia\Application\EditorialMediaReferenceManager;
use App\EditorialMedia\Domain\EditorialMediaConsumer;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\EditorialMedia\Infrastructure\Models\EditorialMediaReference;
use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiArticleTranslation;
use Illuminate\Support\Collection;
use Illuminate\Validation\ValidationException;
use RuntimeException;

final readonly class WikiMediaReferenceSynchronizer
{
    public function __construct(
        private WikiMediaReferenceExtractor $extractor,
        private EditorialMediaReferenceManager $references,
    ) {}

    public function synchronize(WikiArticle $article): void
    {
        $translations = WikiArticleTranslation::query()
            ->where('article_id', $article->id)
            ->orderBy('locale')
            ->get();
        /** @var array<int, list<int>> $expectedByTranslationId */
        $expectedByTranslationId = [];
        /** @var array<int, string> $localesByMediaId */
        $localesByMediaId = [];

        foreach ($translations as $translation) {
            try {
                $expectedByTranslationId[$translation->id] = $this->extractor->extractValidated(
                    $translation->source_markdown,
                );
            } catch (InvalidWikiMediaSyntax $exception) {
                throw ValidationException::withMessages([
                    "translations.{$translation->locale}.source_markdown" => $exception->getMessage(),
                ]);
            }

            foreach ($expectedByTranslationId[$translation->id] as $mediaId) {
                $localesByMediaId[$mediaId] = $translation->locale;
            }
        }

        $expectedMediaIds = [];
        foreach ($expectedByTranslationId as $mediaIds) {
            foreach ($mediaIds as $mediaId) {
                $expectedMediaIds[$mediaId] = $mediaId;
            }
        }
        ksort($expectedMediaIds, SORT_NUMERIC);

        $media = $this->mediaById(array_values($expectedMediaIds));
        $missing = array_values(array_diff(array_values($expectedMediaIds), $media->keys()->all()));

        if ($missing !== []) {
            sort($missing, SORT_NUMERIC);
            $firstMissing = $missing[0];
            $locale = $localesByMediaId[$firstMissing] ?? 'en';

            throw ValidationException::withMessages([
                "translations.{$locale}.source_markdown" => sprintf(
                    'Wiki media %d does not exist in the approved private editorial library.',
                    $firstMissing,
                ),
            ]);
        }

        foreach ($expectedByTranslationId as $translationId => $mediaIds) {
            $consumerId = WikiMediaSyntax::consumerId($translationId);

            foreach ($mediaIds as $mediaId) {
                $item = $media->get($mediaId);
                if (! $item instanceof EditorialMedia) {
                    throw new RuntimeException('Validated Wiki media could not be resolved.');
                }

                $this->references->attach(
                    $item,
                    EditorialMediaConsumer::WIKI,
                    $consumerId,
                    WikiMediaSyntax::usage($mediaId),
                );
            }

            $existing = EditorialMediaReference::query()
                ->where('consumer', EditorialMediaConsumer::WIKI->value)
                ->where('consumer_id', $consumerId)
                ->orderBy('usage')
                ->get();

            foreach ($existing as $reference) {
                if (
                    preg_match('/\Abody\.([1-9][0-9]{0,18})\z/D', $reference->usage, $matches) !== 1
                    || in_array((int) $matches[1], $mediaIds, true)
                ) {
                    continue;
                }

                $this->references->release(
                    EditorialMediaConsumer::WIKI,
                    $consumerId,
                    $reference->usage,
                );
            }
        }
    }

    /**
     * @param  list<int>  $mediaIds
     * @return Collection<int, EditorialMedia>
     */
    private function mediaById(array $mediaIds): Collection
    {
        if ($mediaIds === []) {
            /** @var Collection<int, EditorialMedia> $empty */
            $empty = new Collection;

            return $empty;
        }

        $items = EditorialMedia::query()
            ->whereIn('id', $mediaIds)
            ->where('disk', 'editorial_media')
            ->orderBy('id')
            ->get();
        /** @var array<int, EditorialMedia> $byId */
        $byId = [];

        foreach ($items as $item) {
            $byId[$item->id] = $item;
        }

        /** @var Collection<int, EditorialMedia> $media */
        $media = new Collection($byId);

        return $media;
    }
}
