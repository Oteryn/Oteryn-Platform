@extends('admin.layout')

@section('title', 'Wiki revision history')

@section('content')
    @php($english = $translations->get('en'))
    @php($polish = $translations->get('pl'))

    <div class="page-header">
        <p class="eyebrow">Wiki · Revision history</p>
        <h1>{{ $english?->title ?? $polish?->title ?? 'Article #'.$article->id }}</h1>
        <p class="muted">
            Current status: {{ str($article->status->value)->replace('_', ' ')->title() }}
            · version {{ $article->lock_version }}
            · revisions are append-only
        </p>
    </div>

    <div class="action-row">
        <a class="button button-secondary" href="{{ route('admin.wiki.articles.index') }}">Back to articles</a>
        @if ($article->status->isEditable())
            <a class="button button-secondary" href="{{ route('admin.wiki.articles.edit', $article) }}">Edit article</a>
        @endif
    </div>

    <form class="wiki-filter-form" method="GET" action="{{ route('admin.wiki.articles.revisions', $article) }}">
        <div class="form-field">
            <label for="locale">Locale</label>
            <select id="locale" name="locale">
                <option value="">All locales</option>
                <option value="en" @selected($locale === 'en')>English</option>
                <option value="pl" @selected($locale === 'pl')>Polish</option>
            </select>
        </div>
        <div class="action-row">
            <button type="submit">Apply filter</button>
            <a class="button button-secondary" href="{{ route('admin.wiki.articles.revisions', $article) }}">Reset</a>
        </div>
    </form>

    <div class="table-region" tabindex="0" aria-label="Wiki revision table, horizontally scrollable on small screens">
        <table>
            <thead>
                <tr>
                    <th scope="col">Revision</th>
                    <th scope="col">Title and slug</th>
                    <th scope="col">Editor</th>
                    <th scope="col">Change note</th>
                    <th scope="col">Created</th>
                    <th scope="col">Actions</th>
                </tr>
            </thead>
            <tbody>
                @forelse ($revisions as $revision)
                    <tr>
                        <td>
                            <strong>{{ strtoupper($revision->locale) }} #{{ $revision->revision_number }}</strong><br>
                            <span class="muted">Article version {{ $revision->article_version }}</span>
                            @if ($revision->source_revision_id !== null)
                                <br><span class="badge">Restored from {{ $revision->source_revision_id }}</span>
                            @endif
                        </td>
                        <td>
                            <strong>{{ $revision->title }}</strong><br>
                            <span class="muted">{{ $revision->slug }}</span>
                            <details>
                                <summary>Inspect source</summary>
                                <pre><code>{{ $revision->source_markdown }}</code></pre>
                            </details>
                        </td>
                        <td>{{ $revision->editor_identity_id ?? 'Deleted identity' }}</td>
                        <td>{{ $revision->change_note ?? 'No note' }}</td>
                        <td>{{ $revision->created_at->format('Y-m-d H:i') }}</td>
                        <td>
                            @if ($canPublish && $article->status->isEditable())
                                <form class="form-stack" method="POST" action="{{ route('admin.wiki.articles.revisions.restore', [$article, $revision]) }}">
                                    @csrf
                                    <input type="hidden" name="lock_version" value="{{ $article->lock_version }}">
                                    <label class="visually-hidden" for="change_note_{{ $revision->id }}">Restore note</label>
                                    <input id="change_note_{{ $revision->id }}" name="change_note" type="text" maxlength="500" placeholder="Reason for restore">
                                    <button class="button-secondary" type="submit">Restore as new revision</button>
                                </form>
                            @else
                                <span class="muted">Unpublish to a draft and use a publisher account to restore.</span>
                            @endif
                        </td>
                    </tr>
                @empty
                    <tr>
                        <td colspan="6">No revisions match this filter.</td>
                    </tr>
                @endforelse
            </tbody>
        </table>
    </div>

    {{ $revisions->links() }}
@endsection
