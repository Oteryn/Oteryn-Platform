@extends('admin.layout')

@section('title', 'Manage Pages')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Content</p>
        <h1>Managed pages</h1>
        <p class="muted">Create and maintain public managed pages, publication state and explicit Polish translations.</p>
    </div>

    <div class="action-row">
        <a class="button" href="{{ route('admin.pages.create') }}">Create managed page</a>
    </div>

    @if ($pages->count() === 0)
        <div class="empty-state">
            <strong>No managed pages yet.</strong>
            <p>Create a page to publish durable public content.</p>
        </div>
    @else
        <div class="table-region" tabindex="0" aria-label="Managed pages table, horizontally scrollable on small screens">
            <table>
                <thead>
                    <tr>
                        <th scope="col">Title</th>
                        <th scope="col">Slug</th>
                        <th scope="col">Publication</th>
                        <th scope="col">Actions</th>
                    </tr>
                </thead>
                <tbody>
                    @foreach ($pages as $page)
                        <tr>
                            <td>{{ $page->title }}</td>
                            <td>{{ $page->slug }}</td>
                            <td>
                                @if ($page->published_at)
                                    <span class="badge badge-success">Published</span><br>
                                    <span class="muted">{{ $page->published_at->format('Y-m-d H:i') }}</span>
                                @else
                                    <span class="badge badge-warning">Draft</span>
                                @endif
                            </td>
                            <td>
                                <div class="action-row">
                                    <a class="button button-secondary" href="{{ route('admin.pages.edit', $page) }}">Edit English</a>
                                    <a class="button button-secondary" href="{{ route('admin.pages.translation.edit', $page) }}">Polish translation</a>
                                </div>
                            </td>
                        </tr>
                    @endforeach
                </tbody>
            </table>
        </div>
    @endif

    <div class="pagination">{{ $pages->links() }}</div>
@endsection
