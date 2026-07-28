<?php

use App\Cms\Editorial\EditorialPageKey;
use App\Http\Controllers\Admin\AdminSupportContentController;
use App\Http\Controllers\Support\EditorialPageController;
use App\Http\Controllers\Support\SupportPageController;
use App\Identity\Localization\SetIdentityLocale;
use App\Support\Http\AdminEnforcementController;
use App\Support\Http\AdminPlayerReportController;
use App\Support\Http\AdminSupportTicketController;
use App\Support\Http\EnforcementHistoryController;
use App\Support\Http\PlayerReportController;
use App\Support\Http\SupportTicketController;
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

Route::middleware([SetIdentityLocale::class, 'auth'])
    ->prefix('support')
    ->group(function (): void {
        Route::get('/tickets', [SupportTicketController::class, 'index'])->name('support.tickets.index');
        Route::get('/tickets/create', [SupportTicketController::class, 'create'])->name('support.tickets.create');
        Route::post('/tickets', [SupportTicketController::class, 'store'])
            ->middleware('throttle:6,1')
            ->name('support.tickets.store');
        Route::get('/tickets/{supportTicket}', [SupportTicketController::class, 'show'])->name('support.tickets.show');
        Route::post('/tickets/{supportTicket}/reply', [SupportTicketController::class, 'reply'])
            ->middleware('throttle:12,1')
            ->name('support.tickets.reply');
        Route::put('/tickets/{supportTicket}/status', [SupportTicketController::class, 'status'])
            ->middleware('throttle:12,1')
            ->name('support.tickets.status');

        Route::get('/reports', [PlayerReportController::class, 'index'])->name('support.reports.index');
        Route::get('/reports/create', [PlayerReportController::class, 'create'])->name('support.reports.create');
        Route::post('/reports', [PlayerReportController::class, 'store'])
            ->middleware('throttle:3,1')
            ->name('support.reports.store');
        Route::get('/reports/{playerReport}', [PlayerReportController::class, 'show'])->name('support.reports.show');

        Route::get('/enforcement', [EnforcementHistoryController::class, 'index'])->name('support.enforcement.index');
        Route::get('/enforcement/{enforcementRecord}', [EnforcementHistoryController::class, 'show'])->name('support.enforcement.show');
        Route::post('/enforcement/{enforcementRecord}/acknowledge', [EnforcementHistoryController::class, 'acknowledge'])
            ->middleware('throttle:12,1')
            ->name('support.enforcement.acknowledge');
        Route::post('/enforcement/{enforcementRecord}/appeal', [EnforcementHistoryController::class, 'appeal'])
            ->middleware('throttle:3,1')
            ->name('support.enforcement.appeal');
    });

Route::middleware([SetIdentityLocale::class, 'auth', 'mfa.confirmed', 'admin.permission:support.content.manage'])
    ->prefix('admin/support-content')
    ->group(function (): void {
        Route::get('/', [AdminSupportContentController::class, 'index'])
            ->name('admin.support-content.index');
        Route::get('/{editorialPageKey}/edit', [AdminSupportContentController::class, 'edit'])
            ->name('admin.support-content.edit');
        Route::put('/{editorialPageKey}', [AdminSupportContentController::class, 'update'])
            ->name('admin.support-content.update');
    });

Route::middleware([SetIdentityLocale::class, 'auth', 'mfa.confirmed', 'admin.permission:support.tickets.manage'])
    ->prefix('admin/support/tickets')
    ->group(function (): void {
        Route::get('/', [AdminSupportTicketController::class, 'index'])->name('admin.support.tickets.index');
        Route::get('/{supportTicket}', [AdminSupportTicketController::class, 'show'])->name('admin.support.tickets.show');
        Route::post('/{supportTicket}/reply', [AdminSupportTicketController::class, 'reply'])->name('admin.support.tickets.reply');
        Route::put('/{supportTicket}/status', [AdminSupportTicketController::class, 'status'])->name('admin.support.tickets.status');
    });

Route::middleware([SetIdentityLocale::class, 'auth', 'mfa.confirmed', 'admin.permission:support.reports.manage'])
    ->prefix('admin/moderation/reports')
    ->group(function (): void {
        Route::get('/', [AdminPlayerReportController::class, 'index'])->name('admin.moderation.reports.index');
        Route::get('/{playerReport}', [AdminPlayerReportController::class, 'show'])->name('admin.moderation.reports.show');
        Route::put('/{playerReport}', [AdminPlayerReportController::class, 'update'])->name('admin.moderation.reports.update');
    });

Route::middleware([SetIdentityLocale::class, 'auth', 'mfa.confirmed', 'admin.permission:support.enforcement.manage'])
    ->prefix('admin/moderation/enforcement')
    ->group(function (): void {
        Route::get('/', [AdminEnforcementController::class, 'index'])->name('admin.moderation.enforcement.index');
        Route::get('/create', [AdminEnforcementController::class, 'create'])->name('admin.moderation.enforcement.create');
        Route::post('/', [AdminEnforcementController::class, 'store'])->name('admin.moderation.enforcement.store');
        Route::get('/{enforcementRecord}', [AdminEnforcementController::class, 'show'])->name('admin.moderation.enforcement.show');
        Route::get('/{enforcementRecord}/edit', [AdminEnforcementController::class, 'edit'])->name('admin.moderation.enforcement.edit');
        Route::put('/{enforcementRecord}', [AdminEnforcementController::class, 'update'])->name('admin.moderation.enforcement.update');
        Route::put('/{enforcementRecord}/appeal', [AdminEnforcementController::class, 'appeal'])->name('admin.moderation.enforcement.appeal');
    });
