@extends('game.layout')

@section('title', __('public.game.guild_directory'))
@section('page-class', 'page-shell-wide')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <div class="page-header">
        <p class="eyebrow">{{ __('public.game.community') }}</p>
        <h1>{{ __('public.game.guild_directory') }}</h1>
        <p class="muted">{{ __('public.game.guild_directory_description') }}</p>
    </div>

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
                        <td>{{ $localeFormatter->number($guild->active_member_count) }}</td>
                    </tr>
                @empty
                    <tr><td colspan="2">{{ __('public.game.no_guilds') }}</td></tr>
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
@endsection
