<?php

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReport;
use App\GameProjections\Http\EnforcePublicProjectionHttpBounds;
use App\GameProjections\Http\GuardPublicProjectionPeer;
use App\GameProjections\Http\PublicProjectionSnapshotController;
use App\GameProjections\Http\PublicProjectionWatermarkController;
use App\GameProjections\PublicProjectionFamily;
use App\Http\Controllers\GameAuth\CharacterBootstrapIntentController;
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

// Game Public Projections v1 ingest (removal plan packet C4): one compiled path per family plus the
// family watermark, accepted only from the dedicated public-projection mTLS identities. Off unless
// game-projections.public.enabled is true in testing or preproduction.
foreach (PublicProjectionFamily::cases() as $publicProjectionFamily) {
    Route::post('/internal/v1/game-public/'.$publicProjectionFamily->path(), PublicProjectionSnapshotController::class)
        ->defaults('family', $publicProjectionFamily->value)
        ->middleware([
            GuardPublicProjectionPeer::class,
            EnforcePublicProjectionHttpBounds::class.':'.$publicProjectionFamily->maxBytes(),
        ]);
}

Route::post('/internal/v1/game-public/'.PublicProjectionFamily::WATERMARK_PATH, PublicProjectionWatermarkController::class)
    ->middleware([
        GuardPublicProjectionPeer::class,
        EnforcePublicProjectionHttpBounds::class.':'.PublicProjectionFamily::WATERMARK_MAX_BYTES,
    ]);
