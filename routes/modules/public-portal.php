<?php

use App\Http\Controllers\PublicPortal\PublicHomeController;
use App\Http\Controllers\PublicPortal\PublicRobotsController;
use App\Http\Controllers\PublicPortal\PublicSitemapController;
use App\Http\Controllers\PublicPortal\PublicTodayController;
use Illuminate\Support\Facades\Route;

Route::get('/', PublicHomeController::class)->name('home');
Route::get('/today', PublicTodayController::class)->name('today.index');
Route::get('/sitemap.xml', PublicSitemapController::class)->name('seo.sitemap');
Route::get('/robots.txt', PublicRobotsController::class)->name('seo.robots');
