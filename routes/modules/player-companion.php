<?php

use App\Http\Controllers\PlayerCompanion\SessionAnalysisController;
use Illuminate\Support\Facades\Route;

Route::middleware('auth')->prefix('account/tools/session-analyzer')->group(function (): void {
    Route::get('/', [SessionAnalysisController::class, 'index'])
        ->name('player-companion.session-analyses.index');
    Route::post('/', [SessionAnalysisController::class, 'store'])
        ->middleware('throttle:10,1')
        ->name('player-companion.session-analyses.store');
    Route::get('/{analysis}', [SessionAnalysisController::class, 'show'])
        ->whereNumber('analysis')
        ->name('player-companion.session-analyses.show');
    Route::delete('/{analysis}', [SessionAnalysisController::class, 'destroy'])
        ->whereNumber('analysis')
        ->name('player-companion.session-analyses.destroy');
});
