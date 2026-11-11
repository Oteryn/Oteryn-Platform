@extends('admin.layout')

@section('title', 'Manage News')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Content</p>
        <h1>News</h1>
        <p class="muted">Create and maintain public news posts, publication state and explicit Polish translations.</p>
    </div>

    <div class="action-row">
        <a class="button" href="{{ route('admin.news.create') }}">Create news post</a>
    </div>

    @if ($posts->count() === 0)
        <div class="empty-state">
            <strong>No news posts yet.</strong>
            <p>Create a post to begin managing public news.</p>
        </div>
    @else
        <div class="table-region" tabindex="0" aria-label="News management table, horizontally scrollable on small screens">
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
                    @foreach ($posts as $post)
                        <tr>
                            <td>{{ $post->title }}</td>
                            <td>{{ $post->slug }}</td>
                            <td>
                                @if ($post->published_at)
                                    <span class="badge badge-success">Published</span><br>
                                    <span class="muted">{{ $post->published_at->format('Y-m-d H:i') }}</span>
                                @else
                                    <span class="badge badge-warning">Draft</span>
                                @endif
                            </td>
                            <td>
                                <div class="action-row">
                                    <a class="button button-secondary" href="{{ route('admin.news.edit', $post) }}">Edit English</a>
                                    <a class="button button-secondary" href="{{ route('admin.news.translation.edit', $post) }}">Polish translation</a>
                                </div>
                            </td>
                        </tr>
                    @endforeach
                </tbody>
            </table>
        </div>
    @endif

    <div class="pagination">{{ $posts->links() }}</div>
@endsection
