@extends('admin.layout')

@section('title', 'Manage Downloads')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Content · Downloads</p>
        <h1>Client releases</h1>
        <p class="muted">Manage immutable approved artifact references, browser publication and the separately reconciled updater policy boundary. Executable uploads and private updater signing keys are not supported.</p>
    </div>

    <div class="action-row">
        <a class="button" href="{{ route('admin.downloads.create') }}">Create release draft</a>
        <a class="button button-secondary" href="{{ route('admin.downloads.updater', ['channel' => 'stable']) }}">Stable updater diagnostics</a>
        <a class="button button-secondary" href="{{ route('admin.downloads.updater', ['channel' => 'beta']) }}">Beta updater diagnostics</a>
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
                        <th scope="col">Browser state</th>
                        <th scope="col">Updater identity</th>
                        <th scope="col">Updated</th>
                        <th scope="col">Actions</th>
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
                            <td>
                                @if ($release->updater_release_id)
                                    <span class="badge badge-success">Sequence {{ $release->updater_sequence }}</span>
                                    @if ($release->updater_withdrawn_at)
                                        <span class="badge badge-warning">Withdrawn</span>
                                    @endif
                                    <br><code>{{ $release->updater_release_id }}</code>
                                @else
                                    <span class="muted">Browser-only</span>
                                @endif
                            </td>
                            <td>{{ $release->updated_at?->format('Y-m-d H:i') }}</td>
                            <td>
                                <div class="action-row">
                                    <a class="button button-secondary" href="{{ route('admin.downloads.edit', $release) }}">Manage English</a>
                                    <a class="button button-secondary" href="{{ route('admin.downloads.translation.edit', $release) }}">Polish release notes</a>
                                </div>
                            </td>
                        </tr>
                    @endforeach
                </tbody>
            </table>
        </div>
    @endif

    <div class="pagination">{{ $releases->links() }}</div>
@endsection