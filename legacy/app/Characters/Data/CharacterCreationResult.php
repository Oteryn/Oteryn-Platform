<?php

namespace App\Characters\Data;

/** A recorded `CreateCharacter` intent; its outcome is the receipt Game publishes later. */
final class CharacterCreationResult
{
    public function __construct(
        public readonly string $operationId,
        public readonly string $canonicalName,
    ) {}
}
