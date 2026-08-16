<?php

namespace App\PublicPortal\Today;

enum TodayCardState: string
{
    case PRESENT = 'present';
    case EMPTY = 'empty';
    case UNAVAILABLE = 'unavailable';
}
