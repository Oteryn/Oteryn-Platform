<?php

use App\Http\Controllers\GameAuth\GameLoginTicketIssueController;
use App\Http\Middleware\GameAuth\PreventSensitiveGameAuthResponseCaching;
use App\Payments\Http\TestPaymentProviderEventController;
use Illuminate\Support\Facades\Route;

Route::post('/v1/game-auth/tickets', GameLoginTicketIssueController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        'auth:api',
        'throttle:game-auth-ticket-issue',
    ]);

if (! app()->environment('production')) {
    Route::post('/v1/payments/test/events', TestPaymentProviderEventController::class)
        ->middleware('throttle:30,1')
        ->name('payments.test-provider.events');
}
