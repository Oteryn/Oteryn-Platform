<?php

namespace App\PublicPortal\Today;

enum TodayCardState: string
{
    case PRESENT = 'present';
    case PARTIAL = 'partial';
    case EMPTY = 'empty';
    case UNAVAILABLE = 'unavailable';
}
