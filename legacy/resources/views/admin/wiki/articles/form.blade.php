@extends('admin.layout')

@section('title', $article === null ? 'Create Wiki article' : 'Edit Wiki article')

@push('head')
    <script src="{{ asset('js/media-fallbacks.js') }}" defer></script>
    <script src="{{ asset('js/wiki-admin-media.js') }}" defer></script>
@endpush

@section('content')
    @php($english = $translations->get('en'))
    @php($polish = $translations->get('pl'))
    @php($chosenCategoryIds = array_map('intval', old('category_ids', $selectedCategoryIds)))

    <div class="page-header">
        <p class="eyebrow">Wiki · Articles</p>
        <h1>{{ $article === null ? 'Create Wiki article' : ($english?->title ?? $polish?->title ?? 'Edit Wiki article') }}</h1>
        @if ($article !== null)
            <p class="muted">
                Status: {{ str($article->status->value)->replace('_', ' ')->title() }}
                · version {{ $article->lock_version }}
                · last updated {{ $article->updated_at->format('Y-m-d H:i') }}
            </p>
        @else
            <p class="muted">English is required for every draft. Polish may be added now or before publication.</p>
        @endif
    </div>

    <div class="action-row">
        <a class="button button-secondary" href="{{ route('admin.wiki.articles.index') }}">Back to articles</a>
        @if ($article !== null)
            <a class="button button-secondary" href="{{ route('admin.wiki.articles.revisions', $article) }}">Revision history</a>
            @foreach ($previewUrls as $locale => $previewUrl)
                <a class="button button-secondary" href="{{ $previewUrl }}" target="_blank" rel="noopener">
                    Preview {{ strtoupper($locale) }}
                </a>
            @endforeach
        @endif
    </div>

    <section class="card wiki-media-picker"
             data-wiki-media-picker
             data-index-url="{{ route('admin.wiki.media.index') }}">
        <div class="section-heading">
            <div>
                <p class="eyebrow">Approved EditorialMedia</p>
                <h2>Insert an existing image</h2>
            </div>
        </div>
        <p class="muted">
            This read-only picker grants no upload or deletion authority. Inserted Markdown uses
            <code>![Localized alternative text](wiki-media:ID)</code>; review and localize the alternative text in each translation.
        </p>
        <div class="wiki-media-search">
            <div class="form-field">
                <label for="wiki_media_search">Search approved images</label>
                <input id="wiki_media_search" type="search" maxlength="100"
                       placeholder="Alternative text, original display name or numeric ID"
                       data-wiki-media-search>
            </div>
            <button class="button button-secondary" type="button" data-wiki-media-search-button>Search</button>
        </div>
        <p class="muted" aria-live="polite" data-wiki-media-status>
            Loading approved images...
        </p>
        <div class="wiki-media-results" data-wiki-media-results></div>
        <button class="button button-secondary" type="button" data-wiki-media-more hidden>Load more images</button>
    </section>

    <form class="form-stack" method="POST" action="{{ $article === null ? route('admin.wiki.articles.store') : route('admin.wiki.articles.update', $article) }}">
        @csrf
        @if ($article !== null)
            @method('PUT')
            <input type="hidden" name="lock_version" value="{{ $article->lock_version }}">
        @endif

        <section class="card">
            <h2>Article settings</h2>
            <div class="wiki-form-grid">
                <div class="form-field">
                    <label for="content_type">Content type</label>
                    <input id="content_type" name="content_type" type="text" maxlength="64" required
                           pattern="[a-z0-9]+([._\-][a-z0-9]+)*"
                           value="{{ old('content_type', $article?->content_type ?? 'guide') }}">
                    <p class="form-help">Stable lowercase key, for example <code>guide</code>, <code>system</code> or <code>reference</code>.</p>
                </div>
                <div class="form-field">
                    <label for="sort_order">Display order</label>
                    <input id="sort_order" name="sort_order" type="number" min="0" max="1000000" required
                           value="{{ old('sort_order', $article?->sort_order ?? 0) }}">
                </div>
            </div>
            <label class="wiki-category-option" for="is_featured">
                <input id="is_featured" name="is_featured" type="checkbox" value="1"
                       @checked(old('is_featured', $article?->is_featured ?? false))>
                <span><strong>Feature this article</strong><br><span class="muted">Featured articles may be highlighted on the public Wiki homepage.</span></span>
            </label>
        </section>

        <div class="wiki-translation-grid">
            <fieldset class="card wiki-translation-panel">
                <legend><strong>English</strong> · required</legend>
                <div class="form-field">
                    <label for="translations_en_title">Title</label>
                    <input id="translations_en_title" name="translations[en][title]" type="text" maxlength="200" required
                           value="{{ old('translations.en.title', $english?->title) }}">
                </div>
                <div class="form-field">
                    <label for="translations_en_slug">Slug</label>
                    <input id="translations_en_slug" name="translations[en][slug]" type="text" maxlength="160" required
                           pattern="[a-z0-9]+(-[a-z0-9]+)*"
                           value="{{ old('translations.en.slug', $english?->slug) }}">
                </div>
                <div class="form-field">
                    <label for="translations_en_summary">Summary</label>
                    <textarea id="translations_en_summary" name="translations[en][summary]" rows="4" maxlength="1000">{{ old('translations.en.summary', $english?->summary) }}</textarea>
                    <p class="form-help">A non-empty summary is required before publication.</p>
                </div>
                <div class="form-field">
                    <label for="translations_en_source_markdown">Markdown source</label>
                    <textarea class="wiki-code-field" id="translations_en_source_markdown" name="translations[en][source_markdown]" maxlength="100000">{{ old('translations.en.source_markdown', $english?->source_markdown) }}</textarea>
                    <p class="form-help">Raw HTML, unsafe URL schemes and remote images are rejected by the Wiki renderer.</p>
                </div>
            </fieldset>

            <fieldset class="card wiki-translation-panel">
                <legend><strong>Polish</strong> · required before publication</legend>
                <div class="form-field">
                    <label for="translations_pl_title">Title</label>
                    <input id="translations_pl_title" name="translations[pl][title]" type="text" maxlength="200"
                           value="{{ old('translations.pl.title', $polish?->title) }}">
                </div>
                <div class="form-field">
                    <label for="translations_pl_slug">Slug</label>
                    <input id="translations_pl_slug" name="translations[pl][slug]" type="text" maxlength="160"
                           pattern="[a-z0-9]+(-[a-z0-9]+)*"
                           value="{{ old('translations.pl.slug', $polish?->slug) }}">
                </div>
                <div class="form-field">
                    <label for="translations_pl_summary">Summary</label>
                    <textarea id="translations_pl_summary" name="translations[pl][summary]" rows="4" maxlength="1000">{{ old('translations.pl.summary', $polish?->summary) }}</textarea>
                </div>
                <div class="form-field">
                    <label for="translations_pl_source_markdown">Markdown source</label>
                    <textarea class="wiki-code-field" id="translations_pl_source_markdown" name="translations[pl][source_markdown]" maxlength="100000">{{ old('translations.pl.source_markdown', $polish?->source_markdown) }}</textarea>
                    <p class="form-help">Polish content must be refreshed after English source changes before it appears publicly.</p>
                </div>
            </fieldset>
        </div>

        <section class="card">
            <h2>Categories</h2>
            <p class="muted">The selected order controls the primary breadcrumb and related-article ranking.</p>
            <div class="wiki-category-options">
                @forelse ($categories as $category)
                    <label class="wiki-category-option">
                        <input name="category_ids[]" type="checkbox" value="{{ $category->id }}"
                               @checked(in_array((int) $category->id, $chosenCategoryIds, true))>
                        <span>
                            <strong>{{ $category->en_name ?? $category->key }}</strong>
                            @if ($category->pl_name)
                                <br><span class="muted">{{ $category->pl_name }}</span>
                            @endif
                            @if (! $category->visible)
                                <br><span class="badge badge-warning">Hidden</span>
                            @endif
                        </span>
                    </label>
                @empty
                    <p>No categories exist yet. The article may be saved without a category.</p>
                @endforelse
            </div>
        </section>

        <section class="card">
            <div class="form-field">
                <label for="change_note">Change note</label>
                <input id="change_note" name="change_note" type="text" maxlength="500" value="{{ old('change_note') }}">
                <p class="form-help">Describe the editorial reason without copying article bodies or sensitive information.</p>
            </div>
            <div class="action-row">
                <button type="submit">{{ $article === null ? 'Create draft' : 'Save draft' }}</button>
                <a class="button button-secondary" href="{{ route('admin.wiki.articles.index') }}">Cancel</a>
            </div>
        </section>
    </form>

    @if ($article !== null)
        <section class="card">
            <h2>Lifecycle</h2>
            <p class="muted">Lifecycle writes use the current optimistic-lock version. A concurrent change returns HTTP 409 instead of overwriting work.</p>
            <div class="wiki-status-actions">
                @if ($article->status->value === 'draft')
                    <form method="POST" action="{{ route('admin.wiki.articles.submit-review', $article) }}">
                        @csrf
                        <input type="hidden" name="lock_version" value="{{ $article->lock_version }}">
                        <button type="submit">Submit for review</button>
                    </form>
                @elseif ($article->status->value === 'in_review')
                    <form method="POST" action="{{ route('admin.wiki.articles.return-draft', $article) }}">
                        @csrf
                        <input type="hidden" name="lock_version" value="{{ $article->lock_version }}">
                        <button class="button-secondary" type="submit">Return to draft</button>
                    </form>
                    @if ($canPublish)
                        <form method="POST" action="{{ route('admin.wiki.articles.publish', $article) }}">
                            @csrf
                            <input type="hidden" name="lock_version" value="{{ $article->lock_version }}">
                            <button type="submit">Publish</button>
                        </form>
                    @endif
                @elseif ($article->status->value === 'published' && $canPublish)
                    <form method="POST" action="{{ route('admin.wiki.articles.unpublish', $article) }}">
                        @csrf
                        <input type="hidden" name="lock_version" value="{{ $article->lock_version }}">
                        <button type="submit">Unpublish to draft</button>
                    </form>
                @endif

                @if ($canPublish && $article->status->value !== 'archived')
                    <form method="POST" action="{{ route('admin.wiki.articles.archive', $article) }}">
                        @csrf
                        <input type="hidden" name="lock_version" value="{{ $article->lock_version }}">
                        <button class="button-secondary" type="submit">Archive</button>
                    </form>
                @endif
            </div>
        </section>
    @endif
@endsection
