<?php

namespace Tests\Unit\GameAuth;

use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentRequestDecoder;
use InvalidArgumentException;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

final class CharacterBootstrapIntentRequestDecoderTest extends TestCase
{
    public function test_version_three_is_accepted(): void
    {
        $payload = [
            'contract_version' => 3,
            'operation_id' => '01890f4e-7c00-7000-8000-000000000001',
        ];
        self::assertSame($payload, (new CharacterBootstrapIntentRequestDecoder)->decode(json_encode($payload, JSON_THROW_ON_ERROR)));
    }

    #[DataProvider('unsupportedVersions')]
    public function test_other_versions_and_encodings_are_refused(mixed $version): void
    {
        $this->expectException(InvalidArgumentException::class);
        (new CharacterBootstrapIntentRequestDecoder)->decode(json_encode([
            'contract_version' => $version,
            'operation_id' => '01890f4e-7c00-7000-8000-000000000001',
        ], JSON_THROW_ON_ERROR | JSON_PRESERVE_ZERO_FRACTION));
    }

    /** @return iterable<string, array{mixed}> */
    public static function unsupportedVersions(): iterable
    {
        yield 'version one' => [1];
        yield 'version two' => [2];
        yield 'future version' => [4];
        yield 'string' => ['3'];
        yield 'float' => [3.0];
        yield 'null' => [null];
        yield 'boolean' => [true];
    }
}
