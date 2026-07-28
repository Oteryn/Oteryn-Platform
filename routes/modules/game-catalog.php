<?php

use App\GameCatalog\Http\PublicGameCatalogController;
use Illuminate\Support\Facades\Route;

Route::middleware(['web', 'public.locale'])
    ->defaults('locale', 'en')
    ->group(function (): void {
        Route::get('/wiki/catalog', [PublicGameCatalogController::class, 'index'])->name('legacy.game-catalog.index');
        Route::get('/wiki/items', [PublicGameCatalogController::class, 'items'])->name('legacy.game-catalog.items.index');
        Route::get('/wiki/items/{slug}', [PublicGameCatalogController::class, 'item'])
            ->where('slug', '[a-z0-9][a-z0-9._-]*')
            ->name('legacy.game-catalog.items.show');
        Route::get('/wiki/creatures', [PublicGameCatalogController::class, 'creatures'])->name('legacy.game-catalog.creatures.index');
        Route::get('/wiki/creatures/{slug}', [PublicGameCatalogController::class, 'creature'])
            ->where('slug', '[a-z0-9][a-z0-9._-]*')
            ->name('legacy.game-catalog.creatures.show');
    });

Route::middleware(['web', 'public.locale'])
    ->prefix('{locale}/wiki')
    ->where(['locale' => 'en|pl'])
    ->group(function (): void {
        Route::get('/catalog', [PublicGameCatalogController::class, 'index'])->name('game-catalog.index');
        Route::get('/items', [PublicGameCatalogController::class, 'items'])->name('game-catalog.items.index');
        Route::get('/items/{slug}', [PublicGameCatalogController::class, 'item'])
            ->where('slug', '[a-z0-9][a-z0-9._-]*')
            ->name('game-catalog.items.show');
        Route::get('/creatures', [PublicGameCatalogController::class, 'creatures'])->name('game-catalog.creatures.index');
        Route::get('/creatures/{slug}', [PublicGameCatalogController::class, 'creature'])
            ->where('slug', '[a-z0-9][a-z0-9._-]*')
            ->name('game-catalog.creatures.show');
    });
