<?php

namespace App\CharacterAuthority\Http;

use App\CharacterAuthority\CharacterCommandWire;
use Illuminate\Support\Facades\Route;

/**
 * The three Game-initiated Character Authority command routes, mounted by `routes/internal.php`
 * behind the default-off peer guard. Exposing them on the server's TLS 1.3 mTLS listener for the
 * character-command subjects is part of enablement (Game contract §3, §14 step 4).
 */
final class CharacterCommandRoutes
{
    public static function register(): void
    {
        Route::middleware(GuardCharacterCommandPeer::class)->group(function (): void {
            Route::post(CharacterCommandWire::LIST_PATH, [CharacterCommandController::class, 'pending'])
                ->name('internal.character-authority.commands.pending');
            Route::post(CharacterCommandWire::READ_PATH, [CharacterCommandController::class, 'read'])
                ->name('internal.character-authority.commands.read');
            Route::post(CharacterCommandWire::RECEIPT_PATH, [CharacterCommandController::class, 'receipt'])
                ->name('internal.character-authority.commands.receipt');
        });
    }
}
