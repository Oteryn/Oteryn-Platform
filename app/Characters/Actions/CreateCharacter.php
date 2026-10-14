<?php

namespace App\Characters\Actions;

use App\CharacterAuthority\CharacterCommandClient;
use App\CharacterAuthority\CharacterCommandConflict;
use App\CharacterAuthority\CharacterCommandSettings;
use App\CharacterAuthority\CharacterCommandUnavailable;
use App\CharacterAuthority\CharacterCommandWire;
use App\Characters\Data\CharacterCreationResult;
use App\Characters\Exceptions\CharacterBindingNotReady;
use App\Characters\Exceptions\CharacterCreationUnavailable;
use App\Characters\Exceptions\CharacterInputInvalid;
use App\Characters\Exceptions\CharacterNameInvalid;
use App\Characters\Policies\CharacterNamePolicy;
use App\GameAuth\Worlds\GameWorld;
use App\Identity\Models\Identity;
use Illuminate\Support\Collection;
use InvalidArgumentException;

/**
 * Records a `CreateCharacter` intent for Game Character Authority (Game contract §14 step 3).
 * Platform never creates the character: Game pulls the intent and its receipt decides the outcome.
 *
 * Off unless `character-authority.create.enabled` is set and the command surface is usable; while
 * off, input is still validated but nothing is written and the form answers unavailable. The
 * AccountId comes from the signed-in Identity, never from the browser, and one form submission maps
 * to one idempotency key, so a double submit reuses the same operation.
 */
final class CreateCharacter
{
    /** @var list<int> */
    private const ALLOWED_VOCATIONS = [1, 2, 3, 4, 9];

    /** @var list<int> */
    private const ALLOWED_SEXES = [0, 1];

    private const SUBMISSION = '/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/D';

    public function __construct(
        private readonly CharacterNamePolicy $names,
        private readonly CharacterCommandClient $commands,
    ) {}

    public static function enabled(): bool
    {
        return config('character-authority.create.enabled') === true && CharacterCommandSettings::current() !== null;
    }

    /**
     * Worlds with a canonical WorldId that the form may offer; Game decides whether one accepts
     * new characters (`CHAR_CMD_WORLD_NOT_ACCEPTED`).
     *
     * @return Collection<int, GameWorld>
     */
    public static function worlds(): Collection
    {
        return GameWorld::query()->whereNotNull('world_id')->orderBy('name')->get(['id', 'world_id', 'name']);
    }

    /**
     * @throws CharacterCreationUnavailable
     */
    public function execute(Identity $identity, string $name, int $vocation, int $sex, ?string $worldId, ?string $submission): CharacterCreationResult
    {
        if (! in_array($vocation, self::ALLOWED_VOCATIONS, true)) {
            throw new CharacterInputInvalid('The selected vocation is not available for character creation.');
        }

        if (! in_array($sex, self::ALLOWED_SEXES, true)) {
            throw new CharacterInputInvalid('The selected sex value is not available for character creation.');
        }

        $canonicalName = $this->names->canonicalize($name);

        if (! self::enabled()) {
            throw new CharacterCreationUnavailable('Character creation is not available.');
        }

        try {
            CharacterCommandWire::assertRequestedName($canonicalName);
        } catch (InvalidArgumentException) {
            throw new CharacterNameInvalid('That character name is not allowed.');
        }

        if ($submission === null || preg_match(self::SUBMISSION, $submission) !== 1
            || $worldId === null || ! GameWorld::query()->where('world_id', $worldId)->exists()) {
            throw new CharacterInputInvalid('The character creation input is invalid.');
        }

        $accountId = $identity->getAttribute('account_id');
        if (! is_string($accountId) || $identity->disabled_at !== null || $identity->isTerminated()) {
            throw new CharacterBindingNotReady('The Identity has no eligible AccountId.');
        }

        try {
            $status = $this->commands->requestCreate(
                'web-create:'.$identity->getKey().':'.$submission,
                $accountId,
                $worldId,
                $canonicalName,
            );
        } catch (CharacterCommandConflict) {
            // The same submission was already recorded with other details.
            throw new CharacterInputInvalid('The character creation input is invalid.');
        } catch (CharacterCommandUnavailable) {
            throw new CharacterCreationUnavailable('Character creation is not available.');
        } catch (InvalidArgumentException) {
            throw new CharacterBindingNotReady('The Identity or world identifiers are not canonical.');
        }

        return new CharacterCreationResult($status->operationId, $canonicalName);
    }
}
