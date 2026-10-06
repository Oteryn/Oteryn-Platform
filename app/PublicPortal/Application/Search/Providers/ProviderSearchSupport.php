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

        return array_values(array_map(
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
        ));
    }
}
