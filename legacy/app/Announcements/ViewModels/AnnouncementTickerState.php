<?php

namespace App\Announcements\ViewModels;

enum AnnouncementTickerState: string
{
    case AVAILABLE = 'AVAILABLE';
    case EMPTY = 'EMPTY';
    case UNAVAILABLE = 'UNAVAILABLE';
}
