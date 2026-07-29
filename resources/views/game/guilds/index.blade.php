@extends('game.layout')

@section('title', __('community.guilds.title'))
@section('page-class', 'page-shell-wide community-page')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')

    <div class="page-header">
        <p class="eyebrow">{{ __('public.game.community') }}</p>
        <h1>{{ __('community.guilds.title') }}</h1>
        <p class="muted">{{ __('community.guilds.description') }}</p>
    </div>

    <form class="card community-filter" method="get" action="{{ route('game.guilds.index') }}" role="search">
        <label>
            <span>{{ __('community.guilds.search_label') }}</span>
            <input name="q" type="search" maxlength="80" value="{{ $search ?? '' }}" placeholder="{{ __('community.guilds.search_placeholder') }}">
        </label>
        <div class="action-row">
            <button class="button" type="submit">{{ __('community.guilds.search') }}</button>
            @if ($search !== null)
                <a class="button button-secondary" href="{{ route('game.guilds.index') }}">{{ __('community.guilds.clear') }}</a>
            @endif
        </div>
    </form>

    <div class="card">
        <div class="table-region" tabindex="0" aria-label="{{ __('public.game.guild_directory_table') }}">
            <table class="table-compact">
                <thead>
                <tr>
                    <th scope="col">{{ __('public.game.guild') }}</th>
                    <th scope="col">{{ __('public.game.active_members') }}</th>
                </tr>
                </thead>
                <tbody>
                @forelse ($guilds as $guild)
                    <tr>
                        <td><a href="{{ route('game.guilds.show', ['name' => $guild->name]) }}">{{ $guild->name }}</a></td>
                        <td>{{ $localeFormatter->number((int) $guild->active_member_count) }}</td>
                    </tr>
                @empty
                    <tr><td colspan="2">{{ $search === null ? __('public.game.no_guilds') : __('community.guilds.empty') }}</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>

        @if ($guilds->hasPages())
            <nav class="pagination" aria-label="{{ __('public.game.guild_pages') }}">
                @if ($guilds->onFirstPage())
                    <span class="muted">{{ __('public.pagination.previous') }}</span>
                @else
                    <a href="{{ $guilds->previousPageUrl() }}">{{ __('public.pagination.previous') }}</a>
                @endif
                <span>{{ __('public.pagination.page_of', ['current' => $localeFormatter->number($guilds->currentPage()), 'last' => $localeFormatter->number($guilds->lastPage())]) }}</span>
                @if ($guilds->hasMorePages())
                    <a href="{{ $guilds->nextPageUrl() }}">{{ __('public.pagination.next') }}</a>
                @else
                    <span class="muted">{{ __('public.pagination.next') }}</span>
                @endif
            </nav>
        @endif
    </div>

    <section class="card" aria-labelledby="guild-policy-heading">
        <h2 id="guild-policy-heading">{{ __('community.policy.title') }}</h2>
        <p>{{ __('community.guilds.read_only_policy') }}</p>
    </section>
@endsection
