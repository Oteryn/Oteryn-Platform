<?php

namespace App\Http\Controllers\Accounts;

use App\Accounts\ReadModels\AccountOverviewReadModel;
use App\Accounts\ReadModels\NativeAccountOverviewReadModel;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharacterSummary;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Database\Eloquent\Collection;
use Illuminate\Database\QueryException;
use Illuminate\Http\Request;

final class AccountOverviewController
{
    public function show(Request $request, AccountOverviewReadModel $overview, NativeAccountOverviewReadModel $nativeOverview): View
    {
        $identity = $request->user();

        abort_unless($identity instanceof Identity, 403);

        // Select the explicit native deployment mode before any compatibility reads.
        if (config('game-auth.native_account_characters.enabled') === true || config('character-authority.create.enabled') === true) {
            $native = $nativeOverview->forIdentity($identity);

            return view('identity.account.overview', [
                'identity' => $identity,
                'overview' => $native,
                'profilePreferences' => $this->nativeProfilePreferences($identity, $native['characters']),
            ]);
        }

        return view('identity.account.overview', [
            'identity' => $identity,
            'overview' => $overview->forIdentity($identity),
        ]);
    }

    /**
     * Preferences keyed by the native CharacterId, only for characters the live projection currently
     * lists for this identity. A failed read shows no markers rather than failing the account page.
     *
     * @param  list<NativeAccountCharacterSummary>  $characters
     * @return Collection<string, CharacterProfilePreference>
     */
    private function nativeProfilePreferences(Identity $identity, array $characters): Collection
    {
        $characterIds = array_map(fn (NativeAccountCharacterSummary $character): string => $character->characterId, $characters);
        if ($characterIds === []) {
            return new Collection;
        }

        try {
            return CharacterProfilePreference::query()
                ->where('identity_id', $identity->id)
                ->whereIn('character_id', $characterIds)
                ->get()
                ->keyBy('character_id');
        } catch (QueryException) {
            return new Collection;
        }
    }
}
