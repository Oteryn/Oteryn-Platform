<?php

namespace App\Casts;

final class CastUnavailable extends \RuntimeException
{
    public function __construct()
    {
        parent::__construct('Cast unavailable');
    }
}
