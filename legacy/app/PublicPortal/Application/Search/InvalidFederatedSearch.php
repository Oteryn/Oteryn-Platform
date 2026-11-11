<?php

namespace App\PublicPortal\Application\Search;

use InvalidArgumentException;

final class InvalidFederatedSearch extends InvalidArgumentException
{
    public function __construct(public readonly FederatedSearchError $reason)
    {
        parent::__construct($reason->value);
    }
}
