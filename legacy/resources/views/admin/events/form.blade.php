@extends('admin.layout')

@section('title', $event === null ? 'Create Event' : 'Edit Event')

@section('content')
    @php
        $english = $translations->get('en');
        $polish = $translations->get('pl');
    @endphp

    <div class="page-header">
        <p class="eyebrow">Public portal · Events</p>
        <h1>{{ $event === null ? 'Create event' : 'Edit event' }}</h1>
        <p class="muted">Times are entered and stored in UTC. Content is plain text. Saving content always returns the record to draft.</p>
    </div>

    <div class="card">
        <form class="form-stack" method="POST" action="{{ $event === null ? route('admin.events.store') : route('admin.events.update', $event) }}">
            @csrf
            @if ($event !== null)
                @method('PUT')
                <input type="hidden" name="lock_version" value="{{ old('lock_version', $event->lock_version) }}">
            @endif

            <div class="form-field">
                <label for="starts_at">Starts at (UTC)</label>
                <input id="starts_at" name="starts_at" type="datetime-local" required value="{{ old('starts_at', $event?->starts_at?->format('Y-m-d\TH:i')) }}">
            </div>

            <div class="form-field">
                <label for="ends_at">Ends at (UTC)</label>
                <input id="ends_at" name="ends_at" type="datetime-local" required value="{{ old('ends_at', $event?->ends_at?->format('Y-m-d\TH:i')) }}">
            </div>

            <div class="form-field">
                <label>
                    <input name="featured" type="checkbox" value="1" @checked(old('featured', $event?->featured ?? false))>
                    Featured event
                </label>
            </div>

            <div class="form-field">
                <label for="news_post_id">Related news post</label>
                <select id="news_post_id" name="news_post_id">
                    <option value="">None</option>
                    @foreach ($newsPosts as $newsPost)
                        <option value="{{ $newsPost->id }}" @selected((string) old('news_post_id', $event?->news_post_id) === (string) $newsPost->id)>
                            {{ $newsPost->title }} ({{ $newsPost->slug }})
                        </option>
                    @endforeach
                </select>
            </div>

            <fieldset>
                <legend>English translation (required)</legend>
                <div class="form-field">
                    <label for="translations_en_title">Title</label>
                    <input id="translations_en_title" name="translations[en][title]" type="text" maxlength="200" required value="{{ old('translations.en.title', $english?->title) }}">
                </div>
                <div class="form-field">
                    <label for="translations_en_slug">Slug</label>
                    <input id="translations_en_slug" name="translations[en][slug]" type="text" maxlength="160" required value="{{ old('translations.en.slug', $english?->slug) }}">
                </div>
                <div class="form-field">
                    <label for="translations_en_summary">Summary</label>
                    <textarea id="translations_en_summary" name="translations[en][summary]" rows="4" maxlength="500" required>{{ old('translations.en.summary', $english?->summary) }}</textarea>
                </div>
                <div class="form-field">
                    <label for="translations_en_body">Body (plain text)</label>
                    <textarea id="translations_en_body" name="translations[en][body]" rows="16" maxlength="100000" required>{{ old('translations.en.body', $english?->body) }}</textarea>
                </div>
            </fieldset>

            <fieldset>
                <legend>Polish translation (optional, complete all fields when used)</legend>
                <div class="form-field">
                    <label for="translations_pl_title">Title</label>
                    <input id="translations_pl_title" name="translations[pl][title]" type="text" maxlength="200" value="{{ old('translations.pl.title', $polish?->title) }}">
                </div>
                <div class="form-field">
                    <label for="translations_pl_slug">Slug</label>
                    <input id="translations_pl_slug" name="translations[pl][slug]" type="text" maxlength="160" value="{{ old('translations.pl.slug', $polish?->slug) }}">
                </div>
                <div class="form-field">
                    <label for="translations_pl_summary">Summary</label>
                    <textarea id="translations_pl_summary" name="translations[pl][summary]" rows="4" maxlength="500">{{ old('translations.pl.summary', $polish?->summary) }}</textarea>
                </div>
                <div class="form-field">
                    <label for="translations_pl_body">Body (plain text)</label>
                    <textarea id="translations_pl_body" name="translations[pl][body]" rows="16" maxlength="100000">{{ old('translations.pl.body', $polish?->body) }}</textarea>
                </div>
            </fieldset>

            <div class="action-row">
                <button type="submit">Save event draft</button>
                <a class="button button-secondary" href="{{ route('admin.events.index') }}">Cancel</a>
            </div>
        </form>
    </div>

    @if ($event !== null && $canPublish)
        <div class="card">
            <h2>Publication state</h2>
            <p class="muted">This action requires the separate events.publish permission. The selected state must match the current UTC time boundaries.</p>
            <form class="form-stack" method="POST" action="{{ route('admin.events.status', $event) }}">
                @csrf
                @method('PUT')
                <input type="hidden" name="lock_version" value="{{ $event->lock_version }}">

                <div class="form-field">
                    <label for="status">State</label>
                    <select id="status" name="status" required>
                        @foreach (\App\Events\Models\Event::statuses() as $status)
                            <option value="{{ $status }}" @selected($event->status === $status)>{{ ucfirst($status) }}</option>
                        @endforeach
                    </select>
                </div>

                <button type="submit">Change publication state</button>
            </form>
        </div>
    @endif
@endsection
