@extends('admin.layout')

@section('title', 'Manage Downloads')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Content · Downloads</p>
        <h1>Client releases</h1>
        <p class="muted">Manage immutable approved artifact references. Executable uploads are not supported.</p>
    </div>

    <div class="action-row">
        <a class="button" href="{{ route('admin.downloads.create') }}">Create release draft</a>
        <a class="button button-secondary" href="{{ route('downloads.index') }}">View public Download Center</a>
    </div>

    @if ($releases->count() === 0)
        <div class="empty-state">
            <strong>No client releases yet.</strong>
            <p>Create a draft, add approved artifact metadata, then publish it explicitly.</p>
        </div>
    @else
        <div class="table-region" tabindex="0" aria-label="Client releases table, horizontally scrollable on small screens">
            <table>
                <thead>
                    <tr>
                        <th scope="col">Version</th>
                        <th scope="col">Channel</th>
                        <th scope="col">Artifacts</th>
                        <th scope="col">State</th>
                        <th scope="col">Updated</th>
                        <th scope="col">Action</th>
                    </tr>
                </thead>
                <tbody>
                    @foreach ($releases as $release)
                        <tr>
                            <td>{{ $release->version }}</td>
                            <td>{{ \App\Downloads\DownloadCatalog::channelLabel($release->channel) }}</td>
                            <td>{{ $release->artifacts_count }}</td>
                            <td>
                                @if ($release->published_at)
                                    <span class="badge badge-success">Published</span>
                                    @if ($release->is_current)
                                        <span class="badge badge-success">Current</span>
                                    @endif
                                    <br><span class="muted">{{ $release->published_at->format('Y-m-d H:i') }}</span>
                                @else
                                    <span class="badge badge-warning">Draft</span>
                                @endif
                            </td>
                            <td>{{ $release->updated_at?->format('Y-m-d H:i') }}</td>
                            <td><a class="button button-secondary" href="{{ route('admin.downloads.edit', $release) }}">Manage</a></td>
                        </tr>
                    @endforeach
                </tbody>
            </table>
        </div>
    @endif

    <div class="pagination">{{ $releases->links() }}</div>
@endsection
