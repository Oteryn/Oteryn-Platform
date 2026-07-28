@extends('game.layout')

@section('title', $item->name)
@section('description', $item->description ?? __('game_catalog.items.intro'))
@section('page-class', 'game-catalog-page')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    @php
        $vocations = $item->vocations === null ? null : json_decode($item->vocations, true);
        $attributes = json_decode($item->attributes, true) ?? [];
        $hands = isset($attributes['hands']) && is_int($attributes['hands']) ? $attributes['hands'] : null;
    @endphp

    <nav class="catalog-breadcrumbs" aria-label="{{ __('game_catalog.breadcrumbs') }}">
        <a href="{{ route('game-catalog.index') }}">{{ __('game_catalog.title') }}</a>
        <span aria-hidden="true">/</span>
        <a href="{{ route('game-catalog.items.index') }}">{{ __('game_catalog.items.title') }}</a>
        <span aria-hidden="true">/</span>
        <span aria-current="page">{{ $item->name }}</span>
    </nav>

    <header class="page-header catalog-detail-header">
        <div class="catalog-image-placeholder catalog-image-large" aria-hidden="true"><span>{{ $item->image_key ?? '?' }}</span></div>
        <div>
            <p class="eyebrow">{{ $item->category }}@if($item->weapon_type !== null) · {{ $item->weapon_type }}@endif</p>
            <h1>{{ $item->name }}</h1>
            @if ($item->description !== null)
                <p>{{ $item->description }}</p>
            @endif
        </div>
    </header>

    @include('game-catalog.partials.context')

    <section aria-labelledby="item-runtime-values">
        <h2 id="item-runtime-values">{{ __('game_catalog.items.runtime_values') }}</h2>
        <dl class="catalog-stat-grid">
            <div><dt>{{ __('game_catalog.items.server_id') }}</dt><dd>{{ $item->server_id }}</dd></div>
            <div><dt>{{ __('game_catalog.items.client_id') }}</dt><dd>{{ $item->client_id ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.weapon_type') }}</dt><dd>{{ $item->weapon_type ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.hands') }}</dt><dd>{{ $hands ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.attack') }}</dt><dd>{{ $item->attack ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.elemental_attack') }}</dt><dd>@if($item->element_type !== null && $item->element_value !== null){{ $item->element_type }} {{ $item->element_value }}@else{{ __('game_catalog.unknown') }}@endif</dd></div>
            <div><dt>{{ __('game_catalog.items.defense') }}</dt><dd>{{ $item->defense ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.extra_defense') }}</dt><dd>{{ $item->extra_defense ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.armor') }}</dt><dd>{{ $item->armor ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.range') }}</dt><dd>{{ $item->range ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.level') }}</dt><dd>{{ $item->minimum_level ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.vocations') }}</dt><dd>{{ is_array($vocations) && $vocations !== [] ? implode(', ', $vocations) : __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.imbuement_slots') }}</dt><dd>{{ $item->imbuement_slots ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.upgrade_classification') }}</dt><dd>{{ $item->upgrade_classification ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.weight') }}</dt><dd>{{ $item->weight ?? __('game_catalog.unknown') }}</dd></div>
            <div><dt>{{ __('game_catalog.items.availability') }}</dt><dd>{{ $item->availability }}</dd></div>
        </dl>
    </section>

    <section aria-labelledby="item-loot-sources">
        <h2 id="item-loot-sources">{{ __('game_catalog.items.loot_sources') }}</h2>
        @if ($lootSources === [])
            <div class="empty-state" role="status"><p>{{ __('game_catalog.items.no_visible_loot_sources') }}</p></div>
        @else
            <div class="catalog-table-wrap">
                <table class="catalog-table">
                    <thead><tr><th>{{ __('game_catalog.creatures.name') }}</th><th>{{ __('game_catalog.loot.chance') }}</th><th>{{ __('game_catalog.loot.count') }}</th><th>{{ __('game_catalog.loot.container') }}</th></tr></thead>
                    <tbody>
                        @foreach ($lootSources as $source)
                            <tr>
                                <td><a href="{{ route('game-catalog.creatures.show', ['slug' => \Illuminate\Support\Str::after($source->canonical_key, ':')]) }}">{{ $source->name }}</a></td>
                                <td><code>{{ $source->chance_numerator }}/{{ $source->chance_denominator }}</code></td>
                                <td>{{ $source->minimum_count }}–{{ $source->maximum_count }}</td>
                                <td>{{ $source->container_path ?? __('game_catalog.unknown') }}</td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        @endif
    </section>
@endsection
