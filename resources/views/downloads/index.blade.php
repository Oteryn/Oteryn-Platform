@extends('game.layout')

@section('title', 'Download')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Play Oteryn</p>
        <h1>Download Center</h1>
        <p class="muted">Choose a current approved client build for your operating system.</p>
    </div>

    <nav class="action-row" aria-label="Filter downloads by operating system">
        <a class="button {{ $downloadCenter->platform === null ? '' : 'button-secondary' }}"
           href="{{ route('downloads.index') }}"
           @if($downloadCenter->platform === null) aria-current="page" @endif>All platforms</a>
        @foreach ($platforms as $platform)
            <a class="button {{ $downloadCenter->platform === $platform ? '' : 'button-secondary' }}"
               href="{{ route('downloads.index', ['platform' => $platform]) }}"
               @if($downloadCenter->platform === $platform) aria-current="page" @endif>
                {{ \App\Downloads\DownloadCatalog::platformLabel($platform) }}
            </a>
        @endforeach
    </nav>

    @if ($downloadCenter->state === \App\Downloads\DownloadCenterState::UNAVAILABLE)
        <div class="alert alert-danger" role="status">
            <strong>Downloads are temporarily unavailable.</strong>
            <p>The current release metadata or its approved artifact reference cannot be resolved safely. Try again later.</p>
        </div>
    @elseif ($downloadCenter->state === \App\Downloads\DownloadCenterState::EMPTY)
        <div class="empty-state">
            <strong>No current download is available{{ $downloadCenter->platform ? ' for '.\App\Downloads\DownloadCatalog::platformLabel($downloadCenter->platform) : '' }}.</strong>
            <p>Only published current releases with enabled approved artifacts appear here.</p>
        </div>
    @else
        @foreach ($downloadCenter->releases as $release)
            <article class="card">
                <div class="page-header">
                    <p class="eyebrow">{{ \App\Downloads\DownloadCatalog::channelLabel($release->channel) }} channel</p>
                    <h2>Oteryn Client {{ $release->version }}</h2>
                    <p class="muted">
                        Current {{ $release->channel }} build · Published {{ $release->published_at?->format('Y-m-d H:i') }} UTC
                    </p>
                </div>

                @if ($release->release_notes)
                    <p class="prose-text">{{ $release->release_notes }}</p>
                @endif

                <div class="table-region" tabindex="0" aria-label="Client artifacts, horizontally scrollable on small screens">
                    <table>
                        <thead>
                            <tr>
                                <th scope="col">Platform</th>
                                <th scope="col">Architecture</th>
                                <th scope="col">Filename</th>
                                <th scope="col">Size</th>
                                <th scope="col">SHA-256</th>
                                <th scope="col">Action</th>
                            </tr>
                        </thead>
                        <tbody>
                            @foreach ($release->artifacts as $artifact)
                                <tr>
                                    <td>{{ \App\Downloads\DownloadCatalog::platformLabel($artifact->platform) }}</td>
                                    <td>{{ \App\Downloads\DownloadCatalog::architectureLabel($artifact->architecture) }}</td>
                                    <td>{{ $artifact->filename }}</td>
                                    <td>{{ $artifact->formattedSize() }}</td>
                                    <td><code>{{ $artifact->sha256 }}</code></td>
                                    <td>
                                        <a class="button" href="{{ $artifact->artifact_url }}" rel="noopener noreferrer">Download</a>
                                    </td>
                                </tr>
                            @endforeach
                        </tbody>
                    </table>
                </div>
            </article>
        @endforeach

        <div class="card">
            <h2>Checksum notice</h2>
            <p class="muted">SHA-256 values are administrator-supplied release metadata. Oteryn Platform does not fetch the artifact and does not claim that it independently verified the checksum.</p>
        </div>
    @endif
@endsection
