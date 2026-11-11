<?php

use App\ProductsEntitlements\Http\AdminPremiumTimeController;
use Illuminate\Support\Facades\Route;

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:products.premium.manage'])
    ->prefix('admin/premium')
    ->group(function (): void {
        Route::get('/', [AdminPremiumTimeController::class, 'index'])->name('admin.premium.index');
        Route::post('/grants', [AdminPremiumTimeController::class, 'grant'])->name('admin.premium.grant');
        Route::post('/revocations', [AdminPremiumTimeController::class, 'revoke'])->name('admin.premium.revoke');
    });
