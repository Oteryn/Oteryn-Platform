<?php

namespace Tests\Unit\Identity;

use App\Identity\Support\CanonicalEmail;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class CanonicalEmailTest extends TestCase
{
    #[Test]
    public function it_trims_and_lowercases_email_addresses(): void
    {
        self::assertSame(
            'person@example.com',
            CanonicalEmail::normalize('  Person@Example.COM  '),
        );
    }
}
