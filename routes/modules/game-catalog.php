<?php

use App\GameCatalog\Http\Admin\AdminGameCatalogController;
use App\GameCatalog\Http\Public\PublicGameCatalogController;
use Illuminate\Support\Facades\Route;

Route::get('/wiki/catalog', [PublicGameCatalogController::class, 'index'])
    ->name('game-catalog.index');
Route::get('/wiki/items', [PublicGameCatalogController::class, 'items'])
    ->name('game-catalog.items.index');
Route::get('/wiki/items/{slug}', [PublicGameCatalogController::class, 'item'])
    ->where('slug', '[a-z0-9]+(?:-[a-z0-9]+)*')
    ->name('game-catalog.items.show');
Route::get('/wiki/creatures', [PublicGameCatalogController::class, 'creatures'])
    ->name('game-catalog.creatures.index');
Route::get('/wiki/creatures/{slug}', [PublicGameCatalogController::class, 'creature'])
    ->where('slug', '[a-z0-9]+(?:-[a-z0-9]+)*')
    ->name('game-catalog.creatures.show');

Route::get('/admin/game-catalog', [AdminGameCatalogController::class, 'index'])
    ->middleware(['auth', 'mfa.confirmed', 'admin.permission:game_catalog.access'])
    ->name('admin.game-catalog.index');
Route::get('/admin/game-catalog/profiles', [AdminGameCatalogController::class, 'profiles'])
    ->middleware(['auth', 'mfa.confirmed', 'admin.permission:game_catalog.access'])
    ->name('admin.game-catalog.profiles.index');
Route::get('/admin/game-catalog/profiles/{profile}', [AdminGameCatalogController::class, 'profile'])
    ->whereNumber('profile')
    ->middleware(['auth', 'mfa.confirmed', 'admin.permission:game_catalog.access'])
    ->name('admin.game-catalog.profiles.show');

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:game_catalog.snapshots.view'])
    ->prefix('admin/game-catalog')
    ->group(function (): void {
        Route::get('/snapshots', [AdminGameCatalogController::class, 'snapshots'])->name('admin.game-catalog.snapshots.index');
        Route::get('/snapshots/{snapshot}', [AdminGameCatalogController::class, 'snapshot'])
            ->whereNumber('snapshot')
            ->name('admin.game-catalog.snapshots.show');
        Route::get('/findings', [AdminGameCatalogController::class, 'findings'])->name('admin.game-catalog.findings.index');
        Route::get('/diff', [AdminGameCatalogController::class, 'diff'])->name('admin.game-catalog.diff.index');
    });
