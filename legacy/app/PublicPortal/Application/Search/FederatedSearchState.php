<?php

namespace App\PublicPortal\Application\Search;

enum FederatedSearchState: string
{
    case Complete = 'complete';
    case Partial = 'partial';
    case Unavailable = 'unavailable';
}
