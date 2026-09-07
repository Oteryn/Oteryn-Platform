@extends('game.layout')
@section('title', __('public.game.servers_title'))
@section('portal-family', 'worlds')
@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <header class="page-header utility-masthead">
        <p class="eyebrow">{{ __('portal.worlds.eyebrow') }}</p>
        <h1>{{ __('public.game.servers_title') }}</h1>
        <p class="muted">{{ __('portal.worlds.intro') }}</p>
    </header>
    @if (! $runtimeSnapshot->available)
        <div class="alert alert-warning" role="status">{{ __('public.game.runtime_unavailable_help') }}</div>
    @endif
    <div class="world-directory">
        @forelse ($channels as $channel)
            @php($runtime = $runtimeSnapshot->forChannel((int) $channel->id))
            <article class="card world-row" data-world-state="{{ $channel->maintenance ? 'maintenance' : (! $runtimeSnapshot->available ? 'unavailable' : ($runtime === null ? 'unknown' : strtolower($runtime->status))) }}">
                <div class="world-identity">
                    <p class="eyebrow">{{ __('public.game.channel_id') }} {{ $localeFormatter->number($channel->id) }}</p>
                    <h2>{{ $channel->name }}</h2>
                    <p>{{ $channel->pvp_type }}</p>
                </div>
                <dl class="world-metrics">
                    <div><dt>{{ __('public.game.runtime') }}:</dt><dd>
                        @if (! $runtimeSnapshot->available)
                            <span class="badge badge-warning">{{ __('public.states.unavailable') }}</span>
                        @elseif ($runtime === null)
                            <span class="badge badge-warning">{{ __('public.game.unknown') }}</span>
                        @else
                            <span class="badge {{ strtoupper($runtime->status) === 'ONLINE' ? 'badge-success' : 'badge-warning' }}">{{ $runtime->status }}</span>
                        @endif
                    </dd></div>
                    <div><dt>{{ __('public.game.players_online') }}:</dt><dd class="world-population">{{ $runtimeSnapshot->available && $runtime !== null ? $localeFormatter->number($runtime->playersOnline) : '—' }}</dd></div>
                    <div><dt>{{ __('public.game.max_players') }}:</dt><dd>{{ $localeFormatter->number($channel->max_players) }}</dd></div>
                    <div><dt>{{ __('public.game.pvp_type') }}:</dt><dd>{{ $channel->pvp_type }}</dd></div>
                </dl>
                @if ($runtimeSnapshot->available && $runtime !== null && $runtime->isFull((int) $channel->max_players))
                    <p class="status badge badge-warning">{{ __('public.game.full') }}</p>
                @endif
                @if ($channel->maintenance)
                    <div class="notice alert-warning world-maintenance" role="status"><strong>{{ __('public.game.configured_maintenance') }}</strong>@if ($channel->maintenance_message)<p>{{ $channel->maintenance_message }}</p>@endif</div>
                @endif
            </article>
        @empty
            <div class="empty-state">{{ __('public.game.no_channels') }}</div>
        @endforelse
    </div>
@endsection
