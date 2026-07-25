<?php

use App\Announcements\Http\AdminAnnouncementController;
use Illuminate\Support\Facades\Route;

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:portal.announcements.manage'])
    ->prefix('admin/announcements')
    ->group(function (): void {
        Route::get('/', [AdminAnnouncementController::class, 'index'])->name('admin.announcements.index');
        Route::get('/create', [AdminAnnouncementController::class, 'create'])->name('admin.announcements.create');
        Route::post('/', [AdminAnnouncementController::class, 'store'])->name('admin.announcements.store');
        Route::get('/{siteAnnouncement}/edit', [AdminAnnouncementController::class, 'edit'])->name('admin.announcements.edit');
        Route::put('/{siteAnnouncement}', [AdminAnnouncementController::class, 'update'])->name('admin.announcements.update');
    });
