<?php

namespace Tests\Unit;

use PDO;
use PHPUnit\Framework\TestCase;

class BootstrapTest extends TestCase
{
    public function test_sqlite_driver_is_available_for_tests(): void
    {
        $this->assertContains('sqlite', PDO::getAvailableDrivers());
    }
}
