@extends('admin.layout')

@section('title', 'Game Catalog snapshot #'.$snapshot->id)

@push('head')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <div class="page-header">
        <p class="eyebrow">Game Catalog · Snapshot provenance</p>
        <h1>Snapshot #{{ $snapshot->id }}</h1>
        <p class="muted">Immutable imported content and the current visibility projections that reference it.</p>
    </div>

    @include('game-catalog.admin._navigation')

    <section class="card" aria-labelledby="snapshot-metadata-heading">
        <h2 id="snapshot-metadata-heading">Metadata</h2>
        <dl class="catalog-detail-list">
            <div><dt>Status</dt><dd>{{ $snapshot->status }}</dd></div>
            <div><dt>Contract</dt><dd>{{ $snapshot->contract_version }} / {{ $snapshot->schema_version }}</dd></div>
            <div><dt>Content SHA-256</dt><dd><code>{{ $snapshot->content_sha256 }}</code></dd></div>
            <div><dt>Canary commit</dt><dd><code>{{ $snapshot->canary_commit_sha }}</code></dd></div>
            <div><dt>Datapack commit</dt><dd><code>{{ $snapshot->datapack_commit_sha ?? 'not declared' }}</code></dd></div>
            <div><dt>Protocol profile</dt><dd>{{ $snapshot->protocol_profile }}</dd></div>
            <div><dt>Runtime release</dt><dd>{{ $snapshot->runtime_release }}</dd></div>
            <div><dt>Content target</dt><dd>{{ $snapshot->content_target_release }}</dd></div>
            <div><dt>Verified through</dt><dd>{{ $snapshot->verified_content_through_release ?? 'unknown' }}</dd></div>
            <div><dt>Contains through</dt><dd>{{ $snapshot->contains_content_through_release ?? 'not declared' }}</dd></div>
            <div><dt>Counts</dt><dd>{{ $snapshot->entity_count }} entities / {{ $snapshot->relation_count }} relations</dd></div>
            <div><dt>Generated</dt><dd>{{ $snapshot->generated_at }}</dd></div>
            <div><dt>Imported</dt><dd>{{ $snapshot->imported_at }}</dd></div>
        </dl>
    </section>

    <section class="card" aria-labelledby="snapshot-typed-summary-heading">
        <h2 id="snapshot-typed-summary-heading">Typed candidate summary</h2>
        <dl class="catalog-detail-list">
            @foreach ($entityTypeCounts as $row)
                <div><dt>Entity {{ $row->entity_type }}</dt><dd>{{ $row->record_count }}</dd></div>
            @endforeach
            @foreach ($relationTypeCounts as $row)
                <div><dt>Relation {{ $row->relation_type }}</dt><dd>{{ $row->record_count }}</dd></div>
            @endforeach
            <div><dt>Unknown or unverified entities</dt><dd>{{ $unknownOrUnverifiedEntityCount }}</dd></div>
        </dl>
    </section>

    <section aria-labelledby="snapshot-visibility-heading">
        <h2 id="snapshot-visibility-heading">Profile visibility summary</h2>
        <div class="table-wrap">
            <table>
                <thead><tr><th>Profile</th><th>Visible</th><th>Reason</th><th>Records</th></tr></thead>
                <tbody>
                @forelse ($visibility as $row)
                    <tr>
                        <td><a href="{{ route('admin.game-catalog.profiles.show', $row->profile_id) }}">{{ $row->profile_name }}</a><br><code>{{ $row->profile_key }}</code></td>
                        <td>{{ $row->visible ? 'yes' : 'no' }}</td>
                        <td>{{ $row->reason_code }}</td>
                        <td>{{ $row->record_count }}</td>
                    </tr>
                @empty
                    <tr><td colspan="4">No profile projection references this snapshot.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>
    </section>

    <section aria-labelledby="snapshot-entities-heading">
        <h2 id="snapshot-entities-heading">Entities <span class="muted">(bounded to 200)</span></h2>
        <div class="table-wrap">
            <table>
                <thead><tr><th>Canonical key</th><th>Type</th><th>Introduced</th><th>Removed</th><th>Completeness</th><th>Availability</th><th>Runtime</th><th>Enabled</th></tr></thead>
                <tbody>
                @forelse ($entities as $entity)
                    <tr>
                        <td><code>{{ $entity->canonical_key }}</code></td>
                        <td>{{ $entity->entity_type }}</td>
                        <td>{{ $entity->introduced_release ?? 'unknown' }}</td>
                        <td>{{ $entity->removed_release ?? 'not removed' }}</td>
                        <td>{{ $entity->completeness }}</td>
                        <td>{{ $entity->availability }}</td>
                        <td>{{ $entity->runtime_present ? 'yes' : 'no' }}</td>
                        <td>{{ $entity->enabled ? 'yes' : 'no' }}</td>
                    </tr>
                @empty
                    <tr><td colspan="8">This snapshot contains no entities.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>
    </section>

    <section aria-labelledby="snapshot-relations-heading">
        <h2 id="snapshot-relations-heading">Relations <span class="muted">(bounded to 200)</span></h2>
        <div class="table-wrap">
            <table>
                <thead><tr><th>Canonical key</th><th>Type</th><th>Source</th><th>Target</th><th>Introduced</th><th>Removed</th><th>Completeness</th><th>Availability</th><th>Enabled</th></tr></thead>
                <tbody>
                @forelse ($relations as $relation)
                    <tr>
                        <td><code>{{ $relation->canonical_key }}</code></td>
                        <td>{{ $relation->relation_type }}</td>
                        <td><code>{{ $relation->source_key }}</code></td>
                        <td><code>{{ $relation->target_key ?? 'none' }}</code></td>
                        <td>{{ $relation->introduced_release ?? 'unknown' }}</td>
                        <td>{{ $relation->removed_release ?? 'not removed' }}</td>
                        <td>{{ $relation->completeness }}</td>
                        <td>{{ $relation->availability ?? 'not applicable' }}</td>
                        <td>{{ $relation->enabled ? 'yes' : 'no' }}</td>
                    </tr>
                @empty
                    <tr><td colspan="9">This snapshot contains no relations.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>
    </section>

    <section aria-labelledby="snapshot-findings-heading">
        <div class="section-heading">
            <h2 id="snapshot-findings-heading">Validation findings</h2>
            <a href="{{ route('admin.game-catalog.findings.index', ['snapshot_id' => $snapshot->id]) }}">Open filtered findings</a>
        </div>
        <div class="table-wrap">
            <table>
                <thead><tr><th>Severity</th><th>Code</th><th>Path</th><th>Message</th></tr></thead>
                <tbody>
                @forelse ($findings as $finding)
                    <tr><td>{{ $finding->severity }}</td><td><code>{{ $finding->code }}</code></td><td><code>{{ $finding->path ?? '—' }}</code></td><td>{{ $finding->message }}</td></tr>
                @empty
                    <tr><td colspan="4">No validation findings are attached to this snapshot.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>
    </section>

    <section aria-labelledby="snapshot-audit-heading">
        <h2 id="snapshot-audit-heading">Bounded audit history</h2>
        <div class="table-wrap">
            <table>
                <thead><tr><th>Action</th><th>Actor identity</th><th>Profile</th><th>Metadata</th><th>Created</th></tr></thead>
                <tbody>
                @forelse ($auditEvents as $event)
                    <tr>
                        <td><code>{{ $event->action }}</code></td>
                        <td>{{ $event->actor_identity_id ?? 'operator CLI' }}</td>
                        <td>{{ $event->profile_id === null ? 'none' : '#'.$event->profile_id }}</td>
                        <td><code>{{ $event->metadata }}</code></td>
                        <td>{{ $event->created_at }}</td>
                    </tr>
                @empty
                    <tr><td colspan="5">No activation or rollback audit events reference this snapshot.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>
    </section>
@endsection
