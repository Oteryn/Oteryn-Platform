<?php

namespace App\GameAuth\DeviceSessions;

use RuntimeException;

final class DeviceSessionDenied extends RuntimeException
{
    public function __construct()
    {
        parent::__construct('Remembered device authorization is unavailable. Sign in again.');
    }
}
