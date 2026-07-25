<?php

use App\Http\Controllers\Accounts\AccountOverviewController;
use App\Http\Controllers\Admin\AdminAuditController;
use App\Http\Controllers\Admin\AdminManagedPageController;
use App\Http\Controllers\Admin\AdminNewsController;
use App\Http\Controllers\Admin\AdminRoleController;
use App\Http\Controllers\Characters\CharacterCreationController;
use App\Http\Controllers\Cms\PublicNewsController;
use App\Http\Controllers\Cms\PublicPageController;
use App\Http\Controllers\Identity\Mfa\MfaChallengeController;
use App\Http\Controllers\Identity\Mfa\MfaEnrollmentController;
use App\Http\Controllers\Identity\PasswordChangeController;
use App\Http\Controllers\Identity\PasswordRecoveryController;
use App\Http\Controllers\Identity\PasswordResetController;
use App\Http\Controllers\Identity\RegistrationController;
use App\Http\Controllers\Identity\SessionController;
use App\Http\Controllers\PublicGameData\PublicGameDataController;
use Illuminate\Support\Facades\Route;

Route::get('/register', [RegistrationController::class, 'create'])
    ->middleware('guest')
    ->name('identity.register.create');
Route::post('/register', [RegistrationController::class, 'store'])
    ->middleware(['guest', 'throttle:identity-registration'])
    ->name('identity.register.store');

Route::get('/login', [SessionController::class, 'create'])
    ->middleware('guest')
    ->name('identity.login.create');
Route::post('/login', [SessionController::class, 'store'])
    ->middleware(['guest', 'throttle:identity-login', 'throttle:identity-login-source'])
    ->name('identity.login.store');
Route::post('/logout', [SessionController::class, 'destroy'])
    ->middleware('auth')
    ->name('identity.logout');

Route::get('/mfa/challenge', [MfaChallengeController::class, 'create'])
    ->middleware('guest')
    ->name('identity.mfa.challenge.create');
Route::post('/mfa/challenge', [MfaChallengeController::class, 'store'])
    ->middleware([
        'guest',
        'throttle:identity-mfa-challenge',
        'throttle:identity-mfa-challenge-identity',
        'throttle:identity-mfa-challenge-source',
    ])
    ->name('identity.mfa.challenge.store');

Route::get('/mfa', [MfaEnrollmentController::class, 'show'])
    ->middleware('auth')
    ->name('identity.mfa.settings');
Route::post('/mfa/enroll', [MfaEnrollmentController::class, 'store'])
    ->middleware(['auth', 'throttle:identity-mfa-enrollment'])
    ->name('identity.mfa.enroll');
Route::post('/mfa/confirm', [MfaEnrollmentController::class, 'confirm'])
    ->middleware(['auth', 'throttle:identity-mfa-enrollment'])
    ->name('identity.mfa.confirm');
Route::delete('/mfa', [MfaEnrollmentController::class, 'destroy'])
    ->middleware(['auth', 'throttle:identity-mfa-disable'])
    ->name('identity.mfa.destroy');

Route::get('/forgot-password', [PasswordRecoveryController::class, 'create'])
    ->middleware('guest')
    ->name('password.request');
Route::post('/forgot-password', [PasswordRecoveryController::class, 'store'])
    ->middleware(['guest', 'throttle:identity-password-recovery', 'throttle:identity-password-recovery-source'])
    ->name('password.email');
Route::get('/reset-password/{token}', [PasswordResetController::class, 'create'])
    ->middleware('guest')
    ->name('password.reset');
Route::post('/reset-password', [PasswordResetController::class, 'store'])
    ->middleware(['guest', 'throttle:identity-password-reset'])
    ->name('password.update');

Route::get('/password/change', [PasswordChangeController::class, 'create'])
    ->middleware('auth')
    ->name('identity.password.change.create');
