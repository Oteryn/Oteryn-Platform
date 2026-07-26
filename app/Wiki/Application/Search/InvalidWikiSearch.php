<?php

namespace App\Wiki\Application\Search;

use InvalidArgumentException;

final class InvalidWikiSearch extends InvalidArgumentException
{
    public function __construct(public readonly WikiSearchError $reason)
    {
        parent::__construct($reason->value);
    }
}
