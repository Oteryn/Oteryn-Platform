@extends('admin.layout')

@section('title', 'Game Catalog snapshots')

@push('head')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <div class="page-header">
        <p class="eyebrow">Game Catalog · Immutable imports</p>
        <h1>Snapshots</h1>
        <p class="muted">The table is read-only. Provenance and release boundaries are retained for every imported snapshot.</p>
    </div>

    @include('game-catalog.admin._navigation')

    <div class="table-wrap">
        <table>
            <thead><tr><th>ID</th><th>Status</th><th>SHA-256</th><th>Runtime</th><th>Content target</th><th>Verified through</th><th>Counts</th><th>Imported</th></tr></thead>
            <tbody>
            @forelse ($snapshots as $snapshot)
                <tr>
                    <td><a href="{{ route('admin.game-catalog.snapshots.show', $snapshot->id) }}">#{{ $snapshot->id }}</a></td>
                    <td>{{ $snapshot->status }}</td>
                    <td><code>{{ \Illuminate\Support\Str::limit($snapshot->content_sha256, 24, '…') }}</code></td>
                    <td>{{ $snapshot->runtime_release }}</td>
                    <td>{{ $snapshot->content_target_release }}</td>
                    <td>{{ $snapshot->verified_content_through_release }}</td>
                    <td>{{ $snapshot->entity_count }} entities / {{ $snapshot->relation_count }} relations</td>
                    <td>{{ $snapshot->imported_at }}</td>
                </tr>
            @empty
                <tr><td colspan="8">No Game Catalog snapshots have been imported.</td></tr>
            @endforelse
            </tbody>
        </table>
    </div>
@endsection
