<?php

namespace App\Http\Controllers\CharacterProfiles;

use App\CharacterProfiles\CharacterProfileNotOwned;
use App\CharacterProfiles\CharacterProfilePreferenceService;
use App\Http\Requests\CharacterProfiles\UpdateCharacterProfilePreferenceRequest;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Database\QueryException;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;

final class CharacterProfilePreferenceController
{
    public function edit(
        Request $request,
        string $name,
        CharacterProfilePreferenceService $preferences,
    ): View {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $state = $preferences->editState($identity, $name);
        } catch (CharacterProfileNotOwned) {
            abort(404);
        } catch (QueryException) {
            abort(503, __('character_profiles.unavailable'));
        }

        return view('identity.account.character-profile-preferences', [
            'identity' => $identity,
            ...$state,
        ]);
    }

    public function update(
        UpdateCharacterProfilePreferenceRequest $request,
        string $name,
        CharacterProfilePreferenceService $preferences,
    ): RedirectResponse {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $preferences->update($identity, $name, $request->preferences());
        } catch (CharacterProfileNotOwned) {
            abort(404);
        } catch (QueryException) {
            return back()
                ->withInput()
                ->withErrors(['profile' => __('character_profiles.unavailable')]);
        }

        return redirect()
            ->route('account.characters.profile.edit', ['name' => $name])
            ->with('status', __('character_profiles.updated'));
    }
}
