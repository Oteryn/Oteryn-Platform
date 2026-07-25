@extends('admin.layout')

@section('title', $key->label())

@section('content')
    <div class="page-header">
        <p class="eyebrow">Content · Typed editorial routes</p>
        <h1>{{ $key->label() }}</h1>
        <p class="muted">
            Stable route: {{ route($key->publicRouteName(), absolute: false) }}
            · Fixed CMS slug: {{ $key->managedPageSlug() }}
        </p>
    </div>

    <div class="card">
        <p><strong>Expected topics:</strong> {{ implode(', ', $key->expectedTopics()) }}</p>

        <form class="form-stack" method="POST" action="{{ route('admin.support-content.update', ['editorialPageKey' => $key->value]) }}">
            @csrf
            @method('PUT')

            <div class="form-field">
                <label for="title">Title</label>
                <input id="title" name="title" type="text" maxlength="200" required value="{{ old('title', $page?->title ?? $key->label()) }}">
            </div>

            <div class="form-field">
                <label for="body">Body (plain text)</label>
                <textarea id="body" name="body" rows="24" maxlength="100000" required>{{ old('body', $page?->body) }}</textarea>
                <p class="form-help">HTML and media uploads are not supported. Browser output remains escaped.</p>
            </div>

            @if ($key->isLegal())
                <div class="form-field">
                    <label for="legal_version">Legal version</label>
                    <input id="legal_version" name="legal_version" type="text" maxlength="40" value="{{ old('legal_version', $page?->legal_version) }}">
                    <p class="form-help">Required when published. A published version is immutable; change the version before changing its legal meaning.</p>
                </div>

                <div class="form-field">
                    <label for="legal_effective_date">Effective date</label>
                    <input id="legal_effective_date" name="legal_effective_date" type="date" value="{{ old('legal_effective_date', $page?->legal_effective_date?->format('Y-m-d')) }}">
                </div>
            @endif

            <div class="form-field">
                <label for="published_at">Publish at</label>
                <input id="published_at" name="published_at" type="datetime-local" value="{{ old('published_at', $page?->published_at?->format('Y-m-d\TH:i')) }}">
                <p class="form-help">Leave empty for draft. Future dates remain publicly unavailable until reached.</p>
            </div>

            <div class="action-row">
                <button type="submit">Save editorial page</button>
                <a class="button button-secondary" href="{{ route('admin.support-content.index') }}">Cancel</a>
            </div>
        </form>
    </div>

    @if ($key->isLegal() && $legalVersions->isNotEmpty())
        <section>
            <h2>Preserved published versions</h2>
            <div class="table-region" tabindex="0" aria-label="Preserved legal versions table, horizontally scrollable on small screens">
                <table>
                    <thead>
                        <tr>
                            <th scope="col">Version</th>
                            <th scope="col">Effective date</th>
                            <th scope="col">First published</th>
                        </tr>
                    </thead>
                    <tbody>
                        @foreach ($legalVersions as $version)
                            <tr>
                                <td>{{ $version->version }}</td>
                                <td>{{ $version->effective_date->format('Y-m-d') }}</td>
                                <td>{{ $version->published_at->format('Y-m-d H:i') }}</td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        </section>
    @endif
@endsection
