<?php

use App\Marketplace\Http\AdminMarketplaceController;
use App\Marketplace\Http\MarketplaceAccountController;
use App\Marketplace\Http\MarketplaceBidController;
use App\Marketplace\Http\MarketplaceListingController;
use App\Marketplace\Http\MarketplaceWatchController;
use App\Marketplace\Http\PublicMarketplaceController;
use Illuminate\Support\Facades\Route;

if (! config('marketplace.enabled')) {
    return;
}

Route::get('/bazaar', [PublicMarketplaceController::class, 'index'])->name('marketplace.index');
Route::get('/bazaar/{auction}', [PublicMarketplaceController::class, 'show'])->name('marketplace.show');

Route::middleware('auth')->prefix('account/bazaar')->group(function (): void {
    Route::get('/', MarketplaceAccountController::class)->name('marketplace.account');
    Route::get('/sell', [MarketplaceListingController::class, 'create'])->name('marketplace.listing.create');
    Route::post('/sell', [MarketplaceListingController::class, 'store'])
        ->middleware('throttle:marketplace-listing')
        ->name('marketplace.listing.store');
    Route::post('/{auction}/cancel', [MarketplaceListingController::class, 'cancel'])
        ->middleware('throttle:marketplace-listing')
        ->name('marketplace.listing.cancel');
    Route::post('/{auction}/bids', [MarketplaceBidController::class, 'store'])
        ->middleware('throttle:marketplace-bid')
        ->name('marketplace.bids.store');
    Route::post('/{auction}/purchase', [MarketplaceBidController::class, 'purchase'])
        ->middleware('throttle:marketplace-bid')
        ->name('marketplace.purchase');
    Route::post('/{auction}/watch', [MarketplaceWatchController::class, 'store'])
        ->middleware('throttle:marketplace-watch')
        ->name('marketplace.watch.store');
    Route::delete('/{auction}/watch', [MarketplaceWatchController::class, 'destroy'])
        ->middleware('throttle:marketplace-watch')
        ->name('marketplace.watch.destroy');
});

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:marketplace.manage'])
    ->prefix('admin/marketplace')
    ->group(function (): void {
        Route::get('/', [AdminMarketplaceController::class, 'index'])->name('admin.marketplace.index');
        Route::post('/wallet-adjustments', [AdminMarketplaceController::class, 'adjust'])
            ->name('admin.marketplace.wallet.adjust');
        Route::post('/auctions/{auction}/recover', [AdminMarketplaceController::class, 'recover'])
            ->name('admin.marketplace.auctions.recover');
    });
