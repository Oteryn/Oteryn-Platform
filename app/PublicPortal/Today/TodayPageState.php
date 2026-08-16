<?php

namespace App\PublicPortal\Today;

enum TodayPageState: string
{
    case COMPLETE = 'complete';
    case PARTIAL = 'partial';
    case UNAVAILABLE = 'unavailable';
}
