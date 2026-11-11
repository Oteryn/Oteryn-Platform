@extends('admin.layout')

@section('title', $post === null ? 'Create News Post' : 'Edit News Post')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Content · News</p>
        <h1>{{ $post === null ? 'Create news post' : 'Edit news post' }}</h1>
        <p class="muted">Use a stable slug, a clear title and plain-text content. Publication time controls public visibility.</p>
    </div>

    <div class="card">
        <form class="form-stack" method="POST" action="{{ $post === null ? route('admin.news.store') : route('admin.news.update', $post) }}">
            @csrf
            @if ($post !== null)
                @method('PUT')
            @endif

            <div class="form-field">
                <label for="slug">Slug</label>
                <input id="slug" name="slug" type="text" maxlength="160" required value="{{ old('slug', $post?->slug) }}">
                <p class="form-help">Used in the public news URL.</p>
            </div>
            <div class="form-field">
                <label for="title">Title</label>
                <input id="title" name="title" type="text" maxlength="200" required value="{{ old('title', $post?->title) }}">
            </div>
            <div class="form-field">
                <label for="body">Body (plain text)</label>
                <textarea id="body" name="body" rows="20" maxlength="100000" required>{{ old('body', $post?->body) }}</textarea>
            </div>
            <div class="form-field">
                <label for="published_at">Publish at</label>
                <input id="published_at" name="published_at" type="datetime-local" value="{{ old('published_at', $post?->published_at?->format('Y-m-d\TH:i')) }}">
                <p class="form-help">Leave empty to keep this post as a draft.</p>
            </div>
            <div class="action-row">
                <button type="submit">Save</button>
                <a class="button button-secondary" href="{{ route('admin.news.index') }}">Cancel</a>
            </div>
        </form>
    </div>
@endsection
