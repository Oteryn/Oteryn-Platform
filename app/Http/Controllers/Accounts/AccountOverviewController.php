<?php

namespace App\Http\Controllers\Accounts;

use App\Accounts\ReadModels\AccountOverviewReadModel;
use App\Accounts\ReadModels\NativeAccountOverviewReadModel;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\Request;

final class AccountOverviewController
{
    public function show(Request $request, AccountOverviewReadModel $overview, NativeAccountOverviewReadModel $nativeOverview): View
    {
        $identity = $request->user();

        abort_unless($identity instanceof Identity, 403);

        // Select the explicit native deployment mode before any compatibility reads.
        if (config('game-auth.native_account_characters.enabled') === true || config('character-authority.create.enabled') === true) {
            return view('identity.account.overview', [
                'identity' => $identity,
                'overview' => $nativeOverview->forIdentity($identity),
            ]);
        }

        return view('identity.account.overview', [
            'identity' => $identity,
            'overview' => $overview->forIdentity($identity),
            'profilePreferences' => CharacterProfilePreference::query()
                ->where('identity_id', $identity->id)
                ->get()
                ->keyBy('canary_player_id'),
        ]);
    }
}
