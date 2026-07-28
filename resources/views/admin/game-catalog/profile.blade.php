@extends('admin.layout')

@section('title', 'Game Catalog profile '.$profileRow->key)

@section('content')
    <div class="page-header">
        <p class="eyebrow"><a href="{{ route('admin.game-catalog.index') }}">Game Catalog</a></p>
        <h1>{{ $profileRow->name }}</h1>
        <p class="muted"><code>{{ $profileRow->key }}</code> · active snapshot {{ $profileRow->active_snapshot_id ?? 'none' }}</p>
    </div>

    @if (session('status'))<div class="status-banner" role="status">{{ session('status') }}</div>@endif
    @if ($errors->any())<div class="error-banner" role="alert">{{ $errors->first() }}</div>@endif

    <div class="form-grid">
        <section aria-labelledby="profile-policy-heading">
            <h2 id="profile-policy-heading">Profile policy</h2>
            <form method="POST" action="{{ route('admin.game-catalog.profiles.update', $profileRow->id) }}">
                @csrf
                @method('PUT')
                <label>Target release
                    <select name="target_release" required>
                        @foreach ($releases as $release)
                            <option value="{{ $release->key }}" @selected(old('target_release', $profileRow->target_release_key) === $release->key)>{{ $release->display_label }} ({{ $release->key }})</option>
                        @endforeach
                    </select>
                </label>
                <label><input type="checkbox" name="complete_only" value="1" @checked(old('complete_only', $profileRow->complete_only))> Complete entities and relations only</label>
                <label><input type="checkbox" name="public_enabled" value="1" @checked(old('public_enabled', $profileRow->public_enabled))> Public profile enabled</label>
                <button type="submit">Update profile and recompute visibility</button>
            </form>
        </section>

        <section aria-labelledby="profile-activation-heading">
            <h2 id="profile-activation-heading">Activation / rollback</h2>
            <p class="muted">The selected validated snapshot is checked against this profile and switched transactionally.</p>
            <form method="POST" action="{{ route('admin.game-catalog.profiles.activate', $profileRow->id) }}">
                @csrf
                <label>Snapshot
                    <select name="snapshot_id" required>
                        @foreach ($snapshots as $snapshot)
                            <option value="{{ $snapshot->id }}">#{{ $snapshot->id }} · {{ $snapshot->content_release }} · {{ $snapshot->generated_at }} · {{ substr($snapshot->content_sha256, 0, 12) }}</option>
                        @endforeach
                    </select>
                </label>
                <label>Action
                    <select name="action" required>
                        <option value="activate">Activate</option>
                        <option value="rollback">Rollback to selected snapshot</option>
                    </select>
                </label>
                <label>Audit reason
                    <input type="text" name="reason" maxlength="500" value="{{ old('reason') }}">
                </label>
                <button type="submit">Apply transactionally</button>
            </form>
        </section>
    </div>

    <section aria-labelledby="entity-reasons-heading">
        <h2 id="entity-reasons-heading">Entity visibility reasons</h2>
        @if ($entityReasons->isEmpty())
            <div class="empty-state">No entity projection exists for this profile.</div>
        @else
            <div class="table-region" tabindex="0" aria-label="Entity visibility reasons, horizontally scrollable on small screens">
                <table><thead><tr><th>Visible</th><th>Reason</th><th>Count</th></tr></thead><tbody>
                    @foreach ($entityReasons as $reason)<tr><td>{{ $reason->visible ? 'Yes' : 'No' }}</td><td><code>{{ $reason->reason_code }}</code></td><td>{{ $reason->total }}</td></tr>@endforeach
                </tbody></table>
            </div>
        @endif
    </section>

    <section aria-labelledby="relation-reasons-heading">
        <h2 id="relation-reasons-heading">Relation visibility reasons</h2>
        @if ($relationReasons->isEmpty())
            <div class="empty-state">No relation projection exists for this profile.</div>
        @else
            <div class="table-region" tabindex="0" aria-label="Relation visibility reasons, horizontally scrollable on small screens">
                <table><thead><tr><th>Visible</th><th>Reason</th><th>Count</th></tr></thead><tbody>
                    @foreach ($relationReasons as $reason)<tr><td>{{ $reason->visible ? 'Yes' : 'No' }}</td><td><code>{{ $reason->reason_code }}</code></td><td>{{ $reason->total }}</td></tr>@endforeach
                </tbody></table>
            </div>
        @endif
    </section>

    <section aria-labelledby="hidden-entities-heading">
        <h2 id="hidden-entities-heading">Hidden entity sample</h2>
        @if ($hiddenEntities->isEmpty())
            <div class="empty-state">No hidden entities are recorded.</div>
        @else
            <div class="table-region" tabindex="0" aria-label="Hidden entities, horizontally scrollable on small screens">
                <table><thead><tr><th>Type</th><th>Canonical key</th><th>Reason</th></tr></thead><tbody>
                    @foreach ($hiddenEntities as $entity)<tr><td>{{ $entity->entity_type }}</td><td><code>{{ $entity->canonical_key }}</code></td><td><code>{{ $entity->reason_code }}</code></td></tr>@endforeach
                </tbody></table>
            </div>
        @endif
    </section>

    <section aria-labelledby="activation-history-heading">
        <h2 id="activation-history-heading">Activation history</h2>
        @if ($history->isEmpty())
            <div class="empty-state">No activation history is recorded.</div>
        @else
            <div class="table-region" tabindex="0" aria-label="Activation history, horizontally scrollable on small screens">
                <table><thead><tr><th>Occurred</th><th>Action</th><th>From</th><th>To</th><th>Actor</th><th>Reason</th></tr></thead><tbody>
                    @foreach ($history as $event)<tr><td>{{ $event->occurred_at }}</td><td>{{ $event->action }}</td><td>{{ $event->from_snapshot_id ?? 'none' }}</td><td>{{ $event->to_snapshot_id }}</td><td>{{ $event->actor_identity_id ?? 'system' }}</td><td>{{ $event->reason ?? '—' }}</td></tr>@endforeach
                </tbody></table>
            </div>
        @endif
    </section>
@endsection
