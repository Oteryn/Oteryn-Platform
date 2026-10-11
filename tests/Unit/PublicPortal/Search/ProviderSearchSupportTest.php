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

    public function test_unavailable_provider_result_has_no_hits(): void
    {
        $result = SearchProviderResult::unavailable('wiki');

        $this->assertSame('wiki', $result->provider);
        $this->assertFalse($result->available);
        $this->assertSame([], $result->hits);
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
