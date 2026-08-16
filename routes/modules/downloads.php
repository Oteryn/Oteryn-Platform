<?php

use App\Admin\AdminPermission;
use App\Downloads\DownloadCatalog;
use App\Http\Controllers\Downloads\AdminDownloadController;
use App\Http\Controllers\Downloads\PublicDownloadCenterController;
use Illuminate\Support\Facades\Route;

Route::get('/download/{platform?}', PublicDownloadCenterController::class)
    ->whereIn('platform', DownloadCatalog::platforms())
    ->name('downloads.index');

Route::middleware([
    'auth',
    'mfa.confirmed',
    'admin.permission:'.AdminPermission::MANAGE_DOWNLOADS,
])
    ->prefix('admin/downloads')
    ->group(function (): void {
        Route::get('/', [AdminDownloadController::class, 'index'])->name('admin.downloads.index');
        Route::get('/create', [AdminDownloadController::class, 'create'])->name('admin.downloads.create');
        Route::post('/', [AdminDownloadController::class, 'store'])->name('admin.downloads.store');
        Route::get('/updater/{channel}', [AdminDownloadController::class, 'updater'])
            ->whereIn('channel', DownloadCatalog::channels())
            ->name('admin.downloads.updater');
        Route::post('/updater/{channel}/policies', [AdminDownloadController::class, 'approveUpdaterPolicy'])
            ->whereIn('channel', DownloadCatalog::channels())
            ->name('admin.downloads.updater.policies.store');
        Route::post('/updater/{channel}/generations', [AdminDownloadController::class, 'importUpdaterGeneration'])
            ->whereIn('channel', DownloadCatalog::channels())
            ->name('admin.downloads.updater.generations.store');
        Route::post(
            '/updater/{channel}/generations/{clientUpdateGeneration}/activate',
            [AdminDownloadController::class, 'activateUpdaterGeneration'],
        )
            ->whereIn('channel', DownloadCatalog::channels())
            ->name('admin.downloads.updater.generations.activate');
        Route::get('/{clientRelease}/edit', [AdminDownloadController::class, 'edit'])->name('admin.downloads.edit');
        Route::put('/{clientRelease}', [AdminDownloadController::class, 'update'])->name('admin.downloads.update');
        Route::post('/{clientRelease}/publish', [AdminDownloadController::class, 'publish'])->name('admin.downloads.publish');
        Route::post('/{clientRelease}/updater/enable', [AdminDownloadController::class, 'enableUpdater'])
            ->name('admin.downloads.updater.enable');
        Route::post('/{clientRelease}/updater/withdraw', [AdminDownloadController::class, 'withdrawUpdater'])
            ->name('admin.downloads.updater.withdraw');
    });