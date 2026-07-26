@extends('game.layout')

@section('title', __('public.wiki.search_title'))
@section('robots', 'noindex,follow')
@section('page-class', 'wiki-page')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/wiki.css') }}">
@endpush

@section('content')
    <header class="page-header">
        <p class="eyebrow">{{ __('public.wiki.eyebrow') }}</p>
        <h1>{{ __('public.wiki.search_title') }}</h1>
        @include('wiki.partials.search-form', ['query' => $results->query])
    </header>

    @if ($searchError !== null)
        <div class="notice notice-error" role="alert">{{ $searchError }}</div>
    @elseif ($results->query === '')
        <div class="empty-state" role="status">
            <strong>{{ __('public.wiki.search_ready') }}</strong>
            <p>{{ __('public.wiki.search_ready_help') }}</p>
        </div>
    @elseif ($results->items === [])
        <div class="empty-state" role="status">
            <strong>{{ __('public.wiki.search_empty') }}</strong>
            <p>{{ __('public.wiki.search_empty_help') }}</p>
        </div>
    @else
        <p class="muted">{{ trans_choice('public.wiki.result_count', $results->total, ['count' => $results->total]) }}</p>
        <div class="card-grid">
            @foreach ($results->items as $result)
                <article class="card">
                    <h2><a href="{{ route('wiki.article', ['slug' => $result->slug]) }}">{{ $result->title }}</a></h2>
                    <p>{{ $result->summary }}</p>
                </article>
            @endforeach
        </div>

        @if ($results->lastPage() > 1)
            <nav class="pagination" aria-label="{{ __('public.wiki.search_pages') }}">
                @if ($results->hasPreviousPage())
                    <a href="{{ route('wiki.search', ['q' => $results->query, 'page' => $results->page - 1]) }}" rel="prev">{{ __('public.pagination.previous') }}</a>
                @endif
                <span>{{ __('public.pagination.page_of', ['current' => $results->page, 'last' => $results->lastPage()]) }}</span>
                @if ($results->hasNextPage())
                    <a href="{{ route('wiki.search', ['q' => $results->query, 'page' => $results->page + 1]) }}" rel="next">{{ __('public.pagination.next') }}</a>
                @endif
            </nav>
        @endif
    @endif
@endsection
