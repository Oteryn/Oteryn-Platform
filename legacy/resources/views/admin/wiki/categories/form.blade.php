@extends('admin.layout')

@section('title', $category === null ? 'Create Wiki category' : 'Edit Wiki category')

@section('content')
    @php($english = $translations->get('en'))
    @php($polish = $translations->get('pl'))

    <div class="page-header">
        <p class="eyebrow">Wiki · Categories</p>
        <h1>{{ $category === null ? 'Create Wiki category' : ($english?->name ?? $polish?->name ?? 'Edit Wiki category') }}</h1>
        @if ($category !== null)
            <p class="muted">Stable key {{ $category->key }} · version {{ $category->lock_version }}</p>
        @else
            <p class="muted">English is required. Polish may be added before publishing articles inside the category.</p>
        @endif
    </div>

    <div class="action-row">
        <a class="button button-secondary" href="{{ route('admin.wiki.categories.index') }}">Back to categories</a>
    </div>

    <form class="form-stack" method="POST" action="{{ $category === null ? route('admin.wiki.categories.store') : route('admin.wiki.categories.update', $category) }}">
        @csrf
        @if ($category !== null)
            @method('PUT')
            <input type="hidden" name="lock_version" value="{{ $category->lock_version }}">
        @endif

        <section class="card">
            <h2>Category settings</h2>
            <div class="wiki-form-grid">
                <div class="form-field">
                    <label for="key">Stable key</label>
                    <input id="key" name="key" type="text" maxlength="96" required
                           pattern="[a-z0-9]+([._\-][a-z0-9]+)*"
                           value="{{ old('key', $category?->key) }}">
                </div>
                <div class="form-field">
                    <label for="parent_id">Parent category</label>
                    <select id="parent_id" name="parent_id">
                        <option value="">Root category</option>
                        @foreach ($parentOptions as $parent)
                            <option value="{{ $parent->id }}" @selected((string) old('parent_id', $category?->parent_id) === (string) $parent->id)>
                                {{ $parent->en_name ?? $parent->key }}
                            </option>
                        @endforeach
                    </select>
                </div>
                <div class="form-field">
                    <label for="sort_order">Display order</label>
                    <input id="sort_order" name="sort_order" type="number" min="0" max="1000000" required
                           value="{{ old('sort_order', $category?->sort_order ?? 0) }}">
                </div>
                <label class="wiki-category-option" for="visible">
                    <input id="visible" name="visible" type="checkbox" value="1"
                           @checked(old('visible', $category?->visible ?? true))>
                    <span><strong>Visible publicly</strong><br><span class="muted">Hidden categories and their navigation entries are excluded from public reads.</span></span>
                </label>
            </div>
        </section>

        <div class="wiki-translation-grid">
            <fieldset class="card wiki-translation-panel">
                <legend><strong>English</strong> · required</legend>
                <div class="form-field">
                    <label for="translations_en_name">Name</label>
                    <input id="translations_en_name" name="translations[en][name]" type="text" maxlength="200" required
                           value="{{ old('translations.en.name', $english?->name) }}">
                </div>
                <div class="form-field">
                    <label for="translations_en_slug">Slug</label>
                    <input id="translations_en_slug" name="translations[en][slug]" type="text" maxlength="160" required
                           pattern="[a-z0-9]+(-[a-z0-9]+)*"
                           value="{{ old('translations.en.slug', $english?->slug) }}">
                </div>
                <div class="form-field">
                    <label for="translations_en_description">Description</label>
                    <textarea id="translations_en_description" name="translations[en][description]" rows="8" maxlength="10000">{{ old('translations.en.description', $english?->description) }}</textarea>
                </div>
            </fieldset>

            <fieldset class="card wiki-translation-panel">
                <legend><strong>Polish</strong> · optional</legend>
                <div class="form-field">
                    <label for="translations_pl_name">Name</label>
                    <input id="translations_pl_name" name="translations[pl][name]" type="text" maxlength="200"
                           value="{{ old('translations.pl.name', $polish?->name) }}">
                </div>
                <div class="form-field">
                    <label for="translations_pl_slug">Slug</label>
                    <input id="translations_pl_slug" name="translations[pl][slug]" type="text" maxlength="160"
                           pattern="[a-z0-9]+(-[a-z0-9]+)*"
                           value="{{ old('translations.pl.slug', $polish?->slug) }}">
                </div>
                <div class="form-field">
                    <label for="translations_pl_description">Description</label>
                    <textarea id="translations_pl_description" name="translations[pl][description]" rows="8" maxlength="10000">{{ old('translations.pl.description', $polish?->description) }}</textarea>
                </div>
            </fieldset>
        </div>

        <section class="card">
            <div class="action-row">
                <button type="submit">{{ $category === null ? 'Create category' : 'Save category' }}</button>
                <a class="button button-secondary" href="{{ route('admin.wiki.categories.index') }}">Cancel</a>
            </div>
        </section>
    </form>
@endsection
