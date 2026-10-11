<?php

namespace App\PublicPortal\Application\Search\Providers;

use App\PublicPortal\Application\Search\SearchHit;
use Illuminate\Support\Str;

final class ProviderSearchSupport
{
    public static function snippet(?string $value, int $limit = 180): ?string
    {
        if ($value === null) {
            return null;
        }

        $plain = trim(preg_replace('/\s+/u', ' ', strip_tags($value)) ?? strip_tags($value));

        return $plain === '' ? null : Str::limit($plain, $limit);
    }

    /**
     * Splits one combined, strictly alternating result stream (first, second, first, ...) into the
     * source window each of two streams must supply for the given page, so that consecutive pages
     * never skip or repeat an item and every stream participates even when the limit is 1.
     *
     * @return array{0: array{offset: int, count: int}, 1: array{offset: int, count: int}}
     */
    public static function alternatingWindows(int $page, int $limit): array
    {
        $start = ($page - 1) * $limit;
        $end = $page * $limit;
        $firstOffset = intdiv($start + 1, 2);
        $firstEnd = intdiv($end + 1, 2);
        $secondOffset = intdiv($start, 2);
        $secondEnd = intdiv($end, 2);

        return [
            ['offset' => $firstOffset, 'count' => $firstEnd - $firstOffset],
            ['offset' => $secondOffset, 'count' => $secondEnd - $secondOffset],
        ];
    }

    /**
     * @param  list<SearchHit>  ...$sets
     * @return list<SearchHit>
     */
    public static function interleave(int $limit, array ...$sets): array
    {
        $hits = [];
        $index = 0;

        while (count($hits) < $limit) {
            $added = false;
            foreach ($sets as $set) {
                if (isset($set[$index])) {
                    $hits[] = $set[$index];
                    $added = true;
                    if (count($hits) === $limit) {
                        break 2;
                    }
                }
            }
            if (! $added) {
                break;
            }
            $index++;
        }

        return array_map(
            static fn (SearchHit $hit, int $rank): SearchHit => new SearchHit(
                sourceModule: $hit->sourceModule,
                sourceType: $hit->sourceType,
                stablePublicId: $hit->stablePublicId,
                sourceRevision: $hit->sourceRevision,
                locale: $hit->locale,
                title: $hit->title,
                snippetPlaintext: $hit->snippetPlaintext,
                canonicalUrl: $hit->canonicalUrl,
                providerRank: $rank + 1,
                publishedOrEffectiveAt: $hit->publishedOrEffectiveAt,
                applicability: $hit->applicability,
                freshness: $hit->freshness,
                badges: $hit->badges,
            ),
            $hits,
            array_keys($hits),
        );
    }
}
