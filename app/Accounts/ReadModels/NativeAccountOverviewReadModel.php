<?php

namespace App\Accounts\ReadModels;

use App\Characters\Actions\CreateCharacter;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersAccountView;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharacterSummary;
use App\Identity\Models\Identity;
use App\Identity\Support\CanonicalAccountId;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use UnexpectedValueException;

final class NativeAccountOverviewReadModel
{
    public function __construct(private readonly NativeAccountCharactersReadModel $characters) {}

    /**
     * @return array{
     *     character_creation_allowed: bool,
     *     characters_state: string,
     *     characters: list<NativeAccountCharacterSummary>
     * }
     */
    public function forIdentity(Identity $identity): array
    {
        $accountId = $identity->getAttribute('account_id');
        $eligible = CanonicalAccountId::isValid($accountId)
            && $identity->disabled_at === null && ! $identity->isTerminated();
        $view = new NativeAccountCharactersAccountView(NativeAccountCharactersAccountView::UNAVAILABLE);
        if ($eligible && is_string($accountId)) {
            try {
                $view = DB::transaction(fn (): NativeAccountCharactersAccountView => $this->characters->viewForAccount($accountId, now()->getTimestamp(), lock: true));
            } catch (QueryException|UnexpectedValueException) {
                // A failed projection read is unavailable, never an authoritative empty portfolio.
            }
        }

        return [
            // This permits a command attempt only. Game decides slots, names and world eligibility.
            // Missing/stale list evidence must not prevent a new account's first create request.
            'character_creation_allowed' => $eligible && CreateCharacter::enabled(),
            'characters_state' => $view->state,
            'characters' => $view->state === NativeAccountCharactersAccountView::READY ? $view->characters : [],
        ];
    }
}
