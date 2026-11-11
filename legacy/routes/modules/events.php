<?php

use App\Events\Http\AdminEventController;
use App\Events\Http\PublicEventController;
use Illuminate\Support\Facades\Route;

Route::get('/events', [PublicEventController::class, 'index'])->name('events.index');
Route::get('/events/{slug}', [PublicEventController::class, 'show'])->name('events.show');

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:events.manage'])
    ->prefix('admin/events')
    ->group(function (): void {
        Route::get('/', [AdminEventController::class, 'index'])->name('admin.events.index');
        Route::get('/create', [AdminEventController::class, 'create'])->name('admin.events.create');
        Route::post('/', [AdminEventController::class, 'store'])->name('admin.events.store');
        Route::get('/{event}/edit', [AdminEventController::class, 'edit'])->name('admin.events.edit');
        Route::put('/{event}', [AdminEventController::class, 'update'])->name('admin.events.update');
    });

Route::put('/admin/events/{event}/status', [AdminEventController::class, 'status'])
    ->middleware(['auth', 'mfa.confirmed', 'admin.permission:events.manage', 'admin.permission:events.publish'])
    ->name('admin.events.status');