Route::put('/password/change', [PasswordChangeController::class, 'update'])
    ->middleware(['auth', 'throttle:identity-password-change'])
    ->name('identity.password.change.update');

Route::get('/account', [AccountOverviewController::class, 'show'])
    ->middleware('auth')
    ->name('account.overview');
Route::post('/account/provisioning/retry', [AccountOverviewController::class, 'retry'])
    ->middleware(['auth', 'throttle:6,1'])
    ->name('account.provisioning.retry');
Route::get('/account/characters/create', [CharacterCreationController::class, 'create'])
    ->middleware('auth')
    ->name('account.characters.create');
Route::post('/account/characters', [CharacterCreationController::class, 'store'])
    ->middleware(['auth', 'throttle:character-create'])
    ->name('account.characters.store');

Route::view('/admin', 'admin.dashboard')
    ->middleware(['auth', 'mfa.confirmed', 'admin.permission:admin.access'])
    ->name('admin.dashboard');

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:cms.news.manage'])
    ->prefix('admin/news')
    ->group(function (): void {
        Route::get('/', [AdminNewsController::class, 'index'])->name('admin.news.index');
        Route::get('/create', [AdminNewsController::class, 'create'])->name('admin.news.create');
        Route::post('/', [AdminNewsController::class, 'store'])->name('admin.news.store');
        Route::get('/{newsPost}/edit', [AdminNewsController::class, 'edit'])->name('admin.news.edit');
        Route::put('/{newsPost}', [AdminNewsController::class, 'update'])->name('admin.news.update');
    });

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:cms.pages.manage'])
    ->prefix('admin/pages')
    ->group(function (): void {
        Route::get('/', [AdminManagedPageController::class, 'index'])->name('admin.pages.index');
        Route::get('/create', [AdminManagedPageController::class, 'create'])->name('admin.pages.create');
        Route::post('/', [AdminManagedPageController::class, 'store'])->name('admin.pages.store');
        Route::get('/{managedPage}/edit', [AdminManagedPageController::class, 'edit'])->name('admin.pages.edit');
        Route::put('/{managedPage}', [AdminManagedPageController::class, 'update'])->name('admin.pages.update');
    });

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:admin.roles.manage'])
    ->prefix('admin/roles')
    ->group(function (): void {
        Route::get('/', [AdminRoleController::class, 'index'])->name('admin.roles.index');
        Route::post('/identities/{identity}', [AdminRoleController::class, 'store'])->name('admin.roles.store');
        Route::delete('/identities/{identity}/{roleKey}', [AdminRoleController::class, 'destroy'])->name('admin.roles.destroy');
    });

Route::get('/admin/audit', [AdminAuditController::class, 'index'])
    ->middleware(['auth', 'mfa.confirmed', 'admin.permission:audit.view'])
    ->name('admin.audit.index');

Route::get('/news', [PublicNewsController::class, 'index'])->name('news.index');
Route::get('/news/{slug}', [PublicNewsController::class, 'show'])->name('news.show');
Route::get('/pages/{slug}', [PublicPageController::class, 'show'])->name('pages.show');

Route::get('/highscores', [PublicGameDataController::class, 'highscores'])->name('game.highscores.index');
Route::get('/characters', [PublicGameDataController::class, 'characterSearch'])->name('game.characters.search');
Route::get('/characters/{name}', [PublicGameDataController::class, 'character'])->name('game.characters.show');
Route::get('/guilds/{name}', [PublicGameDataController::class, 'guild'])->name('game.guilds.show');
Route::get('/online', [PublicGameDataController::class, 'online'])->name('game.online.index');
Route::get('/servers', [PublicGameDataController::class, 'servers'])->name('game.servers.index');

$moduleRouteFiles = glob(__DIR__.'/modules/*.php');

if ($moduleRouteFiles !== false) {
    sort($moduleRouteFiles, SORT_STRING);

    foreach ($moduleRouteFiles as $moduleRouteFile) {
        require $moduleRouteFile;
    }
}
