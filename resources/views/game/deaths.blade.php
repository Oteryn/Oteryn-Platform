@extends('game.layout')

@section('title', __('community.deaths.title'))
@section('page-class', 'page-shell-wide community-page')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')

    <header class="page-header">
        <p class="eyebrow">{{ __('public.game.community') }}</p>
        <h1>{{ __('community.deaths.title') }}</h1>
        <p class="muted">{{ __('community.deaths.description') }}</p>
    </header>

    <section class="card">
        <div class="table-region" tabindex="0" aria-label="{{ __('community.deaths.table') }}">
            <table class="table-compact">
                <thead>
                <tr>
                    <th scope="col">{{ __('community.deaths.character') }}</th>
                    <th scope="col">{{ __('community.deaths.level') }}</th>
                    <th scope="col">{{ __('community.deaths.cause') }}</th>
                    <th scope="col">{{ __('community.deaths.time') }}</th>
                </tr>
                </thead>
                <tbody>
                @forelse ($deaths as $death)
                    @php($occurredAt = \Carbon\CarbonImmutable::createFromTimestampUTC((int) $death->time))
                    <tr>
                        <td><a href="{{ route('game.characters.show', ['name' => $death->player_name]) }}">{{ $death->player_name }}</a></td>
                        <td>{{ $localeFormatter->number((int) $death->level) }}</td>
                        <td>{{ $death->killed_by }}</td>
                        <td><time datetime="{{ $occurredAt->toIso8601String() }}">{{ $localeFormatter->dateTime($occurredAt) }}</time></td>
                    </tr>
                @empty
                    <tr><td colspan="4">{{ __('community.deaths.empty') }}</td></tr>
                @endforelse
                </tbody>
            </table>
        </div>

        @if ($deaths->hasPages())
            <nav class="pagination" aria-label="{{ __('community.deaths.pages') }}">
                @if ($deaths->onFirstPage())
                    <span class="muted">{{ __('public.pagination.previous') }}</span>
                @else
                    <a href="{{ $deaths->previousPageUrl() }}">{{ __('public.pagination.previous') }}</a>
                @endif
                <span>{{ __('public.pagination.page_of', ['current' => $localeFormatter->number($deaths->currentPage()), 'last' => $localeFormatter->number($deaths->lastPage())]) }}</span>
                @if ($deaths->hasMorePages())
                    <a href="{{ $deaths->nextPageUrl() }}">{{ __('public.pagination.next') }}</a>
                @else
                    <span class="muted">{{ __('public.pagination.next') }}</span>
                @endif
            </nav>
        @endif
    </section>

    <section class="card" aria-labelledby="death-policy-heading">
        <h2 id="death-policy-heading">{{ __('community.policy.title') }}</h2>
        <p>{{ __('community.policy.world_transfer') }}</p>
        <p>{{ __('community.policy.polls') }}</p>
    </section>
@endsection
