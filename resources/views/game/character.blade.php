@extends('game.layout')

@section('title', $character['name'])
@section('page-class', 'page-shell-wide community-page')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    @inject('characterPresentation', 'App\PublicGameData\CharacterPresentation')

    <header class="page-header">
        <p class="eyebrow">{{ __('community.profile.title') }}</p>
        <h1>{{ $character['name'] }}</h1>
        <p class="muted">{{ __('community.profile.description') }}</p>
    </header>

    <section aria-labelledby="character-overview-heading">
        <h2 id="character-overview-heading">{{ __('community.profile.overview') }}</h2>
        <dl class="stat-grid">
            <div class="stat">
                <dt>{{ __('public.game.level') }}</dt>
                <dd class="display-title">{{ $localeFormatter->number($character['level']) }}</dd>
            </div>
            <div class="stat" data-vocation-id="{{ $character['vocation'] }}">
                <dt>{{ __('public.game.vocation') }}</dt>
                <dd>{{ $characterPresentation->vocationName($character['vocation']) }}</dd>
            </div>
            <div class="stat">
                <dt>{{ __('community.profile.magic_level') }}</dt>
                <dd>{{ $localeFormatter->number($character['magic_level']) }}</dd>
            </div>
            <div class="stat">
                <dt>{{ __('community.profile.boss_points') }}</dt>
                <dd>{{ $localeFormatter->number($character['boss_points']) }}</dd>
            </div>
            <div class="stat">
                <dt>{{ __('public.game.guild') }}</dt>
                <dd>
                    @if ($character['guild_name'] !== null)
                        <a href="{{ route('game.guilds.show', ['name' => $character['guild_name']]) }}">{{ $character['guild_name'] }}</a>
                        @if ($character['guild_rank'] !== null)
                            <span class="muted">· {{ $character['guild_rank'] }}</span>
                        @endif
                    @else
                        {{ __('game.profile.no_guild') }}
                    @endif
                </dd>
            </div>
            <div class="stat">
                <dt>{{ __('community.profile.house') }}</dt>
                <dd>
                    @if ($house !== null)
                        {{ $house['name'] }} · {{ __('community.profile.house_size') }} {{ $localeFormatter->number($house['size']) }}
                    @else
                        {{ __('community.profile.no_house') }}
                    @endif
                </dd>
            </div>
        </dl>
    </section>

    <div class="community-grid">
        <section class="card" aria-labelledby="character-comment-heading">
            <h2 id="character-comment-heading">{{ __('community.profile.comment') }}</h2>
            <p class="community-preserved-text">{{ $character['comment'] !== '' ? $character['comment'] : __('community.profile.no_comment') }}</p>
        </section>

        <section class="card" aria-labelledby="character-status-heading">
            <h2 id="character-status-heading">{{ __('community.profile.status') }}</h2>
            @if ($status === null)
                <p class="muted">{{ __('community.profile.status_private') }}</p>
            @else
                <p><strong>{{ $status['online'] ? __('community.profile.online') : __('community.profile.offline') }}</strong></p>
                <dl class="community-detail-list">
                    <div>
                        <dt>{{ __('community.profile.last_login') }}</dt>
                        <dd>{{ $status['last_login'] === null ? '—' : $localeFormatter->dateTime($status['last_login']) }}</dd>
                    </div>
                    <div>
                        <dt>{{ __('community.profile.last_logout') }}</dt>
                        <dd>{{ $status['last_logout'] === null ? '—' : $localeFormatter->dateTime($status['last_logout']) }}</dd>
                    </div>
                </dl>
            @endif
        </section>
    </div>

    <section class="card" aria-labelledby="character-skills-heading">
        <h2 id="character-skills-heading">{{ __('community.profile.skills') }}</h2>
        <div class="table-region" tabindex="0">
            <table class="table-compact">
                <thead>
                <tr>
                    <th scope="col">{{ __('community.profile.skill') }}</th>
                    <th scope="col">{{ __('community.profile.value') }}</th>
                </tr>
                </thead>
                <tbody>
                @foreach ($character['skills'] as $skill => $value)
                    <tr>
                        <td>{{ __('community.skills.'.$skill) }}</td>
                        <td>{{ $localeFormatter->number($value) }}</td>
                    </tr>
                @endforeach
                </tbody>
            </table>
        </div>
    </section>

    <div class="community-grid">
        <section class="card" aria-labelledby="character-deaths-heading">
            <h2 id="character-deaths-heading">{{ __('community.profile.recent_deaths') }}</h2>
            <ul class="community-event-list">
                @forelse ($deaths as $death)
                    <li>
                        <strong>{{ __('community.profile.killed_by', ['killer' => $death->killed_by]) }}</strong>
                        <span>{{ __('community.profile.killed_at_level', ['level' => $localeFormatter->number((int) $death->level)]) }}</span>
                        <time datetime="{{ \Carbon\CarbonImmutable::createFromTimestampUTC((int) $death->time)->toIso8601String() }}">
                            {{ $localeFormatter->dateTime(\Carbon\CarbonImmutable::createFromTimestampUTC((int) $death->time)) }}
                        </time>
                    </li>
                @empty
                    <li>{{ __('community.profile.no_deaths') }}</li>
                @endforelse
            </ul>
            <a class="button button-secondary" href="{{ route('game.deaths.index') }}">{{ __('community.deaths.title') }}</a>
        </section>

        <section class="card" aria-labelledby="character-kills-heading">
            <h2 id="character-kills-heading">{{ __('community.profile.kill_statistics') }}</h2>
            <p>{{ trans_choice('community.profile.player_kills', $kills['count'], ['count' => $localeFormatter->number($kills['count'])]) }}</p>
            @if ($kills['recent']->isNotEmpty())
                <h3>{{ __('community.profile.recent_victims') }}</h3>
                <ul class="community-event-list">
                    @foreach ($kills['recent'] as $kill)
                        <li>
                            <a href="{{ route('game.characters.show', ['name' => $kill->victim_name]) }}">{{ $kill->victim_name }}</a>
                            <span>{{ __('community.profile.killed_at_level', ['level' => $localeFormatter->number((int) $kill->victim_level)]) }}</span>
                        </li>
                    @endforeach
                </ul>
            @endif
        </section>
    </div>

    <section class="card" aria-labelledby="character-account-heading">
        <h2 id="character-account-heading">{{ __('community.profile.account_characters') }}</h2>
        @if ($related_characters->isEmpty())
            <p class="muted">{{ __('community.profile.account_private') }}</p>
        @else
            <ul class="community-character-list">
                @foreach ($related_characters as $related)
                    <li>
                        <a href="{{ route('game.characters.show', ['name' => $related->name]) }}">{{ $related->name }}</a>
                        <span>{{ $localeFormatter->number((int) $related->level) }} · {{ $characterPresentation->vocationName((int) $related->vocation) }}</span>
                    </li>
                @endforeach
            </ul>
        @endif
    </section>

    <section class="card" aria-labelledby="community-policy-heading">
        <h2 id="community-policy-heading">{{ __('community.policy.title') }}</h2>
        <ul>
            <li>{{ __('community.policy.achievements') }}</li>
            <li>{{ __('community.policy.world_transfer') }}</li>
            <li>{{ __('community.policy.public_enforcement') }}</li>
        </ul>
    </section>
@endsection
