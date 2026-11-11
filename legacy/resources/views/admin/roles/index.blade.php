@extends('admin.layout')

@section('title', 'Manage Administrator Roles')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Access control</p>
        <h1>Administrator roles</h1>
        <p class="muted">Role changes are audited. The final platform administrator cannot be removed.</p>
    </div>

    @if ($identities->count() === 0)
        <div class="empty-state">No Platform identities are available for role management.</div>
    @else
        <div class="table-region" tabindex="0" aria-label="Administrator role management table, horizontally scrollable on small screens">
            <table>
                <thead>
                    <tr>
                        <th scope="col">Identity</th>
                        <th scope="col">Assigned roles</th>
                        <th scope="col">Manage</th>
                    </tr>
                </thead>
                <tbody>
                    @foreach ($identities as $identity)
                        @php
                            $assignedRoles = $rolesByIdentity
                                ->get($identity->id, collect())
                                ->pluck('key')
                                ->all();
                        @endphp
                        <tr>
                            <td>{{ $identity->email }}</td>
                            <td>{{ $assignedRoles === [] ? 'None' : implode(', ', $assignedRoles) }}</td>
                            <td>
                                <div class="table-actions">
                                    @foreach ($availableRoles as $role)
                                        @if (in_array($role, $assignedRoles, true))
                                            <form method="POST" action="{{ route('admin.roles.destroy', ['identity' => $identity, 'roleKey' => $role]) }}">
                                                @csrf
                                                @method('DELETE')
                                                <button class="danger" type="submit">Remove {{ $role }}</button>
                                            </form>
                                        @else
                                            <form method="POST" action="{{ route('admin.roles.store', $identity) }}">
                                                @csrf
                                                <input type="hidden" name="role" value="{{ $role }}">
                                                <button class="button-secondary" type="submit">Assign {{ $role }}</button>
                                            </form>
                                        @endif
                                    @endforeach
                                </div>
                            </td>
                        </tr>
                    @endforeach
                </tbody>
            </table>
        </div>
    @endif

    <div class="pagination">{{ $identities->links() }}</div>
@endsection
