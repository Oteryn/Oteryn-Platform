@extends('game.layout')

@section('title', $guild->name)
@section('page-class', 'page-shell-wide')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <div class="page-header">
        <p class="eyebrow">{{ __('public.game.guild') }}</p>
        <h1>{{ $guild->name }}</h1>
    </div>

    <div class="card stat-grid">
        <div class="stat"><strong>{{ __('public.game.guild_level') }}</strong><br>{{ $localeFormatter->number($guild->level) }}</div>
        <div class="stat"><strong>{{ __('public.game.points') }}</strong><br>{{ $localeFormatter->number($guild->points) }}</div>
        <div class="stat"><strong>{{ __('public.game.residence_id') }}</strong><br>{{ $localeFormatter->number($guild->residence) }}</div>
        <div class="stat"><strong>{{ __('public.game.owner_player_id') }}</strong><br>{{ $localeFormatter->number($guild->ownerid) }}</div>
        @if ($guild->motd !== '')
            <div class="stat"><strong>{{ __('public.game.message') }}</strong><br>{{ $guild->motd }}</div>
        @endif
    </div>

    <section aria-labelledby="guild-members-heading">
        <div class="page-header">
            <h2 id="guild-members-heading">{{ __('public.game.members') }}</h2>
        </div>
        <div class="card">
            <div class="table-region" tabindex="0" aria-label="{{ __('public.game.guild_members_table') }}">
                <table>
                    <thead>
                    <tr>
                        <th scope="col">{{ __('public.game.character') }}</th>
                        <th scope="col">{{ __('public.game.rank_name') }}</th>
                        <th scope="col">{{ __('public.game.nickname') }}</th>
                        <th scope="col">{{ __('public.game.level') }}</th>
                    </tr>
                    </thead>
                    <tbody>
                    @forelse ($members as $member)
                        <tr>
                            <td><a href="{{ route('game.characters.show', ['name' => $member->name]) }}">{{ $member->name }}</a></td>
                            <td>{{ $member->rank_name }}</td>
                            <td>{{ $member->nick ?: '—' }}</td>
                            <td>{{ $localeFormatter->number($member->level) }}</td>
                        </tr>
                    @empty
                        <tr><td colspan="4">{{ __('public.game.no_members') }}</td></tr>
                    @endforelse
                    </tbody>
                </table>
            </div>

            @if ($members->hasPages())
                <nav class="pagination" aria-label="{{ __('public.game.guild_member_pages') }}">
                    @if ($members->onFirstPage())
                        <span class="muted">{{ __('public.pagination.previous') }}</span>
                    @else
                        <a href="{{ $members->previousPageUrl() }}">{{ __('public.pagination.previous') }}</a>
                    @endif
                    <span>{{ __('public.pagination.page_of', ['current' => $localeFormatter->number($members->currentPage()), 'last' => $localeFormatter->number($members->lastPage())]) }}</span>
                    @if ($members->hasMorePages())
                        <a href="{{ $members->nextPageUrl() }}">{{ __('public.pagination.next') }}</a>
                    @else
                        <span class="muted">{{ __('public.pagination.next') }}</span>
                    @endif
                </nav>
            @endif
        </div>
    </section>
@endsection
