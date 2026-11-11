<?php

namespace App\Wiki\Application\Search;

final readonly class WikiSearchPage
{
    /**
     * @param  list<WikiSearchResult>  $items
     */
    public function __construct(
        public string $query,
        public array $items,
        public int $page,
        public int $perPage,
        public int $total,
    ) {}

    public function lastPage(): int
    {
        return max(1, (int) ceil($this->total / $this->perPage));
    }

    public function hasPreviousPage(): bool
    {
        return $this->page > 1;
    }

    public function hasNextPage(): bool
    {
        return $this->page < $this->lastPage();
    }
}
