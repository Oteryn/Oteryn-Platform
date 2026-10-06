<?php

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReport;
use App\Http\Controllers\GameAuth\CharacterBootstrapIntentController;
use App\Http\Controllers\GameAuth\GameLoginContextController;
use App\Http\Controllers\GameAuth\GameLoginTicketRedeemController;
use App\Http\Controllers\GameAuth\NativeAccountCharactersController;
use App\Http\Controllers\GameAuth\NativeAccountCharactersWatermarkController;
use App\Http\Controllers\GameAuth\NativeAdmissionController;
use App\Http\Controllers\GameAuth\NativeEvidenceController;
use App\Http\Controllers\GameAuth\NativeRuntimeStatusController;
use App\Http\Controllers\GameAuth\NativeScopeAssignmentController;
use App\Http\Middleware\GameAuth\EnforceNativeAccountCharactersHttpBounds;
use App\Http\Middleware\GameAuth\EnforceNativeEvidenceHttpBounds;
use App\Http\Middleware\GameAuth\GuardNativeAccountCharactersPeer;
use App\Http\Middleware\GameAuth\GuardNativeRuntimeStatusPeer;
use App\Http\Middleware\GameAuth\PreventSensitiveGameAuthResponseCaching;
use App\Http\Middleware\GameAuth\RequireCharacterBootstrapIntentMtlsPeer;
use App\Http\Middleware\GameAuth\RequireGatewayServiceCredential;
use App\Http\Middleware\GameAuth\RequireNativeEvidenceMtlsPeer;
use App\Http\Middleware\GameAuth\ThrottleNativeEvidencePeer;
use App\ProductsEntitlements\Http\GuardPremiumSnapshotPeer;
use App\ProductsEntitlements\Http\PremiumSnapshotController;
use Illuminate\Support\Facades\Route;

Route::post('/internal/v1/game-auth/tickets/redeem', GameLoginTicketRedeemController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        'throttle:game-auth-ticket-redeem-source',
        RequireGatewayServiceCredential::class,
        'throttle:game-auth-ticket-redeem',
    ]);

Route::get('/internal/v1/game-auth/accounts/{canaryAccountId}/login-context', GameLoginContextController::class)
    ->middleware([RequireGatewayServiceCredential::class, 'throttle:game-auth-ticket-redeem']);

Route::post('/internal/v1/game-auth/native-admissions', NativeAdmissionController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        RequireGatewayServiceCredential::class,
    ]);

Route::post('/internal/v1/game-auth/native-evidence', NativeEvidenceController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        RequireNativeEvidenceMtlsPeer::class,
        ThrottleNativeEvidencePeer::class,
        EnforceNativeEvidenceHttpBounds::class,
    ]);

Route::post('/internal/v1/game-auth/native-runtime-status', NativeRuntimeStatusController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        GuardNativeRuntimeStatusPeer::class.':runtime',
        EnforceNativeEvidenceHttpBounds::class.':'.NativeRuntimeStatusReport::MAX_REQUEST_BYTES,
    ]);

Route::post('/internal/v1/game-auth/native-scope-assignments', NativeScopeAssignmentController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        GuardNativeRuntimeStatusPeer::class.':assignment',
        EnforceNativeEvidenceHttpBounds::class.':'.NativeRuntimeStatusReport::MAX_REQUEST_BYTES,
    ]);

Route::post('/internal/v1/game-auth/native-account-characters', NativeAccountCharactersController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        GuardNativeAccountCharactersPeer::class,
        EnforceNativeAccountCharactersHttpBounds::class.':snapshot',
    ]);

Route::post('/internal/v1/game-auth/native-account-characters/watermark', NativeAccountCharactersWatermarkController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        GuardNativeAccountCharactersPeer::class,
        EnforceNativeAccountCharactersHttpBounds::class.':watermark',
    ]);

Route::post('/internal/v1/game-auth/character-bootstrap-intents/read', CharacterBootstrapIntentController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        RequireCharacterBootstrapIntentMtlsPeer::class,
    ]);

Route::post('/internal/v1/products-entitlements/premium-snapshots/read', PremiumSnapshotController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        GuardPremiumSnapshotPeer::class,
    ]);
