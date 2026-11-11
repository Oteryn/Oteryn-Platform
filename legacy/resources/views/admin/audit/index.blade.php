@extends('admin.layout')

@section('title', 'Administrator Audit')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Operations</p>
        <h1>Administrator audit</h1>
        <p class="muted">Review bounded privileged administration events. Sensitive credentials and authentication secrets are not intended for this surface.</p>
    </div>

    @if ($events->count() === 0)
        <div class="empty-state">No administrator audit events are available.</div>
    @else
        <div class="table-region" tabindex="0" aria-label="Administrator audit table, horizontally scrollable on small screens">
            <table>
                <thead>
                    <tr>
                        <th scope="col">Occurred</th>
                        <th scope="col">Actor</th>
                        <th scope="col">Action</th>
                        <th scope="col">Target</th>
                        <th scope="col">Metadata</th>
                    </tr>
                </thead>
                <tbody>
                    @foreach ($events as $event)
                        <tr>
                            <td>{{ $event->occurred_at }}</td>
                            <td>{{ $event->actor_email ?? 'bootstrap/system' }}</td>
                            <td>{{ $event->action }}</td>
                            <td>{{ $event->target_type }}{{ $event->target_id === null ? '' : ':'.$event->target_id }}</td>
                            <td class="metadata-cell">
                                @if ($event->metadata)
                                    <details>
                                        <summary>View metadata</summary>
                                        <code class="metadata-value">{{ $event->metadata }}</code>
                                    </details>
                                @else
                                    <span class="muted">None</span>
                                @endif
                            </td>
                        </tr>
                    @endforeach
                </tbody>
            </table>
        </div>
    @endif

    <div class="pagination">{{ $events->links() }}</div>
@endsection
