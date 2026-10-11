<?php

namespace App\Http\Controllers\Accounts;

use App\Accounts\Actions\ProvisionCanaryAccount;
use App\Accounts\Exceptions\CanaryAccountProvisioningConflict;
use App\Accounts\Exceptions\CanaryAccountProvisioningException;
use App\Accounts\ReadModels\AccountOverviewReadModel;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
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

    public function retry(
        Request $request,
        AccountOverviewReadModel $overview,
        ProvisionCanaryAccount $provisionCanaryAccount,
    ): RedirectResponse {
        $identity = $request->user();

        abort_unless($identity instanceof Identity, 403);

        $current = $overview->forIdentity($identity);

        if (! $current['retry_allowed']) {
            return redirect()
                ->route('account.overview')
                ->withErrors(['provisioning' => 'Provisioning retry is not available for the current account state.']);
        }

        try {
            $provisionCanaryAccount->execute($identity->id);
        } catch (CanaryAccountProvisioningConflict) {
            return redirect()
                ->route('account.overview')
                ->withErrors(['provisioning' => 'Game account setup requires support assistance and cannot be retried automatically.']);
        } catch (CanaryAccountProvisioningException) {
            return redirect()
                ->route('account.overview')
                ->withErrors(['provisioning' => 'Game account setup is temporarily unavailable. Your existing setup request is preserved and can be retried later.']);
        }

        return redirect()
            ->route('account.overview')
            ->with('status', 'Game account setup completed.');
    }
}
