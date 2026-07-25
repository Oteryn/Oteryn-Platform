@extends('admin.layout')

@section('title', $announcement === null ? 'Create Announcement' : 'Edit Announcement')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Public portal · Announcements</p>
        <h1>{{ $announcement === null ? 'Create announcement' : 'Edit announcement' }}</h1>
        <p class="muted">Content is plain text. Times are entered and stored in UTC. Start is inclusive; end is exclusive.</p>
    </div>

    <div class="card">
        <form class="form-stack" method="POST" action="{{ $announcement === null ? route('admin.announcements.store') : route('admin.announcements.update', $announcement) }}">
            @csrf
            @if ($announcement !== null)
                @method('PUT')
                <input type="hidden" name="lock_version" value="{{ old('lock_version', $announcement->lock_version) }}">
            @endif

            <div class="form-field">
                <label for="title">Title</label>
                <input id="title" name="title" type="text" maxlength="200" required value="{{ old('title', $announcement?->title) }}">
            </div>

            <div class="form-field">
                <label for="body">Body (plain text)</label>
                <textarea id="body" name="body" rows="8" maxlength="10000" required>{{ old('body', $announcement?->body) }}</textarea>
            </div>

            <div class="form-field">
                <label for="severity">Severity</label>
                <select id="severity" name="severity" required>
                    @foreach (\App\Announcements\Models\SiteAnnouncement::severities() as $severity)
                        <option value="{{ $severity }}" @selected(old('severity', $announcement?->severity ?? 'info') === $severity)>
                            {{ ucfirst($severity) }}
                        </option>
                    @endforeach
                </select>
            </div>

            <div class="form-field">
                <label for="starts_at">Starts at (UTC)</label>
                <input id="starts_at" name="starts_at" type="datetime-local" required value="{{ old('starts_at', $announcement?->starts_at?->format('Y-m-d\TH:i')) }}">
            </div>

            <div class="form-field">
                <label for="ends_at">Ends at (UTC)</label>
                <input id="ends_at" name="ends_at" type="datetime-local" value="{{ old('ends_at', $announcement?->ends_at?->format('Y-m-d\TH:i')) }}">
                <p class="form-help">Leave empty for no automatic end.</p>
            </div>

            <div class="form-field">
                <label for="publication_state">Publication state</label>
                <select id="publication_state" name="publication_state" required>
                    @foreach (\App\Announcements\Models\SiteAnnouncement::publicationStates() as $state)
                        <option value="{{ $state }}" @selected(old('publication_state', $announcement?->publication_state ?? 'draft') === $state)>
                            {{ ucfirst($state) }}
                        </option>
                    @endforeach
                </select>
            </div>

            <div class="form-field">
                <label for="action_label">Action label</label>
                <input id="action_label" name="action_label" type="text" maxlength="80" value="{{ old('action_label', $announcement?->action_label) }}">
            </div>

            <div class="form-field">
                <label for="action_url">Action link</label>
                <input id="action_url" name="action_url" type="text" maxlength="2048" value="{{ old('action_url', $announcement?->action_url) }}">
                <p class="form-help">Use an internal path beginning with “/” or an approved HTTPS URL.</p>
            </div>

            <div class="action-row">
                <button type="submit">Save announcement</button>
                <a class="button button-secondary" href="{{ route('admin.announcements.index') }}">Cancel</a>
            </div>
        </form>
    </div>
@endsection
