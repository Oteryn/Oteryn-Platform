<?php

use App\Http\Controllers\Admin\AdminHomepageTemplateController;
use Illuminate\Support\Facades\Route;

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:portal.settings.manage'])
    ->prefix('admin/portal/homepage')
    ->group(function (): void {
        Route::get('/', [AdminHomepageTemplateController::class, 'index'])
            ->name('admin.homepage-templates.index');
        Route::get('/preview/{template}', [AdminHomepageTemplateController::class, 'preview'])
            ->where('template', '[A-Za-z0-9_-]+')
            ->name('admin.homepage-templates.preview');
        Route::put('/active', [AdminHomepageTemplateController::class, 'activate'])
            ->name('admin.homepage-templates.activate');
        Route::post('/rollback', [AdminHomepageTemplateController::class, 'rollback'])
            ->name('admin.homepage-templates.rollback');
    });
