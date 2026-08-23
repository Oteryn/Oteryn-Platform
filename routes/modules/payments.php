<?php

use App\Payments\Http\AdminPaymentReconciliationController;
use App\Payments\Http\PaymentAccountController;
use Illuminate\Support\Facades\Route;

Route::middleware('auth')->prefix('account/payments')->group(function (): void {
    Route::get('/', [PaymentAccountController::class, 'index'])
        ->name('payments.account.index');
    if (! app()->environment('production')) {
        Route::post('/test-checkout', [PaymentAccountController::class, 'storeTestCheckout'])
            ->middleware('throttle:6,1')
            ->name('payments.account.test-checkout.store');
    }
    Route::get('/return/{orderPublicId}', [PaymentAccountController::class, 'returned'])
        ->whereUuid('orderPublicId')
        ->name('payments.account.return');
});

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:payments.reconcile'])
    ->prefix('admin/payments')
    ->group(function (): void {
        Route::get('/reconciliation', [AdminPaymentReconciliationController::class, 'index'])
            ->name('admin.payments.reconciliation.index');
        if (! app()->environment('production')) {
            Route::post('/reconciliation/{reconciliation}/resolve', [AdminPaymentReconciliationController::class, 'resolve'])
                ->middleware('throttle:12,1')
                ->name('admin.payments.reconciliation.resolve');
        }
    });
