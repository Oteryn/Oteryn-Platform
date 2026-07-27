@extends('admin.layout')

@section('title', 'Polish translation')

@push('head')
    <link rel="stylesheet" href="{{ asset('css/admin-translations.css') }}">
@endpush

@section('content')
    @php
        $sourceBody = match ($type) {
            \App\Cms\Editorial\EditorialContentType::ClientRelease => $source->release_notes,
            default => $source->body,
        };
    @endphp

    <div class="page-header">
        <p class="eyebrow">Localization · Polish</p>
        <h1>Polish translation</h1>
        <p class="muted">
            State: <strong>{{ $translationState->value }}</strong>.
            Saving records the current English source revision. A later source edit makes a published translation stale until an editor reviews and saves it again.
        </p>
    </div>

    @if ($translationState === \App\Cms\Editorial\EditorialTranslationState::Stale)
        <div class="alert alert-warning" role="status">
            <strong>The source changed after this translation was reviewed.</strong>
            <p>Polish public routes remain unavailable until the translation is reviewed and saved again.</p>
        </div>
    @elseif ($translationState === \App\Cms\Editorial\EditorialTranslationState::Incomplete)
        <div class="alert alert-warning" role="status">
            <strong>This translation is incomplete.</strong>
            <p>Incomplete records can be saved as drafts but cannot be published.</p>
        </div>
    @endif

    <div class="card form-stack">
        <section aria-labelledby="english-source-heading">
            <h2 id="english-source-heading">English source</h2>
            @if ($type->requiresTitle())
                <p><strong>{{ $source->title }}</strong></p>
            @endif
            <pre class="content-body">{{ $sourceBody }}</pre>
            @if ($type === \App\Cms\Editorial\EditorialContentType::SiteAnnouncement && $source->action_label !== null)
                <p><strong>Action label:</strong> {{ $source->action_label }}</p>
            @endif
        </section>

        <form class="form-stack translation-form" method="POST" action="{{ route($updateRoute, $source) }}">
            @csrf
            @method('PUT')

            @if ($type->requiresTitle())
                <div class="form-field">
                    <label for="title">Polish title</label>
                    <input id="title" name="title" type="text" maxlength="200" value="{{ old('title', $translation?->title) }}">
                </div>
            @endif

            <div class="form-field">
                <label for="body">Polish content (plain text)</label>
                <textarea id="body" name="body" rows="20" maxlength="100000">{{ old('body', $translation?->body) }}</textarea>
            </div>

            @if ($type === \App\Cms\Editorial\EditorialContentType::SiteAnnouncement)
                <div class="form-field">
                    <label for="action_label">Polish action label</label>
                    <input id="action_label" name="action_label" type="text" maxlength="80" value="{{ old('action_label', $translation?->action_label) }}">
                    <p class="form-help">When omitted, the announcement action is hidden on Polish pages rather than using its English label.</p>
                </div>
            @endif

            <div class="form-field">
                <label for="published_at">Publish Polish translation at (UTC)</label>
                <input id="published_at" name="published_at" type="datetime-local" value="{{ old('published_at', $translation?->published_at?->format('Y-m-d\TH:i')) }}">
                <p class="form-help">Leave empty for a draft. Clear every translated field to remove the record. No English content is substituted on Polish routes.</p>
            </div>

            <div class="action-row">
                <button type="submit">Save translation</button>
                <a class="button button-secondary" href="{{ $backUrl }}">Back to source</a>
            </div>
        </form>
    </div>
@endsection
