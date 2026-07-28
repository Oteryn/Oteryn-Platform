@extends('admin.layout')

@section('title', 'Game Catalog administration')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Versioned game data</p>
        <h1>Game Catalog</h1>
        <p class="muted">Inspect immutable imports, validation results, profile targets and active snapshots. Snapshot files are imported only through controlled CLI or deployment paths.</p>
    </div>

    @if (session('status'))<div class="status-banner" role="status">{{ session('status') }}</div>@endif
    @if ($errors->any())<div class="error-banner" role="alert">{{ $errors->first() }}</div>@endif

    <section aria-labelledby="catalog-profiles-heading">
        <h2 id="catalog-profiles-heading">Content profiles</h2>
        @if ($profiles->isEmpty())
            <div class="empty-state">No Game Catalog profiles exist.</div>
        @else
            <div class="table-region" tabindex="0" aria-label="Game Catalog profiles, horizontally scrollable on small screens">
                <table>
                    <thead><tr><th>Profile</th><th>Target release</th><th>Active snapshot</th><th>Complete only</th><th>Public</th></tr></thead>
                    <tbody>
                        @foreach ($profiles as $profile)
                            <tr>
                                <td><a href="{{ route('admin.game-catalog.profiles.show', $profile->id) }}">{{ $profile->name }}</a><br><code>{{ $profile->key }}</code></td>
                                <td>{{ $profile->target_release }}</td>
                                <td>@if($profile->active_snapshot_id)<a href="{{ route('admin.game-catalog.snapshots.show', $profile->active_snapshot_id) }}">#{{ $profile->active_snapshot_id }}</a>@else<span class="muted">Inactive</span>@endif</td>
                                <td>{{ $profile->complete_only ? 'Yes' : 'No' }}</td>
                                <td>{{ $profile->public_enabled ? 'Enabled' : 'Disabled' }}</td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </section>

    <section aria-labelledby="catalog-snapshots-heading">
        <h2 id="catalog-snapshots-heading">Immutable snapshots</h2>
        @if ($snapshots->isEmpty())
            <div class="empty-state">No snapshots have been imported.</div>
        @else
            <div class="table-region" tabindex="0" aria-label="Game Catalog snapshots, horizontally scrollable on small screens">
                <table>
                    <thead><tr><th>ID</th><th>Status</th><th>Content release</th><th>Generated</th><th>Imported</th><th>Counts</th><th>SHA-256</th></tr></thead>
                    <tbody>
                        @foreach ($snapshots as $snapshot)
                            <tr>
                                <td><a href="{{ route('admin.game-catalog.snapshots.show', $snapshot->id) }}">#{{ $snapshot->id }}</a></td>
                                <td>{{ $snapshot->status }}</td>
                                <td>{{ $snapshot->content_release }}</td>
                                <td>{{ $snapshot->generated_at }}</td>
                                <td>{{ $snapshot->imported_at }}</td>
                                <td>{{ $snapshot->entity_count }} entities / {{ $snapshot->relation_count }} relations</td>
                                <td><code>{{ $snapshot->content_sha256 }}</code></td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
            <div class="pagination">{{ $snapshots->links() }}</div>
        @endif
    </section>

    <section aria-labelledby="catalog-imports-heading">
        <h2 id="catalog-imports-heading">Recent import runs</h2>
        @if ($importRuns->isEmpty())
            <div class="empty-state">No import runs have been recorded.</div>
        @else
            <div class="table-region" tabindex="0" aria-label="Game Catalog import runs, horizontally scrollable on small screens">
                <table>
                    <thead><tr><th>ID</th><th>Source name</th><th>Status</th><th>Snapshot</th><th>Findings</th><th>Started</th><th>Finished</th></tr></thead>
                    <tbody>
                        @foreach ($importRuns as $run)
                            <tr>
                                <td>#{{ $run->id }}</td>
                                <td>{{ $run->source_name }}</td>
                                <td>{{ $run->status }}</td>
                                <td>{{ $run->snapshot_id ?? '—' }}</td>
                                <td>{{ $run->finding_count }}</td>
                                <td>{{ $run->started_at }}</td>
                                <td>{{ $run->finished_at ?? '—' }}</td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </section>
@endsection
