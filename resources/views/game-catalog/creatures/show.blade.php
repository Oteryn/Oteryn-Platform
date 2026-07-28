@extends('game.layout')

@section('title', $creature->name)
@section('description', $creature->description ?? __('game_catalog.creatures.intro'))
@section('page-class', 'game-catalog-page')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    @php
        $elements = json_decode($creature->elements, true) ?? [];
        $immunities = json_decode($creature->immunities, true) ?? [];
        $attacks = json_decode($creature->attacks, true) ?? [];
        $defenses = json_decode($creature->defenses, true) ?? [];
    @endphp

    <nav class="catalog-breadcrumbs" aria-label="{{ __('game_catalog.breadcrumbs') }}">
        <a href="{{ route('game-catalog.index') }}">{{ __('game_catalog.title') }}</a>
        <span aria-hidden="true">/</span>
        <a href="{{ route('game-catalog.creatures.index') }}">{{ __('game_catalog.creatures.title') }}</a>
        <span aria-hidden="true">/</span>
        <span aria-current="page">{{ $creature->name }}</span>
    </nav>

    <header class="page-header catalog-detail-header">
        <div class="catalog-image-placeholder catalog-image-large" aria-hidden="true"><span>?</span></div>
        <div>
            <p class="eyebrow">{{ $creature->bestiary_class ?? __('game_catalog.unknown') }}@if($creature->is_boss) · {{ __('game_catalog.creatures.boss') }}@endif</p>
            <h1>{{ $creature->name }}</h1>
            @if ($creature->description !== null)
                <p>{{ $creature->description }}</p>
            @endif
            <p class="muted">{{ __('game_catalog.creatures.no_location_claim') }}</p>
        </div>
    </header>

    @include('game-catalog.partials.context')

    <section aria-labelledby="creature-runtime-values">
        <h2 id="creature-runtime-values">{{ __('game_catalog.creatures.runtime_values') }}</h2>
        <dl class="catalog-stat-grid">
            <div><dt>{{ __('game_catalog.creatures.race_id') }}</dt><dd>{{ $creature->race_id ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.look_type') }}</dt><dd>{{ $creature->look_type ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.health') }}</dt><dd>{{ $creature->health }} / {{ $creature->max_health }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.experience') }}</dt><dd>{{ $creature->experience }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.speed') }}</dt><dd>{{ $creature->speed }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.armor') }}</dt><dd>{{ $creature->armor }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.defense') }}</dt><dd>{{ $creature->defense }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.mitigation') }}</dt><dd>{{ $creature->mitigation ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.boss_status') }}</dt><dd>{{ $creature->is_boss ? __('game_catalog.yes') : __('game_catalog.no') }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.reward_boss') }}</dt><dd>{{ $creature->is_reward_boss ? __('game_catalog.yes') : __('game_catalog.no') }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.bestiary_class') }}</dt><dd>{{ $creature->bestiary_class ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.bestiary_race') }}</dt><dd>{{ $creature->bestiary_race ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.bestiary_to_kill') }}</dt><dd>{{ $creature->bestiary_to_kill ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.charm_points') }}</dt><dd>{{ $creature->charm_points ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.creatures.availability') }}</dt><dd>{{ $creature->availability }}</dd></div>
        </dl>
    </section>

    <div class="catalog-two-column">
        <section aria-labelledby="creature-elements">
            <h2 id="creature-elements">{{ __('game_catalog.creatures.elements') }}</h2>
            @if ($elements === [])<p>{{ __('game_catalog.unknown') }}</p>@else<pre class="catalog-json">{{ json_encode($elements, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE) }}</pre>@endif
        </section>
        <section aria-labelledby="creature-immunities">
            <h2 id="creature-immunities">{{ __('game_catalog.creatures.immunities') }}</h2>
            @if ($immunities === [])<p>{{ __('game_catalog.unknown') }}</p>@else<ul>@foreach($immunities as $immunity)<li>{{ is_scalar($immunity) ? $immunity : json_encode($immunity, JSON_UNESCAPED_SLASHES) }}</li>@endforeach</ul>@endif
        </section>
        <section aria-labelledby="creature-attacks">
            <h2 id="creature-attacks">{{ __('game_catalog.creatures.attacks') }}</h2>
            @if ($attacks === [])<p>{{ __('game_catalog.unknown') }}</p>@else<pre class="catalog-json">{{ json_encode($attacks, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE) }}</pre>@endif
        </section>
        <section aria-labelledby="creature-defenses">
            <h2 id="creature-defenses">{{ __('game_catalog.creatures.defenses') }}</h2>
            @if ($defenses === [])<p>{{ __('game_catalog.unknown') }}</p>@else<pre class="catalog-json">{{ json_encode($defenses, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE) }}</pre>@endif
        </section>
    </div>

    <section aria-labelledby="creature-loot">
        <h2 id="creature-loot">{{ __('game_catalog.creatures.loot') }}</h2>
        @if ($loot === [])
            <div class="empty-state" role="status"><p>{{ __('game_catalog.creatures.no_visible_loot') }}</p></div>
        @else
            <div class="catalog-table-wrap">
                <table class="catalog-table">
                    <thead><tr><th>{{ __('game_catalog.items.name') }}</th><th>{{ __('game_catalog.items.category') }}</th><th>{{ __('game_catalog.loot.chance') }}</th><th>{{ __('game_catalog.loot.count') }}</th><th>{{ __('game_catalog.loot.container') }}</th></tr></thead>
                    <tbody>
                        @foreach ($loot as $drop)
                            <tr>
                                <td><a href="{{ route('game-catalog.items.show', ['slug' => \Illuminate\Support\Str::after($drop->canonical_key, ':')]) }}">{{ $drop->name }}</a></td>
                                <td>{{ $drop->category }}</td>
                                <td><code>{{ $drop->chance_numerator }}/{{ $drop->chance_denominator }}</code></td>
                                <td>{{ $drop->minimum_count }}–{{ $drop->maximum_count }}</td>
                                <td>{{ $drop->container_path ?? __('game_catalog.unknown') }}</td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </section>
@endsection
