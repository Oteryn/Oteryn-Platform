<?php

namespace App\Events\ViewModels;

enum UpcomingEventState: string
{
    case AVAILABLE = 'AVAILABLE';
    case EMPTY = 'EMPTY';
    case UNAVAILABLE = 'UNAVAILABLE';
}
