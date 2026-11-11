@extends('admin.layout')

@section('title', 'Game Catalog snapshot diff')

@push('head')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <div class="page-header">
        <p class="eyebrow">Game Catalog · Immutable comparison</p>
        <h1>Snapshot diff</h1>
        <p class="muted">Compare stable canonical keys and persisted fingerprints without exposing source payloads.</p>
    </div>

    @include('game-catalog.admin._navigation')

    <form class="catalog-filters" method="GET" action="{{ route('admin.game-catalog.diff.index') }}">
        <label class="catalog-field" for="snapshot-a">
            <span>Snapshot A</span>
            <select id="snapshot-a" name="snapshot_a" required>
                <option value="">Select snapshot</option>
                @foreach ($snapshots as $snapshot)
                    <option value="{{ $snapshot->id }}" @selected($selectedSnapshotA === $snapshot->id)>#{{ $snapshot->id }} · {{ $snapshot->status }} · {{ \Illuminate\Support\Str::limit($snapshot->content_sha256, 16, '…') }}</option>
                @endforeach
            </select>
        </label>
        <label class="catalog-field" for="snapshot-b">
            <span>Snapshot B</span>
            <select id="snapshot-b" name="snapshot_b" required>
                <option value="">Select snapshot</option>
                @foreach ($snapshots as $snapshot)
                    <option value="{{ $snapshot->id }}" @selected($selectedSnapshotB === $snapshot->id)>#{{ $snapshot->id }} · {{ $snapshot->status }} · {{ \Illuminate\Support\Str::limit($snapshot->content_sha256, 16, '…') }}</option>
                @endforeach
            </select>
        </label>
        <div><button type="submit">Compare snapshots</button></div>
    </form>

    @if ($diffError !== null)
        <div class="alert alert-danger" role="alert">{{ $diffError }}</div>
    @endif

    @if ($diff !== null)
        <section class="catalog-admin-summary" aria-label="Snapshot diff totals">
            <article class="card"><p class="eyebrow">Added entities</p><p class="catalog-admin-total">{{ count($diff->addedEntities) }}</p></article>
            <article class="card"><p class="eyebrow">Removed entities</p><p class="catalog-admin-total">{{ count($diff->removedEntities) }}</p></article>
            <article class="card"><p class="eyebrow">Changed entities</p><p class="catalog-admin-total">{{ count($diff->changedEntities) }}</p></article>
            <article class="card"><p class="eyebrow">Added relations</p><p class="catalog-admin-total">{{ count($diff->addedRelations) }}</p></article>
            <article class="card"><p class="eyebrow">Removed relations</p><p class="catalog-admin-total">{{ count($diff->removedRelations) }}</p></article>
            <article class="card"><p class="eyebrow">Changed relations</p><p class="catalog-admin-total">{{ count($diff->changedRelations) }}</p></article>
        </section>

        @foreach ([
            'Added entities' => $diff->addedEntities,
            'Removed entities' => $diff->removedEntities,
            'Changed entities' => $diff->changedEntities,
            'Added relations' => $diff->addedRelations,
            'Removed relations' => $diff->removedRelations,
            'Changed relations' => $diff->changedRelations,
        ] as $heading => $keys)
            <section aria-labelledby="diff-{{ \Illuminate\Support\Str::slug($heading) }}">
                <h2 id="diff-{{ \Illuminate\Support\Str::slug($heading) }}">{{ $heading }}</h2>
                @if ($keys === [])
                    <div class="empty-state"><p>No records.</p></div>
                @else
                    <ul class="catalog-relation-list">
                        @foreach ($keys as $key)
                            <li><code>{{ $key }}</code></li>
                        @endforeach
                    </ul>
                @endif
            </section>
        @endforeach
    @endif
@endsection
