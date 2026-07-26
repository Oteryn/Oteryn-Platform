<?php

use App\Wiki\Http\Admin\AdminWikiArticleController;
use App\Wiki\Http\Admin\AdminWikiCategoryController;
use App\Wiki\Http\Admin\AdminWikiController;
use App\Wiki\Http\Admin\AdminWikiLifecycleController;
use App\Wiki\Http\Public\PublicWikiController;
use Illuminate\Support\Facades\Route;

Route::get('/wiki', [PublicWikiController::class, 'index'])->name('wiki.index');
Route::get('/wiki/search', [PublicWikiController::class, 'search'])
    ->middleware('throttle:wiki-search')
    ->name('wiki.search');
Route::get('/wiki/category/{slug}', [PublicWikiController::class, 'category'])
    ->where('slug', '[a-z0-9]+(?:-[a-z0-9]+)*')
    ->name('wiki.category');
Route::get('/wiki/{slug}', [PublicWikiController::class, 'article'])
    ->where('slug', '[a-z0-9]+(?:-[a-z0-9]+)*')
    ->name('wiki.article');

Route::middleware(['auth', 'mfa.confirmed', 'admin.permission:wiki.access'])
    ->prefix('admin/wiki')
    ->name('admin.wiki.')
    ->group(function (): void {
        Route::get('/', AdminWikiController::class)->name('index');
        Route::get('/articles', [AdminWikiArticleController::class, 'index'])->name('articles.index');
        Route::get('/categories', [AdminWikiCategoryController::class, 'index'])->name('categories.index');

        Route::middleware('admin.permission:wiki.articles.manage')->group(function (): void {
            Route::get('/articles/create', [AdminWikiArticleController::class, 'create'])->name('articles.create');
            Route::post('/articles', [AdminWikiArticleController::class, 'store'])->name('articles.store');
            Route::get('/articles/{article}/edit', [AdminWikiArticleController::class, 'edit'])->name('articles.edit');
            Route::put('/articles/{article}', [AdminWikiArticleController::class, 'update'])->name('articles.update');
            Route::get('/articles/{article}/preview/{locale}', [AdminWikiArticleController::class, 'preview'])
                ->where('locale', 'en|pl')
                ->middleware('signed')
                ->name('articles.preview');
            Route::get('/articles/{article}/revisions', [AdminWikiArticleController::class, 'revisions'])
                ->name('articles.revisions');
            Route::post('/articles/{article}/submit-review', [AdminWikiLifecycleController::class, 'submitReview'])
                ->name('articles.submit-review');
            Route::post('/articles/{article}/return-draft', [AdminWikiLifecycleController::class, 'returnDraft'])
                ->name('articles.return-draft');
        });

        Route::middleware('admin.permission:wiki.publish')->group(function (): void {
            Route::post('/articles/{article}/publish', [AdminWikiLifecycleController::class, 'publish'])
                ->name('articles.publish');
            Route::post('/articles/{article}/unpublish', [AdminWikiLifecycleController::class, 'unpublish'])
                ->name('articles.unpublish');
            Route::post('/articles/{article}/archive', [AdminWikiLifecycleController::class, 'archive'])
                ->name('articles.archive');
            Route::post(
                '/articles/{article}/revisions/{revision}/restore',
                [AdminWikiArticleController::class, 'restore'],
            )->name('articles.revisions.restore');
        });

        Route::middleware('admin.permission:wiki.categories.manage')->group(function (): void {
            Route::get('/categories/create', [AdminWikiCategoryController::class, 'create'])->name('categories.create');
            Route::post('/categories', [AdminWikiCategoryController::class, 'store'])->name('categories.store');
            Route::get('/categories/{category}/edit', [AdminWikiCategoryController::class, 'edit'])
                ->name('categories.edit');
            Route::put('/categories/{category}', [AdminWikiCategoryController::class, 'update'])
                ->name('categories.update');
        });
    });
