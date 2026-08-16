@extends('game.layout')

@section('title', __('public.downloads.page_title'))

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <div class="page-header">
        <p class="eyebrow">{{ __('public.downloads.eyebrow') }}</p>
        <h1>{{ __('public.downloads.title') }}</h1>
        <p class="muted">{{ __('public.downloads.description') }}</p>
    </div>

    <nav class="action-row" aria-label="{{ __('public.downloads.filter') }}">
        <a class="button {{ $downloadCenter->platform === null ? '' : 'button-secondary' }}"
           href="{{ route('downloads.index') }}"
           @if($downloadCenter->platform === null) aria-current="page" @endif>{{ __('public.downloads.all_platforms') }}</a>
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
            <strong>{{ __('public.downloads.unavailable') }}</strong>
            <p>{{ __('public.downloads.unavailable_help') }}</p>
        </div>
    @elseif ($downloadCenter->state === \App\Downloads\DownloadCenterState::EMPTY)
        <div class="empty-state">
            <strong>
                @if ($downloadCenter->platform)
                    {{ __('No current download is available for :platform.', [
                        'platform' => \App\Downloads\DownloadCatalog::platformLabel($downloadCenter->platform),
                    ]) }}
                @else
                    {{ __('public.downloads.empty', ['platform' => '']) }}
                @endif
            </strong>
            <p>{{ __('public.downloads.empty_help') }}</p>
        </div>
    @else
        @foreach ($downloadCenter->releases as $release)
            @php
                $updaterState = (string) $release->getAttribute('updater_public_state');
                $updaterPolicyRevision = $release->getAttribute('updater_policy_revision');
                $updaterMode = $release->getAttribute('updater_update_mode');
                $updaterMinimum = $release->getAttribute('updater_minimum_supported_release_sequence');
                $updaterExpires = $release->getAttribute('updater_metadata_expires_at');
            @endphp
            <article class="card">
                <div class="page-header">
                    <p class="eyebrow">{{ __('public.downloads.channel', ['channel' => \App\Downloads\DownloadCatalog::channelLabel($release->channel)]) }}</p>
                    <h2>{{ __('public.downloads.client', ['version' => $release->version]) }}</h2>
                    <p class="muted">
                        {{ __('public.downloads.published', [
                            'channel' => $release->channel,
                            'date' => $localeFormatter->dateTime($release->published_at),
                        ]) }}
                    </p>
                </div>

                @if ($release->release_notes)
                    <p class="prose-text">{{ $release->release_notes }}</p>
                @elseif ((bool) $release->getAttribute('release_notes_translation_unavailable'))
                    <div class="notice" role="status">{{ __('public.downloads.notes_unavailable') }}</div>
                @endif

                <div class="notice" role="status">
                    <strong>{{ __('downloads.updater.title') }}</strong>
                    <p>{{ __('downloads.updater.states.'.$updaterState) }}</p>
                    @if (is_int($updaterPolicyRevision) && is_string($updaterMode) && is_int($updaterMinimum) && $updaterExpires instanceof \DateTimeInterface)
                        <p>{{ __('downloads.updater.policy', [
                            'revision' => $updaterPolicyRevision,
                            'mode' => $updaterMode,
                            'minimum' => $updaterMinimum,
                            'expires' => $updaterExpires->format('Y-m-d H:i:s'),
                        ]) }}</p>
                    @endif
                    <p class="muted">{{ __('downloads.updater.trust_notice') }}</p>
                </div>

                <div class="table-region" tabindex="0" aria-label="{{ __('public.downloads.artifacts_table') }}">
                    <table aria-label="{{ __('public.downloads.artifacts_table') }}">
                        <thead>
                            <tr>
                                <th scope="col">{{ __('public.downloads.platform') }}</th>
                                <th scope="col">{{ __('public.downloads.architecture') }}</th>
                                <th scope="col">{{ __('public.downloads.filename') }}</th>
                                <th scope="col">{{ __('public.downloads.size') }}</th>
                                <th scope="col">SHA-256</th>
                                <th scope="col">{{ __('public.downloads.action') }}</th>
                            </tr>
                        </thead>
                        <tbody>
                            @foreach ($release->artifacts as $artifact)
                                <tr>
                                    <td>{{ \App\Downloads\DownloadCatalog::platformLabel($artifact->platform) }}</td>
                                    <td>{{ \App\Downloads\DownloadCatalog::architectureLabel($artifact->architecture) }}</td>
                                    <td>{{ $artifact->filename }}</td>
                                    <td>{{ $localeFormatter->bytes($artifact->size_bytes) }}</td>
                                    <td><code>{{ $artifact->sha256 }}</code></td>
                                    <td>
                                        <a class="button" href="{{ $artifact->artifact_url }}" rel="noopener noreferrer">{{ __('public.downloads.download') }}</a>
                                    </td>
                                </tr>
                            @endforeach
                        </tbody>
                    </table>
                </div>
            </article>
        @endforeach

        <div class="card">
            <h2>{{ __('public.downloads.checksum_title') }}</h2>
            <p class="muted">{{ __('public.downloads.checksum_help') }}</p>
        </div>
    @endif
@endsection