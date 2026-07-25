@extends('game.layout')

@section('title', __('public.game.online_title'))

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <div class="page-header">
        <p class="eyebrow">{{ __('public.game.live_world') }}</p>
        <h1>{{ __('public.game.online_title') }}</h1>
        <p class="muted">{{ __('public.game.online_description') }}</p>
    </div>

    <div class="card-grid">
        @forelse ($characters as $character)
            <article class="card">
                <h2><a href="{{ route('game.characters.show', ['name' => $character->name]) }}">{{ $character->name }}</a></h2>
                <dl>
                    <dt>{{ __('public.game.level') }}:</dt><dd>{{ $localeFormatter->number($character->level) }}</dd>
                    <dt>{{ __('public.game.vocation') }}:</dt><dd>{{ $localeFormatter->number($character->vocation) }}</dd>
                    <dt>{{ __('public.game.channel') }}:</dt><dd>{{ $character->channel_name }} ({{ __('public.game.channel_id') }} {{ $localeFormatter->number($character->channel_id) }})</dd>
                </dl>
            </article>
        @empty
            <div class="empty-state">{{ __('public.game.no_online') }}</div>
        @endforelse
    </div>

    @if ($characters->hasPages())
        <nav class="pagination" aria-label="{{ __('public.game.online_pages') }}">
            @if ($characters->onFirstPage())
                <span class="muted">{{ __('public.pagination.previous') }}</span>
            @else
                <a href="{{ $characters->previousPageUrl() }}">{{ __('public.pagination.previous') }}</a>
            @endif
            <span>{{ __('public.pagination.page_of', ['current' => $localeFormatter->number($characters->currentPage()), 'last' => $localeFormatter->number($characters->lastPage())]) }}</span>
            @if ($characters->hasMorePages())
                <a href="{{ $characters->nextPageUrl() }}">{{ __('public.pagination.next') }}</a>
            @else
                <span class="muted">{{ __('public.pagination.next') }}</span>
            @endif
        </nav>
    @endif
@endsection
