<?php

use App\Casts\CastPasswordController;
use Illuminate\Support\Facades\Route;

// Web middleware retains CSRF and current account-session fences. No client UI yet.
Route::post('/casts/{cast}/password', CastPasswordController::class)
    ->where('cast', '[0-9a-f]{32}')->middleware(['auth', 'throttle:5,1']);
