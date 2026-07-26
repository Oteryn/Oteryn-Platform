@extends('game.layout')

@section('title', $category->name)
@section('description', $category->description ?? __('public.seo.default_description', ['title' => $category->name]))
@section('page-class', 'wiki-page')

@push('head')
    @if ($category->description !== null)
        <meta name="description" content="{{ \Illuminate\Support\Str::limit(strip_tags($category->description), 155) }}">
    @endif
@endpush
@push('styles')
    <link rel="stylesheet" href="{{ asset('css/wiki.css') }}">
@endpush

@section('content')
    @include('wiki.partials.breadcrumbs', ['breadcrumbs' => $category->breadcrumbs])

    <header class="page-header">
        <p class="eyebrow">{{ __('public.wiki.category') }}</p>
        <h1>{{ $category->name }}</h1>
        @if ($category->description !== null && $category->description !== '')
            <p class="muted">{{ $category->description }}</p>
        @endif
    </header>

    @if ($category->children !== [])
        <section aria-labelledby="wiki-subcategories">
            <div class="section-heading">
                <h2 id="wiki-subcategories">{{ __('public.wiki.subcategories') }}</h2>
            </div>
            <div class="card-grid wiki-category-grid">
                @foreach ($category->children as $child)
                    <article class="card">
                        <h3><a href="{{ route('wiki.category', ['slug' => $child->slug]) }}">{{ $child->name }}</a></h3>
                        @if ($child->description !== null && $child->description !== '')
                            <p>{{ $child->description }}</p>
                        @endif
                        <p class="muted">{{ trans_choice('public.wiki.article_count', $child->articleCount, ['count' => $child->articleCount]) }}</p>
                    </article>
                @endforeach
            </div>
        </section>
    @endif

    <section aria-labelledby="wiki-category-articles">
        <div class="section-heading">
            <h2 id="wiki-category-articles">{{ __('public.wiki.articles') }}</h2>
        </div>
        @if ($category->articles === [])
            <div class="empty-state" role="status">
                <strong>{{ __('public.wiki.category_empty') }}</strong>
                <p>{{ __('public.wiki.category_empty_help') }}</p>
            </div>
        @else
            <div class="card-grid">
                @foreach ($category->articles as $articleCard)
                    @include('wiki.partials.article-card', ['articleCard' => $articleCard])
                @endforeach
            </div>
        @endif
    </section>
@endsection
