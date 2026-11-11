<?php

namespace Tests\Unit\Characters;

use App\Characters\Exceptions\CharacterNameInvalid;
use App\Characters\Policies\CharacterNamePolicy;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

final class CharacterNamePolicyTest extends TestCase
{
    public function test_it_canonicalizes_valid_ascii_names(): void
    {
        $policy = new CharacterNamePolicy;

        self::assertSame('Alice', $policy->canonicalize('alice'));
        self::assertSame('Alice Moon', $policy->canonicalize('  aLiCe   moon  '));
        self::assertSame('Red Blue Green', $policy->canonicalize('red blue green'));
    }

    #[DataProvider('invalidNames')]
    public function test_it_rejects_invalid_or_reserved_names(string $name): void
    {
        $this->expectException(CharacterNameInvalid::class);

        (new CharacterNamePolicy)->canonicalize($name);
    }

    /**
     * @return iterable<string, array{0: string}>
     */
    public static function invalidNames(): iterable
    {
        yield 'tab' => ["Alice\tMoon"];
        yield 'apostrophe' => ["O'Brien"];
        yield 'hyphen' => ['Alice-Moon'];
        yield 'digit' => ['Alice2'];
        yield 'non-ascii' => ['Łukasz'];
        yield 'single-letter-word' => ['A Moon'];
        yield 'four words' => ['One Two Three Four'];
        yield 'reserved role' => ['Alice Admin'];
        yield 'reserved phrase' => ['Game Master'];
        yield 'oteryn prefix' => ['Oterynsupport Hero'];
        yield 'canary prefix' => ['Canarybird Hero'];
    }
}
