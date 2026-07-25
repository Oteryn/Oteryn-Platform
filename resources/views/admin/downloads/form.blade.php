@extends('admin.layout')

@section('title', $release === null ? 'Create Client Release' : 'Manage Client Release')

@section('content')
    @php
        $published = $release?->published_at !== null;
        $artifactRows = old('artifacts');

        if (! is_array($artifactRows)) {
            $artifactRows = [];

            if ($release !== null) {
                $artifactRows = $release->artifacts
                    ->map(static fn ($artifact): array => [
                        'platform' => $artifact->platform,
                        'architecture' => $artifact->architecture,
                        'artifact_url' => $artifact->artifact_url,
                        'filename' => $artifact->filename,
                        'size_bytes' => $artifact->size_bytes,
                        'sha256' => $artifact->sha256,
                        'is_enabled' => $artifact->is_enabled,
                    ])
                    ->values()
                    ->all();
            }
        }

        while (count($artifactRows) < 6 && ! $published) {
            $artifactRows[] = [
                'platform' => '',
                'architecture' => '',
                'artifact_url' => '',
                'filename' => '',
                'size_bytes' => '',
                'sha256' => '',
                'is_enabled' => true,
            ];
        }
    @endphp

    <div class="page-header">
        <p class="eyebrow">Content · Downloads</p>
        <h1>{{ $release === null ? 'Create release draft' : 'Manage client release' }}</h1>
        <p class="muted">Only metadata and direct immutable HTTPS references are stored. The Platform never uploads, fetches or proxies the executable.</p>
    </div>

    @if ($published)
        <div class="alert alert-success" role="status">
            <strong>This release is published{{ $release->is_current ? ' and current' : '' }}.</strong>
            <p>Published version, variant and artifact metadata are immutable. Create a new release to change them.</p>
        </div>

        <div class="card">
            <dl>
                <dt>Version</dt>
                <dd>{{ $release->version }}</dd>
                <dt>Channel</dt>
                <dd>{{ \App\Downloads\DownloadCatalog::channelLabel($release->channel) }}</dd>
                <dt>Published</dt>
                <dd>{{ $release->published_at?->format('Y-m-d H:i') }} UTC</dd>
                <dt>Current</dt>
                <dd>{{ $release->is_current ? 'Yes' : 'No' }}</dd>
            </dl>

            @if ($release->release_notes)
                <h2>Release notes</h2>
                <p class="prose-text">{{ $release->release_notes }}</p>
            @endif

            <div class="table-region" tabindex="0" aria-label="Published release artifacts, horizontally scrollable on small screens">
                <table>
                    <thead>
                        <tr>
                            <th scope="col">Platform</th>
                            <th scope="col">Architecture</th>
                            <th scope="col">Filename</th>
                            <th scope="col">Size bytes</th>
                            <th scope="col">SHA-256</th>
                            <th scope="col">Approved URL</th>
                            <th scope="col">Enabled</th>
                        </tr>
                    </thead>
                    <tbody>
                        @foreach ($release->artifacts as $artifact)
                            <tr>
                                <td>{{ \App\Downloads\DownloadCatalog::platformLabel($artifact->platform) }}</td>
                                <td>{{ \App\Downloads\DownloadCatalog::architectureLabel($artifact->architecture) }}</td>
                                <td>{{ $artifact->filename }}</td>
                                <td>{{ $artifact->size_bytes }}</td>
                                <td><code>{{ $artifact->sha256 }}</code></td>
                                <td><code>{{ $artifact->artifact_url }}</code></td>
                                <td>{{ $artifact->is_enabled ? 'Yes' : 'No' }}</td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>

            @if (! $release->is_current)
                <form class="form-stack" method="POST" action="{{ route('admin.downloads.publish', $release) }}">
                    @csrf
                    <input type="hidden" name="make_current" value="1">
                    <button type="submit">Set as current {{ $release->channel }} release</button>
                </form>
            @endif
        </div>
    @else
        <div class="alert alert-warning" role="status">
            <strong>No executable upload is available.</strong>
            <p>Enter metadata copied from the approved release pipeline. SHA-256 is not verified by the Platform.</p>
        </div>

        <div class="card">
            <form class="form-stack" method="POST" action="{{ $release === null ? route('admin.downloads.store') : route('admin.downloads.update', $release) }}">
                @csrf
                @if ($release !== null)
                    @method('PUT')
                @endif

                <div class="form-field">
                    <label for="version">Version</label>
                    <input id="version" name="version" type="text" maxlength="64" required value="{{ old('version', $release?->version) }}">
                    <p class="form-help">Examples: 1.0.0 or 1.0.0-beta.2.</p>
                </div>

                <div class="form-field">
                    <label for="channel">Channel</label>
                    <select id="channel" name="channel" required>
                        <option value="">Select channel</option>
                        @foreach (\App\Downloads\DownloadCatalog::channels() as $channel)
                            <option value="{{ $channel }}" @selected(old('channel', $release?->channel) === $channel)>
                                {{ \App\Downloads\DownloadCatalog::channelLabel($channel) }}
                            </option>
                        @endforeach
                    </select>
                </div>

                <div class="form-field">
                    <label for="release_notes">Release notes (plain text)</label>
                    <textarea id="release_notes" name="release_notes" rows="8" maxlength="10000">{{ old('release_notes', $release?->release_notes) }}</textarea>
                </div>

                <h2>Artifact variants</h2>
                <p class="muted">Leave unused rows completely empty. At least one complete variant is required.</p>

                <div class="table-region" tabindex="0" aria-label="Artifact metadata editor, horizontally scrollable on small screens">
                    <table>
                        <thead>
                            <tr>
                                <th scope="col">Platform</th>
                                <th scope="col">Architecture</th>
                                <th scope="col">Approved HTTPS URL</th>
                                <th scope="col">Filename</th>
                                <th scope="col">Size bytes</th>
                                <th scope="col">SHA-256</th>
                                <th scope="col">Enabled</th>
                            </tr>
                        </thead>
                        <tbody>
                            @foreach ($artifactRows as $index => $artifact)
                                <tr>
                                    <td>
                                        <label class="sr-only" for="artifact-{{ $index }}-platform">Platform</label>
                                        <select id="artifact-{{ $index }}-platform" name="artifacts[{{ $index }}][platform]">
                                            <option value="">Select</option>
                                            @foreach (\App\Downloads\DownloadCatalog::platforms() as $platform)
                                                <option value="{{ $platform }}" @selected(($artifact['platform'] ?? '') === $platform)>
                                                    {{ \App\Downloads\DownloadCatalog::platformLabel($platform) }}
                                                </option>
                                            @endforeach
                                        </select>
                                    </td>
                                    <td>
                                        <label class="sr-only" for="artifact-{{ $index }}-architecture">Architecture</label>
                                        <select id="artifact-{{ $index }}-architecture" name="artifacts[{{ $index }}][architecture]">
                                            <option value="">Select</option>
                                            @foreach (\App\Downloads\DownloadCatalog::architectures() as $architecture)
                                                <option value="{{ $architecture }}" @selected(($artifact['architecture'] ?? '') === $architecture)>
                                                    {{ \App\Downloads\DownloadCatalog::architectureLabel($architecture) }}
                                                </option>
                                            @endforeach
                                        </select>
                                    </td>
                                    <td>
                                        <label class="sr-only" for="artifact-{{ $index }}-url">Approved HTTPS URL</label>
                                        <input id="artifact-{{ $index }}-url" name="artifacts[{{ $index }}][artifact_url]" type="url" maxlength="2048" value="{{ $artifact['artifact_url'] ?? '' }}">
                                    </td>
                                    <td>
                                        <label class="sr-only" for="artifact-{{ $index }}-filename">Filename</label>
                                        <input id="artifact-{{ $index }}-filename" name="artifacts[{{ $index }}][filename]" type="text" maxlength="255" value="{{ $artifact['filename'] ?? '' }}">
                                    </td>
                                    <td>
                                        <label class="sr-only" for="artifact-{{ $index }}-size">Size bytes</label>
                                        <input id="artifact-{{ $index }}-size" name="artifacts[{{ $index }}][size_bytes]" type="number" min="1" max="21474836480" value="{{ $artifact['size_bytes'] ?? '' }}">
                                    </td>
                                    <td>
                                        <label class="sr-only" for="artifact-{{ $index }}-sha256">SHA-256</label>
                                        <input id="artifact-{{ $index }}-sha256" name="artifacts[{{ $index }}][sha256]" type="text" minlength="64" maxlength="64" value="{{ $artifact['sha256'] ?? '' }}">
                                    </td>
                                    <td>
                                        <input type="hidden" name="artifacts[{{ $index }}][is_enabled]" value="0">
                                        <input id="artifact-{{ $index }}-enabled" name="artifacts[{{ $index }}][is_enabled]" type="checkbox" value="1" @checked((bool) ($artifact['is_enabled'] ?? false))>
                                        <label class="sr-only" for="artifact-{{ $index }}-enabled">Enabled</label>
                                    </td>
                                </tr>
                            @endforeach
                        </tbody>
                    </table>
                </div>

                <div class="action-row">
                    <button type="submit">Save draft</button>
                    <a class="button button-secondary" href="{{ route('admin.downloads.index') }}">Cancel</a>
                </div>
            </form>
        </div>

        @if ($release !== null)
            <div class="card">
                <h2>Publish release</h2>
                <p class="muted">Publication revalidates every enabled URL against the current scheme and host allowlist.</p>
                <div class="action-row">
                    <form method="POST" action="{{ route('admin.downloads.publish', $release) }}">
                        @csrf
                        <input type="hidden" name="make_current" value="1">
                        <button type="submit">Publish and make current</button>
                    </form>
                    <form method="POST" action="{{ route('admin.downloads.publish', $release) }}">
                        @csrf
                        <input type="hidden" name="make_current" value="0">
                        <button class="button-secondary" type="submit">Publish without making current</button>
                    </form>
                </div>
            </div>
        @endif
    @endif

    <div class="action-row">
        <a class="button button-secondary" href="{{ route('admin.downloads.index') }}">Back to releases</a>
    </div>
@endsection
