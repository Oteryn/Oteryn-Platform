<?php

use App\Http\Controllers\CharacterProfiles\CharacterProfilePreferenceController;
use App\Identity\Localization\SetIdentityLocale;
use Illuminate\Support\Facades\Route;

Route::middleware([SetIdentityLocale::class, 'auth'])
    ->prefix('account/characters/{name}/profile')
    ->group(function (): void {
        Route::get('/', [CharacterProfilePreferenceController::class, 'edit'])
            ->name('account.characters.profile.edit');
        Route::put('/', [CharacterProfilePreferenceController::class, 'update'])
            ->middleware('throttle:identity-security-mutation')
            ->name('account.characters.profile.update');
    });
