<?php

use App\Http\Controllers\Admin\AdminEditorialTranslationController;
use App\Localization\LocalizedPublicRouteRegistrar;
use App\Localization\PublicLocale;
use Illuminate\Support\Facades\Route;
use Illuminate\Support\Facades\URL;

file_put_contents('/tmp/pre-localization-routes.json', json_encode(array_keys(Route::getRoutes()->getRoutesByName()), JSON_PRETTY_PRINT | JSON_THROW_ON_ERROR));

$defaultLocale = app(PublicLocale::class)->default();
URL::defaults(['locale' => $defaultLocale]);
app(LocalizedPublicRouteRegistrar::class)->register();

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:cms.news.manage'])
    ->prefix('admin/news/{newsPost}/translations/pl')
    ->group(function (): void {
        Route::get('/', [AdminEditorialTranslationController::class, 'editNews'])
            ->name('admin.news.translation.edit');
        Route::put('/', [AdminEditorialTranslationController::class, 'updateNews'])
            ->name('admin.news.translation.update');
    });

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:cms.pages.manage'])
    ->prefix('admin/pages/{managedPage}/translations/pl')
    ->group(function (): void {
        Route::get('/', [AdminEditorialTranslationController::class, 'editPage'])
            ->name('admin.pages.translation.edit');
        Route::put('/', [AdminEditorialTranslationController::class, 'updatePage'])
            ->name('admin.pages.translation.update');
    });

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:support.content.manage'])
    ->prefix('admin/support-content/pages/{managedPage}/translations/pl')
    ->group(function (): void {
        Route::get('/', [AdminEditorialTranslationController::class, 'editSupport'])
            ->name('admin.support-content.translation.edit');
        Route::put('/', [AdminEditorialTranslationController::class, 'updateSupport'])
            ->name('admin.support-content.translation.update');
    });

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:portal.announcements.manage'])
    ->prefix('admin/announcements/{siteAnnouncement}/translations/pl')
    ->group(function (): void {
        Route::get('/', [AdminEditorialTranslationController::class, 'editAnnouncement'])
            ->name('admin.announcements.translation.edit');
        Route::put('/', [AdminEditorialTranslationController::class, 'updateAnnouncement'])
            ->name('admin.announcements.translation.update');
    });

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:downloads.manage'])
    ->prefix('admin/downloads/{clientRelease}/translations/pl')
    ->group(function (): void {
        Route::get('/', [AdminEditorialTranslationController::class, 'editRelease'])
            ->name('admin.downloads.translation.edit');
        Route::put('/', [AdminEditorialTranslationController::class, 'updateRelease'])
            ->name('admin.downloads.translation.update');
    });
