<?php

namespace App\Wiki\Application\Media;

use App\EditorialMedia\Domain\EditorialMediaConsumer;
use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use App\EditorialMedia\Infrastructure\Models\EditorialMediaReference;
use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Domain\WikiContentRules;
use App\Wiki\Infrastructure\Models\WikiArticleTranslation;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Support\Facades\DB;

final readonly class WikiMediaAccess
{
    public function __construct(private WikiMediaReferenceExtractor $extractor) {}

    public function allowsPreview(
        WikiArticleTranslation $translation,
        EditorialMedia $media,
    ): bool {
        WikiContentRules::assertSupportedLocale($translation->locale);

        try {
            if (! in_array($media->id, $this->extractor->extractValidated($translation->source_markdown), true)) {
                return false;
            }
        } catch (InvalidWikiMediaSyntax) {
            return false;
        }

        return EditorialMediaReference::query()
            ->where('media_id', $media->id)
            ->where('consumer', EditorialMediaConsumer::WIKI->value)
            ->where('consumer_id', WikiMediaSyntax::consumerId($translation->id))
            ->where('usage', WikiMediaSyntax::usage($media->id))
            ->exists();
    }

    public function allowsPublic(EditorialMedia $media, string $locale): bool
    {
        WikiContentRules::assertSupportedLocale($locale);
        $usage = WikiMediaSyntax::usage($media->id);
        $translationIds = EditorialMediaReference::query()
            ->where('media_id', $media->id)
            ->where('consumer', EditorialMediaConsumer::WIKI->value)
            ->where('usage', $usage)
            ->pluck('consumer_id')
            ->map(static function (mixed $value): ?int {
                if (
                    ! is_string($value)
                    || preg_match('/\Atranslation:([1-9][0-9]{0,18})\z/D', $value, $matches) !== 1
                ) {
                    return null;
                }

                $translationId = (int) $matches[1];

                return $translationId > 0 && (string) $translationId === $matches[1]
                    ? $translationId
                    : null;
            })
            ->filter(static fn (?int $value): bool => $value !== null)
            ->map(static fn (?int $value): int => (int) $value)
            ->unique()
            ->values()
            ->all();

        if ($translationIds === []) {
            return false;
        }

        $query = DB::table('wiki_article_translations as wt')
            ->join('wiki_articles as a', 'a.id', '=', 'wt.article_id')
            ->whereIn('wt.id', $translationIds)
            ->where('wt.locale', $locale)
            ->where('a.status', WikiArticleStatus::PUBLISHED->value)
            ->whereNotNull('a.published_at')
            ->where('a.published_at', '<=', now());

        if ($locale === 'pl') {
            $query->join('wiki_article_translations as source_wt', function (JoinClause $join): void {
                $join->on('source_wt.article_id', '=', 'a.id')
                    ->where('source_wt.locale', 'en');
            })->whereColumn('wt.updated_at', '>=', 'source_wt.updated_at');
        }

        foreach ($query->select(['wt.source_markdown'])->get() as $translation) {
            if (is_string($translation->source_markdown)) {
                try {
                    if (
                        in_array(
                            $media->id,
                            $this->extractor->extractValidated($translation->source_markdown),
                            true,
                        )
                    ) {
                        return true;
                    }
                } catch (InvalidWikiMediaSyntax) {
                    continue;
                }
            }
        }

        return false;
    }
}
