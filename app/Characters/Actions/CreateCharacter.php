<?php

namespace App\Characters\Actions;

use App\Characters\Data\CharacterCreationResult;
use App\Characters\Exceptions\CharacterCreationUnavailable;
use App\Characters\Exceptions\CharacterInputInvalid;
use App\Characters\Policies\CharacterNamePolicy;
use App\Identity\Models\Identity;

/**
 * Character creation is switched off until the native game server accepts a
 * create command. Input is still validated, but no gateway is bound and no
 * write or external connection is attempted.
 */
final class CreateCharacter
{
    /** @var list<int> */
    private const ALLOWED_VOCATIONS = [1, 2, 3, 4, 9];

    /** @var list<int> */
    private const ALLOWED_SEXES = [0, 1];

    public function __construct(
        private readonly CharacterNamePolicy $names,
    ) {}

    /**
     * @throws CharacterCreationUnavailable
     */
    public function execute(Identity $identity, string $name, int $vocation, int $sex): CharacterCreationResult
    {
        if (! in_array($vocation, self::ALLOWED_VOCATIONS, true)) {
            throw new CharacterInputInvalid('The selected vocation is not available for character creation.');
        }

        if (! in_array($sex, self::ALLOWED_SEXES, true)) {
            throw new CharacterInputInvalid('The selected sex value is not available for character creation.');
        }

        $this->names->canonicalize($name);

        throw new CharacterCreationUnavailable('Character creation is not available.');
    }
}
