<?php

namespace App\PublicPortal;

enum PublicContentState: string
{
    case AVAILABLE = 'AVAILABLE';
    case EMPTY = 'EMPTY';
    case STALE = 'STALE';
    case UNAVAILABLE = 'UNAVAILABLE';
}
