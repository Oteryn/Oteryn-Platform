@extends('game.layout')

@section('title', __('public.game.servers_title'))

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <div class="page-header">
        <p class="eyebrow">{{ __('public.game.infrastructure') }}</p>
        <h1>{{ __('public.game.servers_title') }}</h1>
        <p class="muted">{{ __('public.game.servers_description') }}</p>
    </div>

    @if (! $runtimeSnapshot->available)
        <div class="alert alert-warning" role="status">{{ __('public.game.runtime_unavailable_help') }}</div>
    @endif

    <div class="card-grid">
        @forelse ($channels as $channel)
            @php($runtime = $runtimeSnapshot->forChannel((int) $channel->id))
            <article class="card">
                <h2>{{ $channel->name }}</h2>
                <p><strong>{{ __('public.game.channel_id') }}:</strong> {{ $localeFormatter->number($channel->id) }}</p>
                <p><strong>{{ __('public.game.pvp_type') }}:</strong> {{ $channel->pvp_type }}</p>
                <p><strong>{{ __('public.game.max_players') }}:</strong> {{ $localeFormatter->number($channel->max_players) }}</p>

                @if (! $runtimeSnapshot->available)
                    <p class="badge badge-warning"><strong>{{ __('public.game.runtime') }}:</strong> {{ __('public.states.unavailable') }}</p>
                @elseif ($runtime === null)
                    <p class="badge badge-warning"><strong>{{ __('public.game.runtime') }}:</strong> {{ __('public.game.unknown') }}</p>
                @else
                    <p class="badge badge-success"><strong>{{ __('public.game.runtime') }}:</strong> {{ $runtime->status }}</p>
                    <p><strong>{{ __('public.game.players_online') }}:</strong> {{ $localeFormatter->number($runtime->playersOnline) }}</p>
                @endif

                @if ($runtimeSnapshot->available && $runtime !== null && $runtime->isFull((int) $channel->max_players))
                    <p class="status badge badge-warning">{{ __('public.game.full') }}</p>
                @endif

                @if ($channel->maintenance)
                    <div class="alert alert-warning">
                        <strong>{{ __('public.game.configured_maintenance') }}</strong>
                        @if ($channel->maintenance_message)
                            <p>{{ $channel->maintenance_message }}</p>
                        @endif
                    </div>
                @endif
            </article>
        @empty
            <div class="empty-state">{{ __('public.game.no_channels') }}</div>
        @endforelse
    </div>
@endsection
