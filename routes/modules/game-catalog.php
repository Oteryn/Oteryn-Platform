<?php

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
