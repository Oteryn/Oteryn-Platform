<?php

namespace Tests\Unit\PublicPortal\Search;

use App\PublicPortal\Application\Search\FederatedSearchError;
use App\PublicPortal\Application\Search\FederatedSearchQuery;
use App\PublicPortal\Application\Search\InvalidFederatedSearch;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

final class FederatedSearchQueryTest extends TestCase
{
    public function test_defaults_cover_every_provider_with_first_page(): void
    {
        $query = FederatedSearchQuery::fromInput('dragon', 'en');

        $this->assertSame('dragon', $query->query);
        $this->assertSame(FederatedSearchQuery::PROVIDERS, $query->providers);
        $this->assertSame([], $query->resultTypes);
        $this->assertSame(1, $query->page);
        $this->assertSame(5, $query->limit);
        $this->assertSame(0, $query->offset());
    }

    public function test_query_whitespace_is_collapsed_and_trimmed(): void
    {
        $query = FederatedSearchQuery::fromInput("  Dragon \t  keep \n", 'pl');

        $this->assertSame('Dragon keep', $query->query);
        $this->assertSame('pl', $query->locale);
    }

    /**
     * @return array<string, array{string, string, FederatedSearchError}>
     */
    public static function invalidQueries(): array
    {
        return [
            'empty' => ['', 'en', FederatedSearchError::Required],
            'blank' => ["  \t ", 'en', FederatedSearchError::Required],
            'single character' => ['x', 'en', FederatedSearchError::TooShort],
            'over eighty characters' => [str_repeat('a', 81), 'en', FederatedSearchError::TooLong],
            'unsupported locale' => ['dragon', 'de', FederatedSearchError::InvalidFilter],
        ];
    }

    #[DataProvider('invalidQueries')]
    public function test_invalid_queries_are_rejected_with_a_reason(string $input, string $locale, FederatedSearchError $expected): void
    {
        try {
            FederatedSearchQuery::fromInput($input, $locale);
            $this->fail('Expected the query to be rejected.');
        } catch (InvalidFederatedSearch $exception) {
            $this->assertSame($expected, $exception->reason);
        }
    }

    public function test_exactly_eighty_characters_is_accepted(): void
    {
        $query = FederatedSearchQuery::fromInput(str_repeat('a', 80), 'en');

        $this->assertSame(80, mb_strlen($query->query));
    }

    public function test_provider_filter_accepts_csv_and_arrays_and_deduplicates(): void
    {
        $csv = FederatedSearchQuery::fromInput('dragon', 'en', 'cms, wiki,cms');
        $array = FederatedSearchQuery::fromInput('dragon', 'en', ['events', 'announcements']);

        $this->assertSame(['cms', 'wiki'], $csv->providers);
        $this->assertSame(['events', 'announcements'], $array->providers);
    }

    public function test_result_type_filter_narrows_providers(): void
    {
        $query = FederatedSearchQuery::fromInput('dragon', 'en', null, 'event');

        $this->assertSame(['events'], $query->providers);
        $this->assertSame(['event'], $query->resultTypes);
        $this->assertTrue($query->allowsType('event'));
        $this->assertFalse($query->allowsType('news'));
    }

    public function test_without_a_type_filter_every_type_is_allowed(): void
    {
        $query = FederatedSearchQuery::fromInput('dragon', 'en');

        $this->assertTrue($query->allowsType('news'));
        $this->assertTrue($query->allowsType('catalog_creature'));
    }

    /**
     * @return array<string, array{mixed, mixed}>
     */
    public static function invalidFilters(): array
    {
        return [
            'unknown provider' => ['bogus', null],
            'unknown type' => [null, 'bogus'],
            'provider and type that cannot intersect' => ['cms', 'event'],
            'non-string provider entry' => [[1], null],
            'empty provider entry' => ['cms,,wiki', null],
            'non-list provider input' => [5, null],
        ];
    }

    #[DataProvider('invalidFilters')]
    public function test_invalid_filters_are_rejected(mixed $providers, mixed $types): void
    {
        try {
            FederatedSearchQuery::fromInput('dragon', 'en', $providers, $types);
            $this->fail('Expected the filter to be rejected.');
        } catch (InvalidFederatedSearch $exception) {
            $this->assertSame(FederatedSearchError::InvalidFilter, $exception->reason);
        }
    }

    /**
     * @return array<string, array{mixed, mixed}>
     */
    public static function outOfBoundsPaging(): array
    {
        return [
            'page zero' => [0, 5],
            'page above maximum' => [101, 5],
            'limit zero' => [1, 0],
            'limit above maximum' => [1, 9],
            'non-numeric page' => ['abc', 5],
            'negative string limit' => [1, '-1'],
        ];
    }

    #[DataProvider('outOfBoundsPaging')]
    public function test_paging_outside_bounds_is_rejected(mixed $page, mixed $limit): void
    {
        try {
            FederatedSearchQuery::fromInput('dragon', 'en', null, null, $page, $limit);
            $this->fail('Expected the paging to be rejected.');
        } catch (InvalidFederatedSearch $exception) {
            $this->assertSame(FederatedSearchError::PageOutsideBounds, $exception->reason);
        }
    }

    public function test_numeric_string_paging_is_accepted_and_offset_is_derived(): void
    {
        $query = FederatedSearchQuery::fromInput('dragon', 'en', null, null, '3', '8');

        $this->assertSame(3, $query->page);
        $this->assertSame(8, $query->limit);
        $this->assertSame(16, $query->offset());
    }
}
