@extends('admin.layout')

@section('title', $page === null ? 'Create Managed Page' : 'Edit Managed Page')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Content · Managed pages</p>
        <h1>{{ $page === null ? 'Create managed page' : 'Edit managed page' }}</h1>
        <p class="muted">Use a stable slug and plain-text content. Publication time controls public visibility.</p>
    </div>

    <div class="card">
        <form class="form-stack" method="POST" action="{{ $page === null ? route('admin.pages.store') : route('admin.pages.update', $page) }}">
            @csrf
            @if ($page !== null)
                @method('PUT')
            @endif

            <div class="form-field">
                <label for="slug">Slug</label>
                <input id="slug" name="slug" type="text" maxlength="160" required value="{{ old('slug', $page?->slug) }}">
                <p class="form-help">Used in the public page URL.</p>
            </div>
            <div class="form-field">
                <label for="title">Title</label>
                <input id="title" name="title" type="text" maxlength="200" required value="{{ old('title', $page?->title) }}">
            </div>
            <div class="form-field">
                <label for="body">Body (plain text)</label>
                <textarea id="body" name="body" rows="20" maxlength="100000" required>{{ old('body', $page?->body) }}</textarea>
            </div>
            <div class="form-field">
                <label for="published_at">Publish at</label>
                <input id="published_at" name="published_at" type="datetime-local" value="{{ old('published_at', $page?->published_at?->format('Y-m-d\TH:i')) }}">
                <p class="form-help">Leave empty to keep this page as a draft.</p>
            </div>
            <div class="action-row">
                <button type="submit">Save</button>
                <a class="button button-secondary" href="{{ route('admin.pages.index') }}">Cancel</a>
            </div>
        </form>
    </div>
@endsection
