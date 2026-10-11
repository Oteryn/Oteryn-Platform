<?php

namespace App\PublicPortal\Application\Search;

use Normalizer;

final readonly class FederatedSearchQuery
{
    /** @var list<string> */
    public const PROVIDERS = ['cms', 'wiki', 'game_catalog', 'events', 'announcements'];

    /** @var list<string> */
    public const RESULT_TYPES = [
        'news',
        'page',
        'wiki_article',
        'catalog_item',
        'catalog_creature',
        'event',
        'announcement',
    ];

    /** @var array<string, list<string>> */
    private const TYPES_BY_PROVIDER = [
        'cms' => ['news', 'page'],
        'wiki' => ['wiki_article'],
        'game_catalog' => ['catalog_item', 'catalog_creature'],
        'events' => ['event'],
        'announcements' => ['announcement'],
    ];

    /**
     * @param  list<string>  $providers
     * @param  list<string>  $resultTypes
     */
    private function __construct(
        public string $query,
        public string $locale,
        public array $providers,
        public array $resultTypes,
        public int $page,
        public int $limit,
    ) {}

    public static function fromInput(
        string $query,
        string $locale,
        mixed $providers = null,
        mixed $resultTypes = null,
        mixed $page = 1,
        mixed $limit = 5,
    ): self {
        $normalized = Normalizer::normalize($query, Normalizer::FORM_C);
        $normalized = is_string($normalized) ? $normalized : $query;
        $normalized = trim(preg_replace('/\s+/u', ' ', $normalized) ?? $normalized);

        if ($normalized === '') {
            throw new InvalidFederatedSearch(FederatedSearchError::Required);
        }
        if (mb_strlen($normalized) < 2) {
            throw new InvalidFederatedSearch(FederatedSearchError::TooShort);
        }
        if (mb_strlen($normalized) > 80) {
            throw new InvalidFederatedSearch(FederatedSearchError::TooLong);
        }
        if (! in_array($locale, ['en', 'pl'], true)) {
            throw new InvalidFederatedSearch(FederatedSearchError::InvalidFilter);
        }

        $providerFilter = self::filter($providers, self::PROVIDERS);
        $typeFilter = self::filter($resultTypes, self::RESULT_TYPES);
        $pageNumber = self::boundedInteger($page, 1, 100);
        $limitNumber = self::boundedInteger($limit, 1, 8);

        $effectiveProviders = $providerFilter === [] ? self::PROVIDERS : $providerFilter;
        if ($typeFilter !== []) {
            $effectiveProviders = array_values(array_filter(
                $effectiveProviders,
                static fn (string $provider): bool => array_intersect(self::TYPES_BY_PROVIDER[$provider], $typeFilter) !== [],
            ));
        }

        if ($effectiveProviders === []) {
            throw new InvalidFederatedSearch(FederatedSearchError::InvalidFilter);
        }

        return new self(
            query: $normalized,
            locale: $locale,
            providers: $effectiveProviders,
            resultTypes: $typeFilter,
            page: $pageNumber,
            limit: $limitNumber,
        );
    }

    public function allowsType(string $type): bool
    {
        return $this->resultTypes === [] || in_array($type, $this->resultTypes, true);
    }

    public function offset(): int
    {
        return ($this->page - 1) * $this->limit;
    }

    /**
     * @param  list<string>  $allowlist
     * @return list<string>
     */
    private static function filter(mixed $input, array $allowlist): array
    {
        if ($input === null || $input === '' || $input === []) {
            return [];
        }

        $values = is_string($input) ? explode(',', $input) : $input;
        if (! is_array($values)) {
            throw new InvalidFederatedSearch(FederatedSearchError::InvalidFilter);
        }

        $normalized = [];
        foreach ($values as $value) {
            if (! is_string($value)) {
                throw new InvalidFederatedSearch(FederatedSearchError::InvalidFilter);
            }
            $value = trim($value);
            if ($value === '' || ! in_array($value, $allowlist, true)) {
                throw new InvalidFederatedSearch(FederatedSearchError::InvalidFilter);
            }
            $normalized[$value] = true;
        }

        return array_keys($normalized);
    }

    private static function boundedInteger(mixed $value, int $minimum, int $maximum): int
    {
        if (is_string($value) && ctype_digit($value)) {
            $value = (int) $value;
        }
        if (! is_int($value) || $value < $minimum || $value > $maximum) {
            throw new InvalidFederatedSearch(FederatedSearchError::PageOutsideBounds);
        }

        return $value;
    }
}
