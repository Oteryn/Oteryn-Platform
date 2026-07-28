@extends('admin.layout')

@section('title', 'Game Catalog')

@push('head')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <div class="page-header">
        <p class="eyebrow">Operations · Verified server content</p>
        <h1>Game Catalog</h1>
        <p class="muted">Inspect immutable snapshots, publication profiles, visibility projections and validation findings. Snapshot import and activation remain operator CLI actions in this slice.</p>
    </div>

    @include('game-catalog.admin._navigation')

    <section class="catalog-admin-summary" aria-label="Game Catalog totals">
        <article class="card"><p class="eyebrow">Snapshots</p><p class="catalog-admin-total">{{ $snapshotCount }}</p></article>
        <article class="card"><p class="eyebrow">Profiles</p><p class="catalog-admin-total">{{ $profileCount }}</p></article>
        <article class="card"><p class="eyebrow">Findings</p><p class="catalog-admin-total">{{ $findingCount }}</p></article>
    </section>

    <section aria-labelledby="catalog-snapshots-heading">
        <div class="section-heading">
            <h2 id="catalog-snapshots-heading">Recent snapshots</h2>
            <a href="{{ route('admin.game-catalog.snapshots.index') }}">View all snapshots</a>
        </div>
        <div class="table-wrap">
            <table>
                <thead><tr><th>ID</th><th>Status</th><th>Content SHA-256</th><th>Target</th><th>Entities</th><th>Relations</th></tr></thead>
                <tbody>
                @forelse ($snapshots as $snapshot)
                    <tr>
                        <td><a href="{{ route('admin.game-catalog.snapshots.show', $snapshot->id) }}">#{{ $snapshot->id }}</a></td>
                        <td>{{ $snapshot->status }}</td>
                        <td><code>{{ \Illuminate\Support\Str::limit($snapshot->content_sha256, 20, '…') }}</code></td>
                        <td>{{ $snapshot->content_target_release }}</td>
                        <td>{{ $snapshot->entity_count }}</td>
                        <td>{{ $snapshot->relation_count }}</td>
                    </tr>
                @empty
                    <tr><td colspan="6">No Game Catalog snapshots have been imported.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>
    </section>

    <section aria-labelledby="catalog-profiles-heading">
        <div class="section-heading">
            <h2 id="catalog-profiles-heading">Publication profiles</h2>
            <a href="{{ route('admin.game-catalog.profiles.index') }}">View all profiles</a>
        </div>
        <div class="table-wrap">
            <table>
                <thead><tr><th>Profile</th><th>Target</th><th>Public</th><th>Active snapshot</th><th>Lock version</th></tr></thead>
                <tbody>
                @forelse ($profiles as $profile)
                    <tr>
                        <td><a href="{{ route('admin.game-catalog.profiles.show', $profile->id) }}">{{ $profile->name }}</a><br><code>{{ $profile->key }}</code></td>
                        <td>{{ $profile->target_release }}</td>
                        <td>{{ $profile->public_enabled ? 'yes' : 'no' }}</td>
                        <td>{{ $profile->active_snapshot_id === null ? 'not active' : '#'.$profile->active_snapshot_id }}</td>
                        <td>{{ $profile->lock_version }}</td>
                    </tr>
                @empty
                    <tr><td colspan="5">No Game Catalog profiles exist.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>
    </section>
@endsection
