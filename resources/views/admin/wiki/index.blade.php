@extends('admin.layout')

@section('title', 'Wiki')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Content · First-party knowledge base</p>
        <h1>Wiki administration</h1>
        <p class="muted">Manage bilingual drafts, review state, publication and category navigation through the audited Wiki lifecycle.</p>
    </div>

    <div class="wiki-admin-grid" aria-label="Wiki content summary">
        <section class="card wiki-admin-stat">
            <span class="muted">Articles</span>
            <strong>{{ $articleCount }}</strong>
        </section>
        <section class="card wiki-admin-stat">
            <span class="muted">Categories</span>
            <strong>{{ $categoryCount }}</strong>
        </section>
        <section class="card wiki-admin-stat">
            <span class="muted">Published</span>
            <strong>{{ $statusCounts['published'] ?? 0 }}</strong>
        </section>
    </div>

    <section class="card">
        <h2>Article workflow</h2>
        <div class="wiki-inline-meta">
            <span class="badge">Draft {{ $statusCounts['draft'] ?? 0 }}</span>
            <span class="badge">In review {{ $statusCounts['in_review'] ?? 0 }}</span>
            <span class="badge badge-success">Published {{ $statusCounts['published'] ?? 0 }}</span>
            <span class="badge badge-warning">Archived {{ $statusCounts['archived'] ?? 0 }}</span>
        </div>
        <div class="action-row">
            <a class="button button-secondary" href="{{ route('admin.wiki.articles.index') }}">Browse articles</a>
            @if ($canManageArticles)
                <a class="button" href="{{ route('admin.wiki.articles.create') }}">Create article</a>
            @endif
            <a class="button button-secondary" href="{{ route('admin.wiki.categories.index') }}">Browse categories</a>
            @if ($canManageCategories)
                <a class="button button-secondary" href="{{ route('admin.wiki.categories.create') }}">Create category</a>
            @endif
        </div>
    </section>

    <section class="card">
        <h2>Permission boundary</h2>
        <ul>
            <li>Article editing: <strong>{{ $canManageArticles ? 'allowed' : 'not granted' }}</strong></li>
            <li>Category management: <strong>{{ $canManageCategories ? 'allowed' : 'not granted' }}</strong></li>
            <li>Publish, unpublish, archive and restore: <strong>{{ $canPublish ? 'allowed' : 'not granted' }}</strong></li>
        </ul>
        <p class="muted">All mutations require the exact permission, confirmed MFA, CSRF protection and bounded audit metadata.</p>
    </section>
@endsection
