@extends('admin.layout')

@section('title', 'Events')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Public portal · Events</p>
        <h1>Events</h1>
        <p class="muted">Content edits return an event to draft. Publication transitions require the separate events.publish permission.</p>
    </div>

    <div class="action-row">
        <a class="button" href="{{ route('admin.events.create') }}">Create event</a>
    </div>

    <div class="table-wrap">
        <table>
            <thead>
            <tr>
                <th>ID</th>
                <th>Status</th>
                <th>Window (UTC)</th>
                <th>Featured</th>
                <th>Version</th>
                <th><span class="sr-only">Actions</span></th>
            </tr>
            </thead>
            <tbody>
            @forelse ($events as $event)
                <tr>
                    <td>{{ $event->id }}</td>
                    <td>{{ ucfirst($event->effectiveStatusAt(now())) }}</td>
                    <td>{{ $event->starts_at->format('Y-m-d H:i') }} – {{ $event->ends_at->format('Y-m-d H:i') }}</td>
                    <td>{{ $event->featured ? 'Yes' : 'No' }}</td>
                    <td>{{ $event->lock_version }}</td>
                    <td><a href="{{ route('admin.events.edit', $event) }}">Edit</a></td>
                </tr>
            @empty
                <tr>
                    <td colspan="6">No events have been created.</td>
                </tr>
            @endforelse
            </tbody>
        </table>
    </div>

    {{ $events->links() }}
@endsection
