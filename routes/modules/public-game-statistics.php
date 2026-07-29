<?php

use App\Http\Controllers\PublicGameData\GuildIndexController;
use App\Http\Controllers\PublicGameData\PublicGameDataController;
use Illuminate\Support\Facades\Route;

Route::get('/guilds', GuildIndexController::class)->name('game.guilds.index');
Route::get('/deaths', [PublicGameDataController::class, 'deaths'])->name('game.deaths.index');
