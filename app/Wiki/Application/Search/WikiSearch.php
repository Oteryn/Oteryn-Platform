<?php

namespace App\Wiki\Application\Search;

interface WikiSearch
{
    public function search(string $locale, string $query, int $page, int $perPage = 12): WikiSearchPage;
}
