<?php

use App\Http\Controllers\PublicGameData\GuildIndexController;
use Illuminate\Support\Facades\Route;

Route::get('/guilds', GuildIndexController::class)->name('game.guilds.index');
