@extends('admin.layout')

@section('title', 'Game Catalog profiles')

@push('head')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <div class="page-header">
        <p class="eyebrow">Game Catalog · Publication policy</p>
        <h1>Profiles</h1>
        <p class="muted">Profiles bind a target release and visibility policies to one active immutable snapshot.</p>
    </div>

    @include('game-catalog.admin._navigation')

    <div class="table-wrap">
        <table>
            <thead><tr><th>Profile</th><th>Target release</th><th>Protocol</th><th>Policies</th><th>Public</th><th>Backports</th><th>Active snapshot</th><th>Lock</th></tr></thead>
            <tbody>
            @forelse ($profiles as $profile)
                <tr>
                    <td><a href="{{ route('admin.game-catalog.profiles.show', $profile->id) }}">{{ $profile->name }}</a><br><code>{{ $profile->key }}</code></td>
                    <td>{{ $profile->target_release }}</td>
                    <td>{{ $profile->protocol_profile ?? 'any compatible' }}</td>
                    <td>{{ $profile->completeness_policy_key }}<br>{{ $profile->availability_policy_key }}<br>{{ $profile->validation_policy_key }}</td>
                    <td>{{ $profile->public_enabled ? 'yes' : 'no' }}</td>
                    <td>{{ $profile->allow_backports ? 'allowed' : 'disabled' }}</td>
                    <td>{{ $profile->active_snapshot_id === null ? 'not active' : '#'.$profile->active_snapshot_id }}</td>
                    <td>{{ $profile->lock_version }}</td>
                </tr>
            @empty
                <tr><td colspan="8">No Game Catalog profiles exist.</td></tr>
            @endforelse
            </tbody>
        </table>
    </div>
@endsection
