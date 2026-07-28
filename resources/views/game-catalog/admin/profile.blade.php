@extends('admin.layout')

@section('title', 'Game Catalog profile '.$profile->name)

@push('head')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <div class="page-header">
        <p class="eyebrow">Game Catalog · Active visibility projection</p>
        <h1>{{ $profile->name }}</h1>
        <p class="muted"><code>{{ $profile->key }}</code> · lock version {{ $profile->lock_version }}</p>
    </div>

    @include('game-catalog.admin._navigation')

    <section class="card" aria-labelledby="profile-policy-heading">
        <h2 id="profile-policy-heading">Profile policy</h2>
        <dl class="catalog-detail-list">
            <div><dt>Target release</dt><dd>{{ $profile->target_release }}</dd></div>
            <div><dt>Protocol profile</dt><dd>{{ $profile->protocol_profile ?? 'any compatible' }}</dd></div>
            <div><dt>Completeness</dt><dd>{{ $profile->completeness_policy_key }}</dd></div>
            <div><dt>Availability</dt><dd>{{ $profile->availability_policy_key }}</dd></div>
            <div><dt>Validation</dt><dd>{{ $profile->validation_policy_key }}</dd></div>
            <div><dt>Public enabled</dt><dd>{{ $profile->public_enabled ? 'yes' : 'no' }}</dd></div>
            <div><dt>Backports</dt><dd>{{ $profile->allow_backports ? 'allowed' : 'disabled' }}</dd></div>
            <div><dt>Active snapshot</dt><dd>
                @if ($profile->active_snapshot_id === null)
                    not active
                @else
                    <a href="{{ route('admin.game-catalog.snapshots.show', $profile->active_snapshot_id) }}">#{{ $profile->active_snapshot_id }}</a>
                    · <code>{{ \Illuminate\Support\Str::limit($profile->active_snapshot_sha256, 24, '…') }}</code>
                    · {{ $profile->active_snapshot_status }}
                @endif
            </dd></div>
        </dl>
    </section>

    <section aria-labelledby="profile-entities-heading">
        <h2 id="profile-entities-heading">Entity visibility <span class="muted">(bounded to 200)</span></h2>
        <div class="table-wrap">
            <table>
                <thead><tr><th>Canonical key</th><th>Type</th><th>Introduced</th><th>Removed</th><th>Completeness</th><th>Availability</th><th>Visible</th><th>Reason</th></tr></thead>
                <tbody>
                @forelse ($entityVisibility as $row)
                    <tr>
                        <td><code>{{ $row->canonical_key }}</code></td>
                        <td>{{ $row->entity_type }}</td>
                        <td>{{ $row->introduced_release ?? 'unknown' }}</td>
                        <td>{{ $row->removed_release ?? 'not removed' }}</td>
                        <td>{{ $row->completeness }}</td>
                        <td>{{ $row->availability }}</td>
                        <td>{{ $row->visible ? 'yes' : 'no' }}</td>
                        <td>{{ $row->reason_code }}</td>
                    </tr>
                @empty
                    <tr><td colspan="8">This profile has no entity projection.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>
    </section>

    <section aria-labelledby="profile-relations-heading">
        <h2 id="profile-relations-heading">Relation visibility <span class="muted">(bounded to 200)</span></h2>
        <div class="table-wrap">
            <table>
                <thead><tr><th>Canonical key</th><th>Type</th><th>Completeness</th><th>Visible</th><th>Reason</th><th>Computed</th></tr></thead>
                <tbody>
                @forelse ($relationVisibility as $row)
                    <tr>
                        <td><code>{{ $row->canonical_key }}</code></td>
                        <td>{{ $row->relation_type }}</td>
                        <td>{{ $row->completeness }}</td>
                        <td>{{ $row->visible ? 'yes' : 'no' }}</td>
                        <td>{{ $row->reason_code }}</td>
                        <td>{{ $row->computed_at }}</td>
                    </tr>
                @empty
                    <tr><td colspan="6">This profile has no relation projection.</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>
    </section>
@endsection
