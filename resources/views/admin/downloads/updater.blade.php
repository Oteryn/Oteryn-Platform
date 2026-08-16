@extends('admin.layout')

@section('title', ucfirst($channel).' Updater Diagnostics')

@section('content')
    @php
        $latestPolicy = $policies->first();
        $activeGeneration = $generations->first(static fn ($generation): bool => $generation->activated_at !== null && $generation->superseded_at === null);
        $defaultRelease = $releases->whereNull('updater_withdrawn_at')->sortByDesc('updater_sequence')->first();
        $defaultMinimum = $releases->min('updater_sequence');
    @endphp

    <div class="page-header">
        <p class="eyebrow">Content · Downloads · Updater</p>
        <h1>{{ \App\Downloads\DownloadCatalog::channelLabel($channel) }} updater diagnostics</h1>
        <p class="muted">Browser publication, approved updater policy, signed-repository reconciliation and Platform-active updater state are separate lifecycle facts.</p>
    </div>

    <div class="alert alert-warning" role="status">
        <strong>No private updater signing key is accepted or stored by this Platform.</strong>
        <p>Imports below are public projections of metadata already produced by the protected TUF boundary. Platform reconciliation checks identity, ordering, expiry, channel, policy and exact target consistency; the first-party updater still performs cryptographic TUF verification. Platform-active does not mean deployed or production-active.</p>
    </div>

    <div class="action-row">
        <a class="button button-secondary" href="{{ route('admin.downloads.index') }}">Back to releases</a>
        <a class="button button-secondary" href="{{ route('downloads.index') }}">Public Download Center</a>
    </div>

    <div class="card">
        <h2>Current Platform updater state</h2>
        @if ($activeGeneration)
            <dl>
                <dt>Generation</dt><dd><code>{{ $activeGeneration->generation_id }}</code></dd>
                <dt>Policy revision</dt><dd>{{ $activeGeneration->policy->revision }}</dd>
                <dt>Update mode</dt><dd>{{ \App\Downloads\DownloadCatalog::updateModeLabel($activeGeneration->policy->update_mode) }}</dd>
                <dt>Minimum sequence</dt><dd>{{ $activeGeneration->policy->minimum_supported_release_sequence }}</dd>
                <dt>Timestamp metadata version</dt><dd>{{ $activeGeneration->timestamp_version }}</dd>
                <dt>Metadata expires</dt><dd>{{ $activeGeneration->metadata_expires_at->format('Y-m-d H:i:s') }} UTC</dd>
                <dt>State</dt><dd><span class="badge badge-success">Platform-active</span></dd>
            </dl>
        @else
            <div class="empty-state">
                <strong>No Platform-active updater generation.</strong>
                <p>Browser downloads can remain published independently. Approve policy, reconcile the exact public signed generation, then activate only the reconciled state.</p>
            </div>
        @endif
    </div>

    <div class="card">
        <h2>Updater-enabled releases</h2>
        @if ($releases->isEmpty())
            <p class="muted">No immutable published release in this channel has an updater identity yet.</p>
        @else
            <div class="table-region" tabindex="0" aria-label="Updater-enabled releases, horizontally scrollable">
                <table>
                    <thead><tr><th>Sequence</th><th>Version</th><th>Release identity</th><th>Targets</th><th>State</th></tr></thead>
                    <tbody>
                        @foreach ($releases as $release)
                            <tr>
                                <td>{{ $release->updater_sequence }}</td>
                                <td>{{ $release->version }}</td>
                                <td><code>{{ $release->updater_release_id }}</code></td>
                                <td>
                                    @foreach ($release->artifacts->where('is_enabled', true) as $artifact)
                                        <div><code>{{ $artifact->platform }}/{{ $artifact->architecture }} → {{ $artifact->updater_target_path }}</code></div>
                                    @endforeach
                                </td>
                                <td>{{ $release->updater_withdrawn_at ? 'Withdrawn' : 'Eligible for policy selection' }}</td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </div>

    <div class="card">
        <h2>Approve a new policy revision</h2>
        <p class="muted">Revision and channel release sequence are monotonic integers. Display versions are never compared for security ordering. An older release sequence can be selected only with explicit rollback authorization in the newer policy.</p>
        @if ($defaultRelease)
            <form class="form-stack" method="POST" action="{{ route('admin.downloads.updater.policies.store', ['channel' => $channel]) }}">
                @csrf
                <input type="hidden" name="operation_id" value="{{ old('operation_id', $operationId) }}">
                <div class="form-field">
                    <label for="current_release_id">Current updater release</label>
                    <select id="current_release_id" name="current_release_id" required>
                        @foreach ($releases->whereNull('updater_withdrawn_at') as $release)
                            <option value="{{ $release->id }}" @selected((int) old('current_release_id', $defaultRelease->id) === $release->id)>
                                Sequence {{ $release->updater_sequence }} · {{ $release->version }} · {{ $release->updater_release_id }}
                            </option>
                        @endforeach
                    </select>
                </div>
                <div class="form-field">
                    <label for="minimum_supported_release_sequence">Minimum supported release sequence</label>
                    <input id="minimum_supported_release_sequence" name="minimum_supported_release_sequence" type="number" min="1" required value="{{ old('minimum_supported_release_sequence', $defaultMinimum) }}">
                </div>
                <div class="form-field">
                    <label for="update_mode">Update mode</label>
                    <select id="update_mode" name="update_mode" required>
                        @foreach (\App\Downloads\DownloadCatalog::updateModes() as $mode)
                            <option value="{{ $mode }}" @selected(old('update_mode', 'optional') === $mode)>{{ \App\Downloads\DownloadCatalog::updateModeLabel($mode) }}</option>
                        @endforeach
                    </select>
                </div>
                <div class="form-field">
                    <label for="rollback_authorization">Rollback authorization</label>
                    <select id="rollback_authorization" name="rollback_authorization" required>
                        <option value="none" @selected(old('rollback_authorization', 'none') === 'none')>None</option>
                        <option value="explicit" @selected(old('rollback_authorization') === 'explicit')>Explicit older-sequence rollback</option>
                    </select>
                </div>

                <fieldset>
                    <legend>Release revocations in this new policy</legend>
                    <p class="muted">Revocation is immutable policy history. It does not delete the release.</p>
                    @foreach ($releases as $release)
                        <label>
                            <input type="checkbox" name="revoked_release_ids[]" value="{{ $release->updater_release_id }}" @checked(in_array($release->updater_release_id, old('revoked_release_ids', []), true))>
                            Sequence {{ $release->updater_sequence }} · {{ $release->version }} · <code>{{ $release->updater_release_id }}</code>
                        </label>
                    @endforeach
                </fieldset>

                <fieldset>
                    <legend>Exact target revocations in this new policy</legend>
                    <p class="muted">Target revocation is scoped to the exact platform/architecture target path and does not imply that byte-distinct siblings are revoked.</p>
                    @foreach ($releases as $release)
                        @foreach ($release->artifacts->where('is_enabled', true) as $artifact)
                            <label>
                                <input type="checkbox" name="revoked_artifact_targets[]" value="{{ $artifact->updater_target_path }}" @checked(in_array($artifact->updater_target_path, old('revoked_artifact_targets', []), true))>
                                {{ $artifact->platform }}/{{ $artifact->architecture }} · <code>{{ $artifact->updater_target_path }}</code>
                            </label>
                        @endforeach
                    @endforeach
                </fieldset>

                <button type="submit">Approve updater policy</button>
            </form>
        @else
            <p class="muted">Enable at least one immutable published release for updater distribution first.</p>
        @endif
    </div>

    <div class="card">
        <h2>Approved policy history</h2>
        @if ($policies->isEmpty())
            <p class="muted">No updater policy revisions are approved for this channel.</p>
        @else
            @foreach ($policies as $policy)
                <details>
                    <summary>Revision {{ $policy->revision }} · sequence {{ $policy->current_release_sequence }} · {{ \App\Downloads\DownloadCatalog::updateModeLabel($policy->update_mode) }}</summary>
                    <dl>
                        <dt>Operation identity</dt><dd><code>{{ $policy->operation_id }}</code></dd>
                        <dt>Policy target</dt><dd><code>{{ $policy->policy_target_path }}</code></dd>
                        <dt>Expected policy SHA-256</dt><dd><code>{{ $policy->policy_document_sha256 }}</code></dd>
                        <dt>Expected policy bytes</dt><dd>{{ $policy->policy_document_length }}</dd>
                        <dt>Rollback</dt><dd>{{ $policy->rollback_authorization }}</dd>
                    </dl>
                    <p class="muted">Exact canonical public policy document prepared for the protected signer/repository boundary:</p>
                    <pre data-policy-revision="{{ $policy->revision }}"><code>{{ $policyDocuments[$policy->id] }}</code></pre>
                </details>
            @endforeach
        @endif
    </div>

    <div class="card">
        <h2>Reconcile public signed-generation metadata</h2>
        <p class="muted">Paste only the bounded public projection produced after protected signing/repository generation. Raw private keys, signing secrets and unmodelled fields are rejected. This reconciliation does not claim that Laravel verified TUF signatures.</p>
        <form class="form-stack" method="POST" action="{{ route('admin.downloads.updater.generations.store', ['channel' => $channel]) }}">
            @csrf
            <div class="form-field">
                <label for="public_metadata_json">Public signed-generation metadata JSON</label>
                <textarea id="public_metadata_json" name="public_metadata_json" rows="18" maxlength="200000" required>{{ old('public_metadata_json') }}</textarea>
            </div>
            <button type="submit">Reconcile public metadata</button>
        </form>
    </div>

    <div class="card">
        <h2>Signed-generation history</h2>
        @if ($generations->isEmpty())
            <p class="muted">No signed repository generation has been reconciled yet.</p>
        @else
            <div class="table-region" tabindex="0" aria-label="Signed generation history, horizontally scrollable">
                <table>
                    <thead><tr><th>Generation</th><th>Policy</th><th>T/S/T versions</th><th>Expiry</th><th>State</th><th>Action</th></tr></thead>
                    <tbody>
                        @foreach ($generations as $generation)
                            <tr>
                                <td><code>{{ $generation->generation_id }}</code></td>
                                <td>{{ $generation->policy->revision }}</td>
                                <td>{{ $generation->targets_version }}/{{ $generation->snapshot_version }}/{{ $generation->timestamp_version }}</td>
                                <td>{{ $generation->metadata_expires_at->format('Y-m-d H:i:s') }} UTC</td>
                                <td>
                                    @if ($generation->superseded_at)
                                        Superseded
                                    @elseif ($generation->activated_at)
                                        <span class="badge badge-success">Platform-active</span>
                                    @else
                                        Reconciled, inactive
                                    @endif
                                </td>
                                <td>
                                    @if (! $generation->activated_at && ! $generation->superseded_at)
                                        <form method="POST" action="{{ route('admin.downloads.updater.generations.activate', ['channel' => $channel, 'clientUpdateGeneration' => $generation]) }}">
                                            @csrf
                                            <button type="submit">Activate Platform updater state</button>
                                        </form>
                                    @endif
                                </td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </div>
@endsection