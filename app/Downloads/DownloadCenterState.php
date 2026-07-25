<?php

namespace App\Downloads;

enum DownloadCenterState: string
{
    case AVAILABLE = 'available';
    case EMPTY = 'empty';
    case UNAVAILABLE = 'unavailable';
}
