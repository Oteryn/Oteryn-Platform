@extends('admin.layout')

@section('title', 'Game Catalog snapshot #'.$snapshot->id)

@section('content')
    <div class="page-header">
        <p class="eyebrow"><a href="{{ route('admin.game-catalog.index') }}">Game Catalog</a></p>
        <h1>Snapshot #{{ $snapshot->id }}</h1>
        <p class="muted">Immutable imported state and bounded validation evidence.</p>
    </div>

    <dl class="definition-grid">
        <div><dt>Status</dt><dd>{{ $snapshot->status }}</dd></div>
        <div><dt>Schema</dt><dd>{{ $snapshot->contract_id }} / {{ $snapshot->schema_version }}</dd></div>
        <div><dt>Runtime release</dt><dd>{{ $snapshot->runtime_release }}</dd></div>
        <div><dt>Content target</dt><dd>{{ $snapshot->content_release }}</dd></div>
        <div><dt>Verified through</dt><dd>{{ $snapshot->verified_release }}</dd></div>
        <div><dt>Generated</dt><dd>{{ $snapshot->generated_at }}</dd></div>
        <div><dt>Imported</dt><dd>{{ $snapshot->imported_at }}</dd></div>
        <div><dt>Canary commit</dt><dd><code>{{ $snapshot->canary_commit_sha }}</code></dd></div>
        <div><dt>Datapack commit</dt><dd><code>{{ $snapshot->datapack_commit_sha ?? 'UNKNOWN' }}</code></dd></div>
        <div><dt>Appearances SHA-256</dt><dd><code>{{ $snapshot->appearances_sha256 }}</code></dd></div>
        <div><dt>Map SHA-256</dt><dd><code>{{ $snapshot->map_sha256 ?? 'UNKNOWN' }}</code></dd></div>
        <div><dt>Content SHA-256</dt><dd><code>{{ $snapshot->content_sha256 }}</code></dd></div>
        <div><dt>Counts</dt><dd>{{ $snapshot->entity_count }} entities / {{ $snapshot->relation_count }} relations</dd></div>
    </dl>

    <section aria-labelledby="snapshot-visibility-heading">
        <h2 id="snapshot-visibility-heading">Profile visibility findings</h2>
        @if ($visibility->isEmpty())
            <div class="empty-state">This snapshot has no materialized profile visibility.</div>
        @else
            <div class="table-region" tabindex="0" aria-label="Snapshot visibility findings, horizontally scrollable on small screens">
                <table>
                    <thead><tr><th>Profile</th><th>Visible</th><th>Reason</th><th>Count</th></tr></thead>
                    <tbody>
                        @foreach ($visibility as $row)
                            <tr><td>{{ $row->profile_key }}</td><td>{{ $row->visible ? 'Yes' : 'No' }}</td><td><code>{{ $row->reason_code }}</code></td><td>{{ $row->total }}</td></tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </section>

    <section aria-labelledby="snapshot-findings-heading">
        <h2 id="snapshot-findings-heading">Validation findings</h2>
        @if ($findings->isEmpty())
            <div class="empty-state">No validation findings are recorded for successful imports of this snapshot.</div>
        @else
            <div class="table-region" tabindex="0" aria-label="Snapshot validation findings, horizontally scrollable on small screens">
                <table>
                    <thead><tr><th>Run</th><th>Severity</th><th>Code</th><th>Path</th><th>Message</th></tr></thead>
                    <tbody>
                        @foreach ($findings as $finding)
                            <tr>
                                <td>#{{ $finding->import_run_id }}</td>
                                <td>{{ $finding->severity }}</td>
                                <td><code>{{ $finding->code }}</code></td>
                                <td><code>{{ $finding->path }}</code></td>
                                <td>{{ $finding->message }}</td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </section>
@endsection
