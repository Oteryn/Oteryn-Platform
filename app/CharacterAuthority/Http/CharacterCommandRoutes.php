<?php

namespace App\CharacterAuthority\Http;

use App\CharacterAuthority\CharacterCommandWire;
use Illuminate\Support\Facades\Route;

/**
 * The three Game-initiated Character Authority command routes. Not mounted by this packet: the
 * enablement packet adds `CharacterCommandRoutes::register()` to the internal route stack, with
 * the server's TLS 1.3 mTLS listener for the character-command subjects (Game contract §3, §14).
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
