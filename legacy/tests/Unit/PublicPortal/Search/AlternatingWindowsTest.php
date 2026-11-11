<?php

namespace Tests\Unit\PublicPortal\Search;

use App\PublicPortal\Application\Search\Providers\ProviderSearchSupport;
use PHPUnit\Framework\TestCase;

final class AlternatingWindowsTest extends TestCase
{
    public function test_consecutive_pages_cover_both_streams_without_skips_or_repeats(): void
    {
        foreach ([1, 2, 3, 5, 8] as $limit) {
            $first = [];
            $second = [];
            for ($page = 1; $page <= 12; $page++) {
                [$firstWindow, $secondWindow] = ProviderSearchSupport::alternatingWindows($page, $limit);

                $this->assertSame($limit, $firstWindow['count'] + $secondWindow['count']);
                array_push($first, ...$this->range($firstWindow));
                array_push($second, ...$this->range($secondWindow));
            }

            $this->assertSame(range(0, count($first) - 1), $first, "first stream, limit {$limit}");
            $this->assertSame(range(0, count($second) - 1), $second, "second stream, limit {$limit}");
            $this->assertLessThanOrEqual(1, abs(count($first) - count($second)));
        }
    }

    public function test_limit_one_alternates_between_streams_so_both_participate(): void
    {
        $this->assertSame(
            [['offset' => 0, 'count' => 1], ['offset' => 0, 'count' => 0]],
            ProviderSearchSupport::alternatingWindows(1, 1),
        );
        $this->assertSame(
            [['offset' => 1, 'count' => 0], ['offset' => 0, 'count' => 1]],
            ProviderSearchSupport::alternatingWindows(2, 1),
        );
    }

    public function test_limit_five_second_page_continues_where_the_first_stopped(): void
    {
        $this->assertSame(
            [['offset' => 0, 'count' => 3], ['offset' => 0, 'count' => 2]],
            ProviderSearchSupport::alternatingWindows(1, 5),
        );
        $this->assertSame(
            [['offset' => 3, 'count' => 2], ['offset' => 2, 'count' => 3]],
            ProviderSearchSupport::alternatingWindows(2, 5),
        );
    }

    /**
     * @param  array{offset: int, count: int}  $window
     * @return list<int>
     */
    private function range(array $window): array
    {
        return $window['count'] === 0 ? [] : range($window['offset'], $window['offset'] + $window['count'] - 1);
    }
}
