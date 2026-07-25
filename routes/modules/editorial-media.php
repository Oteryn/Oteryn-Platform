<?php

use App\Http\Controllers\Admin\AdminEditorialMediaController;
use Illuminate\Support\Facades\Route;

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:media.manage'])
    ->prefix('admin/media')
    ->name('admin.media.')
    ->group(function (): void {
        Route::get('/', [AdminEditorialMediaController::class, 'index'])->name('index');
        Route::post('/', [AdminEditorialMediaController::class, 'store'])->name('store');
        Route::get('/{editorialMedia}/content', [AdminEditorialMediaController::class, 'content'])
            ->whereNumber('editorialMedia')
            ->name('content');
        Route::get('/{editorialMedia}/thumbnail', [AdminEditorialMediaController::class, 'thumbnail'])
            ->whereNumber('editorialMedia')
            ->name('thumbnail');
        Route::delete('/{editorialMedia}', [AdminEditorialMediaController::class, 'destroy'])
            ->whereNumber('editorialMedia')
            ->name('destroy');
    });
