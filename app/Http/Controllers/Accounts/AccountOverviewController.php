<?php

namespace App\Http\Controllers\Accounts;

use App\Accounts\ReadModels\AccountOverviewReadModel;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\Request;

final class AccountOverviewController
{
    public function show(Request $request, AccountOverviewReadModel $overview): View
    {
        $identity = $request->user();

        abort_unless($identity instanceof Identity, 403);

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
