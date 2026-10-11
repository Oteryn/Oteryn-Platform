<?php

namespace App\PublicPortal\Application\Search;

enum FederatedSearchError: string
{
    case Required = 'required';
    case TooShort = 'too_short';
    case TooLong = 'too_long';
    case InvalidFilter = 'invalid_filter';
    case PageOutsideBounds = 'page_outside_bounds';
}
