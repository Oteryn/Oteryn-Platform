<?php

use App\Cms\Editorial\EditorialPageKey;
use App\Http\Controllers\Admin\AdminSupportContentController;
use App\Http\Controllers\Support\EditorialPageController;
use App\Http\Controllers\Support\SupportPageController;
use Illuminate\Support\Facades\Route;

Route::get('/getting-started', EditorialPageController::class)
    ->defaults('editorialPageKey', EditorialPageKey::GettingStarted->value)
    ->name('editorial.getting-started');

Route::get('/server-information', EditorialPageController::class)
    ->defaults('editorialPageKey', EditorialPageKey::ServerInformation->value)
    ->name('editorial.server-information');

Route::get('/support', SupportPageController::class)
    ->defaults('editorialPageKey', EditorialPageKey::Support->value)
    ->name('support.index');

Route::get('/support/report-a-bug', SupportPageController::class)
    ->defaults('editorialPageKey', EditorialPageKey::ReportABug->value)
    ->name('support.report-a-bug');

Route::get('/rules', EditorialPageController::class)
    ->defaults('editorialPageKey', EditorialPageKey::Rules->value)
    ->name('editorial.rules');

Route::get('/legal/terms', EditorialPageController::class)
    ->defaults('editorialPageKey', EditorialPageKey::Terms->value)
    ->name('legal.terms');

Route::get('/legal/privacy', EditorialPageController::class)
    ->defaults('editorialPageKey', EditorialPageKey::Privacy->value)
    ->name('legal.privacy');

Route::get('/legal/cookies', EditorialPageController::class)
    ->defaults('editorialPageKey', EditorialPageKey::Cookies->value)
    ->name('legal.cookies');

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:support.content.manage'])
    ->prefix('admin/support-content')
    ->group(function (): void {
        Route::get('/', [AdminSupportContentController::class, 'index'])
            ->name('admin.support-content.index');
        Route::get('/{editorialPageKey}/edit', [AdminSupportContentController::class, 'edit'])
            ->name('admin.support-content.edit');
        Route::put('/{editorialPageKey}', [AdminSupportContentController::class, 'update'])
            ->name('admin.support-content.update');
    });
