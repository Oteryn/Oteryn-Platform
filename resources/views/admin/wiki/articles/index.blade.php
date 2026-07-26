@extends('admin.layout')

@section('title', 'Wiki articles')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Wiki · Articles</p>
        <h1>Wiki articles</h1>
        <p class="muted">Filter by lifecycle, translation or category. Public routes expose only complete published translations.</p>
    </div>

    <div class="action-row">
        <a class="button button-secondary" href="{{ route('admin.wiki.index') }}">Wiki dashboard</a>
        @if ($canManageArticles)
            <a class="button" href="{{ route('admin.wiki.articles.create') }}">Create article</a>
        @endif
    </div>

    <form class="wiki-filter-form" method="GET" action="{{ route('admin.wiki.articles.index') }}">
        <div class="form-field">
            <label for="status">Status</label>
            <select id="status" name="status">
                <option value="">All statuses</option>
                @foreach ($statuses as $status)
                    <option value="{{ $status->value }}" @selected($filters['status'] === $status->value)>
                        {{ str($status->value)->replace('_', ' ')->title() }}
                    </option>
                @endforeach
            </select>
        </div>
        <div class="form-field">
            <label for="locale">Translation</label>
            <select id="locale" name="locale">
                <option value="">Any locale</option>
                <option value="en" @selected($filters['locale'] === 'en')>English</option>
                <option value="pl" @selected($filters['locale'] === 'pl')>Polish</option>
            </select>
        </div>
        <div class="form-field">
            <label for="category_id">Category</label>
            <select id="category_id" name="category_id">
                <option value="">All categories</option>
                @foreach ($categories as $category)
                    <option value="{{ $category->id }}" @selected($filters['category_id'] === (int) $category->id)>
                        {{ $category->en_name ?? $category->key }}
                    </option>
                @endforeach
            </select>
        </div>
        <div class="action-row">
            <button type="submit">Apply filters</button>
            <a class="button button-secondary" href="{{ route('admin.wiki.articles.index') }}">Reset</a>
        </div>
    </form>

    <div class="table-region" tabindex="0" aria-label="Wiki article table, horizontally scrollable on small screens">
        <table>
            <thead>
                <tr>
                    <th scope="col">Article</th>
                    <th scope="col">Type</th>
                    <th scope="col">Status</th>
                    <th scope="col">Translations</th>
                    <th scope="col">Presentation</th>
                    <th scope="col">Updated</th>
                    <th scope="col">Actions</th>
                </tr>
            </thead>
            <tbody>
                @forelse ($articles as $article)
                    <tr>
                        <td>
                            <strong>{{ $article->en_title ?? $article->pl_title ?? 'Untitled article #'.$article->id }}</strong><br>
                            <span class="muted">ID {{ $article->id }} · version {{ $article->lock_version }}</span>
                        </td>
                        <td>{{ $article->content_type }}</td>
                        <td>
                            @if ($article->status === 'published')
                                <span class="badge badge-success">Published</span>
                            @elseif ($article->status === 'archived')
                                <span class="badge badge-warning">Archived</span>
                            @else
                                <span class="badge">{{ str($article->status)->replace('_', ' ')->title() }}</span>
                            @endif
                        </td>
                        <td>
                            <span class="badge {{ $article->en_title === null ? 'badge-warning' : '' }}">EN</span>
                            <span class="badge {{ $article->pl_title === null ? 'badge-warning' : '' }}">PL</span>
                        </td>
                        <td>
                            {{ $article->is_featured ? 'Featured' : 'Standard' }}<br>
                            <span class="muted">Order {{ $article->sort_order }}</span>
                        </td>
                        <td>{{ \Illuminate\Support\Carbon::parse($article->updated_at)->format('Y-m-d H:i') }}</td>
                        <td>
                            <div class="action-row">
                                @if ($canManageArticles)
                                    <a class="button button-secondary" href="{{ route('admin.wiki.articles.edit', $article->id) }}">Edit</a>
                                @endif
                                <a class="button button-secondary" href="{{ route('admin.wiki.articles.revisions', $article->id) }}">Revisions</a>
                            </div>
                        </td>
                    </tr>
                @empty
                    <tr>
                        <td colspan="7">No Wiki articles match these filters.</td>
                    </tr>
                @endforelse
            </tbody>
        </table>
    </div>

    {{ $articles->links() }}
@endsection
