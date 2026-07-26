<?php

use App\Http\Controllers\PublicPortal\PublicHomeController;
use App\Http\Controllers\PublicPortal\PublicRobotsController;
use App\Http\Controllers\PublicPortal\PublicSitemapController;
use Illuminate\Support\Facades\Route;

Route::get('/', PublicHomeController::class)->name('home');
Route::view('/design/home-v2', 'home-preview')->name('design.home.v2');
Route::get('/sitemap.xml', PublicSitemapController::class)->name('seo.sitemap');
Route::get('/robots.txt', PublicRobotsController::class)->name('seo.robots');
