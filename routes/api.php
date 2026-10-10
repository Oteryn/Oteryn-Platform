<?php

use App\Http\Controllers\GameAuth\DeviceSessions\NativeDeviceSessionController;
use App\Http\Controllers\GameAuth\GameLoginTicketIssueController;
use App\Http\Controllers\GameAuth\NativeAccountCharactersReadController;
use App\Http\Middleware\GameAuth\GuardNativeDeviceSessionHttp;
use App\Http\Middleware\GameAuth\PreventSensitiveGameAuthResponseCaching;
use App\Payments\Http\TestPaymentProviderEventController;
use Illuminate\Support\Facades\Route;

Route::prefix('/v1/game-auth/device-sessions')
    ->middleware([PreventSensitiveGameAuthResponseCaching::class])
    ->group(function (): void {
        Route::post('/', [NativeDeviceSessionController::class, 'enroll'])
            ->middleware(GuardNativeDeviceSessionHttp::class.':enroll');
        Route::middleware(GuardNativeDeviceSessionHttp::class.':device')->group(function (): void {
            Route::post('/native-characters', [NativeDeviceSessionController::class, 'characters']);
            Route::post('/tickets', [NativeDeviceSessionController::class, 'ticket']);
            Route::post('/revoke', [NativeDeviceSessionController::class, 'revoke']);
        });
    });

Route::post('/v1/game-auth/tickets', GameLoginTicketIssueController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        'auth:api',
        'throttle:game-auth-ticket-issue',
    ]);

Route::get('/v1/game-auth/native-characters', NativeAccountCharactersReadController::class)
    ->middleware([
        PreventSensitiveGameAuthResponseCaching::class,
        'auth:api',
        'throttle:game-auth-native-characters',
    ]);

if (! app()->environment('production')) {
    Route::post('/v1/payments/test/events', TestPaymentProviderEventController::class)
        ->middleware('throttle:30,1')
        ->name('payments.test-provider.events');
}
