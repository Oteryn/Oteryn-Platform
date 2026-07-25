@extends('admin.layout')

@section('title', 'Announcements')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Public portal · Announcements</p>
        <h1>Announcements</h1>
        <p class="muted">Manage the audited announcement ticker. Public visibility follows published state and UTC start/end boundaries.</p>
    </div>

    <div class="action-row">
        <a class="button" href="{{ route('admin.announcements.create') }}">Create announcement</a>
    </div>

    <div class="table-wrap">
        <table>
            <thead>
            <tr>
                <th>Title</th>
                <th>Severity</th>
                <th>Publication</th>
                <th>Window (UTC)</th>
                <th>Version</th>
                <th><span class="sr-only">Actions</span></th>
            </tr>
            </thead>
            <tbody>
            @forelse ($announcements as $announcement)
                <tr>
                    <td>{{ $announcement->title }}</td>
                    <td>{{ ucfirst($announcement->severity) }}</td>
                    <td>{{ ucfirst($announcement->publication_state) }}</td>
                    <td>
                        {{ $announcement->starts_at->format('Y-m-d H:i') }}
                        –
                        {{ $announcement->ends_at?->format('Y-m-d H:i') ?? 'No end' }}
                    </td>
                    <td>{{ $announcement->lock_version }}</td>
                    <td><a href="{{ route('admin.announcements.edit', $announcement) }}">Edit</a></td>
                </tr>
            @empty
                <tr>
                    <td colspan="6">No announcements have been created.</td>
                </tr>
            @endforelse
            </tbody>
        </table>
    </div>

    {{ $announcements->links() }}
@endsection
