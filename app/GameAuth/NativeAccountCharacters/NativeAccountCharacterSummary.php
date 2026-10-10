<?php

namespace App\GameAuth\NativeAccountCharacters;

final readonly class NativeAccountCharacterSummary
{
    public function __construct(
        public string $characterId,
        public string $worldId,
        public string $name,
        public string $availability,
    ) {}
}
