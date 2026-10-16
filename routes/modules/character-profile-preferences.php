<?php

use App\Http\Controllers\CharacterProfiles\CharacterProfilePreferenceController;
use App\Identity\Localization\SetIdentityLocale;
use Illuminate\Support\Facades\Route;

// Projection names may contain '/', which route() emits unencoded, so {name} spans path segments.
Route::middleware([SetIdentityLocale::class, 'auth'])
    ->prefix('account/characters/{name}/profile')
    ->where(['name' => '.+'])
    ->group(function (): void {
        Route::get('/', [CharacterProfilePreferenceController::class, 'edit'])
            ->name('account.characters.profile.edit');
        Route::put('/', [CharacterProfilePreferenceController::class, 'update'])
            ->middleware('throttle:identity-security-mutation')
            ->name('account.characters.profile.update');
    });
