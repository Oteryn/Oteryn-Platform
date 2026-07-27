<?php

namespace App\Accounts\ReadModels;

use App\Accounts\Actions\ProvisionCanaryAccount;
use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use App\PublicGameData\CanaryGameDataRepository;
use Illuminate\Database\QueryException;
use Illuminate\Support\Collection;
use stdClass;

final class AccountOverviewReadModel
{
    public const STATE_READY = 'ready';

    public const STATE_PENDING = 'pending';

    public const STATE_RECOVERABLE = 'recoverable';

    public const STATE_CONFLICT = 'conflict';

    public const STATE_UNAVAILABLE = 'unavailable';

    public const CHARACTERS_AVAILABLE = 'available';

    public const CHARACTERS_EMPTY = 'empty';

    public const CHARACTERS_NOT_READY = 'not_ready';

    public const CHARACTERS_UNAVAILABLE = 'unavailable';

    public const CHARACTER_LIMIT = 10;

    public function __construct(private readonly CanaryGameDataRepository $gameData) {}

    /**
     * @return array{
     *     state: string,
     *     label: string,
     *     message: string,
     *     retry_allowed: bool,
     *     character_creation_allowed: bool,
     *     characters_state: string,
     *     characters_message: string,
     *     characters: Collection<int, stdClass>,
     *     character_count: int,
     *     character_limit: int
     * }
     */
    public function forIdentity(Identity $identity): array
    {
        $binding = IdentityCanaryAccount::query()->whereKey($identity->id)->first();

        if ($binding === null) {
            return $this->withoutCharacters($this->unavailableState(
                'We cannot confirm your game account setup right now. Character creation remains unavailable. Contact support if this persists.',
            ));
        }

        if ($binding->isReady()) {
            $accountId = $binding->canary_account_id;
            if (! is_int($accountId)) {
                return $this->withoutCharacters($this->unavailableState(
                    'We cannot confirm the bound game account identifier. Character creation remains unavailable. Contact support for assistance.',
                ));
            }

            return $this->withCharacters([
                'state' => self::STATE_READY,
                'label' => 'Ready',
                'message' => 'Your game account setup is complete and character creation is available.',
                'retry_allowed' => false,
                'character_creation_allowed' => true,
            ], $accountId);
        }

        if ($binding->isConflict()) {
            return $this->withoutCharacters([
                'state' => self::STATE_CONFLICT,
                'label' => 'Support required',
                'message' => 'We cannot complete your game account setup automatically. Contact support for assistance. No replacement account will be created automatically.',
                'retry_allowed' => false,
                'character_creation_allowed' => false,
            ]);
        }

        if (
            $binding->status === IdentityCanaryAccount::STATUS_PENDING
            && $binding->last_failure_code === ProvisionCanaryAccount::FAILURE_DEPENDENCY_UNAVAILABLE
        ) {
            return $this->withoutCharacters([
                'state' => self::STATE_RECOVERABLE,
                'label' => 'Setup interrupted',
                'message' => 'Game account setup was interrupted by a temporary service problem. You can safely retry the existing setup request.',
                'retry_allowed' => true,
                'character_creation_allowed' => false,
            ]);
        }

        if ($binding->status === IdentityCanaryAccount::STATUS_PENDING) {
            return $this->withoutCharacters([
                'state' => self::STATE_PENDING,
                'label' => 'Setup in progress',
                'message' => 'Your game account setup is still in progress. Character creation will become available after setup completes.',
                'retry_allowed' => false,
                'character_creation_allowed' => false,
            ]);
        }

        return $this->withoutCharacters($this->unavailableState(
            'We cannot confirm a valid game account setup state. Character creation remains unavailable. Contact support for assistance.',
        ));
    }

    /**
     * @param  array{state: string, label: string, message: string, retry_allowed: bool, character_creation_allowed: bool}  $overview
     * @return array{
     *     state: string,
     *     label: string,
     *     message: string,
     *     retry_allowed: bool,
     *     character_creation_allowed: bool,
     *     characters_state: string,
     *     characters_message: string,
     *     characters: Collection<int, stdClass>,
     *     character_count: int,
     *     character_limit: int
     * }
     */
    private function withCharacters(array $overview, int $accountId): array
    {
        try {
            $characters = $this->gameData->activeCharactersForAccount($accountId);
        } catch (QueryException) {
            return [
                ...$overview,
                'characters_state' => self::CHARACTERS_UNAVAILABLE,
                'characters_message' => 'Your game account is ready, but the character list is temporarily unavailable.',
                'characters' => $this->emptyCharacters(),
                'character_count' => 0,
                'character_limit' => self::CHARACTER_LIMIT,
            ];
        }

        $count = $characters->count();
        $atLimit = $count >= self::CHARACTER_LIMIT;
        $usageMessage = $count === 1
            ? sprintf('1 of %d active character slots is in use.', self::CHARACTER_LIMIT)
            : sprintf('%d of %d active character slots are in use.', $count, self::CHARACTER_LIMIT);

        return [
            ...$overview,
            'message' => $atLimit
                ? 'Your game account setup is complete. This account has reached the active character limit.'
                : $overview['message'],
            'character_creation_allowed' => $overview['character_creation_allowed'] && ! $atLimit,
            'characters_state' => $count === 0 ? self::CHARACTERS_EMPTY : self::CHARACTERS_AVAILABLE,
            'characters_message' => $count === 0
                ? 'You do not have any active characters yet.'
                : $usageMessage,
            'characters' => $characters,
            'character_count' => $count,
            'character_limit' => self::CHARACTER_LIMIT,
        ];
    }

    /**
     * @param  array{state: string, label: string, message: string, retry_allowed: bool, character_creation_allowed: bool}  $overview
     * @return array{
     *     state: string,
     *     label: string,
     *     message: string,
     *     retry_allowed: bool,
     *     character_creation_allowed: bool,
     *     characters_state: string,
     *     characters_message: string,
     *     characters: Collection<int, stdClass>,
     *     character_count: int,
     *     character_limit: int
     * }
     */
    private function withoutCharacters(array $overview): array
    {
        return [
            ...$overview,
            'characters_state' => self::CHARACTERS_NOT_READY,
            'characters_message' => 'Your characters will appear after the game account setup is ready.',
            'characters' => $this->emptyCharacters(),
            'character_count' => 0,
            'character_limit' => self::CHARACTER_LIMIT,
        ];
    }

    /** @return Collection<int, stdClass> */
    private function emptyCharacters(): Collection
    {
        return new Collection;
    }

    /**
     * @return array{state: string, label: string, message: string, retry_allowed: bool, character_creation_allowed: bool}
     */
    private function unavailableState(string $message): array
    {
        return [
            'state' => self::STATE_UNAVAILABLE,
            'label' => 'Support required',
            'message' => $message,
            'retry_allowed' => false,
            'character_creation_allowed' => false,
        ];
    }
}
