<?php

namespace App\GameAuth\Worlds;

enum GameWorldStatus: string
{
    case Online = 'online';
    case Maintenance = 'maintenance';
    case Offline = 'offline';
    case Unknown = 'unknown';
}
