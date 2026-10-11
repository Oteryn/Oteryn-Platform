<?php

namespace Tests\Unit\PublicPortal\Search;

use App\PublicPortal\Application\Search\Providers\ProviderSearchSupport;
use App\PublicPortal\Application\Search\SearchHit;
use App\PublicPortal\Application\Search\SearchProviderResult;
use PHPUnit\Framework\TestCase;

final class ProviderSearchSupportTest extends TestCase
{
    public function test_snippet_strips_markup_and_collapses_whitespace(): void
    {
        $this->assertSame(
            'Hello brave world',
            ProviderSearchSupport::snippet("<p>Hello   <b>brave</b>\n world</p>"),
        );
    }

    public function test_snippet_is_null_for_null_or_empty_markup(): void
    {
        $this->assertNull(ProviderSearchSupport::snippet(null));
        $this->assertNull(ProviderSearchSupport::snippet('<p>  </p>'));
    }

    public function test_snippet_is_truncated_to_the_limit(): void
    {
        $snippet = ProviderSearchSupport::snippet(str_repeat('a', 500), 20);

        $this->assertNotNull($snippet);
        $this->assertLessThanOrEqual(23, mb_strlen($snippet));
        $this->assertStringEndsWith('...', $snippet);
    }

    public function test_interleave_alternates_sets_and_reranks_from_one(): void
    {
        $hits = ProviderSearchSupport::interleave(
            3,
            [$this->hit('a1', 1), $this->hit('a2', 2)],
            [$this->hit('b1', 1)],
        );

        $this->assertSame(['a1', 'b1', 'a2'], array_map(static fn (SearchHit $hit): string => $hit->stablePublicId, $hits));
        $this->assertSame([1, 2, 3], array_map(static fn (SearchHit $hit): int => $hit->providerRank, $hits));
    }

    public function test_interleave_respects_the_limit(): void
    {
        $hits = ProviderSearchSupport::interleave(
            2,
            [$this->hit('a1', 1), $this->hit('a2', 2)],
            [$this->hit('b1', 1), $this->hit('b2', 2)],
        );

        $this->assertSame(['a1', 'b1'], array_map(static fn (SearchHit $hit): string => $hit->stablePublicId, $hits));
    }

    public function test_interleave_of_nothing_is_empty(): void
    {
        $this->assertSame([], ProviderSearchSupport::interleave(5, [], []));
    }

    public function test_interleave_window_reproduces_the_global_alternating_stream_for_odd_limits(): void
    {
        foreach ([1, 3, 5, 7] as $limit) {
            $global = [];
            for ($page = 1; $page <= 6; $page++) {
                [$firstWindow, $secondWindow] = ProviderSearchSupport::alternatingWindows($page, $limit);
                $first = $this->hits('f', $firstWindow);
                $second = $this->hits('s', $secondWindow);

                foreach (ProviderSearchSupport::interleaveWindow($page, $limit, $first, $second) as $hit) {
                    $global[] = $hit->stablePublicId;
                }
            }

            $expected = [];
            for ($index = 0; count($expected) < 6 * $limit; $index++) {
                $expected[] = 'f'.$index;
                $expected[] = 's'.$index;
            }
            $expected = array_slice($expected, 0, 6 * $limit);
            $this->assertSame($expected, $global, "limit {$limit}");
        }
    }

    public function test_interleave_window_starts_with_the_second_stream_on_an_odd_start(): void
    {
        [$firstWindow, $secondWindow] = ProviderSearchSupport::alternatingWindows(2, 5);
        $hits = ProviderSearchSupport::interleaveWindow(
            2,
            5,
            $this->hits('f', $firstWindow),
            $this->hits('s', $secondWindow),
        );

        $this->assertSame(['s2', 'f3', 's3', 'f4', 's4'], array_map(static fn (SearchHit $hit): string => $hit->stablePublicId, $hits));
        $this->assertSame([1, 2, 3, 4, 5], array_map(static fn (SearchHit $hit): int => $hit->providerRank, $hits));
    }

    public function test_unavailable_provider_result_has_no_hits(): void
    {
        $result = SearchProviderResult::unavailable('wiki');

        $this->assertSame('wiki', $result->provider);
        $this->assertFalse($result->available);
        $this->assertSame([], $result->hits);
    }

    /**
     * @param  array{offset: int, count: int}  $window
     * @return list<SearchHit>
     */
    private function hits(string $prefix, array $window): array
    {
        $hits = [];
        for ($index = 0; $index < $window['count']; $index++) {
            $hits[] = $this->hit($prefix.($window['offset'] + $index), $index + 1);
        }

        return $hits;
    }

    private function hit(string $id, int $rank): SearchHit
    {
        return new SearchHit(
            sourceModule: 'Test',
            sourceType: 'news',
            stablePublicId: $id,
            sourceRevision: null,
            locale: 'en',
            title: $id,
            snippetPlaintext: null,
            canonicalUrl: '/'.$id,
            providerRank: $rank,
        );
    }
}
