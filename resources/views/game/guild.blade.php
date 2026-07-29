@extends('game.layout')

@section('title', $guild->name)
@section('page-class', 'page-shell-wide community-page')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')

    <div class="page-header">
        <p class="eyebrow">{{ __('public.game.guild') }}</p>
        <h1>{{ $guild->name }}</h1>
    </div>

    <dl class="card stat-grid">
        <div class="stat">
            <dt>{{ __('public.game.guild_level') }}</dt>
            <dd>{{ $localeFormatter->number((int) $guild->level) }}</dd>
        </div>
        <div class="stat">
            <dt>{{ __('public.game.points') }}</dt>
            <dd>{{ $localeFormatter->number((int) $guild->points) }}</dd>
        </div>
        <div class="stat">
            <dt>{{ __('community.guilds.owner') }}</dt>
            <dd>
                @if (is_string($guild->owner_name) && $guild->owner_name !== '')
                    <a href="{{ route('game.characters.show', ['name' => $guild->owner_name]) }}">{{ $guild->owner_name }}</a>
                @else
                    —
                @endif
            </dd>
        </div>
    </dl>

    @if ($guild->motd !== '')
        <section class="card" aria-labelledby="guild-message-heading">
            <h2 id="guild-message-heading">{{ __('public.game.message') }}</h2>
            <p class="community-preserved-text">{{ $guild->motd }}</p>
        </section>
    @endif

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
                            <td>{{ $localeFormatter->number((int) $member->level) }}</td>
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

    <section class="card" aria-labelledby="guild-read-only-heading">
        <h2 id="guild-read-only-heading">{{ __('community.policy.title') }}</h2>
        <p>{{ __('community.guilds.read_only_policy') }}</p>
    </section>
@endsection
