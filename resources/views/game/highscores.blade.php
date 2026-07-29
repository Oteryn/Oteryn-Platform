@extends('game.layout')

@section('title', __('community.highscores.title'))
@section('page-class', 'page-shell-wide community-page')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    @inject('characterPresentation', 'App\PublicGameData\CharacterPresentation')

    <div class="page-header">
        <p class="eyebrow">{{ __('public.game.rankings') }}</p>
        <h1>{{ __('community.highscores.title') }}</h1>
        <p class="muted">{{ __('community.highscores.description') }}</p>
    </div>

    <form class="card community-filter" method="get" action="{{ route('game.highscores.index') }}" aria-label="{{ __('community.highscores.filters') }}">
        <div class="form-grid">
            <label>
                <span>{{ __('community.highscores.category') }}</span>
                <select name="category">
                    @foreach ($categories as $categoryOption)
                        <option value="{{ $categoryOption }}" @selected($category === $categoryOption)>
                            {{ __('community.highscores.categories.'.$categoryOption) }}
                        </option>
                    @endforeach
                </select>
            </label>
            <label>
                <span>{{ __('community.highscores.vocation') }}</span>
                <select name="vocation">
                    <option value="">{{ __('community.highscores.all_vocations') }}</option>
                    @foreach ($vocations as $vocationOption)
                        <option value="{{ $vocationOption }}" @selected($vocation === $vocationOption)>
                            {{ $characterPresentation->vocationName($vocationOption) }}
                        </option>
                    @endforeach
                </select>
            </label>
        </div>
        <input type="hidden" name="scope" value="global">
        <div class="action-row">
            <button class="button" type="submit">{{ __('community.highscores.apply') }}</button>
        </div>
        <p class="muted community-policy-note">
            <strong>{{ __('community.highscores.scope') }}:</strong>
            {{ __('community.highscores.global_scope') }}. {{ __('community.highscores.global_scope_help') }}
        </p>
    </form>

    <div class="card">
        <div class="table-region" tabindex="0" aria-label="{{ __('community.highscores.title') }}">
            <table class="table-compact">
                <thead>
                <tr>
                    <th scope="col">{{ __('public.game.rank') }}</th>
                    <th scope="col">{{ __('public.game.character') }}</th>
                    <th scope="col">{{ __('community.highscores.score') }}</th>
                    <th scope="col">{{ __('public.game.level') }}</th>
                    <th scope="col">{{ __('public.game.vocation') }}</th>
                </tr>
                </thead>
                <tbody>
                @forelse ($players as $player)
                    <tr>
                        <td>{{ $localeFormatter->number(($players->firstItem() ?? 1) + $loop->index) }}</td>
                        <td><a href="{{ route('game.characters.show', ['name' => $player->name]) }}">{{ $player->name }}</a></td>
                        <td>{{ $localeFormatter->number((int) $player->score) }}</td>
                        <td>{{ $localeFormatter->number((int) $player->level) }}</td>
                        <td>{{ $characterPresentation->vocationName((int) $player->vocation) }}</td>
                    </tr>
                @empty
                    <tr><td colspan="5">{{ __('community.highscores.empty') }}</td></tr>
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
