<?php

namespace App\GameAuth\NativeAccountCharacters;

final readonly class NativeAccountCharactersAccountView
{
    public const READY = 'ready';

    public const MISSING = 'missing';

    public const INVALID = 'invalid';

    public const STALE = 'stale';

    public const UNAVAILABLE = 'unavailable';

    /** @param list<NativeAccountCharacterSummary> $characters */
    public function __construct(
        public string $state,
        public array $characters = [],
    ) {}
}
