<?php

namespace App\Http\Controllers\Characters;

use App\CharacterAuthority\CharacterCommandClient;
use App\CharacterAuthority\CharacterCommandUnavailable;
use App\Characters\Actions\CreateCharacter;
use App\Characters\CharacterCreationOutcome;
use App\Characters\Exceptions\CharacterAccountMissing;
use App\Characters\Exceptions\CharacterBindingNotReady;
use App\Characters\Exceptions\CharacterCreationUnavailable;
use App\Characters\Exceptions\CharacterInputInvalid;
use App\Characters\Exceptions\CharacterLimitReached;
use App\Characters\Exceptions\CharacterNameConflict;
use App\Characters\Exceptions\CharacterNameInvalid;
use App\Http\Requests\Characters\CreateCharacterRequest;
use App\Identity\Models\Identity;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Str;

final class CharacterCreationController
{
    public function create(): View
    {
        $enabled = CreateCharacter::enabled();

        return view('characters.create', [
            'commandsEnabled' => $enabled,
            'worlds' => $enabled ? CreateCharacter::worlds() : collect(),
            // One key per rendered form: a double submit reuses the same command operation.
            'submission' => $enabled ? (string) Str::uuid() : null,
        ]);
    }

    public function store(CreateCharacterRequest $request, CreateCharacter $createCharacter): RedirectResponse
    {
        $identity = $request->user();

        abort_unless($identity instanceof Identity, 403);

        try {
            $result = $createCharacter->execute(
                $identity,
                $request->characterName(),
                $request->vocation(),
                $request->sex(),
                $request->world(),
                $request->submission(),
            );
        } catch (CharacterNameInvalid $exception) {
            return back()->withInput()->withErrors(['name' => $exception->getMessage()]);
        } catch (CharacterInputInvalid) {
            return back()->withInput()->withErrors(['character' => 'The character creation input is invalid.']);
        } catch (CharacterBindingNotReady) {
            return back()->withInput()->withErrors(['character' => 'Your game account is not ready for character creation.']);
        } catch (CharacterNameConflict) {
            return back()->withInput()->withErrors(['name' => 'That character name is not available.']);
        } catch (CharacterLimitReached) {
            return back()->withInput()->withErrors(['character' => 'This account already has the maximum number of active characters.']);
        } catch (CharacterAccountMissing) {
            return back()->withInput()->withErrors(['character' => 'Your bound game account is unavailable.']);
        } catch (CharacterCreationUnavailable) {
            return back()->withInput()->withErrors(['character' => 'Character creation is temporarily unavailable.']);
        }

        // The account routes take their locale from the query string, so carry it to the result page.
        return redirect()->route('account.characters.requests.show', [
            'operation' => $result->operationId,
            'locale' => app()->getLocale(),
        ]);
    }

    /** The outcome of one of the signed-in Identity's create requests, from Game's receipt. */
    public function show(Request $request, string $operation, CharacterCommandClient $commands): View
    {
        $identity = $request->user();

        abort_unless($identity instanceof Identity, 403);
        $accountId = $identity->getAttribute('account_id');
        abort_unless(is_string($accountId), 404);

        try {
            $status = $commands->createStatusFor($operation, $accountId);
        } catch (CharacterCommandUnavailable) {
            abort(503);
        }
        abort_if($status === null, 404);

        return view('characters.request', ['outcome' => CharacterCreationOutcome::of($status)]);
    }
}
