@extends('admin.layout')

@section('title', 'Wiki categories')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Wiki · Categories</p>
        <h1>Wiki categories</h1>
        <p class="muted">Manage the bilingual category tree used by public navigation, breadcrumbs and related-article ranking.</p>
    </div>

    <div class="action-row">
        <a class="button button-secondary" href="{{ route('admin.wiki.index') }}">Wiki dashboard</a>
        <a class="button" href="{{ route('admin.wiki.categories.create') }}">Create category</a>
    </div>

    <div class="table-region" tabindex="0" aria-label="Wiki category table, horizontally scrollable on small screens">
        <table>
            <thead>
                <tr>
                    <th scope="col">Category</th>
                    <th scope="col">Parent</th>
                    <th scope="col">Translations</th>
                    <th scope="col">Visibility</th>
                    <th scope="col">Articles</th>
                    <th scope="col">Version</th>
                    <th scope="col">Actions</th>
                </tr>
            </thead>
            <tbody>
                @forelse ($categories as $category)
                    <tr>
                        <td>
                            <strong>{{ $category->en_name ?? $category->key }}</strong><br>
                            <span class="muted">{{ $category->key }} · order {{ $category->sort_order }}</span>
                        </td>
                        <td>{{ $category->parent_name ?? 'Root' }}</td>
                        <td>
                            <span class="badge {{ $category->en_name === null ? 'badge-warning' : '' }}">EN</span>
                            <span class="badge {{ $category->pl_name === null ? 'badge-warning' : '' }}">PL</span>
                        </td>
                        <td>
                            @if ($category->visible)
                                <span class="badge badge-success">Visible</span>
                            @else
                                <span class="badge badge-warning">Hidden</span>
                            @endif
                        </td>
                        <td>{{ $category->article_count }}</td>
                        <td>{{ $category->lock_version }}</td>
                        <td>
                            <a class="button button-secondary" href="{{ route('admin.wiki.categories.edit', $category->id) }}">Edit</a>
                        </td>
                    </tr>
                @empty
                    <tr>
                        <td colspan="7">No Wiki categories exist yet.</td>
                    </tr>
                @endforelse
            </tbody>
        </table>
    </div>
@endsection
