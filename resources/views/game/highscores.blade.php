@extends('game.layout')

@section('title', __('public.game.highscores_title'))
@section('page-class', 'page-shell-wide')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    @inject('characterPresentation', 'App\PublicGameData\CharacterPresentation')
    <div class="page-header">
        <p class="eyebrow">{{ __('public.game.rankings') }}</p>
        <h1>{{ __('public.game.highscores_title') }}</h1>
        <p class="muted">{{ __('public.game.highscores_description') }}</p>
    </div>

    <div class="card">
        <div class="table-region" tabindex="0" aria-label="{{ __('public.game.highscores_table') }}">
            <table class="table-compact">
                <thead>
                <tr>
                    <th scope="col">{{ __('public.game.rank') }}</th>
                    <th scope="col">{{ __('public.game.character') }}</th>
                    <th scope="col">{{ __('public.game.level') }}</th>
                    <th scope="col">{{ __('public.game.vocation') }}</th>
                </tr>
                </thead>
                <tbody>
                @forelse ($players as $player)
                    <tr>
                        <td>{{ $localeFormatter->number(($players->firstItem() ?? 1) + $loop->index) }}</td>
                        <td><a href="{{ route('game.characters.show', ['name' => $player->name]) }}">{{ $player->name }}</a></td>
                        <td>{{ $localeFormatter->number($player->level) }}</td>
                        <td>{{ $characterPresentation->vocationName((int) $player->vocation) }}</td>
                    </tr>
                @empty
                    <tr><td colspan="4">{{ __('public.game.no_characters') }}</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>

        @if ($players->hasPages())
            <nav class="pagination" aria-label="{{ __('public.game.highscore_pages') }}">
                @if ($players->onFirstPage())
                    <span class="muted">{{ __('public.pagination.previous') }}</span>
                @else
                    <a href="{{ $players->previousPageUrl() }}">{{ __('public.pagination.previous') }}</a>
                @endif
                <span>{{ __('public.pagination.page_of', ['current' => $localeFormatter->number($players->currentPage()), 'last' => $localeFormatter->number($players->lastPage())]) }}</span>
                @if ($players->hasMorePages())
                    <a href="{{ $players->nextPageUrl() }}">{{ __('public.pagination.next') }}</a>
                @else
                    <span class="muted">{{ __('public.pagination.next') }}</span>
                @endif
            </nav>
        @endif
    </div>
@endsection
