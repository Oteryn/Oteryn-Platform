<?php

namespace App\Wiki\Application\Search;

enum WikiSearchError: string
{
    case TooShort = 'too_short';
    case TooLong = 'too_long';
    case PageOutsideBounds = 'page_outside_bounds';
}
