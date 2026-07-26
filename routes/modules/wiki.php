<?php

use App\Wiki\Http\Public\PublicWikiController;
use Illuminate\Support\Facades\Route;

Route::get('/wiki', [PublicWikiController::class, 'index'])->name('wiki.index');
Route::get('/wiki/search', [PublicWikiController::class, 'search'])
    ->middleware('throttle:wiki-search')
    ->name('wiki.search');
Route::get('/wiki/category/{slug}', [PublicWikiController::class, 'category'])
    ->where('slug', '[a-z0-9]+(?:-[a-z0-9]+)*')
    ->name('wiki.category');
Route::get('/wiki/{slug}', [PublicWikiController::class, 'article'])
    ->where('slug', '[a-z0-9]+(?:-[a-z0-9]+)*')
    ->name('wiki.article');
