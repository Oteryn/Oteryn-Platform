<?php

use App\CharacterAuthority\Http\CharacterCommandRoutes;
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
use App\ProductsEntitlements\Store\Http\GuardStorePeer;
use App\ProductsEntitlements\Store\Http\StoreBalanceController;
use App\ProductsEntitlements\Store\Http\StoreHistoryController;
use App\ProductsEntitlements\Store\Http\StorePendingController;
use App\ProductsEntitlements\Store\Http\StorePurchaseController;
use App\ProductsEntitlements\Store\Http\StoreReadController;
use App\ProductsEntitlements\Store\Http\StoreReceiptController;
use App\ProductsEntitlements\Store\Http\StoreReconcileController;
use App\ProductsEntitlements\Store\Http\ValidateStoreRequest;
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

// Game Character Authority commands v1 (Game contract §4): the command consumer lists PENDING
// intents, reads one and publishes its receipt. Every path answers 503 unless
// character-authority.commands.enabled is true in testing or preproduction; the deployed mTLS
// listener does not expose them until enablement (§14 step 4).
CharacterCommandRoutes::register();

// Store foundation: guarded, default off, with one handler per future implementation lane.
Route::middleware(GuardStorePeer::class.':purchase')->group(function (): void {
    Route::post('/internal/v1/products-entitlements/store/balance/read', StoreBalanceController::class)
        ->middleware(ValidateStoreRequest::class.':ReadStoreBalanceV1,512');
    Route::post('/internal/v1/products-entitlements/store/history/read', StoreHistoryController::class)
        ->middleware(ValidateStoreRequest::class.':ReadStoreHistoryPageV1,512');
    Route::post('/internal/v1/products-entitlements/store/purchases', StorePurchaseController::class)
        ->middleware(ValidateStoreRequest::class.':PurchaseStoreOfferV1,1024');
});
Route::middleware(GuardStorePeer::class.':delivery')->group(function (): void {
    Route::post('/internal/v1/products-entitlements/store/operations/pending', StorePendingController::class)
        ->middleware(ValidateStoreRequest::class.':ListPendingStoreOperationsV1,512');
    Route::post('/internal/v1/products-entitlements/store/operations/read', StoreReadController::class)
        ->middleware(ValidateStoreRequest::class.':ReadStoreOperationIntentV1,512');
    Route::post('/internal/v1/products-entitlements/store/operations/receipt', StoreReceiptController::class)
        ->middleware(ValidateStoreRequest::class.':PublishStoreDeliveryReceiptV1,1024');
    Route::post('/internal/v1/products-entitlements/store/operations/reconcile', StoreReconcileController::class)
        ->middleware(ValidateStoreRequest::class.':ListStoreOperationStatesV1,512');
});
