@extends('game.layout')

@section('title', $creature->name)
@section('description', $creature->description ?? $creature->name)
@section('page-class', 'game-catalog-page catalog-detail')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <header class="page-header catalog-hero">
        <p class="eyebrow"><a href="{{ route('game-catalog.creatures.index') }}">{{ __('game_catalog.back_to_creatures') }}</a></p>
        <h1>{{ $creature->name }}</h1>
        <div class="catalog-meta">
            @if ($creature->bestiaryClass !== null)<span class="catalog-chip">{{ $creature->bestiaryClass }}</span>@endif
            @if ($creature->boss)<span class="catalog-chip">{{ __('game_catalog.boss') }}</span>@endif
        </div>
        @if ($creature->description !== null && $creature->description !== '')<p>{{ $creature->description }}</p>@endif
    </header>

    <section aria-labelledby="creature-attributes">
        <h2 id="creature-attributes">{{ __('game_catalog.creatures') }}</h2>
        <dl class="catalog-detail-list">
            <div><dt>{{ __('game_catalog.health') }}</dt><dd>{{ $creature->health }}</dd></div>
            <div><dt>{{ __('game_catalog.max_health') }}</dt><dd>{{ $creature->maxHealth }}</dd></div>
            <div><dt>{{ __('game_catalog.experience') }}</dt><dd>{{ $creature->experience }}</dd></div>
            <div><dt>{{ __('game_catalog.speed') }}</dt><dd>{{ $creature->speed }}</dd></div>
            <div><dt>{{ __('game_catalog.armor') }}</dt><dd>{{ $creature->armor }}</dd></div>
            <div><dt>{{ __('game_catalog.defense') }}</dt><dd>{{ $creature->defense }}</dd></div>
            @if ($creature->mitigation !== null)<div><dt>{{ __('game_catalog.mitigation') }}</dt><dd>{{ $creature->mitigation }}</dd></div>@endif
            <div><dt>{{ __('game_catalog.boss') }}</dt><dd>{{ $creature->boss ? __('game_catalog.yes') : __('game_catalog.no') }}</dd></div>
            <div><dt>{{ __('game_catalog.reward_boss') }}</dt><dd>{{ $creature->rewardBoss ? __('game_catalog.yes') : __('game_catalog.no') }}</dd></div>
            @if ($creature->bestiaryClass !== null)<div><dt>{{ __('game_catalog.bestiary_class') }}</dt><dd>{{ $creature->bestiaryClass }}</dd></div>@endif
            @if ($creature->bestiaryRace !== null)<div><dt>{{ __('game_catalog.bestiary_race') }}</dt><dd>{{ $creature->bestiaryRace }}</dd></div>@endif
            @if ($creature->bestiaryOccurrence !== null)<div><dt>{{ __('game_catalog.bestiary_occurrence') }}</dt><dd>{{ $creature->bestiaryOccurrence }}</dd></div>@endif
            @if ($creature->bestiaryToKill !== null)<div><dt>{{ __('game_catalog.bestiary_to_kill') }}</dt><dd>{{ $creature->bestiaryToKill }}</dd></div>@endif
            @if ($creature->charmPoints !== null)<div><dt>{{ __('game_catalog.charm_points') }}</dt><dd>{{ $creature->charmPoints }}</dd></div>@endif
        </dl>
    </section>

    <section aria-labelledby="creature-loot">
        <h2 id="creature-loot">{{ __('game_catalog.loot') }}</h2>
        @if ($creature->loot === [])
            <div class="empty-state" role="status"><strong>{{ __('game_catalog.no_loot') }}</strong></div>
        @else
            <ul class="catalog-relation-list">
                @foreach ($creature->loot as $loot)
                    <li>
                        <a href="{{ route('game-catalog.items.show', ['slug' => $loot->slug]) }}">{{ $loot->name }}</a>
                        <span>{{ __('game_catalog.chance') }}: {{ __('game_catalog.chance_ratio', ['numerator' => $loot->chanceNumerator, 'denominator' => $loot->chanceDenominator]) }}</span>
                        <span>{{ __('game_catalog.count') }}: {{ $loot->minimumCount === $loot->maximumCount ? __('game_catalog.count_single', ['count' => $loot->minimumCount]) : __('game_catalog.count_range', ['minimum' => $loot->minimumCount, 'maximum' => $loot->maximumCount]) }}</span>
                    </li>
                @endforeach
            </ul>
        @endif
    </section>
@endsection
