@extends('admin.layout')

@section('title', 'Game Catalog findings')

@push('head')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <div class="page-header">
        <p class="eyebrow">Game Catalog · Validation diagnostics</p>
        <h1>Findings</h1>
        <p class="muted">Messages are bounded during import and shown only to administrators with snapshot inspection permission.</p>
    </div>

    @include('game-catalog.admin._navigation')

    <form class="catalog-filters" method="GET" action="{{ route('admin.game-catalog.findings.index') }}">
        <label class="catalog-field" for="finding-severity">
            <span>Severity</span>
            <select id="finding-severity" name="severity">
                <option value="">All severities</option>
                @foreach (['error', 'warning', 'info'] as $severity)
                    <option value="{{ $severity }}" @selected($selectedSeverity === $severity)>{{ $severity }}</option>
                @endforeach
            </select>
        </label>
        <label class="catalog-field" for="finding-snapshot">
            <span>Snapshot</span>
            <select id="finding-snapshot" name="snapshot_id">
                <option value="">All snapshots</option>
                @foreach ($snapshots as $snapshot)
                    <option value="{{ $snapshot->id }}" @selected($selectedSnapshotId === $snapshot->id)>#{{ $snapshot->id }} · {{ \Illuminate\Support\Str::limit($snapshot->content_sha256, 16, '…') }}</option>
                @endforeach
            </select>
        </label>
        <div><button type="submit">Filter findings</button></div>
    </form>

    <div class="table-wrap">
        <table>
            <thead><tr><th>ID</th><th>Snapshot</th><th>Severity</th><th>Code</th><th>Path</th><th>Message</th><th>Created</th></tr></thead>
            <tbody>
            @forelse ($findings as $finding)
                <tr>
                    <td>#{{ $finding->id }}</td>
                    <td>
                        @if ($finding->snapshot_id === null)
                            none
                        @else
                            <a href="{{ route('admin.game-catalog.snapshots.show', $finding->snapshot_id) }}">#{{ $finding->snapshot_id }}</a>
                        @endif
                    </td>
                    <td>{{ $finding->severity }}</td>
                    <td><code>{{ $finding->code }}</code></td>
                    <td><code>{{ $finding->path ?? '—' }}</code></td>
                    <td>{{ $finding->message }}</td>
                    <td>{{ $finding->created_at }}</td>
                </tr>
            @empty
                <tr><td colspan="7">No validation findings match these filters.</td></tr>
            @endforelse
            </tbody>
        </table>
    </div>
@endsection
